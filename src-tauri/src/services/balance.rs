//! 供应商余额查询服务
//!
//! 支持 DeepSeek、StepFun、SiliconFlow、OpenRouter、Novita AI 的账户余额查询。
//! 返回 UsageResult 格式，与现有用量系统无缝对接。
//!
//! 错误通道语义（与 coding_plan / subscription 两个服务保持一致）：
//! - `Err(String)` = 瞬时传输失败（网络不可达/超时/读体中断）。前端 invoke reject，
//!   react-query 触发 retry 并保留上一次成功的 data（天然 keep-last-good）。
//! - `Ok(success:false)` = 确定性失败（空 key/未知供应商/鉴权/非 2xx/响应体非法 JSON），
//!   立即透出错误文案。判定按 reqwest 错误种类在折叠点完成，不依赖错误文案匹配。

use crate::http_client::read_json;
use crate::provider::{UsageData, UsageResult};
use std::time::Duration;

// ── 供应商检测 ──────────────────────────────────────────────

enum BalanceProvider {
    DeepSeek,
    StepFun,
    StepFunIntl,
    SiliconFlow,
    SiliconFlowEn,
    OpenRouter,
    NovitaAI,
}

fn detect_provider(base_url: &str) -> Option<BalanceProvider> {
    let url = base_url.to_lowercase();
    if url.contains("api.deepseek.com") {
        Some(BalanceProvider::DeepSeek)
    } else if url.contains("api.stepfun.com") {
        Some(BalanceProvider::StepFun)
    } else if url.contains("api.stepfun.ai") {
        Some(BalanceProvider::StepFunIntl)
    } else if url.contains("api.siliconflow.cn") {
        Some(BalanceProvider::SiliconFlow)
    } else if url.contains("api.siliconflow.com") {
        Some(BalanceProvider::SiliconFlowEn)
    } else if url.contains("openrouter.ai") {
        Some(BalanceProvider::OpenRouter)
    } else if url.contains("api.novita.ai") {
        Some(BalanceProvider::NovitaAI)
    } else {
        None
    }
}

fn make_error(msg: String) -> UsageResult {
    UsageResult {
        success: false,
        data: None,
        error: Some(msg),
    }
}

fn make_auth_error(status: reqwest::StatusCode) -> UsageResult {
    UsageResult {
        success: false,
        data: Some(vec![UsageData {
            plan_name: None,
            remaining: None,
            total: None,
            used: None,
            unit: None,
            is_valid: Some(false),
            invalid_message: Some(format!("Authentication failed (HTTP {status})")),
            extra: None,
        }]),
        error: Some(format!("Authentication failed (HTTP {status})")),
    }
}

// ── DeepSeek ────────────────────────────────────────────────
// GET https://api.deepseek.com/user/balance
// Response: { balance_infos: [{ currency, total_balance, granted_balance, topped_up_balance }], is_available }

async fn query_deepseek(api_key: &str) -> Result<UsageResult, String> {
    let client = crate::http_client::get();

    let resp = client
        .get("https://api.deepseek.com/user/balance")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(make_auth_error(status));
    }

    let body: serde_json::Value = match read_json(resp).await? {
        Ok(body) => body,
        Err(error) => return Ok(make_error(error)),
    };

    let is_available = body
        .get("is_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let mut data = Vec::new();

    if let Some(infos) = body.get("balance_infos").and_then(|v| v.as_array()) {
        for info in infos {
            let currency = info
                .get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or("CNY");
            let total = parse_f64_field(info, "total_balance");

            data.push(UsageData {
                plan_name: Some(currency.to_string()),
                remaining: total,
                total: None,
                used: None,
                unit: Some(currency.to_string()),
                is_valid: Some(is_available),
                invalid_message: if !is_available {
                    Some("Insufficient balance".to_string())
                } else {
                    None
                },
                extra: None,
            });
        }
    }

    Ok(UsageResult {
        success: true,
        data: if data.is_empty() { None } else { Some(data) },
        error: None,
    })
}

// ── StepFun ─────────────────────────────────────────────────
// GET https://api.stepfun.com/v1/accounts（国内）/ https://api.stepfun.ai/v1/accounts（国际）
// Response: { object, type, balance, total_cash_balance, total_voucher_balance }
// 两站账号与 key 各自独立。接口文档没写币种，按两站定价页：国内按元、国际按美元。

async fn query_stepfun(api_key: &str, intl: bool) -> Result<UsageResult, String> {
    let client = crate::http_client::get();

    let (url, plan_name, unit) = if intl {
        (
            "https://api.stepfun.ai/v1/accounts",
            "StepFun (Intl)",
            "USD",
        )
    } else {
        ("https://api.stepfun.com/v1/accounts", "StepFun", "CNY")
    };
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(make_auth_error(status));
    }

    let body: serde_json::Value = match read_json(resp).await? {
        Ok(body) => body,
        Err(error) => return Ok(make_error(error)),
    };

    let balance = parse_f64_field(&body, "balance").unwrap_or(0.0);

    Ok(UsageResult {
        success: true,
        data: Some(vec![UsageData {
            plan_name: Some(plan_name.to_string()),
            remaining: Some(balance),
            total: None,
            used: None,
            unit: Some(unit.to_string()),
            is_valid: Some(true),
            invalid_message: None,
            extra: None,
        }]),
        error: None,
    })
}

