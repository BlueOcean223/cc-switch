//! Cost Calculator - 计算用量成本
//!
//! 使用高精度 Decimal 类型避免浮点数精度问题。
//!
//! 输入是互不重叠的几桶 token（见 [`TokenUsage`]）：各导入器负责把 CLI 日志的口径
//! 换算成这几桶，这里不再按 app 猜 input 里含不含缓存。

use super::parser::TokenUsage;
use rust_decimal::Decimal;
#[cfg(test)]
use std::str::FromStr;

/// 计价规则的版本，参与定价指纹。改了这里的算法（档位、倍率、缓存写入、调价前单价的
/// 选用），或者入库 token 换算成计价桶的口径（`usage_stats::fresh_input_tokens`）时加一：
/// 已入库的成本是按旧规则算的，下次启动发现指纹变了会按新规则全部重算。调价记录
/// （`price_history`）和定价表的变化不用改它，指纹里已经包含。
pub const PRICING_RULES_VERSION: u32 = 1;

/// 成本明细。各项已经乘过档位倍率，相加等于 `total_cost`。
#[derive(Debug, Clone, PartialEq)]
pub struct CostBreakdown {
    pub input_cost: Decimal,
    pub output_cost: Decimal,
    pub cache_read_cost: Decimal,
    pub cache_creation_cost: Decimal,
    pub total_cost: Decimal,
}

impl CostBreakdown {
    /// 入库用的五项字符串：输入、输出、缓存读、缓存写、合计。
    pub fn to_strings(&self) -> [String; 5] {
        [
            self.input_cost.to_string(),
            self.output_cost.to_string(),
            self.cache_read_cost.to_string(),
            self.cache_creation_cost.to_string(),
            self.total_cost.to_string(),
        ]
    }

    /// 没有定价时入库的五项 "0"。
    pub fn zero_strings() -> [String; 5] {
        std::array::from_fn(|_| "0".to_string())
    }
}

/// 请求的服务档位。OpenAI 的 priority（Codex 的 fast 模式）和 Claude 的 fast 模式
/// 按标准价的固定倍数计费，倍数随模型不同，记在定价表的 `priority_multiplier`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServiceTier {
    #[default]
    Standard,
    Priority,
}

impl ServiceTier {
    /// `proxy_request_logs.service_tier` 列的取值：标准档存空串。
    pub fn as_db_str(self) -> &'static str {
        match self {
            ServiceTier::Standard => "",
            ServiceTier::Priority => "priority",
        }
    }

    pub fn from_db_str(value: &str) -> Self {
        match value {
            "priority" => ServiceTier::Priority,
            _ => ServiceTier::Standard,
        }
    }
}

/// 超长上下文档位：一次请求的提示长度（未命中缓存的输入 + 缓存读 + 缓存写）
/// 超过 `threshold_tokens` 时，整次请求按高价计费。OpenAI（GPT-5.4 起 272K）、
/// Gemini 2.5 Pro 和开了 1M 上下文的 Claude Sonnet 4/4.5（200K）都是这个规则：
/// 输入侧（输入、缓存读、缓存写）乘 `input_multiplier`，输出乘 `output_multiplier`。
/// 千问、豆包等按提示长度分多档（如 32K、128K），超过几档就按阈值最高的那档算。
#[derive(Debug, Clone, PartialEq)]
pub struct LongContextPricing {
    pub threshold_tokens: u64,
    pub input_multiplier: Decimal,
    pub output_multiplier: Decimal,
}

/// 基础四项单价（USD / 百万 token）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BasePrices {
    pub input: Decimal,
    pub output: Decimal,
    pub cache_read: Decimal,
    /// 缓存写入单价（Anthropic 的 5 分钟缓存，或其他厂商唯一的写入价）
    pub cache_creation: Decimal,
}

/// 调价前的单价：`until`（Unix 秒）之前的请求按 `prices` 计价。超长上下文和
/// priority 倍率沿用当前定价。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarlierPrices {
    pub until: i64,
    pub prices: BasePrices,
}

