//! 使用统计服务
//!
//! 提供使用量数据的聚合查询功能

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::sql_helpers::{
    fresh_input_sql, real_total_tokens_sql, INPUT_TOKEN_SEMANTICS_FRESH,
    INPUT_TOKEN_SEMANTICS_LEGACY, INPUT_TOKEN_SEMANTICS_TOTAL,
};
use crate::token_usage::calculator::{
    BasePrices, CostBreakdown, CostCalculator, LongContextPricing, ModelPricing, ServiceTier,
};
use crate::token_usage::parser::TokenUsage;
use crate::token_usage::price_history::with_price_history;
use chrono::{Local, NaiveDate, TimeZone, Timelike};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// 使用量汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub total_requests: u64,
    pub total_cost: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub success_rate: f32,
    /// input + output + cache_creation + cache_read — the total tokens
    /// actually processed by the model (including cache hits). Used as the
    /// headline "real consumption" number in the usage hero.
    pub real_total_tokens: u64,
    /// cache_read / (input + cache_creation + cache_read). Range 0.0–1.0.
    /// Reported as a fraction; multiply by 100 in UI for percentage display.
    pub cache_hit_rate: f64,
}

/// Per-app-type usage summary used by the dashboard breakdown rail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummaryByApp {
    pub app_type: String,
    pub summary: UsageSummary,
}

/// Helper: compute (real_total, hit_rate) from the four token counters.
/// All inputs must already be cache-normalized (i.e. input excludes cache).
fn derive_real_total_and_hit_rate(
    fresh_input: u64,
    output: u64,
    cache_creation: u64,
    cache_read: u64,
) -> (u64, f64) {
    let real_total = fresh_input + output + cache_creation + cache_read;
    let cacheable_input = fresh_input + cache_creation + cache_read;
    let hit_rate = if cacheable_input > 0 {
        cache_read as f64 / cacheable_input as f64
    } else {
        0.0
    };
    (real_total, hit_rate)
}

/// 汇总查询的一行（请求数、花费、四类 Token、成功数）转成 [`UsageSummary`]。
/// 列顺序须与 `get_usage_summary` / `get_session_usage_summary` 的 SELECT 一致。
fn usage_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UsageSummary> {
    let total_requests: i64 = row.get(0)?;
    let total_cost: f64 = row.get(1)?;
    let total_input_tokens: i64 = row.get(2)?;
    let total_output_tokens: i64 = row.get(3)?;
    let total_cache_creation_tokens: i64 = row.get(4)?;
    let total_cache_read_tokens: i64 = row.get(5)?;
    let success_count: i64 = row.get(6)?;

    let success_rate = if total_requests > 0 {
        (success_count as f32 / total_requests as f32) * 100.0
    } else {
        0.0
    };

    let (real_total_tokens, cache_hit_rate) = derive_real_total_and_hit_rate(
        total_input_tokens as u64,
        total_output_tokens as u64,
        total_cache_creation_tokens as u64,
        total_cache_read_tokens as u64,
    );

    Ok(UsageSummary {
        total_requests: total_requests as u64,
        total_cost: format!("{total_cost:.6}"),
        total_input_tokens: total_input_tokens as u64,
        total_output_tokens: total_output_tokens as u64,
        total_cache_creation_tokens: total_cache_creation_tokens as u64,
        total_cache_read_tokens: total_cache_read_tokens as u64,
        success_rate,
        real_total_tokens,
        cache_hit_rate,
    })
}

/// 每日统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStats {
    pub date: String,
    pub request_count: u64,
    pub total_cost: String,
    pub total_tokens: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
}

/// Provider 统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStats {
    pub provider_id: String,
    pub provider_name: String,
    pub request_count: u64,
    /// 真实消耗 Tokens（新增输入 + 输出 + 缓存写入 + 缓存命中），与指标卡同口径。
    pub total_tokens: u64,
    pub total_cost: String,
    pub success_rate: f32,
    pub avg_latency_ms: u64,
    /// 速度的分子：满足条件（有首字、输出 >= 100 token、耗时 > 首字）的明细请求的输出 token 之和。
    /// 汇总速度 = speed_output_tokens / (speed_generation_ms / 1000)，不是逐条平均。
    /// 日汇总（rollup）没有逐条计时，不计入。
    pub speed_output_tokens: u64,
    /// 速度的分母：同一批请求的 (latency_ms - first_token_ms) 之和，单位毫秒。
    pub speed_generation_ms: u64,
    /// 估算速度的分子：会话日志导入的请求里，有估算耗时、输出 >= 200 token 的那些的输出之和。
    /// 和上面那组分开累计：估算的耗时含首字等待，口径不同，不能加在一起。
    pub est_speed_output_tokens: u64,
    /// 估算速度的分母：同一批请求的 latency_ms 之和，单位毫秒。
    pub est_speed_duration_ms: u64,
}

/// 计速度的门槛：输出少于这个数的请求（工具调用这类）不算速度，避免 0.1 秒回 15 个 token 算出离谱的数。
pub const SPEED_MIN_OUTPUT_TOKENS: i64 = 100;

/// 生成窗口（耗时 − 首字）短于这个毫秒数时不算速度：中转站缓冲后一次性吐出、
/// 或短回复整段落在同一个网络包里，算出来的是传输突发而不是生成速度。
pub const SPEED_MIN_GENERATION_MS: i64 = 100;

/// 明细行能不能计速度的 SQL 条件（和前端 `isSpeedEligible` 同口径）。
fn speed_eligible_sql(alias: &str) -> String {
    format!(
        "{alias}.first_token_ms IS NOT NULL AND {alias}.output_tokens >= {SPEED_MIN_OUTPUT_TOKENS} \
         AND {alias}.latency_ms - {alias}.first_token_ms >= {SPEED_MIN_GENERATION_MS}"
    )
}

/// 估算速度的输出门槛：估算的耗时含首字等待，输出越少首字占比越大、算出来越偏低，
/// 所以比精确口径的门槛高。实测 200–300 token 的请求比长请求低约四分之一，
/// 100–200 的低约四成；而 Codex 的请求只有四分之一超过 500，门槛再高大半行都是空的。
pub const SPEED_ESTIMATE_MIN_OUTPUT_TOKENS: i64 = 200;

/// 估算耗时短于这个毫秒数时不估速度：输出 200 token 以上却不到 1 秒，多半是起点取晚了。
pub const SPEED_ESTIMATE_MIN_DURATION_MS: i64 = 1000;

/// 明细行能不能估速度的 SQL 条件（和前端 `isSpeedEstimateEligible` 同口径）：
/// 会话日志导入的行（没有首字计时），耗时是导入时按日志时间戳估的。
fn speed_estimate_eligible_sql(alias: &str) -> String {
    let data_source = data_source_expr(alias);
    format!(
        "{alias}.first_token_ms IS NULL AND {data_source} <> 'proxy' \
         AND {alias}.output_tokens >= {SPEED_ESTIMATE_MIN_OUTPUT_TOKENS} \
         AND {alias}.latency_ms >= {SPEED_ESTIMATE_MIN_DURATION_MS}"
    )
}

/// 模型统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStats {
    pub model: String,
    pub request_count: u64,
    /// 真实消耗 Tokens（新增输入 + 输出 + 缓存写入 + 缓存命中），与指标卡同口径。
    pub total_tokens: u64,
    pub total_cost: String,
    pub avg_cost_per_request: String,
    pub success_rate: f32,
    /// 速度的分子分母，口径同 [`ProviderStats::speed_output_tokens`] 那四个字段。
    pub speed_output_tokens: u64,
    pub speed_generation_ms: u64,
    pub est_speed_output_tokens: u64,
    pub est_speed_duration_ms: u64,
    /// 定价表里查得到这个模型。查不到时成本是 0，界面标"未定价"；价格本身是 0 的模型不标。
    pub has_pricing: bool,
}

/// 请求日志过滤器
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFilters {
    pub app_type: Option<String>,
    pub provider_name: Option<String>,
    pub model: Option<String>,
    pub status_code: Option<u16>,
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
}

/// 分页请求日志响应
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedLogs {
    pub data: Vec<RequestLogDetail>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

/// 请求日志详情
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogDetail {
    pub request_id: String,
    pub provider_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
    pub app_type: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_model: Option<String>,
    pub cost_multiplier: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_creation_tokens: u32,
    /// 读出时已换成未命中缓存的输入（FRESH），不进 API。
    #[serde(skip)]
    pub input_token_semantics: i64,
    pub input_cost_usd: String,
    pub output_cost_usd: String,
    pub cache_read_cost_usd: String,
    pub cache_creation_cost_usd: String,
    pub total_cost_usd: String,
    pub is_streaming: bool,
    pub latency_ms: u64,
    pub first_token_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub status_code: u16,
    pub error_message: Option<String>,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
    /// 写入时实际用于计价的模型名。None = v11 前的历史行，"" = 未计价的错误行。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing_model: Option<String>,
    /// 按 [`row_pricing`] 的规则在定价表里查得到模型，查询后由 [`fill_has_pricing`] 填。
    #[serde(default)]
    pub has_pricing: bool,
}

/// 把 26 列的查询结果映射为 `RequestLogDetail`。
///
/// 调用方的 SELECT **必须**按以下顺序返回 26 列：
/// `request_id, provider_id, provider_name, app_type, model, request_model,
///  cost_multiplier, input_tokens, output_tokens, cache_read_tokens,
///  cache_creation_tokens, input_cost_usd, output_cost_usd, cache_read_cost_usd,
///  cache_creation_cost_usd, total_cost_usd, is_streaming, latency_ms,
///  first_token_ms, duration_ms, status_code, error_message, created_at,
///  data_source, pricing_model, input_token_semantics`
///
/// 不需要 provider_name 时（如 backfill）SELECT `NULL AS provider_name` 占位即可。
fn row_to_request_log_detail(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestLogDetail> {
    let app_type: String = row.get(3)?;
    let semantics: i64 = row.get(25)?;
    let cache_read_tokens: i64 = row.get(9)?;
    let cache_creation_tokens: i64 = row.get(10)?;
    // 界面只看未命中缓存的输入；旧行按存储口径换算
    let input_tokens = fresh_input_tokens(
        &app_type,
        semantics,
        row.get(7)?,
        cache_read_tokens,
        cache_creation_tokens,
    );
    Ok(RequestLogDetail {
        request_id: row.get(0)?,
        provider_id: row.get(1)?,
        provider_name: row.get(2)?,
        app_type,
        model: row.get(4)?,
        request_model: row.get(5)?,
        cost_multiplier: row
            .get::<_, Option<String>>(6)?
            .unwrap_or_else(|| "1".to_string()),
        input_tokens,
        output_tokens: row.get::<_, i64>(8)? as u32,
        cache_read_tokens: clamp_u32(cache_read_tokens),
        cache_creation_tokens: clamp_u32(cache_creation_tokens),
        input_cost_usd: row.get(11)?,
        output_cost_usd: row.get(12)?,
        cache_read_cost_usd: row.get(13)?,
        cache_creation_cost_usd: row.get(14)?,
        total_cost_usd: row.get(15)?,
        is_streaming: row.get::<_, i64>(16)? != 0,
        latency_ms: row.get::<_, i64>(17)? as u64,
        first_token_ms: row.get::<_, Option<i64>>(18)?.map(|v| v as u64),
        duration_ms: row.get::<_, Option<i64>>(19)?.map(|v| v as u64),
        status_code: row.get::<_, i64>(20)? as u16,
        error_message: row.get(21)?,
        created_at: row.get(22)?,
        data_source: row.get(23)?,
        pricing_model: row.get(24)?,
        input_token_semantics: INPUT_TOKEN_SEMANTICS_FRESH,
        has_pricing: false,
    })
}

/// SQL fragment: resolve provider_name with fallback for session-based entries.
/// Session logs use placeholder provider_ids (e.g., `_session`, `_<app>_session`)
/// that don't exist in the providers table — the CASE expression below is the
/// authoritative mapping from placeholder to readable name.
fn provider_name_coalesce(log_alias: &str, provider_alias: &str) -> String {
    format!(
        "COALESCE({provider_alias}.name, CASE {log_alias}.provider_id \
         WHEN '_session' THEN 'Claude (Session)' \
         WHEN '_codex_session' THEN 'Codex (Session)' \
         WHEN '_gemini_session' THEN 'Gemini (Session)' \
         WHEN '_opencode_session' THEN 'OpenCode (Session)' \
         WHEN '_grok_session' THEN 'Grok Build (Session)' \
         WHEN '_mcode_session' THEN 'MiniMax Code (Session)' \
         WHEN '_pi_session' THEN 'Pi (Session)' \
         ELSE {log_alias}.provider_id END)"
    )
}

/// SQL 片段：把指定别名的 `data_source` 包成 COALESCE，NULL 视作 'proxy'。
///
/// 防御 schema v9 之前可能写入的 NULL data_source 行。所有用到 data_source 的
/// 查询都应通过此 helper 生成片段，避免遗漏。
fn data_source_expr(log_alias: &str) -> String {
    format!("COALESCE({log_alias}.data_source, 'proxy')")
}

/// SQL 标量表达式：把旧版 Claude Desktop 网关留下的 `claude-desktop` app_type
/// 在展示口径上折叠进 `claude`，其余 app_type 原样返回。
///
/// 网关已经移除，这类行只会出现在旧的按天汇总里。把任一参与“按应用筛选/分组”
/// 的 `app_type` 列包进此表达式，`= 'claude'` 过滤就会同时命中两者，`GROUP BY`
/// 也会把两者合并，已存储的行不用改。包裹后该列上的索引在此比较中失效，
/// 这些查询都带时间过滤，可以接受。
fn folded_app_type_sql(column: &str) -> String {
    format!("CASE WHEN {column} = 'claude-desktop' THEN 'claude' ELSE {column} END")
}

/// SQL 片段：把日志/汇总行 LEFT JOIN 到 providers 表以取得供应商名称。
/// `proxy_request_logs` 与 `usage_daily_rollups` 的 (provider_id, app_type)
/// 形状相同，两者皆可作为 `log_alias`。providers 主键即 (id, app_type)，
/// 连接至多 1:1，不会放大行数。
fn providers_join(log_alias: &str, provider_alias: &str) -> String {
    format!(
        "LEFT JOIN providers {provider_alias} \
         ON {log_alias}.provider_id = {provider_alias}.id \
         AND {log_alias}.app_type = {provider_alias}.app_type"
    )
}

/// SQL 标量表达式：行的「有效计价模型」—— pricing_model 非空优先，NULL/'' 回落
/// model。这是 `get_model_stats` 的分组键，也是 Dashboard 模型筛选的匹配口径：
/// 筛选值来自模型统计列表，两边必须用同一表达式才能选得中。
fn effective_model_sql(alias: &str) -> String {
    format!("COALESCE(NULLIF({alias}.pricing_model, ''), {alias}.model)")
}

/// 把 Dashboard 顶部的 Provider/模型筛选追加到查询条件。
///
/// Provider 按展示名精确匹配（复用 [`provider_name_coalesce`]，会话占位行的
/// 可读名如 "Claude (Session)" 也能选中）；模型按 [`effective_model_sql`] 匹配。
/// 注意：传入 `provider_name` 时调用方必须把 [`providers_join`] 拼进 FROM，
/// 否则 `{provider_alias}.name` 无法解析。
fn push_provider_model_filters(
    conditions: &mut Vec<String>,
    params: &mut Vec<Box<dyn rusqlite::ToSql>>,
    log_alias: &str,
    provider_alias: &str,
    provider_name: Option<&str>,
    model: Option<&str>,
) {
    if let Some(name) = provider_name {
        conditions.push(format!(
            "{} = ?",
            provider_name_coalesce(log_alias, provider_alias)
        ));
        params.push(Box::new(name.to_string()));
    }
    if let Some(m) = model {
        conditions.push(format!("{} = ?", effective_model_sql(log_alias)));
        params.push(Box::new(m.to_string()));
    }
}

/// 请求日志总数缓存：每次刷新都 `COUNT(*)` 一遍明细，大范围下最耗时。
/// 筛选条件、起点没变，且连接上没有任何写入（`total_changes` 没变，增删改都算）
/// 时复用上次的总数；结束时间往后推时（「当天」这类活动窗口每次刷新都会变），
/// 再确认新增的时间段里没有行。另设 [`LOG_COUNT_CACHE_TTL`] 兜底。
pub(crate) struct LogCountCache {
    key: String,
    end_date: Option<i64>,
    changes: u64,
    computed_at: std::time::Instant,
    total: u32,
}

/// `settings` 表里上次全量重算成本时的定价指纹（`model_pricing` 和计价规则），启动时
/// 据此判断定价变了没有。
pub(crate) const USAGE_PRICING_FINGERPRINT_KEY: &str = "usage_pricing_fingerprint";

/// 定价表之外也决定成本的规则：计价规则版本和调价记录。发新版只改了它们时，启动时
/// 也会按新规则重算已入库的成本。
fn pricing_rules_text() -> String {
    format!(
        "rules={};history={}",
        crate::token_usage::calculator::PRICING_RULES_VERSION,
        crate::token_usage::price_history::fingerprint_text()
    )
}

const LOG_COUNT_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

/// 这条连接打开以来增删改过的总行数（SQLite 内置函数），任何写入都会让它变。
fn connection_total_changes(conn: &Connection) -> Result<u64, AppError> {
    Ok(conn.query_row("SELECT total_changes()", [], |row| row.get::<_, i64>(0))? as u64)
}

fn log_count_cache_key(filters: &LogFilters) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}",
        filters.app_type,
        filters.provider_name,
        filters.model,
        filters.status_code,
        filters.start_date
    )
}

