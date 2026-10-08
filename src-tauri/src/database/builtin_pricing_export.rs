//! v21 迁移：把用户在数据库里改过的内置模型价格导出到 `model-pricing.json`。
//!
//! 上游 CC Switch 在有 `model-pricing.json` 之前（2026-07-28 以前），改价只写数据库。
//! ccs-lite 每次启动按代码里的内置价覆盖内置模型的行，覆盖文件之后再应用，所以这些
//! 只在数据库里的改动会在第一次启动时丢掉。迁移在第一次 seed 之前运行，在这里把它们
//! 写进覆盖文件。
//!
//! 判断"改过"：价格既不等于现在的内置价，也不等于上游以前写入过的任何内置价
//! （[`PREVIOUS_BUILTIN_PRICES`]）。比较按数值，"0.20" 和 "0.2" 算相同。

use std::str::FromStr;

use rusqlite::{Connection, OptionalExtension};
use rust_decimal::Decimal;

use super::schema::BUILTIN_MODEL_PRICES;
use super::Database;
use crate::error::AppError;
use crate::services::model_pricing::ModelPricingInfo;

/// 上游写入过、和现在的内置价不同的价格：d35726e2 及以前每个版本的 seed 值、迁移里的
/// 修正值，以及 `repair_current_model_pricing` 修正前后的值。每项：(模型, [输入, 输出,
/// 缓存读, 缓存写])。
///
/// 上游 seed 用 `INSERT OR IGNORE`，repair 只修列出来的旧值，所以老用户的库里可能停着
/// 任何一个版本的 seed 值，只看 d35726e2 一个版本会把它们当成手改价。由
/// `git log d35726e2 -- src-tauri/src/database/schema.rs` 的每个版本生成（不含测试代码）。
pub(crate) const PREVIOUS_BUILTIN_PRICES: &[(&str, [&str; 4])] = &[
    ("claude-sonnet-5", ["3", "15", "0.30", "3.75"]),
    ("deepseek-chat", ["0.14", "0.28", "0.0028", "0"]),
    ("deepseek-chat", ["0.27", "1.10", "0.07", "0"]),
    ("deepseek-chat", ["0.28", "0.42", "0.028", "0"]),
    ("deepseek-reasoner", ["0.14", "0.28", "0.0028", "0"]),
    ("deepseek-reasoner", ["0.28", "0.42", "0.028", "0"]),
    ("deepseek-reasoner", ["0.55", "2.19", "0.14", "0"]),
    ("deepseek-v3", ["2.00", "8.00", "0.40", "0"]),
    ("deepseek-v3.1", ["4.00", "12.00", "0.80", "0"]),
    ("deepseek-v3.2", ["2.00", "3.00", "0.40", "0"]),
    ("deepseek-v4-flash", ["0.14", "0.28", "0.0028", "0"]),
    ("deepseek-v4-flash", ["0.14", "0.28", "0.028", "0"]),
    ("deepseek-v4-flash", ["0.44", "1.32", "0.014", "0"]),
    ("deepseek-v4-flash-0731", ["0.14", "0.28", "0.0028", "0"]),
    ("deepseek-v4-flash-0731", ["0.44", "1.32", "0.014", "0"]),
    ("deepseek-v4-pro", ["0.3", "1.2", "0.006", "0"]),
    ("deepseek-v4-pro", ["0.435", "0.87", "0.003625", "0"]),
    ("deepseek-v4-pro", ["1.68", "3.36", "0.14", "0"]),
    ("devstral-2-2512", ["0.40", "0.90", "0.04", "0"]),
    ("doubao-seed-2-0-code", ["0.47", "2.37", "0", "0"]),
    (
        "doubao-seed-2-0-code-preview-latest",
        ["0.47", "2.37", "0", "0"],
    ),
    ("doubao-seed-2-0-lite", ["0.25", "2", "0", "0"]),
    ("doubao-seed-2-0-mini", ["0.03", "0.31", "0", "0"]),
    ("doubao-seed-2-0-pro", ["0.47", "2.37", "0", "0"]),
    ("doubao-seed-code", ["1.20", "8.00", "0.24", "0"]),
    ("gemini-3-pro-preview", ["2", "12", "0", "0"]),
    ("gemini-3.6-flash", ["1.50", "7.50", "0.15", "0"]),
    ("glm-4.6", ["0.28", "1.11", "0.03", "0"]),
    ("glm-4.6", ["2.00", "8.00", "0.40", "0"]),
    ("glm-4.7", ["0.39", "1.75", "0.04", "0"]),
    ("glm-4.7", ["2.00", "8.00", "0.40", "0"]),
    ("glm-5", ["0.72", "2.30", "0", "0"]),
    ("glm-5.1", ["0.95", "3.15", "0", "0"]),
    ("gpt-5.1-codex-mini", ["1.25", "10", "0.125", "0"]),
    ("gpt-5.6", ["5", "30", "0.50", "6.25"]),
    ("gpt-5.6-high", ["5", "30", "0.50", "6.25"]),
    ("gpt-5.6-low", ["5", "30", "0.50", "6.25"]),
    ("gpt-5.6-luna", ["1", "6", "0.10", "0"]),
    ("gpt-5.6-luna", ["1", "6", "0.10", "1.25"]),
    ("gpt-5.6-medium", ["5", "30", "0.50", "6.25"]),
    ("gpt-5.6-minimal", ["5", "30", "0.50", "6.25"]),
    ("gpt-5.6-sol", ["5", "30", "0.50", "0"]),
    ("gpt-5.6-sol", ["5", "30", "0.50", "6.25"]),
    ("gpt-5.6-terra", ["2.50", "15", "0.25", "0"]),
    ("gpt-5.6-terra", ["2.50", "15", "0.25", "3.125"]),
    ("gpt-5.6-xhigh", ["5", "30", "0.50", "6.25"]),
    ("grok-3", ["3", "15", "0.75", "0"]),
    ("grok-3-mini", ["0.25", "0.50", "0.075", "0"]),
    ("grok-4", ["3", "15", "0.75", "0"]),
    ("grok-4-1-fast-non-reasoning", ["0.20", "0.50", "0.05", "0"]),
    ("grok-4-1-fast-reasoning", ["0.20", "0.50", "0.05", "0"]),
    (
        "grok-4.20-0309-non-reasoning",
        ["1.25", "2.50", "0.20", "0"],
    ),
    ("grok-4.20-0309-non-reasoning", ["2", "6", "0.20", "0"]),
    ("grok-4.20-0309-reasoning", ["1.25", "2.50", "0.20", "0"]),
    ("grok-4.20-0309-reasoning", ["2", "6", "0.20", "0"]),
    ("grok-4.3", ["1.25", "2.50", "0.20", "0"]),
    ("grok-4.5", ["2", "6", "0.30", "0"]),
    ("grok-4.5", ["2", "6", "0.50", "0"]),
    ("grok-4.5-build", ["2", "6", "0.30", "0"]),
    ("grok-4.6", ["2", "6", "0.50", "0"]),
    ("grok-4.7", ["2", "6", "0.50", "0"]),
    ("grok-build-0.1", ["1", "2", "0.20", "0"]),
    ("grok-code-fast-1", ["0.20", "1.50", "0.02", "0"]),
    ("grok-code-fast-1", ["1", "2", "0.20", "0"]),
    ("kimi-k2-0905", ["4.00", "16.00", "1.00", "0"]),
    ("kimi-k2-thinking", ["4.00", "16.00", "1.00", "0"]),
    ("kimi-k2-turbo", ["8.00", "58.00", "1.00", "0"]),
    ("kimi-k2.5", ["0.60", "2.50", "0.10", "0"]),
    ("kimi-k2.6", ["0.60", "2.50", "0.10", "0"]),
    ("mimo-v2-flash", ["0", "0", "0", "0"]),
    ("mimo-v2-pro", ["1", "3", "0", "0"]),
    ("mimo-v2.5", ["0.09", "0.29", "0.009", "0"]),
    ("mimo-v2.5", ["0.14", "0.29", "0.0028", "0"]),
    ("mimo-v2.5-pro", ["1", "3", "0", "0"]),
    ("minimax-m2", ["0.27", "0.95", "0.03", "0"]),
    ("minimax-m2", ["2.10", "8.40", "0.21", "0"]),
    ("minimax-m2.1", ["0.27", "0.95", "0.03", "0"]),
    ("minimax-m2.1", ["2.10", "8.40", "0.21", "0"]),
    ("minimax-m2.1-lightning", ["2.10", "16.80", "0.21", "0"]),
    ("minimax-m2.5", ["0.12", "0.95", "0.03", "0"]),
    ("minimax-m2.5", ["0.15", "0.95", "0.03", "0"]),
    ("minimax-m3", ["0.60", "2.40", "0.12", "0"]),
    ("o3-mini", ["0.55", "2.20", "0.55", "0"]),
    ("qwen3-coder-flash", ["0.195", "0.975", "0", "0"]),
    ("qwen3-coder-plus", ["0.65", "3.25", "0", "0"]),
    ("qwen3.5-plus", ["0.26", "1.56", "0", "0"]),
    ("qwen3.6-plus", ["0.325", "1.95", "0", "0"]),
];