/// 模型定价信息
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPricing {
    /// 当前单价
    pub prices: BasePrices,
    /// 调价前的单价，按 `until` 从早到晚排列
    pub earlier: Vec<EarlierPrices>,
    /// 超长上下文档位，按阈值从低到高排列
    pub long_context_tiers: Vec<LongContextPricing>,
    /// priority / fast 档相对标准价的倍数；1 表示该模型没有这个档位或倍数未知。
    pub priority_multiplier: Decimal,
}

impl ModelPricing {
    /// 请求发生时（Unix 秒）有效的单价。
    pub fn prices_at(&self, created_at: i64) -> BasePrices {
        self.earlier
            .iter()
            .find(|earlier| created_at < earlier.until)
            .map_or(self.prices, |earlier| earlier.prices)
    }

    /// 去掉超长上下文档位。一行用量是多次请求的合计时（Grok Build 按轮记录，
    /// 一轮包含多次模型调用），看不出单次请求的提示长度，按标准价计。
    pub fn without_long_context(self) -> Self {
        Self {
            long_context_tiers: Vec::new(),
            ..self
        }
    }
}

/// Anthropic 1 小时缓存写入的价格是输入价的 2 倍（5 分钟写入是 1.25 倍），
/// 所有 Claude 模型一致，所以不单独存一列。
const ONE_HOUR_CACHE_WRITE_INPUT_MULTIPLIER: u32 = 2;

/// 成本计算器
pub struct CostCalculator;

impl CostCalculator {
    /// 计算一次请求的成本。
    ///
    /// - 单价按请求发生的时间 `created_at`（Unix 秒）取，见 [`ModelPricing::prices_at`]。
    /// - 各桶 token 互不重叠：`input_tokens` 是未命中缓存的输入。
    /// - `cache_creation_tokens` 里有 `cache_creation_1h_tokens` 个按 1 小时缓存计价，
    ///   其余按缓存写入单价。
    /// - 超长上下文档位按整次请求生效，见 [`LongContextPricing`]。
    /// - priority / fast 档在最后乘到每一项上。
    pub fn calculate(
        usage: &TokenUsage,
        pricing: &ModelPricing,
        tier: ServiceTier,
        created_at: i64,
    ) -> CostBreakdown {
        let million = Decimal::from(1_000_000);
        let prices = pricing.prices_at(created_at);

        let prompt_tokens = usage.prompt_tokens();
        let (input_multiplier, output_multiplier) = pricing
            .long_context_tiers
            .iter()
            .rev()
            .find(|lc| prompt_tokens > lc.threshold_tokens)
            .map_or((Decimal::ONE, Decimal::ONE), |lc| {
                (lc.input_multiplier, lc.output_multiplier)
            });
        let tier_multiplier = match tier {
            ServiceTier::Standard => Decimal::ONE,
            ServiceTier::Priority => pricing.priority_multiplier,
        };
        let input_side = input_multiplier * tier_multiplier;
        let output_side = output_multiplier * tier_multiplier;

        let one_hour_tokens = usage
            .cache_creation_1h_tokens
            .min(usage.cache_creation_tokens);
        let five_minute_tokens = usage.cache_creation_tokens - one_hour_tokens;
        let one_hour_rate = prices.input * Decimal::from(ONE_HOUR_CACHE_WRITE_INPUT_MULTIPLIER);

        let input_cost = Decimal::from(usage.input_tokens) * prices.input / million * input_side;
        let output_cost =
            Decimal::from(usage.output_tokens) * prices.output / million * output_side;
        let cache_read_cost =
            Decimal::from(usage.cache_read_tokens) * prices.cache_read / million * input_side;
        let cache_creation_cost = (Decimal::from(five_minute_tokens) * prices.cache_creation
            + Decimal::from(one_hour_tokens) * one_hour_rate)
            / million
            * input_side;

        CostBreakdown {
            input_cost,
            output_cost,
            cache_read_cost,
            cache_creation_cost,
            total_cost: input_cost + output_cost + cache_read_cost + cache_creation_cost,
        }
    }
}