// ── SiliconFlow ─────────────────────────────────────────────
// GET https://api.siliconflow.com/v1/user/info（国际站）
// Response: { code, data: { balance, chargeBalance, totalBalance, status } }
//
// 国内站的 /v1/user/info 已于 2026-08-14 停止服务，官方说替代接口上线后另行通知
// （docs.siliconflow.cn 更新公告 2026-08-11；截至 2026-09-28 未发布）。

const SILICONFLOW_CN_RETIRED: &str = "SiliconFlow China retired its balance API \
     (/v1/user/info) on 2026-08-14 and has not published a replacement yet";

async fn query_siliconflow(api_key: &str) -> Result<UsageResult, String> {
    let client = crate::http_client::get();
    let url = "https://api.siliconflow.com/v1/user/info";

    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(make_auth_error(status));
    }

    let body: serde_json::Value = match read_json(resp).await? {
        Ok(body) => body,
        Err(error) => return Ok(make_error(error)),
    };

    let data = match body.get("data") {
        Some(d) => d,
        None => return Ok(make_error("Missing 'data' field in response".to_string())),
    };

    let total_balance = parse_f64_field(data, "totalBalance").unwrap_or(0.0);

    Ok(UsageResult {
        success: true,
        data: Some(vec![UsageData {
            plan_name: Some("SiliconFlow (EN)".to_string()),
            remaining: Some(total_balance),
            total: None,
            used: None,
            unit: Some("USD".to_string()),
            is_valid: Some(true),
            invalid_message: None,
            extra: None,
        }]),
        error: None,
    })
}

// ── OpenRouter ──────────────────────────────────────────────
// GET https://openrouter.ai/api/v1/key
// Response: { data: { limit, limit_remaining, limit_reset, usage, ... } }
//
// 账户余额接口 /api/v1/credits 只认 Management key（普通 key 回 403），而
// Management key 不能用于推理，供应商配置里的 key 不会是它。所以只查当前 key：
// 设了消费上限（limit）就显示上限还剩多少，没设就只能显示这个 key 的累计用量。

async fn query_openrouter(api_key: &str) -> Result<UsageResult, String> {
    let client = crate::http_client::get();

    let resp = client
        .get("https://openrouter.ai/api/v1/key")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(make_auth_error(status));
    }

    let body: serde_json::Value = match read_json(resp).await? {
        Ok(body) => body,
        Err(error) => return Ok(make_error(error)),
    };

    Ok(openrouter_key_usage(body.get("data").unwrap_or(&body)))
}

fn openrouter_key_usage(key: &serde_json::Value) -> UsageResult {
    let usage = parse_f64_field(key, "usage");
    let limit = parse_f64_field(key, "limit");
    let data = match (limit, parse_f64_field(key, "limit_remaining")) {
        (Some(limit), Some(remaining)) => UsageData {
            plan_name: Some("OpenRouter".to_string()),
            remaining: Some(remaining),
            total: Some(limit),
            used: Some(limit - remaining),
            unit: Some("USD".to_string()),
            is_valid: Some(remaining > 0.0),
            invalid_message: (remaining <= 0.0).then(|| "Key spending limit reached".to_string()),
            extra: key
                .get("limit_reset")
                .and_then(|v| v.as_str())
                .map(|reset| format!("Key limit resets {reset}")),
        },
        // 不限额的 key：没有余额可显示，不能当成 0
        _ => UsageData {
            plan_name: Some("OpenRouter".to_string()),
            remaining: None,
            total: None,
            used: usage,
            unit: Some("USD".to_string()),
            is_valid: Some(true),
            invalid_message: None,
            extra: None,
        },
    };
    UsageResult {
        success: true,
        data: Some(vec![data]),
        error: None,
    }
}

// ── Novita AI ───────────────────────────────────────────────
// GET https://api.novita.ai/openapi/v1/billing/balance/detail（官方 "User Balance Info"）
// Response: { availableBalance, cashBalance, creditLimit, pendingCharges, outstandingInvoices }
// 金额都是字符串，单位 0.0001 USD；availableBalance = cashBalance + creditLimit

async fn query_novita(api_key: &str) -> Result<UsageResult, String> {
    let client = crate::http_client::get();

    let resp = client
        .get("https://api.novita.ai/openapi/v1/billing/balance/detail")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(make_auth_error(status));
    }

    let body: serde_json::Value = match read_json(resp).await? {
        Ok(body) => body,
        Err(error) => return Ok(make_error(error)),
    };

    Ok(novita_balance(&body))
}

