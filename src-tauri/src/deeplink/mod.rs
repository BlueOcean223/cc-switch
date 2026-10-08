//! Deep link import functionality for CC Switch
//!
//! This module implements the ccslite:// protocol (and accepts upstream's
//! ccswitch:// links) for importing configurations via deep links. Supports importing:
//! - Provider configurations (Claude/Codex/Gemini)
//! - MCP server configurations
//! - Prompts
//! - Skills
//!

mod mcp;
mod parser;
mod prompt;
mod provider;
mod skill;
mod utils;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

// Re-export public API
pub use mcp::import_mcp_from_deeplink;
pub use parser::parse_deeplink_url;
pub use prompt::import_prompt_from_deeplink;
pub use provider::{import_provider_from_deeplink, parse_and_merge_config};
pub use skill::import_skill_from_deeplink;

/// 系统注册给 ccs-lite 的 URL scheme。上游 CC Switch 注册的是 `ccswitch`。
pub const DEEPLINK_SCHEME: &str = "ccslite";
/// 解析时接受的 scheme：自己的，以及上游一键导入链接用的 `ccswitch`
pub const DEEPLINK_SCHEMES: &[&str] = &[DEEPLINK_SCHEME, "ccswitch"];

/// 是否是 ccs-lite 能处理的深链接（`ccslite://` 或 `ccswitch://`）
///
/// 按字节比较：参数可能是任意命令行参数（第二个实例的可执行文件路径等），按字符串
/// 下标切片遇到多字节字符会 panic。
pub fn is_deeplink_url(url: &str) -> bool {
    let bytes = url.as_bytes();
    DEEPLINK_SCHEMES.iter().any(|scheme| {
        bytes.len() > scheme.len() + 3
            && bytes[..scheme.len()].eq_ignore_ascii_case(scheme.as_bytes())
            && bytes[scheme.len()..].starts_with(b"://")
    })
}

/// Deep link import request model
///
/// Represents a parsed deep link URL ready for processing.
/// This struct contains all possible fields for all resource types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkImportRequest {
    /// Protocol version (e.g., "v1")
    pub version: String,
    /// Resource type to import: "provider" | "prompt" | "mcp" | "skill"
    pub resource: String,

    // ============ Common fields ============
    /// Target application (claude/codex/gemini) - for provider, prompt, skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// Resource name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether to enable after import (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    // ============ Provider-specific fields ============
    /// Provider homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// API endpoint/base URL (supports comma-separated multiple URLs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Optional provider icon name (maps to built-in SVG)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional notes/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Optional Haiku model (Claude only, v3.7.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haiku_model: Option<String>,
    /// Optional Sonnet model (Claude only, v3.7.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sonnet_model: Option<String>,
    /// Optional Opus model (Claude only, v3.7.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opus_model: Option<String>,

    // ============ Prompt-specific fields ============
    /// Base64 encoded Markdown content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Prompt description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // ============ MCP-specific fields ============
    /// Target applications for MCP (comma-separated: "claude,codex,gemini")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps: Option<String>,

    // ============ Skill-specific fields ============
    /// GitHub repository (format: "owner/name")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// Skill directory name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    // ============ Config file fields (v3.8+) ============
    /// Base64 encoded config content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    /// Config format (json/toml)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_format: Option<String>,
    /// Remote config URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_url: Option<String>,

    // ============ Usage script fields (v3.9+) ============
    /// Whether to enable usage query. Defaults to **disabled** — carrying a script
    /// is not itself a decision to run it; the link must say `usageEnabled=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_enabled: Option<bool>,
    /// Base64 encoded usage query script code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_script: Option<String>,
    /// Usage query API key (if different from provider API key)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_api_key: Option<String>,
    /// Usage query base URL (if different from provider endpoint)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_base_url: Option<String>,
    /// Usage query access token (for NewAPI template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_access_token: Option<String>,
    /// Usage query user ID (for NewAPI template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_user_id: Option<String>,
    /// Auto query interval in minutes (0 to disable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_auto_interval: Option<u64>,
}
