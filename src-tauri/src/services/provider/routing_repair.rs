//! 修复上游 CC Switch 本地路由留在客户端配置里的状态：把当前供应商重新写进 live。
//!
//! 检测规则见 `live::legacy_routing`。上游代理还在监听时说明上游正在接管，这里不修复，
//! 只告诉用户；在 ccs-lite 里切换或重新写入会覆盖上游的接管。

use serde::Serialize;

use crate::app_config::AppType;
use crate::error::AppError;
use crate::live::legacy_routing::{self, LegacyRouting};
use crate::provider::Provider;
use crate::store::AppState;

use super::{claude_direct, codex_direct, gemini_direct, grok_direct, ProviderService};

/// 客户端配置停在上游路由状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveRoutingState {
    pub base_url: Option<String>,
    /// 上游代理还在监听（上游正在接管）。
    pub upstream_active: bool,
}

/// 一次检查的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum RoutingRepair {
    /// 已把当前供应商重新写进 live。
    Repaired,
    /// 上游代理还在监听，没有修复。
    UpstreamActive,
    /// 没有当前供应商，没有修复。
    NoCurrentProvider,
    Failed {
        message: String,
    },
}

fn state_of(routing: LegacyRouting) -> LiveRoutingState {
    LiveRoutingState {
        upstream_active: legacy_routing::upstream_is_listening(&routing),
        base_url: routing.base_url,
    }
}

impl ProviderService {
    /// live 是否停在上游路由状态；是的话顺带判断上游代理是否还在监听。
    pub fn live_routing_state(app_type: &AppType) -> Option<LiveRoutingState> {
        legacy_routing::live_routing_state(app_type).map(state_of)
    }

    /// 把当前供应商重新写进 live，不改指针：关键字段换成当前供应商的，所有供应商带进来的
    /// 独有字段（值相同）都删，Codex、Grok 里上游路由留下的表一并删掉。没有当前供应商时
    /// 返回 `false`，不写文件。当前供应商依赖已移除的本地路由、或行里也是占位 Key 时报错：
    /// 照写回去客户端还是连不上。
    pub fn reapply_current(state: &AppState, app_type: &AppType) -> Result<bool, AppError> {
        if !legacy_routing::is_supported(app_type) {
            return Ok(false);
        }
        let _switch_guard = crate::mode::lock_settled_blocking(state, app_type)?;
        let Some(provider) =
            crate::settings::get_effective_current_provider_row(&state.db, app_type)?
        else {
            return Ok(false);
        };
        super::ensure_usable_without_routing(app_type, &provider)?;
        let providers = state.db.get_all_providers(app_type.as_str())?;
        let all: Vec<&Provider> = providers.values().collect();
        let db = state.db.as_ref();
        match app_type {
            AppType::Claude => {
                claude_direct::reapply_clearing(db, &all, &provider)?;
            }
            AppType::Codex => {
                codex_direct::reapply_clearing(db, &state.codex_oauth_manager, &all, &provider)?;
            }
            AppType::Gemini => {
                gemini_direct::reapply(db, &provider)?;
            }
            AppType::GrokBuild => {
                grok_direct::reapply(db, Some(&provider), &provider)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// live 停在上游路由状态、上游代理又不在监听时，重新写入当前供应商。live 不在路由
    /// 状态时返回 `None`。
    pub fn repair_legacy_routing(state: &AppState, app_type: &AppType) -> Option<RoutingRepair> {
        let routing = Self::live_routing_state(app_type)?;
        let app = app_type.as_str();
        if routing.upstream_active {
            log::warn!(
                "上游 CC Switch 正在接管 {app}（{}）；在 ccs-lite 里切换会覆盖它",
                routing
                    .base_url
                    .as_deref()
                    .unwrap_or(legacy_routing::DEFAULT_PROXY_URL)
            );
            return Some(RoutingRepair::UpstreamActive);
        }
        Some(match Self::reapply_current(state, app_type) {
            Ok(true) => {
                log::info!("{app} 的配置停在上游 CC Switch 的路由状态，已重新写入当前供应商");
                RoutingRepair::Repaired
            }
            Ok(false) => {
                log::warn!("{app} 的配置停在上游 CC Switch 的路由状态，但没有当前供应商，未修复");
                RoutingRepair::NoCurrentProvider
            }
            Err(error) => {
                log::warn!("{app} 的配置停在上游 CC Switch 的路由状态，重新写入失败: {error}");
                RoutingRepair::Failed {
                    message: error.to_string(),
                }
            }
        })
    }
}