fn prices_key(prices: [&str; 4]) -> Option<[Decimal; 4]> {
    let mut key = [Decimal::ZERO; 4];
    for (slot, value) in key.iter_mut().zip(prices) {
        *slot = Decimal::from_str(value.trim()).ok()?.normalize();
    }
    Some(key)
}

/// 内置模型里价格被用户改过的行。
pub(crate) fn hand_edited_builtin_prices(
    conn: &Connection,
) -> Result<Vec<ModelPricingInfo>, AppError> {
    if !Database::table_exists(conn, "model_pricing")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT display_name, input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
         FROM model_pricing WHERE model_id = ?1",
    )?;
    let mut edited = Vec::new();
    for (model_id, _, input, output, cache_read, cache_creation) in BUILTIN_MODEL_PRICES {
        let Some(row) = stmt
            .query_row([model_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    [
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ],
                ))
            })
            .optional()?
        else {
            continue;
        };
        let (display_name, stored) = row;
        let Some(stored_key) = prices_key(stored.each_ref().map(String::as_str)) else {
            continue;
        };
        let known = std::iter::once([*input, *output, *cache_read, *cache_creation])
            .chain(
                PREVIOUS_BUILTIN_PRICES
                    .iter()
                    .filter(|(previous, _)| previous == model_id)
                    .map(|(_, prices)| *prices),
            )
            .filter_map(prices_key)
            .any(|key| key == stored_key);
        if known {
            continue;
        }
        let [input, output, cache_read, cache_creation] = stored;
        edited.push(ModelPricingInfo {
            model_id: model_id.to_string(),
            display_name,
            input_cost_per_million: input,
            output_cost_per_million: output,
            cache_read_cost_per_million: cache_read,
            cache_creation_cost_per_million: cache_creation,
            long_context_tiers: None,
        });
    }
    Ok(edited)
}

