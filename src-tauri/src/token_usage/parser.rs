//! Token 使用量（用量统计扫描 CLI 会话日志时使用）

use serde::{Deserialize, Serialize};

/// Session 日志 request_id 前缀，与 `session_usage.rs` 中的格式保持一致
pub const SESSION_REQUEST_ID_PREFIX: &str = "session:";

/// Token 使用量统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_creation_tokens: u32,
    /// 实际使用的模型名称（如果可用）
    pub model: Option<String>,
}