/// 能复用缓存时返回缓存的总数。
fn cached_log_count(
    cache: &Option<LogCountCache>,
    conn: &Connection,
    filters: &LogFilters,
) -> Result<Option<u32>, AppError> {
    let Some(entry) = cache.as_ref() else {
        return Ok(None);
    };
    if entry.key != log_count_cache_key(filters)
        || entry.changes != connection_total_changes(conn)?
        || entry.computed_at.elapsed() > LOG_COUNT_CACHE_TTL
    {
        return Ok(None);
    }
    match (entry.end_date, filters.end_date) {
        (cached, current) if cached == current => Ok(Some(entry.total)),
        (Some(cached), Some(current)) if current > cached => {
            // 结束时间往后推了：新增区间里没有任何行，总数就不变
            let has_new_rows: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM proxy_request_logs
                  WHERE created_at > ?1 AND created_at <= ?2)",
                params![cached, current],
                |row| row.get(0),
            )?;
            Ok((!has_new_rows).then_some(entry.total))
        }
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, Default)]
struct RollupDateBounds {
    start: Option<String>,
    end: Option<String>,
    is_empty: bool,
}

fn local_datetime_from_timestamp(ts: i64) -> Result<chrono::DateTime<Local>, AppError> {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .ok_or_else(|| AppError::Database(format!("无法解析本地时间戳: {ts}")))
}

