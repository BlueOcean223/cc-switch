//! Token 使用量（用量统计扫描 CLI 会话日志时使用）

use serde::{Deserialize, Serialize};

/// Session 日志 request_id 前缀，与 `session_usage.rs` 中的格式保持一致
pub const SESSION_REQUEST_ID_PREFIX: &str = "session:";

/// 一次请求的 token 用量，各桶互不重叠。
///
/// 各 CLI 日志的口径不同：Claude、OpenCode、Pi 的 input 本来就不含缓存；
/// Codex、Gemini、Grok 的 input 含缓存读（和缓存写），导入器要先减掉再填这里。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 未命中缓存的输入
    pub input_tokens: u32,
    /// 输出，含推理 / thinking
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    /// 全部缓存写入（含 1 小时缓存）
    pub cache_creation_tokens: u32,
    /// `cache_creation_tokens` 中按 1 小时缓存写入的部分（只有 Anthropic 有）
    pub cache_creation_1h_tokens: u32,
}

impl TokenUsage {
    /// 提示长度：未命中缓存的输入 + 缓存读 + 缓存写。超长上下文档位按它判断。
    pub fn prompt_tokens(&self) -> u64 {
        u64::from(self.input_tokens)
            + u64::from(self.cache_read_tokens)
            + u64::from(self.cache_creation_tokens)
    }

    pub fn is_empty(&self) -> bool {
        self.input_tokens == 0
            && self.output_tokens == 0
            && self.cache_read_tokens == 0
            && self.cache_creation_tokens == 0
    }
}