#[cfg(test)]
impl ModelPricing {
    /// 从字符串创建只有基础四项单价的定价（无超长上下文档位、无 priority 倍率）
    pub fn from_strings(
        input: &str,
        output: &str,
        cache_read: &str,
        cache_creation: &str,
    ) -> Result<Self, rust_decimal::Error> {
        Ok(Self {
            prices: BasePrices {
                input: Decimal::from_str(input)?,
                output: Decimal::from_str(output)?,
                cache_read: Decimal::from_str(cache_read)?,
                cache_creation: Decimal::from_str(cache_creation)?,
            },
            earlier: Vec::new(),
            long_context_tiers: Vec::new(),
            priority_multiplier: Decimal::ONE,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 没有调价记录的定价里，请求时间不影响结果
    const AT: i64 = 1_790_000_000;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn usage(input: u32, output: u32, cache_read: u32, cache_creation: u32) -> TokenUsage {
        TokenUsage {
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache_read,
            cache_creation_tokens: cache_creation,
            cache_creation_1h_tokens: 0,
        }
    }

    #[test]
    fn test_cost_calculation() {
        let pricing = ModelPricing::from_strings("3.0", "15.0", "0.3", "3.75").unwrap();
        let cost = CostCalculator::calculate(
            &usage(1000, 500, 200, 100),
            &pricing,
            ServiceTier::Standard,
            AT,
        );

        // input: 1000 * 3.0 / 1M = 0.003
        assert_eq!(cost.input_cost, dec("0.003"));
        // output: 500 * 15.0 / 1M = 0.0075
        assert_eq!(cost.output_cost, dec("0.0075"));
        // cache_read: 200 * 0.3 / 1M = 0.00006
        assert_eq!(cost.cache_read_cost, dec("0.00006"));
        // cache_creation: 100 * 3.75 / 1M = 0.000375
        assert_eq!(cost.cache_creation_cost, dec("0.000375"));
        assert_eq!(cost.total_cost, dec("0.010935"));
    }

    #[test]
    fn one_hour_cache_writes_cost_twice_the_input_price() {
        // claude-opus-4-8：输入 $5，5 分钟写入 $6.25，1 小时写入 = $10
        let pricing = ModelPricing::from_strings("5", "25", "0.5", "6.25").unwrap();
        let mut u = usage(2, 1, 0, 1000);
        u.cache_creation_1h_tokens = 600;
        let cost = CostCalculator::calculate(&u, &pricing, ServiceTier::Standard, AT);
        // 400 × 6.25 / 1M + 600 × 10 / 1M = 0.0025 + 0.006
        assert_eq!(cost.cache_creation_cost, dec("0.0085"));
    }

    #[test]
    fn one_hour_tokens_never_exceed_total_cache_writes() {
        let pricing = ModelPricing::from_strings("5", "25", "0.5", "6.25").unwrap();
        let mut u = usage(0, 0, 0, 100);
        u.cache_creation_1h_tokens = 500;
        let cost = CostCalculator::calculate(&u, &pricing, ServiceTier::Standard, AT);
        assert_eq!(cost.cache_creation_cost, dec("0.001"));
    }

    fn gpt_5_6_sol() -> ModelPricing {
        ModelPricing {
            long_context_tiers: vec![LongContextPricing {
                threshold_tokens: 272_000,
                input_multiplier: dec("2"),
                output_multiplier: dec("1.5"),
            }],
            priority_multiplier: dec("2"),
            ..ModelPricing::from_strings("4", "20", "0.4", "5").unwrap()
        }
    }

    #[test]
    fn long_context_tier_applies_to_the_whole_request() {
        let pricing = gpt_5_6_sol();
        // 提示长度 = 10_000 + 270_000 = 280_000 > 272K
        let cost = CostCalculator::calculate(
            &usage(10_000, 1_000, 270_000, 0),
            &pricing,
            ServiceTier::Standard,
            AT,
        );
        assert_eq!(cost.input_cost, dec("0.08")); // 10K × $8
        assert_eq!(cost.cache_read_cost, dec("0.216")); // 270K × $0.8
        assert_eq!(cost.output_cost, dec("0.03")); // 1K × $30
    }

    #[test]
    fn the_highest_tier_the_prompt_exceeds_applies() {
        // 千问 qwen3-max：32K 以上输入 ×2、输出 ×2，128K 以上 ×2.5
        let pricing = ModelPricing {
            long_context_tiers: vec![
                LongContextPricing {
                    threshold_tokens: 32_000,
                    input_multiplier: dec("2"),
                    output_multiplier: dec("2"),
                },
                LongContextPricing {
                    threshold_tokens: 128_000,
                    input_multiplier: dec("2.5"),
                    output_multiplier: dec("2.5"),
                },
            ],
            ..ModelPricing::from_strings("1.2", "6", "0.24", "0").unwrap()
        };
        let input_cost = |prompt| {
            CostCalculator::calculate(&usage(prompt, 0, 0, 0), &pricing, ServiceTier::Standard, AT)
                .input_cost
        };
        assert_eq!(input_cost(20_000), dec("0.024")); // 20K × $1.2
        assert_eq!(input_cost(100_000), dec("0.24")); // 100K × $2.4
        assert_eq!(input_cost(200_000), dec("0.6")); // 200K × $3
    }

    #[test]
    fn prompt_at_the_threshold_stays_at_standard_price() {
        let pricing = gpt_5_6_sol();
        let cost = CostCalculator::calculate(
            &usage(2_000, 1_000, 270_000, 0),
            &pricing,
            ServiceTier::Standard,
            AT,
        );
        assert_eq!(cost.input_cost, dec("0.008"));
        assert_eq!(cost.output_cost, dec("0.02"));
    }

    #[test]
    fn priority_tier_multiplies_every_bucket() {
        let pricing = gpt_5_6_sol();
        let cost = CostCalculator::calculate(
            &usage(1_000, 1_000, 1_000, 0),
            &pricing,
            ServiceTier::Priority,
            AT,
        );
        assert_eq!(cost.input_cost, dec("0.008"));
        assert_eq!(cost.output_cost, dec("0.04"));
        assert_eq!(cost.cache_read_cost, dec("0.0008"));
        assert_eq!(cost.total_cost, dec("0.0488"));
    }

    #[test]
    fn priority_without_a_known_multiplier_is_billed_at_standard() {
        let pricing = ModelPricing::from_strings("3", "15", "0.3", "3.75").unwrap();
        let standard = CostCalculator::calculate(
            &usage(1_000, 1_000, 0, 0),
            &pricing,
            ServiceTier::Standard,
            AT,
        );
        let priority = CostCalculator::calculate(
            &usage(1_000, 1_000, 0, 0),
            &pricing,
            ServiceTier::Priority,
            AT,
        );
        assert_eq!(standard, priority);
    }

    #[test]
    fn requests_before_a_price_change_use_the_earlier_prices() {
        // gpt-5.6-sol：2026-08-21 前 $5 / $30
        let changed_at = 1_787_295_600;
        let pricing = ModelPricing {
            earlier: vec![EarlierPrices {
                until: changed_at,
                prices: ModelPricing::from_strings("5", "30", "0.5", "6.25")
                    .unwrap()
                    .prices,
            }],
            ..gpt_5_6_sol()
        };
        let u = usage(1_000, 1_000, 0, 0);
        let before = CostCalculator::calculate(&u, &pricing, ServiceTier::Priority, changed_at - 1);
        assert_eq!(before.total_cost, dec("0.07")); // (1K × $5 + 1K × $30) × 2
        let after = CostCalculator::calculate(&u, &pricing, ServiceTier::Priority, changed_at);
        assert_eq!(after.total_cost, dec("0.048"));
    }

    #[test]
    fn service_tier_round_trips_through_the_db_string() {
        for tier in [ServiceTier::Standard, ServiceTier::Priority] {
            assert_eq!(ServiceTier::from_db_str(tier.as_db_str()), tier);
        }
        assert_eq!(ServiceTier::from_db_str("flex"), ServiceTier::Standard);
    }
}