fn compute_rollup_date_bounds(
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> Result<RollupDateBounds, AppError> {
    let start = match start_ts {
        Some(ts) => {
            let local = local_datetime_from_timestamp(ts)?;
            let day = local.date_naive();
            if local.time().num_seconds_from_midnight() == 0 {
                Some(day.format("%Y-%m-%d").to_string())
            } else {
                day.succ_opt()
                    .map(|next| next.format("%Y-%m-%d").to_string())
            }
        }
        None => None,
    };

    let end = match end_ts {
        Some(ts) => {
            let local = local_datetime_from_timestamp(ts)?;
            let day = local.date_naive();
            if local.time().hour() == 23 && local.time().minute() == 59 {
                Some(day.format("%Y-%m-%d").to_string())
            } else {
                day.pred_opt()
                    .map(|prev| prev.format("%Y-%m-%d").to_string())
            }
        }
        None => None,
    };

    let is_empty = matches!((&start, &end), (Some(start), Some(end)) if start > end);

    Ok(RollupDateBounds {
        start,
        end,
        is_empty,
    })
}

fn push_rollup_date_filters(
    conditions: &mut Vec<String>,
    params: &mut Vec<Box<dyn rusqlite::ToSql>>,
    column: &str,
    bounds: &RollupDateBounds,
) {
    if bounds.is_empty {
        conditions.push("1 = 0".to_string());
        return;
    }

    if let Some(start) = &bounds.start {
        conditions.push(format!("{column} >= ?"));
        params.push(Box::new(start.clone()));
    }

    if let Some(end) = &bounds.end {
        conditions.push(format!("{column} <= ?"));
        params.push(Box::new(end.clone()));
    }
}

fn local_day_start_rfc3339(day: NaiveDate) -> String {
    let local_midnight = day
        .and_hms_opt(0, 0, 0)
        .and_then(|naive| match Local.from_local_datetime(&naive) {
            chrono::LocalResult::Single(dt) => Some(dt),
            chrono::LocalResult::Ambiguous(earliest, _) => Some(earliest),
            chrono::LocalResult::None => None,
        })
        .unwrap_or_else(Local::now);

    local_midnight.to_rfc3339()
}

impl Database {
    /// 获取使用量汇总
    pub fn get_usage_summary(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
        app_type: Option<&str>,
        provider_name: Option<&str>,
        model: Option<&str>,
    ) -> Result<UsageSummary, AppError> {
        let conn = lock_conn!(self.conn);

        // Build detail WHERE clause
        let mut conditions: Vec<String> = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(start) = start_date {
            conditions.push("l.created_at >= ?".to_string());
            params_vec.push(Box::new(start));
        }
        if let Some(end) = end_date {
            conditions.push("l.created_at <= ?".to_string());
            params_vec.push(Box::new(end));
        }
        if let Some(at) = app_type {
            conditions.push(format!("{} = ?", folded_app_type_sql("l.app_type")));
            params_vec.push(Box::new(at.to_string()));
        }
        push_provider_model_filters(
            &mut conditions,
            &mut params_vec,
            "l",
            "p",
            provider_name,
            model,
        );

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        let detail_join = if provider_name.is_some() {
            providers_join("l", "p")
        } else {
            String::new()
        };

        // Only include rolled-up rows for full local days that are fully covered by the range.
        let mut rollup_conditions: Vec<String> = Vec::new();
        let mut rollup_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let rollup_bounds = compute_rollup_date_bounds(start_date, end_date)?;

        push_rollup_date_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r.date",
            &rollup_bounds,
        );
        if let Some(at) = app_type {
            rollup_conditions.push(format!("{} = ?", folded_app_type_sql("r.app_type")));
            rollup_params.push(Box::new(at.to_string()));
        }
        push_provider_model_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r",
            "p2",
            provider_name,
            model,
        );

        let rollup_where = if rollup_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", rollup_conditions.join(" AND "))
        };
        let rollup_join = if provider_name.is_some() {
            providers_join("r", "p2")
        } else {
            String::new()
        };

        let fresh_input_detail = fresh_input_sql("l");
        let fresh_input_rollup = fresh_input_sql("r");
        let sql = format!(
            "SELECT
                COALESCE(d.total_requests, 0) + COALESCE(r.total_requests, 0),
                COALESCE(d.total_cost, 0) + COALESCE(r.total_cost, 0),
                COALESCE(d.total_input_tokens, 0) + COALESCE(r.total_input_tokens, 0),
                COALESCE(d.total_output_tokens, 0) + COALESCE(r.total_output_tokens, 0),
                COALESCE(d.total_cache_creation_tokens, 0) + COALESCE(r.total_cache_creation_tokens, 0),
                COALESCE(d.total_cache_read_tokens, 0) + COALESCE(r.total_cache_read_tokens, 0),
                COALESCE(d.success_count, 0) + COALESCE(r.success_count, 0)
            FROM
                (SELECT
                    COUNT(*) as total_requests,
                    COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0) as total_cost,
                    COALESCE(SUM({fresh_input_detail}), 0) as total_input_tokens,
                    COALESCE(SUM(l.output_tokens), 0) as total_output_tokens,
                    COALESCE(SUM(l.cache_creation_tokens), 0) as total_cache_creation_tokens,
                    COALESCE(SUM(l.cache_read_tokens), 0) as total_cache_read_tokens,
                    COALESCE(SUM(CASE WHEN l.status_code >= 200 AND l.status_code < 300 THEN 1 ELSE 0 END), 0) as success_count
                 FROM proxy_request_logs l {detail_join} {where_clause}) d,
                (SELECT
                    COALESCE(SUM(r.request_count), 0) as total_requests,
                    COALESCE(SUM(CAST(r.total_cost_usd AS REAL)), 0) as total_cost,
                    COALESCE(SUM({fresh_input_rollup}), 0) as total_input_tokens,
                    COALESCE(SUM(r.output_tokens), 0) as total_output_tokens,
                    COALESCE(SUM(r.cache_creation_tokens), 0) as total_cache_creation_tokens,
                    COALESCE(SUM(r.cache_read_tokens), 0) as total_cache_read_tokens,
                    COALESCE(SUM(r.success_count), 0) as success_count
                 FROM usage_daily_rollups r {rollup_join} {rollup_where}) r"
        );

        // Combine params: detail params first, then rollup params
        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec;
        all_params.extend(rollup_params);
        let param_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();

        let result = conn.query_row(&sql, param_refs.as_slice(), usage_summary_from_row)?;

        Ok(result)
    }

    /// 单个会话的用量汇总（会话阅读页头部）：总 Token 和花费的口径同 Dashboard。
    ///
    /// 只数会话日志导入的行：它们带客户端自己的会话 ID，经不经过路由都会导入；
    /// 代理行的会话 ID 是代理侧推断的，未必对得上，两边都数还会重复。
    /// 明细 30 天后汇总进按天表并删除（按天表没有会话维度），更早的会话查不到。
    pub fn get_session_usage_summary(
        &self,
        app_type: &str,
        session_id: &str,
    ) -> Result<UsageSummary, AppError> {
        let conn = lock_conn!(self.conn);
        let fresh_input = fresh_input_sql("l");
        let app_type_expr = folded_app_type_sql("l.app_type");
        let data_source = data_source_expr("l");
        let sql = format!(
            "SELECT
                COUNT(*),
                COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0),
                COALESCE(SUM({fresh_input}), 0),
                COALESCE(SUM(l.output_tokens), 0),
                COALESCE(SUM(l.cache_creation_tokens), 0),
                COALESCE(SUM(l.cache_read_tokens), 0),
                COALESCE(SUM(CASE WHEN l.status_code >= 200 AND l.status_code < 300 THEN 1 ELSE 0 END), 0)
             FROM proxy_request_logs l
             WHERE l.session_id = ?1 AND {app_type_expr} = ?2 AND {data_source} <> 'proxy'"
        );

        let result = conn.query_row(
            &sql,
            rusqlite::params![session_id, app_type],
            usage_summary_from_row,
        )?;

        Ok(result)
    }

    /// 按 app_type 维度拆分的使用量汇总，用于 Dashboard 的分应用展示条。
    /// 返回所有有数据的 app_type，按 real_total_tokens 降序。
    ///
    /// Single SQL with `GROUP BY app_type` — avoids the N+1 round-trip that
    /// would result from invoking `get_usage_summary` once per app_type.
    pub fn get_usage_summary_by_app(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
        provider_name: Option<&str>,
        model: Option<&str>,
    ) -> Result<Vec<UsageSummaryByApp>, AppError> {
        let conn = lock_conn!(self.conn);

        let mut detail_conditions: Vec<String> = Vec::new();
        let mut detail_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(start) = start_date {
            detail_conditions.push("l.created_at >= ?".to_string());
            detail_params.push(Box::new(start));
        }
        if let Some(end) = end_date {
            detail_conditions.push("l.created_at <= ?".to_string());
            detail_params.push(Box::new(end));
        }
        push_provider_model_filters(
            &mut detail_conditions,
            &mut detail_params,
            "l",
            "p",
            provider_name,
            model,
        );
        let detail_where = if detail_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", detail_conditions.join(" AND "))
        };
        let detail_join = if provider_name.is_some() {
            providers_join("l", "p")
        } else {
            String::new()
        };

        let rollup_bounds = compute_rollup_date_bounds(start_date, end_date)?;
        let mut rollup_conditions: Vec<String> = Vec::new();
        let mut rollup_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        push_rollup_date_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r.date",
            &rollup_bounds,
        );
        push_provider_model_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r",
            "p2",
            provider_name,
            model,
        );
        let rollup_where = if rollup_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", rollup_conditions.join(" AND "))
        };
        let rollup_join = if provider_name.is_some() {
            providers_join("r", "p2")
        } else {
            String::new()
        };

        let fresh_input_detail = fresh_input_sql("l");
        let fresh_input_rollup = fresh_input_sql("r");
        // 折叠 claude-desktop → claude：内层投影成同一桶名，外层 GROUP BY 自然合并。
        let detail_app_type = folded_app_type_sql("l.app_type");
        let rollup_app_type = folded_app_type_sql("r.app_type");

        let sql = format!(
            "SELECT app_type,
                SUM(req_count) as req_count,
                SUM(cost) as cost,
                SUM(input_t) as input_t,
                SUM(output_t) as output_t,
                SUM(cache_create_t) as cache_create_t,
                SUM(cache_read_t) as cache_read_t,
                SUM(success_count) as success_count
            FROM (
                SELECT {detail_app_type} as app_type,
                    COUNT(*) as req_count,
                    COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0) as cost,
                    COALESCE(SUM({fresh_input_detail}), 0) as input_t,
                    COALESCE(SUM(l.output_tokens), 0) as output_t,
                    COALESCE(SUM(l.cache_creation_tokens), 0) as cache_create_t,
                    COALESCE(SUM(l.cache_read_tokens), 0) as cache_read_t,
                    COALESCE(SUM(CASE WHEN l.status_code >= 200 AND l.status_code < 300 THEN 1 ELSE 0 END), 0) as success_count
                FROM proxy_request_logs l {detail_join} {detail_where}
                GROUP BY l.app_type
                UNION ALL
                SELECT {rollup_app_type} as app_type,
                    COALESCE(SUM(r.request_count), 0),
                    COALESCE(SUM(CAST(r.total_cost_usd AS REAL)), 0),
                    COALESCE(SUM({fresh_input_rollup}), 0),
                    COALESCE(SUM(r.output_tokens), 0),
                    COALESCE(SUM(r.cache_creation_tokens), 0),
                    COALESCE(SUM(r.cache_read_tokens), 0),
                    COALESCE(SUM(r.success_count), 0)
                FROM usage_daily_rollups r {rollup_join} {rollup_where}
                GROUP BY r.app_type
            )
            GROUP BY app_type"
        );

        let mut combined: Vec<Box<dyn rusqlite::ToSql>> = detail_params;
        combined.extend(rollup_params);
        let refs: Vec<&dyn rusqlite::ToSql> = combined.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            let app_type: String = row.get(0)?;
            let total_requests: i64 = row.get(1)?;
            let total_cost: f64 = row.get(2)?;
            let total_input_tokens: i64 = row.get(3)?;
            let total_output_tokens: i64 = row.get(4)?;
            let total_cache_creation_tokens: i64 = row.get(5)?;
            let total_cache_read_tokens: i64 = row.get(6)?;
            let success_count: i64 = row.get(7)?;

            let success_rate = if total_requests > 0 {
                (success_count as f32 / total_requests as f32) * 100.0
            } else {
                0.0
            };
            let (real_total_tokens, cache_hit_rate) = derive_real_total_and_hit_rate(
                total_input_tokens as u64,
                total_output_tokens as u64,
                total_cache_creation_tokens as u64,
                total_cache_read_tokens as u64,
            );

            Ok(UsageSummaryByApp {
                app_type,
                summary: UsageSummary {
                    total_requests: total_requests as u64,
                    total_cost: format!("{total_cost:.6}"),
                    total_input_tokens: total_input_tokens as u64,
                    total_output_tokens: total_output_tokens as u64,
                    total_cache_creation_tokens: total_cache_creation_tokens as u64,
                    total_cache_read_tokens: total_cache_read_tokens as u64,
                    success_rate,
                    real_total_tokens,
                    cache_hit_rate,
                },
            })
        })?;

        let mut summaries = Vec::new();
        for row in rows {
            let item = row?;
            if item.summary.total_requests == 0 && item.summary.real_total_tokens == 0 {
                continue;
            }
            summaries.push(item);
        }
        summaries.sort_by(|a, b| {
            b.summary
                .real_total_tokens
                .cmp(&a.summary.real_total_tokens)
        });
        Ok(summaries)
    }

    /// 获取每日趋势（滑动窗口，<=24h 按小时，>24h 按天，窗口与汇总一致）
    pub fn get_daily_trends(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
        app_type: Option<&str>,
        provider_name: Option<&str>,
        model: Option<&str>,
    ) -> Result<Vec<DailyStats>, AppError> {
        let conn = lock_conn!(self.conn);

        let end_ts = end_date.unwrap_or_else(|| Local::now().timestamp());
        let mut start_ts = start_date.unwrap_or_else(|| end_ts - 24 * 60 * 60);

        if start_ts >= end_ts {
            start_ts = end_ts - 24 * 60 * 60;
        }

        let duration = end_ts - start_ts;
        if duration <= 24 * 60 * 60 {
            let bucket_seconds: i64 = 60 * 60;
            let mut bucket_count: i64 = if duration <= 0 {
                1
            } else {
                (duration + bucket_seconds - 1) / bucket_seconds
            };

            if bucket_count < 1 {
                bucket_count = 1;
            }

            let mut extra_conditions: Vec<String> = Vec::new();
            let mut extra_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if let Some(at) = app_type {
                extra_conditions.push(format!("{} = ?", folded_app_type_sql("l.app_type")));
                extra_params.push(Box::new(at.to_string()));
            }
            push_provider_model_filters(
                &mut extra_conditions,
                &mut extra_params,
                "l",
                "p",
                provider_name,
                model,
            );
            let extra_filter = extra_conditions
                .iter()
                .map(|c| format!("AND {c}"))
                .collect::<Vec<_>>()
                .join(" ");
            let detail_join = if provider_name.is_some() {
                providers_join("l", "p")
            } else {
                String::new()
            };
            let fresh_input = fresh_input_sql("l");
            let sql = format!(
                "SELECT
                    CAST((l.created_at - ?1) / ?3 AS INTEGER) as bucket_idx,
                    COUNT(*) as request_count,
                    COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0) as total_cost,
                    COALESCE(SUM({fresh_input} + l.output_tokens), 0) as total_tokens,
                    COALESCE(SUM({fresh_input}), 0) as total_input_tokens,
                    COALESCE(SUM(l.output_tokens), 0) as total_output_tokens,
                    COALESCE(SUM(l.cache_creation_tokens), 0) as total_cache_creation_tokens,
                    COALESCE(SUM(l.cache_read_tokens), 0) as total_cache_read_tokens
                FROM proxy_request_logs l {detail_join}
                WHERE l.created_at >= ?1 AND l.created_at <= ?2
                  {extra_filter}
                GROUP BY bucket_idx
                ORDER BY bucket_idx ASC"
            );

            let mut stmt = conn.prepare(&sql)?;
            let row_mapper = |row: &rusqlite::Row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    DailyStats {
                        date: String::new(),
                        request_count: row.get::<_, i64>(1)? as u64,
                        total_cost: format!("{:.6}", row.get::<_, f64>(2)?),
                        total_tokens: row.get::<_, i64>(3)? as u64,
                        total_input_tokens: row.get::<_, i64>(4)? as u64,
                        total_output_tokens: row.get::<_, i64>(5)? as u64,
                        total_cache_creation_tokens: row.get::<_, i64>(6)? as u64,
                        total_cache_read_tokens: row.get::<_, i64>(7)? as u64,
                    },
                ))
            };

            let mut map: HashMap<i64, DailyStats> = HashMap::new();

            let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = vec![
                Box::new(start_ts),
                Box::new(end_ts),
                Box::new(bucket_seconds),
            ];
            all_params.extend(extra_params);
            let param_refs: Vec<&dyn rusqlite::ToSql> =
                all_params.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(param_refs.as_slice(), row_mapper)?;
            for row in rows {
                let (mut bucket_idx, stat) = row?;
                if bucket_idx < 0 {
                    continue;
                }
                if bucket_idx >= bucket_count {
                    bucket_idx = bucket_count - 1;
                }
                map.insert(bucket_idx, stat);
            }

            let mut stats = Vec::with_capacity(bucket_count as usize);
            for i in 0..bucket_count {
                let bucket_start_ts = start_ts + i * bucket_seconds;
                let bucket_start = local_datetime_from_timestamp(bucket_start_ts)?;
                let date = bucket_start.to_rfc3339();

                if let Some(mut stat) = map.remove(&i) {
                    stat.date = date;
                    stats.push(stat);
                } else {
                    stats.push(DailyStats {
                        date,
                        request_count: 0,
                        total_cost: "0.000000".to_string(),
                        total_tokens: 0,
                        total_input_tokens: 0,
                        total_output_tokens: 0,
                        total_cache_creation_tokens: 0,
                        total_cache_read_tokens: 0,
                    });
                }
            }

            return Ok(stats);
        }

        let start_day = local_datetime_from_timestamp(start_ts)?.date_naive();
        let end_day = local_datetime_from_timestamp(end_ts)?.date_naive();
        let bucket_count = (end_day.signed_duration_since(start_day).num_days() + 1) as usize;

        let mut extra_conditions: Vec<String> = Vec::new();
        let mut extra_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(at) = app_type {
            extra_conditions.push(format!("{} = ?", folded_app_type_sql("l.app_type")));
            extra_params.push(Box::new(at.to_string()));
        }
        push_provider_model_filters(
            &mut extra_conditions,
            &mut extra_params,
            "l",
            "p",
            provider_name,
            model,
        );
        let extra_filter = extra_conditions
            .iter()
            .map(|c| format!("AND {c}"))
            .collect::<Vec<_>>()
            .join(" ");
        let detail_join = if provider_name.is_some() {
            providers_join("l", "p")
        } else {
            String::new()
        };
        let fresh_input = fresh_input_sql("l");
        let detail_sql = format!(
            "SELECT
                date(l.created_at, 'unixepoch', 'localtime') as bucket_date,
                COUNT(*) as request_count,
                COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0) as total_cost,
                COALESCE(SUM({fresh_input} + l.output_tokens), 0) as total_tokens,
                COALESCE(SUM({fresh_input}), 0) as total_input_tokens,
                COALESCE(SUM(l.output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(l.cache_creation_tokens), 0) as total_cache_creation_tokens,
                COALESCE(SUM(l.cache_read_tokens), 0) as total_cache_read_tokens
            FROM proxy_request_logs l {detail_join}
            WHERE l.created_at >= ?1 AND l.created_at <= ?2
              {extra_filter}
            GROUP BY bucket_date
            ORDER BY bucket_date ASC"
        );

        let mut detail_stmt = conn.prepare(&detail_sql)?;
        let detail_row_mapper = |row: &rusqlite::Row| {
            Ok((
                row.get::<_, String>(0)?,
                DailyStats {
                    date: String::new(),
                    request_count: row.get::<_, i64>(1)? as u64,
                    total_cost: format!("{:.6}", row.get::<_, f64>(2)?),
                    total_tokens: row.get::<_, i64>(3)? as u64,
                    total_input_tokens: row.get::<_, i64>(4)? as u64,
                    total_output_tokens: row.get::<_, i64>(5)? as u64,
                    total_cache_creation_tokens: row.get::<_, i64>(6)? as u64,
                    total_cache_read_tokens: row.get::<_, i64>(7)? as u64,
                },
            ))
        };

        let mut map: HashMap<NaiveDate, DailyStats> = HashMap::new();
        let mut detail_all_params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(start_ts), Box::new(end_ts)];
        detail_all_params.extend(extra_params);
        let detail_param_refs: Vec<&dyn rusqlite::ToSql> =
            detail_all_params.iter().map(|p| p.as_ref()).collect();
        let detail_rows = detail_stmt.query_map(detail_param_refs.as_slice(), detail_row_mapper)?;

        for row in detail_rows {
            let (bucket_date, stat) = row?;
            let date = NaiveDate::parse_from_str(&bucket_date, "%Y-%m-%d")
                .map_err(|err| AppError::Database(format!("解析趋势日期失败: {err}")))?;
            map.insert(date, stat);
        }

        let rollup_bounds = compute_rollup_date_bounds(Some(start_ts), Some(end_ts))?;
        let mut rollup_conditions = Vec::new();
        let mut rollup_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        push_rollup_date_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r.date",
            &rollup_bounds,
        );
        if let Some(at) = app_type {
            rollup_conditions.push(format!("{} = ?", folded_app_type_sql("r.app_type")));
            rollup_params.push(Box::new(at.to_string()));
        }
        push_provider_model_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r",
            "p2",
            provider_name,
            model,
        );

        let rollup_where = if rollup_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", rollup_conditions.join(" AND "))
        };
        let rollup_join = if provider_name.is_some() {
            providers_join("r", "p2")
        } else {
            String::new()
        };

        let fresh_input_rollup = fresh_input_sql("r");
        let rollup_sql = format!(
            "SELECT
                r.date,
                COALESCE(SUM(r.request_count), 0),
                COALESCE(SUM(CAST(r.total_cost_usd AS REAL)), 0),
                COALESCE(SUM({fresh_input_rollup} + r.output_tokens), 0),
                COALESCE(SUM({fresh_input_rollup}), 0),
                COALESCE(SUM(r.output_tokens), 0),
                COALESCE(SUM(r.cache_creation_tokens), 0),
                COALESCE(SUM(r.cache_read_tokens), 0)
            FROM usage_daily_rollups r {rollup_join}
            {rollup_where}
            GROUP BY r.date
            ORDER BY r.date ASC"
        );

        let mut rollup_stmt = conn.prepare(&rollup_sql)?;
        let rollup_row_mapper = |row: &rusqlite::Row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get::<_, i64>(1)? as u64,
                    row.get::<_, f64>(2)?,
                    row.get::<_, i64>(3)? as u64,
                    row.get::<_, i64>(4)? as u64,
                    row.get::<_, i64>(5)? as u64,
                    row.get::<_, i64>(6)? as u64,
                    row.get::<_, i64>(7)? as u64,
                ),
            ))
        };
        let rollup_param_refs: Vec<&dyn rusqlite::ToSql> =
            rollup_params.iter().map(|param| param.as_ref()).collect();
        let rollup_rows = rollup_stmt.query_map(rollup_param_refs.as_slice(), rollup_row_mapper)?;

        for row in rollup_rows {
            let (bucket_date, (req, cost, tok, inp, out, cc, cr)) = row?;
            let date = NaiveDate::parse_from_str(&bucket_date, "%Y-%m-%d")
                .map_err(|err| AppError::Database(format!("解析 rollup 趋势日期失败: {err}")))?;
            let entry = map.entry(date).or_insert_with(|| DailyStats {
                date: String::new(),
                request_count: 0,
                total_cost: "0.000000".to_string(),
                total_tokens: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_creation_tokens: 0,
                total_cache_read_tokens: 0,
            });
            entry.request_count += req;
            let existing_cost: f64 = entry.total_cost.parse().unwrap_or(0.0);
            entry.total_cost = format!("{:.6}", existing_cost + cost);
            entry.total_tokens += tok;
            entry.total_input_tokens += inp;
            entry.total_output_tokens += out;
            entry.total_cache_creation_tokens += cc;
            entry.total_cache_read_tokens += cr;
        }

        let mut stats = Vec::with_capacity(bucket_count);
        let mut current_day = start_day;
        for _ in 0..bucket_count {
            let date = local_day_start_rfc3339(current_day);

            if let Some(mut stat) = map.remove(&current_day) {
                stat.date = date;
                stats.push(stat);
            } else {
                stats.push(DailyStats {
                    date,
                    request_count: 0,
                    total_cost: "0.000000".to_string(),
                    total_tokens: 0,
                    total_input_tokens: 0,
                    total_output_tokens: 0,
                    total_cache_creation_tokens: 0,
                    total_cache_read_tokens: 0,
                });
            }

            current_day = current_day.succ_opt().unwrap_or(current_day);
        }

        Ok(stats)
    }

    /// 获取 Provider 统计
    pub fn get_provider_stats(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
        app_type: Option<&str>,
        provider_name: Option<&str>,
        model: Option<&str>,
    ) -> Result<Vec<ProviderStats>, AppError> {
        let conn = lock_conn!(self.conn);

        let mut detail_conditions: Vec<String> = Vec::new();
        let mut detail_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(start) = start_date {
            detail_conditions.push("l.created_at >= ?".to_string());
            detail_params.push(Box::new(start));
        }
        if let Some(end) = end_date {
            detail_conditions.push("l.created_at <= ?".to_string());
            detail_params.push(Box::new(end));
        }
        if let Some(at) = app_type {
            detail_conditions.push(format!("{} = ?", folded_app_type_sql("l.app_type")));
            detail_params.push(Box::new(at.to_string()));
        }
        push_provider_model_filters(
            &mut detail_conditions,
            &mut detail_params,
            "l",
            "p",
            provider_name,
            model,
        );
        let detail_where = if detail_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", detail_conditions.join(" AND "))
        };

        let mut rollup_conditions = Vec::new();
        let mut rollup_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let rollup_bounds = compute_rollup_date_bounds(start_date, end_date)?;
        push_rollup_date_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r.date",
            &rollup_bounds,
        );
        if let Some(at) = app_type {
            rollup_conditions.push(format!("{} = ?", folded_app_type_sql("r.app_type")));
            rollup_params.push(Box::new(at.to_string()));
        }
        push_provider_model_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r",
            "p2",
            provider_name,
            model,
        );
        let rollup_where = if rollup_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", rollup_conditions.join(" AND "))
        };

        // UNION detail logs + rollup data, then aggregate
        let detail_pname = provider_name_coalesce("l", "p");
        let rollup_pname = provider_name_coalesce("r", "p2");
        let real_total_detail = real_total_tokens_sql("l");
        let real_total_rollup = real_total_tokens_sql("r");
        let speed_ok = speed_eligible_sql("l");
        let est_ok = speed_estimate_eligible_sql("l");
        let sql = format!(
            "SELECT
                provider_id, app_type, provider_name,
                SUM(request_count) as request_count,
                SUM(total_tokens) as total_tokens,
                SUM(total_cost) as total_cost,
                SUM(success_count) as success_count,
                CASE WHEN SUM(request_count) > 0
                    THEN SUM(latency_sum) / SUM(request_count)
                    ELSE 0 END as avg_latency,
                SUM(speed_output) as speed_output,
                SUM(speed_gen_ms) as speed_gen_ms,
                SUM(est_output) as est_output,
                SUM(est_ms) as est_ms
            FROM (
                SELECT l.provider_id, l.app_type,
                    {detail_pname} as provider_name,
                    COUNT(*) as request_count,
                    COALESCE(SUM({real_total_detail}), 0) as total_tokens,
                    COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0) as total_cost,
                    COALESCE(SUM(CASE WHEN l.status_code >= 200 AND l.status_code < 300 THEN 1 ELSE 0 END), 0) as success_count,
                    COALESCE(SUM(l.latency_ms), 0) as latency_sum,
                    COALESCE(SUM(CASE WHEN {speed_ok} THEN l.output_tokens ELSE 0 END), 0) as speed_output,
                    COALESCE(SUM(CASE WHEN {speed_ok} THEN l.latency_ms - l.first_token_ms ELSE 0 END), 0) as speed_gen_ms,
                    COALESCE(SUM(CASE WHEN {est_ok} THEN l.output_tokens ELSE 0 END), 0) as est_output,
                    COALESCE(SUM(CASE WHEN {est_ok} THEN l.latency_ms ELSE 0 END), 0) as est_ms
                FROM proxy_request_logs l
                LEFT JOIN providers p ON l.provider_id = p.id AND l.app_type = p.app_type
                {detail_where}
                GROUP BY l.provider_id, l.app_type
                UNION ALL
                SELECT r.provider_id, r.app_type,
                    {rollup_pname} as provider_name,
                    COALESCE(SUM(r.request_count), 0),
                    COALESCE(SUM({real_total_rollup}), 0),
                    COALESCE(SUM(CAST(r.total_cost_usd AS REAL)), 0),
                    COALESCE(SUM(r.success_count), 0),
                    COALESCE(SUM(r.avg_latency_ms * r.request_count), 0),
                    0,
                    0,
                    0,
                    0
                FROM usage_daily_rollups r
                LEFT JOIN providers p2 ON r.provider_id = p2.id AND r.app_type = p2.app_type
                {rollup_where}
                GROUP BY r.provider_id, r.app_type
            )
            GROUP BY provider_id, app_type
            ORDER BY total_cost DESC"
        );

        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = detail_params;
        params.extend(rollup_params);
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let row_mapper = |row: &rusqlite::Row| {
            let request_count: i64 = row.get(3)?;
            let success_count: i64 = row.get(6)?;
            let success_rate = if request_count > 0 {
                (success_count as f32 / request_count as f32) * 100.0
            } else {
                0.0
            };

            Ok(ProviderStats {
                provider_id: row.get(0)?,
                provider_name: row.get(2)?,
                request_count: request_count as u64,
                total_tokens: row.get::<_, i64>(4)? as u64,
                total_cost: format!("{:.6}", row.get::<_, f64>(5)?),
                success_rate,
                avg_latency_ms: row.get::<_, f64>(7)? as u64,
                speed_output_tokens: row.get::<_, i64>(8)?.max(0) as u64,
                speed_generation_ms: row.get::<_, i64>(9)?.max(0) as u64,
                est_speed_output_tokens: row.get::<_, i64>(10)?.max(0) as u64,
                est_speed_duration_ms: row.get::<_, i64>(11)?.max(0) as u64,
            })
        };

        let rows = stmt.query_map(param_refs.as_slice(), row_mapper)?;

        let mut stats = Vec::new();
        for row in rows {
            stats.push(row?);
        }

        Ok(stats)
    }

    /// 获取模型统计
    pub fn get_model_stats(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
        app_type: Option<&str>,
        provider_name: Option<&str>,
        model: Option<&str>,
    ) -> Result<Vec<ModelStats>, AppError> {
        let conn = lock_conn!(self.conn);

        let mut detail_conditions: Vec<String> = Vec::new();
        let mut detail_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(start) = start_date {
            detail_conditions.push("l.created_at >= ?".to_string());
            detail_params.push(Box::new(start));
        }
        if let Some(end) = end_date {
            detail_conditions.push("l.created_at <= ?".to_string());
            detail_params.push(Box::new(end));
        }
        if let Some(at) = app_type {
            detail_conditions.push(format!("{} = ?", folded_app_type_sql("l.app_type")));
            detail_params.push(Box::new(at.to_string()));
        }
        push_provider_model_filters(
            &mut detail_conditions,
            &mut detail_params,
            "l",
            "p",
            provider_name,
            model,
        );
        let detail_where = if detail_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", detail_conditions.join(" AND "))
        };
        let detail_join = if provider_name.is_some() {
            providers_join("l", "p")
        } else {
            String::new()
        };

        let mut rollup_conditions = Vec::new();
        let mut rollup_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let rollup_bounds = compute_rollup_date_bounds(start_date, end_date)?;
        push_rollup_date_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r.date",
            &rollup_bounds,
        );
        if let Some(at) = app_type {
            rollup_conditions.push(format!("{} = ?", folded_app_type_sql("r.app_type")));
            rollup_params.push(Box::new(at.to_string()));
        }
        push_provider_model_filters(
            &mut rollup_conditions,
            &mut rollup_params,
            "r",
            "p2",
            provider_name,
            model,
        );
        let rollup_where = if rollup_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", rollup_conditions.join(" AND "))
        };
        let rollup_join = if provider_name.is_some() {
            providers_join("r", "p2")
        } else {
            String::new()
        };

        // UNION detail logs + rollup data
        //
        // 分组键用「有效计价模型」：pricing_model 非空时优先（成本就是按它的
        // 定价算的，金额与定价表自洽），NULL/'' 回落 model。默认 response 计价
        // 模式下两者相同，行为不变；request 模式 + 路由接管下，钱挂在实际计价
        // 基准名下，而不是上游回显/客户端别名名下。
        let real_total_detail = real_total_tokens_sql("l");
        let real_total_rollup = real_total_tokens_sql("r");
        let detail_model = effective_model_sql("l");
        let rollup_model = effective_model_sql("r");
        let speed_ok = speed_eligible_sql("l");
        let est_ok = speed_estimate_eligible_sql("l");
        let sql = format!(
            "SELECT
                model,
                SUM(request_count) as request_count,
                SUM(total_tokens) as total_tokens,
                SUM(total_cost) as total_cost,
                SUM(success_count) as success_count,
                SUM(speed_output) as speed_output,
                SUM(speed_gen_ms) as speed_gen_ms,
                SUM(est_output) as est_output,
                SUM(est_ms) as est_ms
            FROM (
                SELECT {detail_model} as model,
                    COUNT(*) as request_count,
                    COALESCE(SUM({real_total_detail}), 0) as total_tokens,
                    COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0) as total_cost,
                    COALESCE(SUM(CASE WHEN l.status_code >= 200 AND l.status_code < 300 THEN 1 ELSE 0 END), 0) as success_count,
                    COALESCE(SUM(CASE WHEN {speed_ok} THEN l.output_tokens ELSE 0 END), 0) as speed_output,
                    COALESCE(SUM(CASE WHEN {speed_ok} THEN l.latency_ms - l.first_token_ms ELSE 0 END), 0) as speed_gen_ms,
                    COALESCE(SUM(CASE WHEN {est_ok} THEN l.output_tokens ELSE 0 END), 0) as est_output,
                    COALESCE(SUM(CASE WHEN {est_ok} THEN l.latency_ms ELSE 0 END), 0) as est_ms
                FROM proxy_request_logs l
                {detail_join}
                {detail_where}
                GROUP BY {detail_model}
                UNION ALL
                SELECT {rollup_model},
                    COALESCE(SUM(r.request_count), 0),
                    COALESCE(SUM({real_total_rollup}), 0),
                    COALESCE(SUM(CAST(r.total_cost_usd AS REAL)), 0),
                    COALESCE(SUM(r.success_count), 0),
                    0,
                    0,
                    0,
                    0
                FROM usage_daily_rollups r
                {rollup_join}
                {rollup_where}
                GROUP BY {rollup_model}
            )
            GROUP BY model
            ORDER BY total_cost DESC"
        );

        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = detail_params;
        params.extend(rollup_params);
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let row_mapper = |row: &rusqlite::Row| {
            let request_count: i64 = row.get(1)?;
            let total_cost: f64 = row.get(3)?;
            let avg_cost = if request_count > 0 {
                total_cost / request_count as f64
            } else {
                0.0
            };
            let success_count: i64 = row.get(4)?;
            let success_rate = if request_count > 0 {
                (success_count as f32 / request_count as f32) * 100.0
            } else {
                0.0
            };

            Ok(ModelStats {
                model: row.get(0)?,
                request_count: request_count as u64,
                total_tokens: row.get::<_, i64>(2)? as u64,
                total_cost: format!("{total_cost:.6}"),
                avg_cost_per_request: format!("{avg_cost:.6}"),
                success_rate,
                speed_output_tokens: row.get::<_, i64>(5)?.max(0) as u64,
                speed_generation_ms: row.get::<_, i64>(6)?.max(0) as u64,
                est_speed_output_tokens: row.get::<_, i64>(7)?.max(0) as u64,
                est_speed_duration_ms: row.get::<_, i64>(8)?.max(0) as u64,
                has_pricing: false,
            })
        };

        let rows = stmt.query_map(param_refs.as_slice(), row_mapper)?;

        let mut stats = Vec::new();
        for row in rows {
            let mut stat = row?;
            // 分组键就是有效计价模型
            stat.has_pricing = !is_placeholder_pricing_model(&stat.model)
                && find_model_pricing_row(&conn, &stat.model)?.is_some();
            stats.push(stat);
        }

        Ok(stats)
    }

    /// 获取请求日志列表（分页）
    pub fn get_request_logs(
        &self,
        filters: &LogFilters,
        page: u32,
        page_size: u32,
    ) -> Result<PaginatedLogs, AppError> {
        let conn = lock_conn!(self.conn);

        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref app_type) = filters.app_type {
            // 仅过滤口径折叠 claude-desktop→claude；行投影仍返回原始 app_type，
            // 详情面板据此展示真实入口（路由接管账单审计需要）。
            conditions.push(format!("{} = ?", folded_app_type_sql("l.app_type")));
            params.push(Box::new(app_type.clone()));
        }
        // 与 Dashboard 顶部下拉筛选同口径：Provider 按展示名精确匹配（会话占位
        // 行如 "Claude (Session)" 也能命中），模型按有效计价模型匹配。
        push_provider_model_filters(
            &mut conditions,
            &mut params,
            "l",
            "p",
            filters.provider_name.as_deref(),
            filters.model.as_deref(),
        );
        if let Some(status) = filters.status_code {
            conditions.push("l.status_code = ?".to_string());
            params.push(Box::new(status as i64));
        }
        if let Some(start) = filters.start_date {
            conditions.push("l.created_at >= ?".to_string());
            params.push(Box::new(start));
        }
        if let Some(end) = filters.end_date {
            conditions.push("l.created_at <= ?".to_string());
            params.push(Box::new(end));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // 获取总数
        let count_sql = format!(
            "SELECT COUNT(*) FROM proxy_request_logs l
             LEFT JOIN providers p ON l.provider_id = p.id AND l.app_type = p.app_type
             {where_clause}"
        );
        let mut count_cache = lock_conn!(self.log_count_cache);
        let total: u32 = match cached_log_count(&count_cache, &conn, filters)? {
            Some(total) => total,
            None => {
                let count_params: Vec<&dyn rusqlite::ToSql> =
                    params.iter().map(|p| p.as_ref()).collect();
                let total = conn.query_row(&count_sql, count_params.as_slice(), |row| {
                    row.get::<_, i64>(0).map(|v| v as u32)
                })?;
                *count_cache = Some(LogCountCache {
                    key: log_count_cache_key(filters),
                    end_date: filters.end_date,
                    changes: connection_total_changes(&conn)?,
                    computed_at: std::time::Instant::now(),
                    total,
                });
                total
            }
        };
        drop(count_cache);

        // 获取数据
        let offset = page * page_size;
        params.push(Box::new(page_size as i64));
        params.push(Box::new(offset as i64));

        let logs_pname = provider_name_coalesce("l", "p");
        let sql = format!(
            "SELECT l.request_id, l.provider_id, {logs_pname} as provider_name, l.app_type, l.model,
                    l.request_model, l.cost_multiplier,
                    l.input_tokens, l.output_tokens, l.cache_read_tokens, l.cache_creation_tokens,
                    l.input_cost_usd, l.output_cost_usd, l.cache_read_cost_usd, l.cache_creation_cost_usd, l.total_cost_usd,
                    l.is_streaming, l.latency_ms, l.first_token_ms, l.duration_ms,
                    l.status_code, l.error_message, l.created_at, l.data_source, l.pricing_model,
                    l.input_token_semantics
             FROM proxy_request_logs l
             LEFT JOIN providers p ON l.provider_id = p.id AND l.app_type = p.app_type
             {where_clause}
             ORDER BY l.created_at DESC
             LIMIT ? OFFSET ?"
        );

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), row_to_request_log_detail)?;

        let mut logs = rows.collect::<Result<Vec<_>, _>>()?;
        fill_has_pricing(&conn, &mut logs)?;

        Ok(PaginatedLogs {
            data: logs,
            total,
            page,
            page_size,
        })
    }

    /// 获取单个请求详情
    pub fn get_request_detail(
        &self,
        request_id: &str,
    ) -> Result<Option<RequestLogDetail>, AppError> {
        let conn = lock_conn!(self.conn);

        let detail_pname = provider_name_coalesce("l", "p");
        let detail_sql = format!(
            "SELECT l.request_id, l.provider_id, {detail_pname} as provider_name, l.app_type, l.model,
                    l.request_model, l.cost_multiplier,
                    l.input_tokens, l.output_tokens, l.cache_read_tokens, l.cache_creation_tokens,
                    l.input_cost_usd, l.output_cost_usd, l.cache_read_cost_usd, l.cache_creation_cost_usd, l.total_cost_usd,
                    l.is_streaming, l.latency_ms, l.first_token_ms, l.duration_ms,
                    l.status_code, l.error_message, l.created_at, l.data_source, l.pricing_model,
                    l.input_token_semantics
             FROM proxy_request_logs l
             LEFT JOIN providers p ON l.provider_id = p.id AND l.app_type = p.app_type
             WHERE l.request_id = ?"
        );
        let result = conn.query_row(&detail_sql, [request_id], row_to_request_log_detail);

        match result {
            Ok(detail) => {
                let mut logs = [detail];
                fill_has_pricing(&conn, &mut logs)?;
                let [detail] = logs;
                Ok(Some(detail))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }
}

/// 重算成本时读出的一行明细。
struct RepriceRow {
    request_id: String,
    model: String,
    request_model: Option<String>,
    pricing_model: Option<String>,
    usage: TokenUsage,
    service_tier: ServiceTier,
    /// 一行是多次请求的合计（Grok Build 按轮记录），不按超长上下文档位计
    sums_requests: bool,
    created_at: i64,
    stored: [String; 5],
}

impl Database {
    /// 按当前定价重算全部本地计价的明细成本。
    ///
    /// 重算后记下当前定价的指纹，见 [`Self::reprice_usage_costs_if_pricing_changed`]。
    pub(crate) fn reprice_usage_costs(&self) -> Result<u64, AppError> {
        let conn = lock_conn!(self.conn);
        let repriced = Self::reprice_usage_costs_on_conn(&conn, None, None)?;
        Self::store_pricing_fingerprint(&conn)?;
        Ok(repriced)
    }

    /// 只重算和 `model_id` 相关的明细；用于单个模型的定价更新。不更新定价指纹：
    /// 别的模型的定价可能也变了而没重算，下次启动时会全量重算一次。
    pub(crate) fn reprice_usage_costs_for_model(&self, model_id: &str) -> Result<u64, AppError> {
        let conn = lock_conn!(self.conn);
        Self::reprice_usage_costs_on_conn(&conn, Some(model_id), None)
    }

    /// 定价或计价规则和上次全量重算时不同才重算（启动时用）。返回 `None` 表示都没变。
    pub(crate) fn reprice_usage_costs_if_pricing_changed(&self) -> Result<Option<u64>, AppError> {
        let conn = lock_conn!(self.conn);
        let current = Self::pricing_fingerprint(&conn)?;
        let stored: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [USAGE_PRICING_FINGERPRINT_KEY],
                |row| row.get(0),
            )
            .optional()?;
        if stored.as_deref() == Some(current.as_str()) {
            return Ok(None);
        }
        let repriced = Self::reprice_usage_costs_on_conn(&conn, None, None)?;
        Self::write_pricing_fingerprint(&conn, &current)?;
        Ok(Some(repriced))
    }

    fn store_pricing_fingerprint(conn: &Connection) -> Result<(), AppError> {
        let fingerprint = Self::pricing_fingerprint(conn)?;
        Self::write_pricing_fingerprint(conn, &fingerprint)
    }

    fn write_pricing_fingerprint(conn: &Connection, fingerprint: &str) -> Result<(), AppError> {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![USAGE_PRICING_FINGERPRINT_KEY, fingerprint],
        )
        .map_err(|e| AppError::Database(format!("保存定价指纹失败: {e}")))?;
        Ok(())
    }

    /// 定价的指纹：计价规则（见 [`pricing_rules_text`]）加上 `model_pricing` 全表按
    /// model_id 排序后所有列，取 SHA-256。
    fn pricing_fingerprint(conn: &Connection) -> Result<String, AppError> {
        Self::pricing_fingerprint_with(conn, &pricing_rules_text())
    }

    fn pricing_fingerprint_with(conn: &Connection, rules: &str) -> Result<String, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM model_pricing ORDER BY model_id")?;
        let columns = stmt.column_count();
        let mut text = String::from(rules);
        text.push('\u{1d}');
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            for index in 0..columns {
                let value = match row.get_ref(index)? {
                    rusqlite::types::ValueRef::Null => String::from("\\N"),
                    rusqlite::types::ValueRef::Integer(v) => v.to_string(),
                    rusqlite::types::ValueRef::Real(v) => v.to_string(),
                    rusqlite::types::ValueRef::Text(v) | rusqlite::types::ValueRef::Blob(v) => {
                        String::from_utf8_lossy(v).into_owned()
                    }
                };
                text.push_str(&value);
                text.push('\u{1f}');
            }
            text.push('\u{1e}');
        }
        Ok(crate::live::engine::sha256_hex(text.as_bytes()))
    }

    /// 成本是 token 和定价算出来的派生值：定价补上、改了、删了，已入库的明细都按
    /// 当前定价重算，查不到定价的记 0。
    ///
    /// 不动两类行：工具日志自带成本的（`native_cost = 1`，OpenCode、Pi、Grok、mcode
    /// 用工具记下的费用），和旧版本地路由记录的行（当时按上游计价，倍率也已作废）。
    ///
    /// `before` 只重算早于这个时间的行（汇总前用）。
    pub(crate) fn reprice_usage_costs_on_conn(
        conn: &Connection,
        only_model_id: Option<&str>,
        before: Option<i64>,
    ) -> Result<u64, AppError> {
        let mut rows = {
            let mut stmt = conn.prepare(
                "SELECT request_id, app_type, model, request_model, pricing_model,
                        input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                        cache_creation_1h_tokens, input_token_semantics, service_tier,
                        input_cost_usd, output_cost_usd, cache_read_cost_usd,
                        cache_creation_cost_usd, total_cost_usd, created_at
                 FROM proxy_request_logs
                 WHERE native_cost = 0
                   AND COALESCE(data_source, 'proxy') <> 'proxy'
                   AND (?1 IS NULL OR created_at < ?1)
                   AND (input_tokens > 0 OR output_tokens > 0
                        OR cache_read_tokens > 0 OR cache_creation_tokens > 0)",
            )?;
            let mapped = stmt.query_map([before], |row| {
                let app_type: String = row.get(1)?;
                let input_tokens: i64 = row.get(5)?;
                let cache_read_tokens: i64 = row.get(7)?;
                let cache_creation_tokens: i64 = row.get(8)?;
                let semantics: i64 = row.get(10)?;
                Ok(RepriceRow {
                    request_id: row.get(0)?,
                    model: row.get(2)?,
                    request_model: row.get(3)?,
                    pricing_model: row.get(4)?,
                    usage: TokenUsage {
                        input_tokens: fresh_input_tokens(
                            &app_type,
                            semantics,
                            input_tokens,
                            cache_read_tokens,
                            cache_creation_tokens,
                        ),
                        output_tokens: clamp_u32(row.get(6)?),
                        cache_read_tokens: clamp_u32(cache_read_tokens),
                        cache_creation_tokens: clamp_u32(cache_creation_tokens),
                        cache_creation_1h_tokens: clamp_u32(row.get(9)?),
                    },
                    service_tier: ServiceTier::from_db_str(&row.get::<_, String>(11)?),
                    sums_requests: app_type == "grokbuild",
                    created_at: row.get(17)?,
                    stored: [
                        row.get(12)?,
                        row.get(13)?,
                        row.get(14)?,
                        row.get(15)?,
                        row.get(16)?,
                    ],
                })
            })?;
            mapped.collect::<Result<Vec<_>, _>>()?
        };

        // 精准重算的行筛选必须与查价层共用 candidates 归一化：SQL 精确匹配会漏掉
        // 以原始别名落库的行（如 openrouter/anthropic/claude-sonnet-4.5:free）。
        // 误纳无害——重算结果不变的行不会写回。
        if let Some(model_id) = only_model_id {
            let target = model_pricing_candidates(model_id);
            rows.retain(|row| {
                pricing_scope_matches(
                    [
                        Some(row.model.as_str()),
                        row.request_model.as_deref(),
                        row.pricing_model.as_deref(),
                    ],
                    &target,
                )
            });
        }
        if rows.is_empty() {
            return Ok(0);
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(format!("启动用量成本重算事务失败: {e}")))?;
        let mut pricing_cache: HashMap<String, Option<ModelPricing>> = HashMap::new();
        let mut updated = 0u64;
        {
            let mut update = tx.prepare(
                "UPDATE proxy_request_logs
                 SET input_cost_usd = ?1, output_cost_usd = ?2, cache_read_cost_usd = ?3,
                     cache_creation_cost_usd = ?4, total_cost_usd = ?5
                 WHERE request_id = ?6",
            )?;
            for row in &rows {
                let pricing = row_pricing(&tx, &mut pricing_cache, row)?.map(|p| {
                    if row.sums_requests {
                        p.without_long_context()
                    } else {
                        p
                    }
                });
                let costs = match pricing {
                    Some(pricing) => CostCalculator::calculate(
                        &row.usage,
                        &pricing,
                        row.service_tier,
                        row.created_at,
                    )
                    .to_strings(),
                    None => CostBreakdown::zero_strings(),
                };
                if costs_equal(&costs, &row.stored) {
                    continue;
                }
                update
                    .execute(params![
                        costs[0],
                        costs[1],
                        costs[2],
                        costs[3],
                        costs[4],
                        row.request_id
                    ])
                    .map_err(|e| AppError::Database(format!("更新请求成本失败: {e}")))?;
                updated += 1;
            }
        }
        tx.commit()
            .map_err(|e| AppError::Database(format!("提交用量成本重算事务失败: {e}")))?;

        if updated > 0 {
            log::info!("已按当前定价重算 {updated} 条用量成本");
        }
        Ok(updated)
    }
}

