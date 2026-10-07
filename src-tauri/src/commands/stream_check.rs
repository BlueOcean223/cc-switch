//! 供应商连通性检查命令
//!
//! 注意：本检查只探测 base_url 是否可达，不发真实大模型请求。详见 `services::stream_check`。

use crate::app_config::AppType;
use crate::error::AppError;
use crate::services::stream_check::{StreamCheckConfig, StreamCheckResult, StreamCheckService};
use crate::store::AppState;
use tauri::State;

/// 连通性检查（单个供应商）
#[tauri::command]
pub async fn stream_check_provider(
    state: State<'_, AppState>,
    app_type: AppType,
    provider_id: String,
) -> Result<StreamCheckResult, AppError> {
    let providers = state.db.get_all_providers(app_type.as_str())?;
    let provider = providers
        .get(&provider_id)
        .ok_or_else(|| AppError::Message(format!("供应商 {provider_id} 不存在")))?;

    StreamCheckService::check_with_retry(&app_type, provider, &StreamCheckConfig::default()).await
}