fn novita_balance(body: &serde_json::Value) -> UsageResult {
    // 金额单位为 0.0001 USD
    let usd = |field| parse_f64_field(body, field).map(|v| v / 10000.0);
    let Some(available) = usd("availableBalance") else {
        return make_error("Unrecognized Novita balance response".to_string());
    };
    // 可用余额含信用额度，拆开写进说明
    let extra = match (usd("cashBalance"), usd("creditLimit")) {
        (Some(cash), Some(credit)) if credit > 0.0 => {
            Some(format!("Top-up {cash:.2} + credit limit {credit:.2} USD"))
        }
        _ => None,
    };

    UsageResult {
        success: true,
        data: Some(vec![UsageData {
            plan_name: Some("Novita AI".to_string()),
            remaining: Some(available),
            total: None,
            used: None,
            unit: Some("USD".to_string()),
            is_valid: Some(available > 0.0),
            invalid_message: (available <= 0.0).then(|| "No balance remaining".to_string()),
            extra,
        }]),
        error: None,
    }
}

// ── 工具函数 ────────────────────────────────────────────────

/// 解析 JSON 字段为 f64，兼容数字和字符串格式
fn parse_f64_field(obj: &serde_json::Value, field: &str) -> Option<f64> {
    obj.get(field).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

// ── 公开入口 ────────────────────────────────────────────────

/// 查询余额。瞬时传输失败返回 `Err`（前端 reject → retry + 保留上次成功值），
/// 确定性失败返回 `Ok(success:false)`（见模块级文档）。
pub async fn get_balance(base_url: &str, api_key: &str) -> Result<UsageResult, String> {
    if api_key.trim().is_empty() {
        return Ok(UsageResult {
            success: false,
            data: None,
            error: Some("API key is empty".to_string()),
        });
    }

    let provider = match detect_provider(base_url) {
        Some(p) => p,
        None => {
            return Ok(UsageResult {
                success: false,
                data: None,
                error: Some("Unknown balance provider".to_string()),
            })
        }
    };

    match provider {
        BalanceProvider::DeepSeek => query_deepseek(api_key).await,
        BalanceProvider::StepFun => query_stepfun(api_key, false).await,
        BalanceProvider::StepFunIntl => query_stepfun(api_key, true).await,
        BalanceProvider::SiliconFlow => Ok(make_error(SILICONFLOW_CN_RETIRED.to_string())),
        BalanceProvider::SiliconFlowEn => query_siliconflow(api_key).await,
        BalanceProvider::OpenRouter => query_openrouter(api_key).await,
        BalanceProvider::NovitaAI => query_novita(api_key).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn siliconflow_china_reports_the_retired_api_without_a_request() {
        let result = get_balance("https://api.siliconflow.cn/v1", "sk-test")
            .await
            .expect("determinate result");
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some(SILICONFLOW_CN_RETIRED));
    }

    #[test]
    fn openrouter_key_with_a_limit_reports_what_is_left() {
        // 官方 /api/v1/key 示例节选
        let key = serde_json::json!({
            "limit": 100, "limit_remaining": 74.5, "limit_reset": "monthly", "usage": 25.5
        });
        let data = &openrouter_key_usage(&key).data.unwrap()[0];
        assert_eq!(data.remaining, Some(74.5));
        assert_eq!(data.total, Some(100.0));
        assert_eq!(data.used, Some(25.5));
        assert_eq!(data.is_valid, Some(true));
        assert_eq!(data.extra.as_deref(), Some("Key limit resets monthly"));

        let spent = serde_json::json!({ "limit": 10, "limit_remaining": 0, "usage": 10 });
        let data = &openrouter_key_usage(&spent).data.unwrap()[0];
        assert_eq!(data.is_valid, Some(false));
    }

    #[test]
    fn novita_balance_converts_units_and_shows_credit_limit() {
        // 官方示例原样
        let body = serde_json::json!({
            "availableBalance": "1000000", "cashBalance": "800000", "creditLimit": "200000",
            "pendingCharges": "0", "outstandingInvoices": "0"
        });
        let data = &novita_balance(&body).data.unwrap()[0];
        assert_eq!(data.remaining, Some(100.0));
        assert_eq!(
            data.extra.as_deref(),
            Some("Top-up 80.00 + credit limit 20.00 USD")
        );

        let result = novita_balance(&serde_json::json!({ "balance": 1 }));
        assert!(!result.success);
    }

    #[test]
    fn stepfun_hosts_pick_their_own_site() {
        assert!(matches!(
            detect_provider("https://api.stepfun.ai/v1"),
            Some(BalanceProvider::StepFunIntl)
        ));
        assert!(matches!(
            detect_provider("https://api.stepfun.com/step_plan/v1"),
            Some(BalanceProvider::StepFun)
        ));
    }

    #[test]
    fn openrouter_unlimited_key_reports_usage_only() {
        let key = serde_json::json!({
            "limit": null, "limit_remaining": null, "limit_reset": null, "usage": 12.34
        });
        let result = openrouter_key_usage(&key);
        assert!(result.success);
        let data = &result.data.unwrap()[0];
        assert_eq!(data.remaining, None);
        assert_eq!(data.total, None);
        assert_eq!(data.used, Some(12.34));
        assert_eq!(data.is_valid, Some(true));
    }
}