/// 明细行的计价基准：写入时记下的 `pricing_model`，否则 `model`；`model` 是解析失败
/// 留下的占位符（""、"unknown"）时才退回 `request_model`。
fn pricing_base<'a>(
    pricing_model: Option<&'a str>,
    model: &'a str,
    request_model: Option<&'a str>,
) -> Option<&'a str> {
    [pricing_model, Some(model), request_model]
        .into_iter()
        .flatten()
        .find(|name| !is_placeholder_pricing_model(name))
}

fn cached_pricing(
    conn: &Connection,
    cache: &mut HashMap<String, Option<ModelPricing>>,
    model: &str,
) -> Result<Option<ModelPricing>, AppError> {
    if let Some(found) = cache.get(model) {
        return Ok(found.clone());
    }
    let found = find_model_pricing_row(conn, model)?;
    cache.insert(model.to_string(), found.clone());
    Ok(found)
}

fn row_pricing(
    conn: &Connection,
    cache: &mut HashMap<String, Option<ModelPricing>>,
    row: &RepriceRow,
) -> Result<Option<ModelPricing>, AppError> {
    match pricing_base(
        row.pricing_model.as_deref(),
        &row.model,
        row.request_model.as_deref(),
    ) {
        Some(model) => cached_pricing(conn, cache, model),
        None => Ok(None),
    }
}

