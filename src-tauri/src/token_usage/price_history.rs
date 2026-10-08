//! 官方调过价的模型在调价前的单价。
//!
//! 定价表只存当前价，用当前价去算调价前的请求会算错。这里按模型 id 记下每次调价
//! 前的单价，查价时挂到 [`ModelPricing::earlier`] 上，计价时按请求时间选用。
//! 调价时刻取官方公告日的美国太平洋时间零点。

use super::calculator::{BasePrices, EarlierPrices, ModelPricing};
use rust_decimal::Decimal;
use std::str::FromStr;

struct PriceChange {
    model_ids: &'static [&'static str],
    /// 调价时刻（Unix 秒），之前的请求按 `prices` 计价
    until: i64,
    /// 输入、输出、缓存读、缓存写
    prices: [&'static str; 4],
}

/// 定价表里按 GPT-5.6 Sol 计价的 id：裸名 gpt-5.6 是 Sol 的官方别名，带 effort 后缀的
/// 是 Codex 的记账形态。
const GPT_5_6_SOL_IDS: &[&str] = &[
    "gpt-5.6-sol",
    "gpt-5.6",
    "gpt-5.6-low",
    "gpt-5.6-medium",
    "gpt-5.6-high",
    "gpt-5.6-xhigh",
    "gpt-5.6-minimal",
];

/// 来源：OpenAI 社区公告 "20% price reduction for GPT 5.6 Sol"
/// （community.openai.com/t/1391726），附 2026-07-30 和 2026-08-21 两次调价的新旧价格表。
const PRICE_CHANGES: &[PriceChange] = &[
    // 2026-07-30：Terra 降 20%，Luna 降 80%
    PriceChange {
        model_ids: &["gpt-5.6-terra"],
        until: 1_785_394_800,
        prices: ["2.50", "15", "0.25", "3.125"],
    },
    PriceChange {
        model_ids: &["gpt-5.6-luna"],
        until: 1_785_394_800,
        prices: ["1", "6", "0.10", "1.25"],
    },
    // 2026-08-21：Sol 促销价开始，OpenAI 说持续三个月
    PriceChange {
        model_ids: GPT_5_6_SOL_IDS,
        until: 1_787_295_600,
        prices: ["5", "30", "0.50", "6.25"],
    },
];

/// `model_id`（定价表里的 id）调价前的单价，按调价时刻从早到晚排列。
pub fn earlier_prices(model_id: &str) -> Vec<EarlierPrices> {
    let mut earlier: Vec<EarlierPrices> = PRICE_CHANGES
        .iter()
        .filter(|change| change.model_ids.contains(&model_id))
        .map(|change| {
            let [input, output, cache_read, cache_creation] = change
                .prices
                .map(|value| Decimal::from_str(value).expect("built-in price"));
            EarlierPrices {
                until: change.until,
                prices: BasePrices {
                    input,
                    output,
                    cache_read,
                    cache_creation,
                },
            }
        })
        .collect();
    earlier.sort_by_key(|change| change.until);
    earlier
}

/// 全部调价记录的文本，参与定价指纹：加了或改了调价记录，下次启动按新的历史价重算。
pub fn fingerprint_text() -> String {
    PRICE_CHANGES
        .iter()
        .map(|change| {
            format!(
                "{}@{}={}",
                change.model_ids.join(","),
                change.until,
                change.prices.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// 给查到的当前定价挂上调价前的单价。
pub fn with_price_history(model_id: &str, pricing: ModelPricing) -> ModelPricing {
    ModelPricing {
        earlier: earlier_prices(model_id),
        ..pricing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_prices_parse() {
        for change in PRICE_CHANGES {
            for id in change.model_ids {
                assert!(!earlier_prices(id).is_empty(), "{id}");
            }
        }
    }

    #[test]
    fn fingerprint_text_lists_every_change() {
        let text = fingerprint_text();
        for change in PRICE_CHANGES {
            assert!(text.contains(&change.until.to_string()), "{text}");
            assert!(text.contains(&change.prices.join(",")), "{text}");
        }
    }

    #[test]
    fn sol_aliases_share_the_sol_history() {
        assert_eq!(
            earlier_prices("gpt-5.6-xhigh"),
            earlier_prices("gpt-5.6-sol")
        );
        assert!(earlier_prices("gpt-5.6-cyber").is_empty());
    }
}