/// 把改过的内置价写进覆盖文件（文件里已有这个模型、或已删除的不动）。没有要导出的
/// 行时不碰文件。导出的模型启动后提示用户一次（见 `init_status`）。
pub(crate) fn export_hand_edited_builtin_prices(conn: &Connection) -> Result<(), AppError> {
    let edited = hand_edited_builtin_prices(conn)?;
    if edited.is_empty() {
        return Ok(());
    }
    let added = crate::services::model_pricing::add_missing_overrides(edited)?;
    if !added.is_empty() {
        log::info!(
            "已把数据库里改过的内置模型价格导出到 model-pricing.json: {}",
            added.join(", ")
        );
        crate::init_status::add_exported_builtin_prices(&added);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_prices(conn: &Connection, model_id: &str, prices: [&str; 4]) {
        conn.execute(
            "UPDATE model_pricing SET input_cost_per_million = ?2,
                 output_cost_per_million = ?3, cache_read_cost_per_million = ?4,
                 cache_creation_cost_per_million = ?5
             WHERE model_id = ?1",
            rusqlite::params![model_id, prices[0], prices[1], prices[2], prices[3]],
        )
        .expect("update prices");
    }

    #[test]
    fn previous_prices_name_built_in_models_and_parse() {
        for (model_id, prices) in PREVIOUS_BUILTIN_PRICES {
            assert!(
                BUILTIN_MODEL_PRICES.iter().any(|(id, ..)| id == model_id),
                "{model_id} is not a built-in model"
            );
            assert!(prices_key(*prices).is_some(), "{model_id}: {prices:?}");
        }
    }

    /// 老版本上游 seed 的值（repair 没修过）停在库里，不是用户改的
    #[test]
    fn seed_values_of_older_upstream_versions_are_not_hand_edits() {
        let db = Database::memory().expect("memory db");
        let conn = db.conn.lock().expect("lock db");
        // 5376ea04 的 seed（人民币价当美元写），46051317 和 b1103c8a 的 seed
        set_prices(&conn, "deepseek-v3", ["2.00", "8.00", "0.40", "0"]);
        set_prices(&conn, "deepseek-chat", ["0.28", "0.42", "0.028", "0"]);
        set_prices(&conn, "gemini-3-pro-preview", ["2", "12", "0", "0"]);
        set_prices(&conn, "mimo-v2-flash", ["0", "0", "0", "0"]);
        // 用户改过的价格
        set_prices(&conn, "glm-5", ["9", "9", "0", "0"]);

        let edited: Vec<String> = hand_edited_builtin_prices(&conn)
            .expect("compare prices")
            .into_iter()
            .map(|entry| entry.model_id)
            .collect();
        assert_eq!(edited, ["glm-5"]);
    }
}