/// 按 [`row_pricing`] 的规则给每行填 `has_pricing`。
fn fill_has_pricing(conn: &Connection, logs: &mut [RequestLogDetail]) -> Result<(), AppError> {
    let mut cache = HashMap::new();
    for log in logs {
        log.has_pricing = match pricing_base(
            log.pricing_model.as_deref(),
            &log.model,
            log.request_model.as_deref(),
        ) {
            Some(model) => cached_pricing(conn, &mut cache, model)?.is_some(),
            None => false,
        };
    }
    Ok(())
}

/// 数值相同就算相等（"0" 与 "0.000000"、小数位数不同的旧值）。
fn costs_equal(new: &[String; 5], stored: &[String; 5]) -> bool {
    new.iter().zip(stored).all(|(a, b)| {
        match (
            rust_decimal::Decimal::from_str(a),
            rust_decimal::Decimal::from_str(b),
        ) {
            (Ok(a), Ok(b)) => a == b,
            _ => a == b,
        }
    })
}

fn clamp_u32(value: i64) -> u32 {
    value.clamp(0, i64::from(u32::MAX)) as u32
}

/// 把明细里存的 input 换成未命中缓存的输入（与 [`fresh_input_sql`] 同一规则）。
fn fresh_input_tokens(
    app_type: &str,
    semantics: i64,
    input_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
) -> u32 {
    // 分支和保护条件与 SQL 版一致：缓存比输入还多的旧行原样返回输入
    let inclusive = crate::services::sql_helpers::is_cache_inclusive_app(app_type);
    let fresh = if semantics == INPUT_TOKEN_SEMANTICS_FRESH {
        input_tokens
    } else if inclusive
        && semantics == INPUT_TOKEN_SEMANTICS_TOTAL
        && input_tokens >= cache_read_tokens + cache_creation_tokens
    {
        input_tokens - cache_read_tokens - cache_creation_tokens
    } else if inclusive
        && semantics == INPUT_TOKEN_SEMANTICS_LEGACY
        && input_tokens >= cache_read_tokens
    {
        // v12 及更早：input 含缓存读，不含缓存写
        input_tokens - cache_read_tokens
    } else {
        input_tokens
    };
    clamp_u32(fresh)
}

pub(crate) fn find_model_pricing(conn: &Connection, model_id: &str) -> Option<ModelPricing> {
    find_model_pricing_row(conn, model_id).ok().flatten()
}

pub(crate) fn find_model_pricing_row(
    conn: &Connection,
    model_id: &str,
) -> Result<Option<ModelPricing>, AppError> {
    let candidates = model_pricing_candidates(model_id);
    if candidates.is_empty() {
        return Ok(None);
    }

    for candidate in &candidates {
        if let Some(row) = query_model_pricing(conn, PricingMatch::Exact, candidate)? {
            return Ok(Some(row));
        }
    }

    for candidate in &candidates {
        if should_try_pricing_prefix_match(candidate) {
            if let Some(row) = query_model_pricing(conn, PricingMatch::Prefix, candidate)? {
                return Ok(Some(row));
            }
        }
    }

    Ok(None)
}

/// 精准重算的行筛选：行的任一模型字段归一化后与目标模型的 candidates 相交，
/// 或可按查价层的前缀规则命中目标，即视为相关。镜像 find_model_pricing_row 的
/// 匹配语义，宁可误纳（重算结果不变的行不会写回）不可漏筛。
fn pricing_scope_matches<'a>(
    fields: impl IntoIterator<Item = Option<&'a str>>,
    target_candidates: &[String],
) -> bool {
    fields.into_iter().flatten().any(|field| {
        model_pricing_candidates(field).iter().any(|candidate| {
            target_candidates.iter().any(|target| {
                target == candidate
                    || (should_try_pricing_prefix_match(candidate)
                        && target
                            .strip_prefix(candidate.as_str())
                            .is_some_and(|rest| rest.starts_with('-')))
            })
        })
    })
}

pub(crate) fn is_placeholder_pricing_model(model_id: &str) -> bool {
    let normalized = model_id.trim().to_ascii_lowercase();
    normalized.is_empty() || matches!(normalized.as_str(), "unknown" | "null" | "none")
}

#[derive(Clone, Copy)]
enum PricingMatch {
    Exact,
    /// `<model>-%` 里最短的一行
    Prefix,
}

fn query_model_pricing(
    conn: &Connection,
    mode: PricingMatch,
    model_id: &str,
) -> Result<Option<ModelPricing>, AppError> {
    const COLUMNS: &str = "model_id, input_cost_per_million, output_cost_per_million,
        cache_read_cost_per_million, cache_creation_cost_per_million,
        long_context_tiers, priority_multiplier";
    let (sql, param) = match mode {
        PricingMatch::Exact => (
            format!("SELECT {COLUMNS} FROM model_pricing WHERE model_id = ?1"),
            model_id.to_string(),
        ),
        PricingMatch::Prefix => (
            format!(
                "SELECT {COLUMNS} FROM model_pricing WHERE model_id LIKE ?1
                 ORDER BY LENGTH(model_id) ASC LIMIT 1"
            ),
            format!("{model_id}-%"),
        ),
    };
    let row = conn
        .query_row(&sql, [param], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .optional()
        .map_err(|e| AppError::Database(format!("查询模型定价失败: {e}")))?;
    let Some((matched_id, input, output, cache_read, cache_creation, tiers, priority)) = row else {
        return Ok(None);
    };
    let parse = |label: &str, value: &str| {
        rust_decimal::Decimal::from_str(value)
            .map_err(|e| AppError::Database(format!("解析模型 {model_id} 的{label}失败: {e}")))
    };
    let long_context_tiers =
        crate::services::model_pricing::long_context_tiers_from_json(&matched_id, &tiers)
            .into_iter()
            .filter(|tier| tier.threshold_tokens > 0)
            .map(|tier| {
                Ok(LongContextPricing {
                    threshold_tokens: tier.threshold_tokens as u64,
                    input_multiplier: parse("超长上下文输入倍率", &tier.input_multiplier)?,
                    output_multiplier: parse("超长上下文输出倍率", &tier.output_multiplier)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
    let pricing = ModelPricing {
        prices: BasePrices {
            input: parse("输入价格", &input)?,
            output: parse("输出价格", &output)?,
            cache_read: parse("缓存读取价格", &cache_read)?,
            cache_creation: parse("缓存写入价格", &cache_creation)?,
        },
        earlier: Vec::new(),
        long_context_tiers,
        priority_multiplier: parse("priority 倍率", &priority)?,
    };
    Ok(Some(with_price_history(&matched_id, pricing)))
}

fn model_pricing_candidates(model_id: &str) -> Vec<String> {
    let cleaned = clean_model_id_for_pricing(model_id);
    if is_placeholder_pricing_model(&cleaned) {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let mut queue = vec![cleaned];

    while let Some(candidate) = queue.pop() {
        if !push_unique_candidate(&mut candidates, candidate.clone()) {
            continue;
        }

        if let Some(stripped) = strip_known_model_namespace(&candidate) {
            queue.push(stripped);
        }
        if let Some(stripped) = strip_claude_desktop_non_anthropic_prefix(&candidate) {
            queue.push(stripped);
        }
        if let Some(stripped) = strip_bedrock_model_version_suffix(&candidate) {
            queue.push(stripped);
        }
        if let Some(stripped) = strip_model_date_suffix(&candidate) {
            queue.push(stripped);
        }
        if let Some(stripped) = strip_reasoning_effort_suffix(&candidate) {
            queue.push(stripped);
        }
        if candidate.starts_with("claude-") && candidate.contains('.') {
            queue.push(candidate.replace('.', "-"));
        }
    }

    candidates
}

fn clean_model_id_for_pricing(model_id: &str) -> String {
    let normalized = model_id
        .rsplit_once('/')
        .map_or(model_id, |(_, r)| r)
        .split(':')
        .next()
        .unwrap_or(model_id)
        .trim()
        .replace('@', "-")
        .to_ascii_lowercase();

    normalized
        .trim_end_matches(crate::model_capabilities::ONE_M_CONTEXT_MARKER)
        .trim()
        .to_string()
}

fn push_unique_candidate(candidates: &mut Vec<String>, candidate: String) -> bool {
    if candidate.is_empty() || candidates.iter().any(|existing| existing == &candidate) {
        return false;
    }
    candidates.push(candidate);
    true
}

fn strip_known_model_namespace(model_id: &str) -> Option<String> {
    if let Some(pos) = model_id.rfind("claude-") {
        if pos > 0 {
            return Some(model_id[pos..].to_string());
        }
    }

    for marker in [
        "openai.",
        "anthropic.",
        "google.",
        "moonshot.",
        "moonshotai.",
        "bedrock.",
        "global.",
    ] {
        if let Some(stripped) = model_id.strip_prefix(marker) {
            return Some(stripped.to_string());
        }
    }

    None
}

fn strip_claude_desktop_non_anthropic_prefix(model_id: &str) -> Option<String> {
    const NON_ANTHROPIC_MARKERS: &[&str] = &[
        "abab",
        "ark-code",
        "arctic",
        "astron",
        "codex",
        "command-r",
        "deepseek",
        "doubao",
        "ernie",
        "gemini",
        "gemma",
        "glm",
        "gpt",
        "grok",
        "hermes",
        "hy3",
        "hunyuan",
        "jamba",
        "kimi",
        "lfm",
        "llama",
        "longcat",
        "mercury",
        "mimo",
        "minimax",
        "mistral",
        "mixtral",
        "moonshot",
        "nemotron",
        "nova-",
        "openai",
        "qianfan",
        "qwen",
        "seed-",
        "solar",
        "stepfun",
    ];

    let rest = model_id.strip_prefix("claude-")?;
    NON_ANTHROPIC_MARKERS
        .iter()
        .any(|marker| rest.starts_with(marker))
        .then(|| rest.to_string())
}

fn strip_bedrock_model_version_suffix(model_id: &str) -> Option<String> {
    let (base, suffix) = model_id.rsplit_once("-v")?;
    (!base.is_empty() && !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
        .then(|| base.to_string())
}

fn strip_model_date_suffix(model_id: &str) -> Option<String> {
    let bytes = model_id.as_bytes();
    if bytes.len() > 11 {
        let start = bytes.len() - 11;
        let suffix = &bytes[start..];
        let is_iso_date = suffix[0] == b'-'
            && suffix[1..5].iter().all(|b| b.is_ascii_digit())
            && suffix[5] == b'-'
            && suffix[6..8].iter().all(|b| b.is_ascii_digit())
            && suffix[8] == b'-'
            && suffix[9..11].iter().all(|b| b.is_ascii_digit());
        if is_iso_date {
            return Some(model_id[..start].to_string());
        }
    }

    let (base, suffix) = model_id.rsplit_once('-')?;
    if base.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // 8 位 YYYYMMDD（如 -20250615；OpenAI / Claude / 通义千问等）。
    if suffix.len() == 8 {
        return Some(base.to_string());
    }
    // 6 位 YYMMDD（如 -260628；火山方舟 doubao-seed-*、部分国产厂商）。
    // 6 位比 8 位更易误伤非日期尾巴（如 -123456 的版本号），故额外校验
    // 月 01-12、日 01-31 才剥离；剥不动时退回 None 由上层精确匹配兜底。
    if suffix.len() == 6 {
        let month: u32 = suffix[2..4].parse().unwrap_or(0);
        let day: u32 = suffix[4..6].parse().unwrap_or(0);
        if (1..=12).contains(&month) && (1..=31).contains(&day) {
            return Some(base.to_string());
        }
    }
    None
}

fn strip_reasoning_effort_suffix(model_id: &str) -> Option<String> {
    for suffix in ["-minimal", "-low", "-medium", "-high", "-xhigh"] {
        if let Some(stripped) = model_id.strip_suffix(suffix) {
            if !stripped.is_empty() {
                return Some(stripped.to_string());
            }
        }
    }
    None
}

fn should_try_pricing_prefix_match(model_id: &str) -> bool {
    let dash_count = model_id.matches('-').count();

    if model_id.starts_with("claude-") {
        return dash_count >= 3;
    }

    if ["o1", "o3", "o4", "o5"]
        .iter()
        .any(|prefix| model_id.starts_with(prefix))
    {
        return dash_count >= 1;
    }

    const PREFIX_MATCH_FAMILIES: &[&str] = &[
        "gpt-",
        "gemini-",
        "deepseek-",
        "qwen-",
        "glm-",
        "kimi-",
        "minimax-",
    ];

    PREFIX_MATCH_FAMILIES
        .iter()
        .any(|prefix| model_id.starts_with(prefix))
        && dash_count >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::sql_helpers::INPUT_TOKEN_SEMANTICS_LEGACY;

    fn local_ts(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> i64 {
        match Local.with_ymd_and_hms(year, month, day, hour, minute, second) {
            chrono::LocalResult::Single(dt) => dt.timestamp(),
            chrono::LocalResult::Ambiguous(earliest, _) => earliest.timestamp(),
            chrono::LocalResult::None => panic!("valid local datetime"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_usage_log(
        conn: &Connection,
        request_id: &str,
        app_type: &str,
        provider_id: &str,
        model: &str,
        data_source: &str,
        created_at: i64,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_creation_tokens: i64,
        status_code: i64,
        total_cost_usd: &str,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd,
                total_cost_usd, latency_ms, status_code, created_at, data_source
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, '0', '0', '0', '0', ?, 100, ?, ?, ?)",
            params![
                request_id,
                provider_id,
                app_type,
                model,
                model,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                total_cost_usd,
                status_code,
                created_at,
                data_source
            ],
        )?;
        Ok(())
    }

    /// 只有定价表里查不到的模型才是"未定价"；价格为 0 的模型查得到。
    #[test]
    fn has_pricing_follows_the_pricing_table_not_the_cost() -> Result<(), AppError> {
        let db = Database::memory()?;
        add_pricing(&db, "free-model", "0", "0")?;
        {
            let conn = lock_conn!(db.conn);
            for (id, model) in [("free", "free-model"), ("missing", "no-such-model-x")] {
                insert_usage_log(
                    &conn, id, "claude", "p", model, "proxy", 1_000, 10, 2, 0, 0, 200, "0",
                )?;
            }
        }

        let stats = db.get_model_stats(None, None, None, None, None)?;
        let priced = |model: &str| {
            stats
                .iter()
                .find(|s| s.model == model)
                .map(|s| s.has_pricing)
        };
        assert_eq!(priced("free-model"), Some(true));
        assert_eq!(priced("no-such-model-x"), Some(false));

        let filters = LogFilters::default();
        let logs = db.get_request_logs(&filters, 0, 10)?.data;
        let priced = |id: &str| {
            logs.iter()
                .find(|l| l.request_id == id)
                .map(|l| l.has_pricing)
        };
        assert_eq!(priced("free"), Some(true));
        assert_eq!(priced("missing"), Some(false));
        assert!(db.get_request_detail("free")?.unwrap().has_pricing);
        assert!(!db.get_request_detail("missing")?.unwrap().has_pricing);
        Ok(())
    }

    /// 日志总数缓存：没有写入时复用；插入、删除或结束时间推后有新行时都要重数。
    #[test]
    fn test_request_log_total_cache_invalidates_on_writes() -> Result<(), AppError> {
        let db = Database::memory()?;
        let insert = |id: &str, created_at: i64| -> Result<(), AppError> {
            let conn = lock_conn!(db.conn);
            insert_usage_log(
                &conn,
                id,
                "claude",
                "anthropic",
                "claude-sonnet-4-5",
                "proxy",
                created_at,
                10,
                2,
                0,
                0,
                200,
                "0.01",
            )
        };
        insert("a", 1_000)?;
        insert("b", 2_000)?;

        let mut filters = LogFilters {
            start_date: Some(0),
            end_date: Some(5_000),
            ..LogFilters::default()
        };
        assert_eq!(db.get_request_logs(&filters, 0, 10)?.total, 2);
        // 没有写入：复用缓存，结果不变
        assert_eq!(db.get_request_logs(&filters, 0, 10)?.total, 2);

        // 插入后必须重数
        insert("c", 3_000)?;
        assert_eq!(db.get_request_logs(&filters, 0, 10)?.total, 3);

        // 删除后也要重数
        {
            let conn = lock_conn!(db.conn);
            conn.execute("DELETE FROM proxy_request_logs WHERE request_id = 'a'", [])?;
        }
        assert_eq!(db.get_request_logs(&filters, 0, 10)?.total, 2);

        // 结束时间往后推：新增区间里没有行就复用，有行（之前就存在的未来时间行）就重数
        filters.end_date = Some(6_000);
        assert_eq!(db.get_request_logs(&filters, 0, 10)?.total, 2);
        insert("d", 9_000)?;
        filters.end_date = Some(5_000);
        assert_eq!(db.get_request_logs(&filters, 0, 10)?.total, 2);
        filters.end_date = Some(10_000);
        assert_eq!(db.get_request_logs(&filters, 0, 10)?.total, 3);

        Ok(())
    }

    #[test]
    fn test_claude_desktop_folds_into_claude_for_display() -> Result<(), AppError> {
        let db = Database::memory()?;
        let ts = local_ts(2026, 6, 10, 12, 0, 0);

        {
            let conn = lock_conn!(db.conn);
            // 一条 Claude Code 行 + 一条 Claude Desktop 网关行，同一时间窗。
            insert_usage_log(
                &conn,
                "cc-1",
                "claude",
                "p-claude",
                "claude-sonnet-4-5",
                "proxy",
                ts,
                100,
                10,
                0,
                0,
                200,
                "0.5",
            )?;
            insert_usage_log(
                &conn,
                "cd-1",
                "claude-desktop",
                "p-desktop",
                "claude-opus-4-8",
                "proxy",
                ts,
                200,
                20,
                0,
                0,
                200,
                "1.5",
            )?;
        }

        // ① 分应用汇总：desktop 折叠进 claude，不再单列 claude-desktop 桶。
        let by_app = db.get_usage_summary_by_app(None, None, None, None)?;
        assert_eq!(by_app.len(), 1, "应只剩一个合并后的 claude 桶");
        assert_eq!(by_app[0].app_type, "claude");
        assert_eq!(by_app[0].summary.total_requests, 2, "两条行都计入 claude");
        assert!(
            !by_app.iter().any(|a| a.app_type == "claude-desktop"),
            "不应再出现 claude-desktop 桶"
        );

        // ② 选中 claude 过滤：汇总应同时覆盖 desktop 行。
        let claude_summary = db.get_usage_summary(None, None, Some("claude"), None, None)?;
        assert_eq!(claude_summary.total_requests, 2);

        // ③ 请求日志按 claude 过滤返回两行，且 desktop 行投影仍是原始 app_type。
        let logs = db.get_request_logs(
            &LogFilters {
                app_type: Some("claude".to_string()),
                ..Default::default()
            },
            0, // 页码从 0 开始
            50,
        )?;
        assert_eq!(logs.total, 2, "claude 过滤含 desktop 行");
        assert!(
            logs.data.iter().any(|r| r.app_type == "claude-desktop"),
            "详情面板需要看到真实入口，行投影不可被折叠"
        );

        // ④ 折叠不外溢：codex 过滤为空。
        let codex_summary = db.get_usage_summary(None, None, Some("codex"), None, None)?;
        assert_eq!(codex_summary.total_requests, 0);

        Ok(())
    }

    /// 重算测试用的一行明细，未写的字段取会话导入的默认形态。
    struct RowSpec<'a> {
        request_id: &'a str,
        app_type: &'a str,
        model: &'a str,
        request_model: Option<&'a str>,
        pricing_model: Option<&'a str>,
        data_source: &'a str,
        semantics: i64,
        input: i64,
        output: i64,
        cache_read: i64,
        cache_creation: i64,
        cache_creation_1h: i64,
        service_tier: &'a str,
        native_cost: bool,
        total_cost: &'a str,
        created_at: i64,
    }

    impl Default for RowSpec<'_> {
        fn default() -> Self {
            Self {
                request_id: "row",
                app_type: "claude",
                model: "claude-opus-4-8",
                request_model: None,
                pricing_model: None,
                data_source: "session_log",
                semantics: INPUT_TOKEN_SEMANTICS_FRESH,
                input: 0,
                output: 0,
                cache_read: 0,
                cache_creation: 0,
                cache_creation_1h: 0,
                service_tier: "",
                native_cost: false,
                total_cost: "0",
                // 2026-09-21，晚于内置的调价记录
                created_at: 1_790_000_000,
            }
        }
    }

    fn insert_row(db: &Database, row: RowSpec<'_>) -> Result<(), AppError> {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model, pricing_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                cache_creation_1h_tokens, input_token_semantics, service_tier, native_cost,
                total_cost_usd, latency_ms, status_code, created_at, data_source
            ) VALUES (?1, 'p', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                      0, 200, ?16, ?15)",
            params![
                row.request_id,
                row.app_type,
                row.model,
                row.request_model.unwrap_or(row.model),
                row.pricing_model,
                row.input,
                row.output,
                row.cache_read,
                row.cache_creation,
                row.cache_creation_1h,
                row.semantics,
                row.service_tier,
                row.native_cost,
                row.total_cost,
                row.data_source,
                row.created_at,
            ],
        )?;
        Ok(())
    }

    fn stored_total(db: &Database, request_id: &str) -> Result<rust_decimal::Decimal, AppError> {
        let conn = lock_conn!(db.conn);
        let total: String = conn.query_row(
            "SELECT total_cost_usd FROM proxy_request_logs WHERE request_id = ?1",
            [request_id],
            |row| row.get(0),
        )?;
        Ok(rust_decimal::Decimal::from_str(&total).expect("decimal cost"))
    }

    fn dec(value: &str) -> rust_decimal::Decimal {
        rust_decimal::Decimal::from_str(value).unwrap()
    }

    fn add_pricing(
        db: &Database,
        model_id: &str,
        input: &str,
        output: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO model_pricing (model_id, display_name, input_cost_per_million, output_cost_per_million)
             VALUES (?1, ?1, ?2, ?3)",
            params![model_id, input, output],
        )?;
        Ok(())
    }

    #[test]
    fn reprice_fills_missing_and_corrects_stale_costs() -> Result<(), AppError> {
        let db = Database::memory()?;
        // claude-opus-4-8：$5 / $25
        insert_row(
            &db,
            RowSpec {
                request_id: "zero",
                input: 1000,
                output: 1000,
                ..Default::default()
            },
        )?;
        insert_row(
            &db,
            RowSpec {
                request_id: "stale",
                input: 1000,
                output: 1000,
                total_cost: "99",
                ..Default::default()
            },
        )?;

        assert_eq!(db.reprice_usage_costs()?, 2);
        assert_eq!(stored_total(&db, "zero")?, dec("0.03"));
        assert_eq!(stored_total(&db, "stale")?, dec("0.03"));
        // 已经是当前价的行不再写回
        assert_eq!(db.reprice_usage_costs()?, 0);
        Ok(())
    }

    #[test]
    fn rust_fresh_input_matches_the_sql_expression() -> Result<(), AppError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE t (app_type TEXT, input_token_semantics INTEGER, input_tokens INTEGER,
                             cache_read_tokens INTEGER, cache_creation_tokens INTEGER);",
        )?;
        let cases = [
            ("codex", INPUT_TOKEN_SEMANTICS_LEGACY, 1000, 300, 0),
            ("codex", INPUT_TOKEN_SEMANTICS_LEGACY, 100, 999, 0),
            ("codex", INPUT_TOKEN_SEMANTICS_TOTAL, 1000, 300, 200),
            ("codex", INPUT_TOKEN_SEMANTICS_TOTAL, 100, 300, 200),
            ("codex", INPUT_TOKEN_SEMANTICS_FRESH, 100, 300, 200),
            ("claude", INPUT_TOKEN_SEMANTICS_LEGACY, 100, 999, 0),
            ("claude", INPUT_TOKEN_SEMANTICS_TOTAL, 1000, 300, 200),
        ];
        let sql = format!(
            "SELECT {} FROM t",
            crate::services::sql_helpers::fresh_input_sql("")
        );
        for (app, semantics, input, read, creation) in cases {
            conn.execute("DELETE FROM t", [])?;
            conn.execute(
                "INSERT INTO t VALUES (?1, ?2, ?3, ?4, ?5)",
                params![app, semantics, input, read, creation],
            )?;
            let from_sql: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
            let from_rust = fresh_input_tokens(app, semantics, input, read, creation);
            assert_eq!(
                i64::from(from_rust),
                from_sql,
                "{app} semantics={semantics} input={input} read={read} creation={creation}"
            );
        }
        Ok(())
    }

    #[test]
    fn startup_reprice_runs_only_after_pricing_changes() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_row(
            &db,
            RowSpec {
                request_id: "zero",
                input: 1000,
                output: 1000,
                ..Default::default()
            },
        )?;

        assert_eq!(db.reprice_usage_costs_if_pricing_changed()?, Some(1));
        assert_eq!(db.reprice_usage_costs_if_pricing_changed()?, None);
        add_pricing(&db, "brand-new-model", "1", "2")?;
        assert_eq!(db.reprice_usage_costs_if_pricing_changed()?, Some(0));
        assert_eq!(db.reprice_usage_costs_if_pricing_changed()?, None);
        Ok(())
    }

    /// 定价表没变、计价规则或调价记录变了（发了新版）也要重算。
    #[test]
    fn startup_reprice_runs_after_the_pricing_rules_change() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_row(
            &db,
            RowSpec {
                request_id: "zero",
                input: 1000,
                output: 1000,
                ..Default::default()
            },
        )?;
        assert!(db.reprice_usage_costs_if_pricing_changed()?.is_some());
        assert_eq!(db.reprice_usage_costs_if_pricing_changed()?, None);

        {
            let conn = lock_conn!(db.conn);
            let rules = pricing_rules_text();
            assert_eq!(
                Database::pricing_fingerprint(&conn)?,
                Database::pricing_fingerprint_with(&conn, &rules)?
            );
            assert_ne!(
                Database::pricing_fingerprint_with(&conn, "rules=0;history=")?,
                Database::pricing_fingerprint(&conn)?
            );
            // 上一版按别的规则算出的指纹
            let previous = Database::pricing_fingerprint_with(&conn, "rules=0;history=")?;
            Database::write_pricing_fingerprint(&conn, &previous)?;
        }
        assert!(db.reprice_usage_costs_if_pricing_changed()?.is_some());
        assert_eq!(db.reprice_usage_costs_if_pricing_changed()?, None);
        Ok(())
    }

    #[test]
    fn rollup_reprices_only_the_rows_it_rolls_up() -> Result<(), AppError> {
        let db = Database::memory()?;
        let now = chrono::Local::now().timestamp();
        for (request_id, created_at) in [("old", now - 60 * 86_400), ("recent", now - 3_600)] {
            insert_row(
                &db,
                RowSpec {
                    request_id,
                    input: 1000,
                    output: 1000,
                    total_cost: "99",
                    created_at,
                    ..Default::default()
                },
            )?;
        }

        db.rollup_and_prune(30)?;

        assert_eq!(stored_total(&db, "recent")?, dec("99"));
        let rolled: String = lock_conn!(db.conn).query_row(
            "SELECT total_cost_usd FROM usage_daily_rollups",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(dec(&rolled), dec("0.03"));
        Ok(())
    }

    #[test]
    fn v21_migration_keeps_tool_reported_costs() -> Result<(), AppError> {
        let conn = Connection::open_in_memory()?;
        Database::create_tables_on_conn(&conn)?;
        // 上游 v20 的明细表还没有 native_cost 列
        conn.execute_batch("ALTER TABLE proxy_request_logs DROP COLUMN native_cost;")?;
        for (request_id, source, cost) in [
            ("opencode", "opencode_session", "0.42"),
            ("pi", "pi_session", "0.1"),
            ("grok", "grok_session", "0.2"),
            ("mcode", "mcode_session", "0.3"),
            ("mcode-free", "mcode_session", "0"),
            ("opencode-unpriced", "opencode_session", "0"),
            ("claude", "session_log", "0.5"),
        ] {
            conn.execute(
                "INSERT INTO proxy_request_logs (request_id, provider_id, app_type, model,
                     input_tokens, output_tokens, total_cost_usd, latency_ms, status_code,
                     created_at, data_source)
                 VALUES (?1, 'p', 'x', 'unpriced-model', 1000, 1000, ?2, 0, 200, 100, ?3)",
                params![request_id, cost, source],
            )?;
        }
        Database::set_user_version(&conn, 20)?;

        Database::apply_schema_migrations_on_conn(&conn)?;
        Database::reprice_usage_costs_on_conn(&conn, None, None)?;

        let rows: Vec<(String, i64, String)> = conn
            .prepare(
                "SELECT request_id, native_cost, total_cost_usd FROM proxy_request_logs
                 ORDER BY request_id",
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<_, _>>()?;
        let expected = [
            ("claude", 0, "0"),
            ("grok", 1, "0.2"),
            ("mcode", 1, "0.3"),
            ("mcode-free", 1, "0"),
            ("opencode", 1, "0.42"),
            ("opencode-unpriced", 0, "0"),
            ("pi", 1, "0.1"),
        ];
        let actual: Vec<(&str, i64, rust_decimal::Decimal)> = rows
            .iter()
            .map(|(id, native, cost)| (id.as_str(), *native, dec(cost)))
            .collect();
        let expected: Vec<(&str, i64, rust_decimal::Decimal)> = expected
            .iter()
            .map(|(id, native, cost)| (*id, *native, dec(cost)))
            .collect();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn reprice_converts_stored_input_semantics_to_fresh_input() -> Result<(), AppError> {
        let db = Database::memory()?;
        // gpt-5.5：$5 输入 / $0.5 缓存读；未命中缓存的输入都是 1000
        let codex = |request_id, semantics, input, cache_creation| RowSpec {
            request_id,
            app_type: "codex",
            model: "gpt-5.5",
            data_source: "codex_session",
            semantics,
            input,
            cache_read: 2000,
            cache_creation,
            ..Default::default()
        };
        insert_row(&db, codex("legacy", INPUT_TOKEN_SEMANTICS_LEGACY, 3000, 0))?;
        insert_row(&db, codex("total", INPUT_TOKEN_SEMANTICS_TOTAL, 3000, 0))?;
        insert_row(&db, codex("fresh", INPUT_TOKEN_SEMANTICS_FRESH, 1000, 0))?;

        assert_eq!(db.reprice_usage_costs()?, 3);
        for request_id in ["legacy", "total", "fresh"] {
            // 1000 × $5 + 2000 × $0.5
            assert_eq!(stored_total(&db, request_id)?, dec("0.006"), "{request_id}");
        }
        Ok(())
    }

    #[test]
    fn reprice_applies_one_hour_cache_priority_and_long_context() -> Result<(), AppError> {
        let db = Database::memory()?;
        // claude-opus-4-8：5 分钟写入 $6.25，1 小时写入 = 2 × $5
        insert_row(
            &db,
            RowSpec {
                request_id: "one-hour",
                cache_creation: 1000,
                cache_creation_1h: 600,
                ..Default::default()
            },
        )?;
        // gpt-5.6-sol：$4 / $20，priority ×2；提示超过 272K 输入侧 ×2、输出 ×1.5
        let sol = |request_id, input, service_tier| RowSpec {
            request_id,
            app_type: "codex",
            model: "gpt-5.6-sol",
            data_source: "codex_session",
            input,
            output: 1000,
            service_tier,
            ..Default::default()
        };
        insert_row(&db, sol("priority", 1000, "priority"))?;
        insert_row(&db, sol("long", 300_000, ""))?;
        // 2026-08-21 降价前按 $5 / $30
        insert_row(
            &db,
            RowSpec {
                created_at: 1_787_000_000,
                ..sol("before-price-cut", 1000, "")
            },
        )?;

        assert_eq!(db.reprice_usage_costs()?, 4);
        assert_eq!(stored_total(&db, "one-hour")?, dec("0.0085"));
        assert_eq!(stored_total(&db, "priority")?, dec("0.048"));
        // 300K × $8 + 1K × $30
        assert_eq!(stored_total(&db, "long")?, dec("2.43"));
        assert_eq!(stored_total(&db, "before-price-cut")?, dec("0.035"));
        Ok(())
    }

    /// Grok Build 一行是一轮的合计，提示长度加起来超过 200K 也按标准价
    #[test]
    fn reprice_ignores_long_context_for_grok_turn_totals() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_row(
            &db,
            RowSpec {
                request_id: "grok-turn",
                app_type: "grokbuild",
                model: "grok-4.6",
                data_source: "grok_session",
                input: 300_000,
                output: 1000,
                ..Default::default()
            },
        )?;
        assert_eq!(db.reprice_usage_costs()?, 1);
        // 300K × $2 + 1K × $6
        assert_eq!(stored_total(&db, "grok-turn")?, dec("0.606"));
        Ok(())
    }

    #[test]
    fn reprice_leaves_native_and_routing_rows_alone() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_row(
            &db,
            RowSpec {
                request_id: "native",
                input: 1000,
                native_cost: true,
                total_cost: "1.23",
                ..Default::default()
            },
        )?;
        insert_row(
            &db,
            RowSpec {
                request_id: "routing",
                input: 1000,
                data_source: "proxy",
                total_cost: "4.56",
                ..Default::default()
            },
        )?;

        assert_eq!(db.reprice_usage_costs()?, 0);
        assert_eq!(stored_total(&db, "native")?, dec("1.23"));
        assert_eq!(stored_total(&db, "routing")?, dec("4.56"));
        Ok(())
    }

    #[test]
    fn reprice_follows_pricing_model_and_placeholder_fallback() -> Result<(), AppError> {
        let db = Database::memory()?;
        // 写入时锚定的 pricing_model 缺价：不能改用 model 列（有价的别名）
        insert_row(
            &db,
            RowSpec {
                request_id: "anchored",
                model: "claude-sonnet-4-6",
                pricing_model: Some("kimi-k2-novel"),
                input: 1_000_000,
                ..Default::default()
            },
        )?;
        // model 是解析失败的占位符时才退回 request_model
        insert_row(
            &db,
            RowSpec {
                request_id: "placeholder",
                model: "unknown",
                request_model: Some("claude-opus-4-8"),
                input: 1000,
                ..Default::default()
            },
        )?;

        assert_eq!(db.reprice_usage_costs()?, 1);
        assert_eq!(stored_total(&db, "anchored")?, dec("0"));
        assert_eq!(stored_total(&db, "placeholder")?, dec("0.005"));

        add_pricing(&db, "kimi-k2-novel", "0.6", "2.5")?;
        assert_eq!(db.reprice_usage_costs_for_model("kimi-k2-novel")?, 1);
        assert_eq!(stored_total(&db, "anchored")?, dec("0.6"));
        Ok(())
    }

    #[test]
    fn scoped_reprice_matches_raw_alias_rows() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_row(
            &db,
            RowSpec {
                request_id: "alias",
                model: "openrouter/moonshot/kimi-k2-novel:free",
                input: 1_000_000,
                ..Default::default()
            },
        )?;
        assert_eq!(db.reprice_usage_costs()?, 0);

        add_pricing(&db, "kimi-k2-novel", "0.6", "2.5")?;
        assert_eq!(db.reprice_usage_costs_for_model("kimi-k2-novel")?, 1);
        assert_eq!(stored_total(&db, "alias")?, dec("0.6"));
        Ok(())
    }

    #[test]
    fn reprice_zeroes_costs_when_pricing_is_removed() -> Result<(), AppError> {
        let db = Database::memory()?;
        add_pricing(&db, "kimi-k2-novel", "0.6", "2.5")?;
        insert_row(
            &db,
            RowSpec {
                request_id: "removed",
                model: "kimi-k2-novel",
                input: 1_000_000,
                ..Default::default()
            },
        )?;
        assert_eq!(db.reprice_usage_costs()?, 1);
        assert_eq!(stored_total(&db, "removed")?, dec("0.6"));

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "DELETE FROM model_pricing WHERE model_id = 'kimi-k2-novel'",
                [],
            )?;
        }
        assert_eq!(db.reprice_usage_costs()?, 1);
        assert_eq!(stored_total(&db, "removed")?, dec("0"));
        Ok(())
    }

    #[test]
    fn test_get_usage_summary() -> Result<(), AppError> {
        let db = Database::memory()?;

        // 插入测试数据
        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params!["req1", "p1", "claude", "claude-3", 100, 50, "0.01", 100, 200, 1000],
            )?;
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params!["req2", "p1", "claude", "claude-3", 200, 100, "0.02", 150, 200, 2000],
            )?;
        }

        let summary = db.get_usage_summary(None, None, None, None, None)?;
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.success_rate, 100.0);

        Ok(())
    }

    #[test]
    fn test_get_session_usage_summary_counts_only_session_log_rows() -> Result<(), AppError> {
        let db = Database::memory()?;

        {
            let conn = lock_conn!(db.conn);
            // (request_id, app_type, session_id, data_source, input, output, cache_read, cost)
            let rows = [
                (
                    "s1",
                    "claude",
                    "sess-a",
                    "session_log",
                    10,
                    200,
                    5000,
                    "0.10",
                ),
                (
                    "s2",
                    "claude-desktop",
                    "sess-a",
                    "session_log",
                    20,
                    300,
                    7000,
                    "0.20",
                ),
                // 代理行即便会话 ID 相同也不数，避免与会话日志重复
                ("p1", "claude", "sess-a", "proxy", 10, 200, 5000, "0.10"),
                ("s3", "claude", "sess-b", "session_log", 1, 1, 1, "9.00"),
                ("s4", "codex", "sess-a", "codex_session", 1, 1, 0, "9.00"),
            ];
            for (id, app, session, source, input, output, cache_read, cost) in rows {
                conn.execute(
                    "INSERT INTO proxy_request_logs (
                        request_id, provider_id, app_type, model,
                        input_tokens, output_tokens, cache_read_tokens, total_cost_usd,
                        latency_ms, status_code, created_at, session_id, data_source
                    ) VALUES (?, 'p1', ?, 'claude-x', ?, ?, ?, ?, 0, 200, 1000, ?, ?)",
                    params![id, app, input, output, cache_read, cost, session, source],
                )?;
            }
        }

        let summary = db.get_session_usage_summary("claude", "sess-a")?;
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.real_total_tokens, 10 + 200 + 5000 + 20 + 300 + 7000);
        assert_eq!(summary.total_cost, "0.300000");

        let empty = db.get_session_usage_summary("claude", "missing")?;
        assert_eq!(empty.total_requests, 0);

        Ok(())
    }

    #[test]
    fn test_get_usage_summary_excludes_partial_rollup_boundary_days() -> Result<(), AppError> {
        let db = Database::memory()?;
        let start = local_ts(2024, 1, 1, 12, 0, 0);
        let end = local_ts(2024, 1, 3, 12, 0, 0);

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-01-01",
                    "claude",
                    "p1",
                    "claude-3",
                    10,
                    10,
                    1000,
                    500,
                    0,
                    0,
                    "1.00",
                    100
                ],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-01-02",
                    "claude",
                    "p1",
                    "claude-3",
                    20,
                    19,
                    2000,
                    1000,
                    0,
                    0,
                    "2.00",
                    120
                ],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-01-03",
                    "claude",
                    "p1",
                    "claude-3",
                    30,
                    29,
                    3000,
                    1500,
                    0,
                    0,
                    "3.00",
                    140
                ],
            )?;
        }

        let summary = db.get_usage_summary(Some(start), Some(end), Some("claude"), None, None)?;
        assert_eq!(summary.total_requests, 20);
        assert_eq!(summary.total_input_tokens, 2000);
        assert_eq!(summary.total_output_tokens, 1000);

        Ok(())
    }

    #[test]
    fn test_get_request_detail_reads_proxy_and_session_rows() -> Result<(), AppError> {
        let db = Database::memory()?;
        let ts = local_ts(2026, 6, 10, 12, 0, 0);

        {
            let conn = lock_conn!(db.conn);
            // providers 也有 created_at / cost_multiplier 列：详情查询的列必须带表别名，
            // 否则 SQLite 报 ambiguous column name，整个命令失败。
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, created_at) VALUES
                 ('prov-a', 'claude', 'Packy', '{}', 1)",
                [],
            )?;
            insert_usage_log(
                &conn,
                "a-1",
                "claude",
                "prov-a",
                "claude-sonnet-4-6",
                "proxy",
                ts,
                100,
                10,
                0,
                0,
                200,
                "1.0",
            )?;
            insert_usage_log(
                &conn,
                "session:msg_1",
                "claude",
                "_session",
                "claude-sonnet-4-6",
                "session_log",
                ts,
                999,
                99,
                0,
                0,
                200,
                "0.5",
            )?;
        }

        let proxy = db.get_request_detail("a-1")?.expect("proxy row");
        assert_eq!(proxy.provider_name.as_deref(), Some("Packy"));
        assert_eq!(proxy.created_at, ts);
        assert_eq!(proxy.input_tokens, 100);

        let session = db
            .get_request_detail("session:msg_1")?
            .expect("session row");
        assert_eq!(session.provider_name.as_deref(), Some("Claude (Session)"));
        assert_eq!(session.created_at, ts);

        assert!(db.get_request_detail("missing")?.is_none());

        Ok(())
    }

    #[test]
    fn test_provider_and_model_filters_cover_detail_and_rollup() -> Result<(), AppError> {
        let db = Database::memory()?;
        let detail_ts = local_ts(2026, 6, 10, 12, 0, 0);

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config) VALUES
                 ('prov-a', 'claude', 'Packy', '{}'),
                 ('prov-b', 'claude', 'DeepSeek', '{}')",
                [],
            )?;

            insert_usage_log(
                &conn,
                "a-1",
                "claude",
                "prov-a",
                "claude-sonnet-4-6",
                "proxy",
                detail_ts,
                100,
                10,
                0,
                0,
                200,
                "1.0",
            )?;
            insert_usage_log(
                &conn,
                "b-1",
                "claude",
                "prov-b",
                "deepseek-v3",
                "proxy",
                detail_ts,
                200,
                20,
                0,
                0,
                200,
                "2.0",
            )?;
            // 会话占位行：providers 表无此 id，展示名走 CASE 映射。
            insert_usage_log(
                &conn,
                "s-1",
                "claude",
                "_session",
                "claude-sonnet-4-6",
                "session_log",
                detail_ts,
                999,
                99,
                0,
                0,
                200,
                "0.5",
            )?;
            // 计价模型与请求模型不同的行：模型筛选必须按有效计价模型命中。
            insert_usage_log(
                &conn,
                "a-2",
                "claude",
                "prov-a",
                "alias-model",
                "proxy",
                detail_ts,
                50,
                5,
                0,
                0,
                200,
                "0.3",
            )?;
            conn.execute(
                "UPDATE proxy_request_logs SET pricing_model = 'real-model' WHERE request_id = 'a-2'",
                [],
            )?;

            // rollup 历史日行：无范围过滤时全部计入。
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES
                ('2026-06-08', 'claude', 'prov-a', 'claude-sonnet-4-6', 5, 5, 500, 50, 0, 0, '5.0', 100),
                ('2026-06-08', 'claude', 'prov-b', 'deepseek-v3', 7, 7, 700, 70, 0, 0, '7.0', 100)",
                [],
            )?;
        }

        // ① 汇总按 Provider 展示名过滤：明细 + rollup 都命中。
        let packy = db.get_usage_summary(None, None, None, Some("Packy"), None)?;
        assert_eq!(packy.total_requests, 7, "a-1 + a-2 + rollup 5");

        // ② 汇总按模型过滤（有效计价模型口径）。
        let deepseek = db.get_usage_summary(None, None, None, None, Some("deepseek-v3"))?;
        assert_eq!(deepseek.total_requests, 8, "b-1 + rollup 7");

        // ③ pricing_model 优先于 model：alias-model 查不到，real-model 查得到。
        let by_alias = db.get_usage_summary(None, None, None, None, Some("alias-model"))?;
        assert_eq!(by_alias.total_requests, 0);
        let by_real = db.get_usage_summary(None, None, None, None, Some("real-model"))?;
        assert_eq!(by_real.total_requests, 1);

        // ④ 会话占位行可按可读名选中。
        let session = db.get_usage_summary(None, None, None, Some("Claude (Session)"), None)?;
        assert_eq!(session.total_requests, 1);

        // ⑤ Provider 统计 + 模型过滤：只剩 DeepSeek 一行。
        let provider_stats = db.get_provider_stats(None, None, None, None, Some("deepseek-v3"))?;
        assert_eq!(provider_stats.len(), 1);
        assert_eq!(provider_stats[0].provider_name, "DeepSeek");
        assert_eq!(provider_stats[0].request_count, 8);

        // ⑥ 模型统计 + Provider 过滤：只剩 Packy 名下的模型。
        let model_stats = db.get_model_stats(None, None, None, Some("Packy"), None)?;
        let models: Vec<&str> = model_stats.iter().map(|m| m.model.as_str()).collect();
        assert!(models.contains(&"claude-sonnet-4-6"));
        assert!(models.contains(&"real-model"));
        assert!(!models.contains(&"deepseek-v3"));

        // ⑦ 分应用汇总（Hero 卡片数据源）同样受过滤影响。
        let by_app = db.get_usage_summary_by_app(None, None, Some("Packy"), None)?;
        assert_eq!(by_app.len(), 1);
        assert_eq!(by_app[0].app_type, "claude");
        assert_eq!(by_app[0].summary.total_requests, 7);

        // ⑧ 趋势（>24h 走天分桶 + rollup 分支）。
        let t_start = local_ts(2026, 6, 8, 0, 0, 0);
        let t_end = local_ts(2026, 6, 10, 23, 59, 0);
        let trends = db.get_daily_trends(Some(t_start), Some(t_end), None, Some("Packy"), None)?;
        let total_req: u64 = trends.iter().map(|d| d.request_count).sum();
        assert_eq!(total_req, 7, "明细 2 + rollup 5");

        // ⑨ 趋势 ≤24h 走小时分桶分支（?1/?2/?3 编号参数与追加过滤混用的路径），
        //    同时验证 Provider + 模型组合过滤。
        let h_start = local_ts(2026, 6, 10, 0, 0, 0);
        let h_end = local_ts(2026, 6, 10, 20, 0, 0);
        let hourly = db.get_daily_trends(
            Some(h_start),
            Some(h_end),
            None,
            Some("Packy"),
            Some("claude-sonnet-4-6"),
        )?;
        let hourly_req: u64 = hourly.iter().map(|d| d.request_count).sum();
        assert_eq!(hourly_req, 1, "仅 a-1 命中（a-2 计价模型不同）");

        // ⑩ 请求日志列表与下拉同口径：精确名 + 有效计价模型。
        let logs = db.get_request_logs(
            &LogFilters {
                provider_name: Some("Packy".to_string()),
                model: Some("real-model".to_string()),
                ..Default::default()
            },
            0,
            10,
        )?;
        assert_eq!(logs.total, 1);
        assert_eq!(logs.data[0].request_id, "a-2");

        Ok(())
    }

    #[test]
    fn test_get_usage_summary_includes_end_day_rollup_for_minute_precision_end_time(
    ) -> Result<(), AppError> {
        let db = Database::memory()?;
        let start = local_ts(2024, 1, 1, 0, 0, 0);
        let end = local_ts(2024, 1, 2, 23, 59, 0);

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-01-01",
                    "claude",
                    "p1",
                    "claude-3",
                    10,
                    10,
                    1000,
                    500,
                    0,
                    0,
                    "1.00",
                    100
                ],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-01-02",
                    "claude",
                    "p1",
                    "claude-3",
                    20,
                    19,
                    2000,
                    1000,
                    0,
                    0,
                    "2.00",
                    120
                ],
            )?;
        }

        let summary = db.get_usage_summary(Some(start), Some(end), Some("claude"), None, None)?;
        assert_eq!(summary.total_requests, 30);
        assert_eq!(summary.total_input_tokens, 3000);
        assert_eq!(summary.total_output_tokens, 1500);

        Ok(())
    }

    #[test]
    fn test_get_model_stats() -> Result<(), AppError> {
        let db = Database::memory()?;

        // 插入测试数据
        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "req1",
                    "p1",
                    "claude",
                    "claude-3-sonnet",
                    100,
                    50,
                    "0.01",
                    100,
                    200,
                    1000
                ],
            )?;
        }

        let stats = db.get_model_stats(None, None, None, None, None)?;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].model, "claude-3-sonnet");
        assert_eq!(stats[0].request_count, 1);

        Ok(())
    }

    #[test]
    fn test_get_model_stats_success_rate_and_speed_per_model() -> Result<(), AppError> {
        let db = Database::memory()?;

        {
            let conn = lock_conn!(db.conn);
            let insert = |id: &str,
                          model: &str,
                          output: i64,
                          latency: i64,
                          first: Option<i64>,
                          status: i64,
                          source: &str| {
                conn.execute(
                    "INSERT INTO proxy_request_logs (
                        request_id, provider_id, app_type, model,
                        input_tokens, output_tokens, total_cost_usd,
                        latency_ms, first_token_ms, status_code, created_at, data_source
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        id, "p1", "claude", model, 10, output, "0", latency, first, status, 1000,
                        source
                    ],
                )
            };
            // fast：精确速度 1000 token / 10000 ms，另有一条失败请求
            insert("fast-ok", "fast", 1_000, 11_000, Some(1_000), 200, "proxy")?;
            insert("fast-err", "fast", 0, 500, None, 500, "proxy")?;
            // slow：只有会话日志，估算速度 400 token / 8000 ms
            insert("slow-a", "slow", 400, 8_000, None, 200, "session_log")?;
        }

        let stats = db.get_model_stats(None, None, None, None, None)?;
        let fast = stats.iter().find(|s| s.model == "fast").expect("fast");
        assert_eq!(fast.request_count, 2);
        assert!((fast.success_rate - 50.0).abs() < f32::EPSILON);
        assert_eq!(fast.speed_output_tokens, 1_000);
        assert_eq!(fast.speed_generation_ms, 10_000);
        assert_eq!(fast.est_speed_output_tokens, 0);

        let slow = stats.iter().find(|s| s.model == "slow").expect("slow");
        assert!((slow.success_rate - 100.0).abs() < f32::EPSILON);
        assert_eq!(slow.speed_output_tokens, 0);
        assert_eq!(slow.est_speed_output_tokens, 400);
        assert_eq!(slow.est_speed_duration_ms, 8_000);

        Ok(())
    }

    #[test]
    fn test_get_provider_stats_with_time_filter() -> Result<(), AppError> {
        let db = Database::memory()?;

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params!["old", "p1", "claude", "claude-3", 100, 50, "0.01", 100, 200, 1000],
            )?;
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params!["new", "p1", "claude", "claude-3", 200, 75, "0.02", 120, 200, 2000],
            )?;
        }

        let stats = db.get_provider_stats(Some(1500), Some(2500), Some("claude"), None, None)?;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].provider_id, "p1");
        assert_eq!(stats[0].request_count, 1);
        assert_eq!(stats[0].total_tokens, 275);

        Ok(())
    }

    #[test]
    fn test_get_provider_stats_speed_sums_only_eligible_requests() -> Result<(), AppError> {
        let db = Database::memory()?;

        {
            let conn = lock_conn!(db.conn);
            let insert = |id: &str, output: i64, latency: i64, first: Option<i64>| {
                conn.execute(
                    "INSERT INTO proxy_request_logs (
                        request_id, provider_id, app_type, model,
                        input_tokens, output_tokens, total_cost_usd,
                        latency_ms, first_token_ms, status_code, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![id, "p1", "claude", "m", 10, output, "0", latency, first, 200, 1000],
                )
            };
            // 计入：1000 token / (11000 - 1000) ms
            insert("ok-a", 1000, 11_000, Some(1_000))?;
            // 计入：300 token / (4000 - 1000) ms
            insert("ok-b", 300, 4_000, Some(1_000))?;
            // 不计：输出不到 100
            insert("short", 50, 2_000, Some(100))?;
            // 不计：没有首字（会话日志 / 非流式）
            insert("no-ttft", 5_000, 9_000, None)?;
            // 不计：耗时不大于首字
            insert("zero-gen", 500, 1_000, Some(1_000))?;
            // 不计：生成窗口不到 100ms，是传输突发
            insert("burst", 500, 1_050, Some(1_000))?;
        }

        let stats = db.get_provider_stats(None, None, None, None, None)?;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].request_count, 6);
        assert_eq!(stats[0].speed_output_tokens, 1_300);
        assert_eq!(stats[0].speed_generation_ms, 13_000);
        // 路由服务的行没有首字（非流式）也不算估算速度
        assert_eq!(stats[0].est_speed_output_tokens, 0);
        assert_eq!(stats[0].est_speed_duration_ms, 0);

        Ok(())
    }

    #[test]
    fn test_get_provider_stats_estimated_speed_sums_session_rows() -> Result<(), AppError> {
        let db = Database::memory()?;

        {
            let conn = lock_conn!(db.conn);
            let insert = |id: &str, output: i64, latency: i64, source: &str| {
                conn.execute(
                    "INSERT INTO proxy_request_logs (
                        request_id, provider_id, app_type, model,
                        input_tokens, output_tokens, total_cost_usd,
                        latency_ms, status_code, created_at, data_source
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        id, "_session", "claude", "m", 10, output, "0", latency, 200, 1000, source
                    ],
                )
            };
            // 计入：2000 token / 20000 ms
            insert("ok-a", 2_000, 20_000, "session_log")?;
            // 计入：500 token / 5000 ms
            insert("ok-b", 200, 5_000, "session_log")?;
            // 不计：输出不到 200
            insert("short", 199, 5_000, "session_log")?;
            // 不计：没估出耗时
            insert("no-timing", 3_000, 0, "session_log")?;
            // 不计：耗时不到 1 秒
            insert("too-fast", 800, 900, "session_log")?;
        }

        let stats = db.get_provider_stats(None, None, None, None, None)?;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].est_speed_output_tokens, 2_200);
        assert_eq!(stats[0].est_speed_duration_ms, 25_000);
        // 估算的不混进精确口径
        assert_eq!(stats[0].speed_output_tokens, 0);
        assert_eq!(stats[0].speed_generation_ms, 0);

        Ok(())
    }

    #[test]
    fn test_get_provider_stats_labels_opencode_session_provider() -> Result<(), AppError> {
        let db = Database::memory()?;

        {
            let conn = lock_conn!(db.conn);
            insert_usage_log(
                &conn,
                "opencode-session",
                "opencode",
                "_opencode_session",
                "opencode-model",
                "opencode_session",
                1000,
                100,
                50,
                0,
                0,
                200,
                "0.01",
            )?;
        }

        let stats = db.get_provider_stats(None, None, Some("opencode"), None, None)?;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].provider_id, "_opencode_session");
        assert_eq!(stats[0].provider_name, "OpenCode (Session)");

        Ok(())
    }

    #[test]
    fn test_get_provider_stats_excludes_partial_rollup_boundary_days() -> Result<(), AppError> {
        let db = Database::memory()?;
        let start = local_ts(2024, 2, 1, 12, 0, 0);
        let end = local_ts(2024, 2, 3, 12, 0, 0);

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-02-01",
                    "claude",
                    "p-rollup",
                    "claude-3",
                    5,
                    5,
                    500,
                    250,
                    0,
                    0,
                    "0.50",
                    100
                ],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-02-02",
                    "claude",
                    "p-rollup",
                    "claude-3",
                    8,
                    7,
                    800,
                    400,
                    0,
                    0,
                    "0.80",
                    120
                ],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-02-03",
                    "claude",
                    "p-rollup",
                    "claude-3",
                    12,
                    11,
                    1200,
                    600,
                    0,
                    0,
                    "1.20",
                    140
                ],
            )?;
        }

        let stats = db.get_provider_stats(Some(start), Some(end), Some("claude"), None, None)?;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].provider_id, "p-rollup");
        assert_eq!(stats[0].request_count, 8);
        assert_eq!(stats[0].total_tokens, 1200);

        Ok(())
    }

    #[test]
    fn test_get_daily_trends_respects_shorter_than_24_hours() -> Result<(), AppError> {
        let db = Database::memory()?;

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "req-short",
                    "p1",
                    "claude",
                    "claude-3",
                    100,
                    50,
                    "0.01",
                    100,
                    200,
                    10_800
                ],
            )?;
        }

        let stats = db.get_daily_trends(Some(0), Some(15 * 60 * 60), Some("claude"), None, None)?;
        assert_eq!(stats.len(), 15);
        assert_eq!(stats[3].request_count, 1);

        Ok(())
    }

    #[test]
    fn test_get_daily_trends_groups_ranges_longer_than_24_hours_by_local_day(
    ) -> Result<(), AppError> {
        let db = Database::memory()?;
        let start = local_ts(2024, 3, 1, 12, 0, 0);
        let end = local_ts(2024, 3, 3, 12, 0, 0);

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "day-1-detail",
                    "p1",
                    "claude",
                    "claude-3",
                    100,
                    50,
                    "0.01",
                    100,
                    200,
                    local_ts(2024, 3, 1, 13, 0, 0)
                ],
            )?;
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "day-3-detail",
                    "p1",
                    "claude",
                    "claude-3",
                    200,
                    75,
                    "0.02",
                    110,
                    200,
                    local_ts(2024, 3, 3, 10, 0, 0)
                ],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-03-02",
                    "claude",
                    "p1",
                    "claude-3",
                    4,
                    4,
                    400,
                    200,
                    0,
                    0,
                    "0.40",
                    120
                ],
            )?;
        }

        let stats = db.get_daily_trends(Some(start), Some(end), Some("claude"), None, None)?;
        assert_eq!(stats.len(), 3);
        assert_eq!(stats[0].request_count, 1);
        assert_eq!(stats[0].total_tokens, 150);
        assert_eq!(stats[1].request_count, 4);
        assert_eq!(stats[1].total_tokens, 600);
        assert_eq!(stats[2].request_count, 1);
        assert_eq!(stats[2].total_tokens, 275);

        Ok(())
    }

    #[test]
    fn test_provider_and_model_stats_tokens_sum_to_summary_real_total() -> Result<(), AppError> {
        let db = Database::memory()?;

        {
            let conn = lock_conn!(db.conn);
            // Claude：input 不含缓存。真实消耗 = 100 + 200 + 1000 + 5000
            insert_usage_log(
                &conn,
                "claude-1",
                "claude",
                "p-claude",
                "claude-x",
                "session_log",
                1000,
                100,
                200,
                5000,
                1000,
                200,
                "0.10",
            )?;
            // Codex：input 已含缓存命中 600。真实消耗 = (1000 - 600) + 50 + 600
            insert_usage_log(
                &conn,
                "codex-1",
                "codex",
                "p-codex",
                "gpt-x",
                "codex_session",
                1000,
                1000,
                50,
                600,
                0,
                200,
                "0.05",
            )?;
            // 日汇总行同样要带上缓存。真实消耗 = (900 - 300) + 40 + 300 + 0
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2020-01-01",
                    "codex",
                    "p-codex",
                    "gpt-x",
                    3,
                    3,
                    900,
                    40,
                    300,
                    0,
                    "0.30",
                    100
                ],
            )?;
        }

        let summary = db.get_usage_summary(None, None, None, None, None)?;
        assert_eq!(summary.real_total_tokens, 6300 + 1050 + 940);

        let providers = db.get_provider_stats(None, None, None, None, None)?;
        let tokens_of = |id: &str| {
            providers
                .iter()
                .find(|s| s.provider_id == id)
                .map(|s| s.total_tokens)
        };
        assert_eq!(tokens_of("p-claude"), Some(6300));
        assert_eq!(tokens_of("p-codex"), Some(1050 + 940));
        let provider_sum: u64 = providers.iter().map(|s| s.total_tokens).sum();
        assert_eq!(provider_sum, summary.real_total_tokens);

        let models = db.get_model_stats(None, None, None, None, None)?;
        let model_sum: u64 = models.iter().map(|s| s.total_tokens).sum();
        assert_eq!(model_sum, summary.real_total_tokens);

        Ok(())
    }

    #[test]
    fn test_get_model_stats_excludes_partial_rollup_boundary_days() -> Result<(), AppError> {
        let db = Database::memory()?;
        let start = local_ts(2024, 4, 1, 12, 0, 0);
        let end = local_ts(2024, 4, 3, 12, 0, 0);

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-04-01",
                    "claude",
                    "p1",
                    "claude-3-haiku",
                    6,
                    6,
                    600,
                    300,
                    0,
                    0,
                    "0.60",
                    100
                ],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-04-02",
                    "claude",
                    "p1",
                    "claude-3-haiku",
                    9,
                    8,
                    900,
                    450,
                    0,
                    0,
                    "0.90",
                    110
                ],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model,
                    request_count, success_count, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    "2024-04-03",
                    "claude",
                    "p1",
                    "claude-3-haiku",
                    12,
                    11,
                    1200,
                    600,
                    0,
                    0,
                    "1.20",
                    130
                ],
            )?;
        }

        let stats = db.get_model_stats(Some(start), Some(end), Some("claude"), None, None)?;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].model, "claude-3-haiku");
        assert_eq!(stats[0].request_count, 9);
        assert_eq!(stats[0].total_tokens, 1350);

        Ok(())
    }

    #[test]
    fn test_strip_model_date_suffix_is_utf8_safe() {
        assert_eq!(
            strip_model_date_suffix("模型-2026-05-14").as_deref(),
            Some("模型")
        );
        assert_eq!(strip_model_date_suffix("abc🚀12345678"), None);
    }

    #[test]
    fn test_strip_model_date_suffix_handles_six_digit_yymmdd() {
        // 火山方舟 6 位 YYMMDD 后缀应被剥离（doubao 全系都用这种格式）。
        assert_eq!(
            strip_model_date_suffix("doubao-seed-2-1-pro-260628").as_deref(),
            Some("doubao-seed-2-1-pro")
        );
        assert_eq!(
            strip_model_date_suffix("doubao-seed-1-6-250615").as_deref(),
            Some("doubao-seed-1-6")
        );
        // 8 位 YYYYMMDD 仍照旧剥离。
        assert_eq!(
            strip_model_date_suffix("claude-3-5-sonnet-20241022").as_deref(),
            Some("claude-3-5-sonnet")
        );
        // 月/日非法的 6 位尾巴（版本号等）不剥离，避免误伤。
        assert_eq!(strip_model_date_suffix("foo-bar-123456"), None); // 月=34
        assert_eq!(strip_model_date_suffix("widget-209900"), None); // 月=99
        assert_eq!(strip_model_date_suffix("gizmo-251200"), None); // 日=00
    }

    #[test]
    fn test_pricing_resolves_volcengine_dated_model_to_bare_seed_row() -> Result<(), AppError> {
        // 回归：火山真实用量带 6 位日期后缀（doubao-seed-2-1-pro-260628），
        // 必须能归一化命中定价表里的裸名 seed 行（doubao-seed-2-1-pro），否则成本显示 $0。
        let db = Database::memory()?;
        let conn = lock_conn!(db.conn);

        conn.execute(
            "INSERT OR REPLACE INTO model_pricing (
                model_id, display_name, input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
            ) VALUES ('doubao-seed-2-1-pro', 'Doubao Seed 2.1 Pro', '0.84', '4.2', '0.17', '0')",
            [],
        )?;

        let row = find_model_pricing_row(&conn, "doubao-seed-2-1-pro-260628")?;
        assert!(
            row.is_some(),
            "带日期的火山模型应通过 6 位日期剥离命中裸名定价行"
        );
        let pricing = row.unwrap();
        assert_eq!(pricing.prices.input.to_string(), "0.84");
        assert_eq!(pricing.prices.output.to_string(), "4.2");

        Ok(())
    }

    #[test]
    fn test_prefix_pricing_does_not_match_short_base_model_to_variant() -> Result<(), AppError> {
        let db = Database::memory()?;
        let conn = lock_conn!(db.conn);

        conn.execute("DELETE FROM model_pricing WHERE model_id LIKE 'gpt-5%'", [])?;
        for (model_id, display_name) in [("gpt-5-mini", "GPT-5 Mini"), ("gpt-5-pro", "GPT-5 Pro")] {
            conn.execute(
                "INSERT INTO model_pricing (
                    model_id, display_name, input_cost_per_million, output_cost_per_million,
                    cache_read_cost_per_million, cache_creation_cost_per_million
                ) VALUES (?1, ?2, '1', '2', '0', '0')",
                params![model_id, display_name],
            )?;
        }

        let result = find_model_pricing_row(&conn, "gpt-5")?;
        assert!(
            result.is_none(),
            "缺少 gpt-5 基础定价时，不应前缀误匹配到 gpt-5-mini/gpt-5-pro"
        );

        Ok(())
    }

    #[test]
    fn test_model_pricing_matching() -> Result<(), AppError> {
        let db = Database::memory()?;
        let conn = lock_conn!(db.conn);

        // 准备额外定价数据，覆盖前缀/后缀清洗场景
        conn.execute(
            "INSERT OR REPLACE INTO model_pricing (
                model_id, display_name, input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
            ) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                "claude-haiku-4.5",
                "Claude Haiku 4.5",
                "1.0",
                "2.0",
                "0.0",
                "0.0"
            ],
        )?;

        // 测试精确匹配（seed_model_pricing 已预置 claude-sonnet-4-5-20250929）
        let result = find_model_pricing_row(&conn, "claude-sonnet-4-5-20250929")?;
        assert!(
            result.is_some(),
            "应该能精确匹配 claude-sonnet-4-5-20250929"
        );

        // 清洗：去除前缀和冒号后缀
        let result = find_model_pricing_row(&conn, "anthropic/claude-haiku-4.5")?;
        assert!(
            result.is_some(),
            "带前缀的模型 anthropic/claude-haiku-4.5 应能匹配到 claude-haiku-4.5"
        );
        let result = find_model_pricing_row(&conn, "moonshotai/kimi-k2-0905:exa")?;
        assert!(
            result.is_some(),
            "带前缀+冒号后缀的模型应清洗后匹配到 kimi-k2-0905"
        );

        // 清洗：@ 替换为 -（seed_model_pricing 已预置 gpt-5.2-codex-low）
        let result = find_model_pricing_row(&conn, "gpt-5.2-codex@low")?;
        assert!(
            result.is_some(),
            "带 @ 分隔符的模型 gpt-5.2-codex@low 应能匹配到 gpt-5.2-codex-low"
        );
        let result = find_model_pricing_row(&conn, "OpenAI/GPT-5.5@HIGH")?;
        assert!(
            result.is_some(),
            "大小写混合的 GPT-5.5 模型应能归一化匹配到 gpt-5.5-high"
        );
        let result = find_model_pricing_row(&conn, "OpenAI/GPT-5.5-2026-05-14")?;
        assert!(
            result.is_some(),
            "OpenAI 日期后缀模型应能回退到 gpt-5.5 基础定价"
        );
        let result = find_model_pricing_row(&conn, "google/gemini-3-pro-preview-20260514")?;
        assert!(
            result.is_some(),
            "Gemini 日期后缀模型应能回退到 gemini-3-pro-preview 基础定价"
        );

        // Claude Desktop route 短 ID：应通过前缀匹配到带日期的定价
        let result = find_model_pricing_row(&conn, "claude-haiku-4-5")?;
        assert!(
            result.is_some(),
            "Claude Desktop 短路由 claude-haiku-4-5 应能匹配到 claude-haiku-4-5-20251001"
        );
        let result = find_model_pricing_row(&conn, "anthropic/claude-opus-4.8")?;
        assert!(
            result.is_some(),
            "聚合商点号格式 anthropic/claude-opus-4.8 应能匹配到 claude-opus-4-8"
        );

        // Claude Desktop 旧版/异常包装的非 Anthropic route：claude-gpt-5.5 → gpt-5.5
        let result = find_model_pricing_row(&conn, "claude-gpt-5.5")?;
        assert!(
            result.is_some(),
            "带 claude- 包装的非 Anthropic 模型应能剥离后匹配到真实模型定价"
        );

        // Bedrock/Vertex 常见形态：provider 前缀 + -vN 后缀 + :0 修饰
        let result =
            find_model_pricing_row(&conn, "global.anthropic.claude-haiku-4-5-20251001-v1:0")?;
        assert!(
            result.is_some(),
            "Bedrock/Vertex 风格 Claude 模型 ID 应能归一化到基础 Claude 模型定价"
        );
        let result = find_model_pricing_row(&conn, "global.anthropic.claude-opus-4-8-v1:0")?;
        assert!(
            result.is_some(),
            "Bedrock 风格 Claude Opus 4.8 模型 ID 应能归一化到基础 Claude 模型定价"
        );
        let result = find_model_pricing_row(&conn, "claude-opus-4-8@20260527")?;
        assert!(
            result.is_some(),
            "Vertex 风格 Claude Opus 4.8 模型 ID 应能归一化到基础 Claude 模型定价"
        );

        // Reasoning effort 后缀：没有专门价格时回退到基础模型
        let result = find_model_pricing_row(&conn, "gpt-5.4@low")?;
        assert!(
            result.is_some(),
            "缺少专门 effort 价格时应回退到 gpt-5.4 基础模型定价"
        );

        // Kimi Code 是订阅/额度模型，不应伪装成公开按 token 计费模型
        let result = find_model_pricing_row(&conn, "kimi-for-coding")?;
        assert!(result.is_none(), "kimi-for-coding 没有固定 token 单价");

        // 测试不存在的模型
        let result = find_model_pricing_row(&conn, "unknown-model-123")?;
        assert!(result.is_none(), "不应该匹配不存在的模型");

        Ok(())
    }
}
