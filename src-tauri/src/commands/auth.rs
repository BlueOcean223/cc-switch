//! 认证中心：Codex 托管 ChatGPT 账号的登录、列表和移除。

use tauri::State;

use crate::app_config::AppType;
use crate::codex_oauth_auth::{CodexOAuthError, GitHubAccount, GitHubDeviceCodeResponse};
use crate::commands::codex_oauth::CodexOAuthState;
use crate::store::AppState;

const AUTH_PROVIDER_CODEX_OAUTH: &str = "codex_oauth";

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthAccount {
    pub id: String,
    pub provider: String,
    pub login: String,
    pub authenticated_at: i64,
    /// 旧账号缺少写入原生 Codex auth.json 所需的 id_token。
    pub reauth_required: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthStatus {
    pub provider: String,
    pub authenticated: bool,
    pub accounts: Vec<ManagedAuthAccount>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthDeviceCodeResponse {
    pub provider: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

fn ensure_auth_provider(auth_provider: &str) -> Result<(), String> {
    if auth_provider == AUTH_PROVIDER_CODEX_OAUTH {
        Ok(())
    } else {
        Err(format!("Unsupported auth provider: {auth_provider}"))
    }
}

fn map_account(account: GitHubAccount) -> ManagedAuthAccount {
    ManagedAuthAccount {
        reauth_required: account.reauth_required,
        id: account.id,
        provider: AUTH_PROVIDER_CODEX_OAUTH.to_string(),
        login: account.login,
        authenticated_at: account.authenticated_at,
    }
}

fn map_device_code_response(response: GitHubDeviceCodeResponse) -> ManagedAuthDeviceCodeResponse {
    ManagedAuthDeviceCodeResponse {
        provider: AUTH_PROVIDER_CODEX_OAUTH.to_string(),
        device_code: response.device_code,
        user_code: response.user_code,
        verification_uri: response.verification_uri,
        expires_in: response.expires_in,
        interval: response.interval,
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_start_login(
    auth_provider: String,
    target_account_id: Option<String>,
    codex_state: State<'_, CodexOAuthState>,
) -> Result<ManagedAuthDeviceCodeResponse, String> {
    ensure_auth_provider(&auth_provider)?;
    let response = codex_state
        .0
        .start_device_flow(target_account_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(map_device_code_response(response))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_poll_for_account(
    auth_provider: String,
    device_code: String,
    app_state: State<'_, AppState>,
    codex_state: State<'_, CodexOAuthState>,
) -> Result<Option<ManagedAuthAccount>, String> {
    ensure_auth_provider(&auth_provider)?;
    let auth_manager = &codex_state.0;
    match auth_manager
        .poll_for_token(&device_code, || async {
            app_state
                .switch_locks
                .lock_for_app(AppType::Codex.as_str())
                .await
        })
        .await
    {
        Ok(account) => Ok(account.map(map_account)),
        Err(CodexOAuthError::AuthorizationPending) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_cancel_login(
    auth_provider: String,
    device_code: String,
    codex_state: State<'_, CodexOAuthState>,
) -> Result<bool, String> {
    ensure_auth_provider(&auth_provider)?;
    Ok(codex_state.0.cancel_device_flow(&device_code).await)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_list_accounts(
    auth_provider: String,
    codex_state: State<'_, CodexOAuthState>,
) -> Result<Vec<ManagedAuthAccount>, String> {
    ensure_auth_provider(&auth_provider)?;
    let status = codex_state.0.get_status().await;
    Ok(status.accounts.into_iter().map(map_account).collect())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_get_status(
    auth_provider: String,
    codex_state: State<'_, CodexOAuthState>,
) -> Result<ManagedAuthStatus, String> {
    ensure_auth_provider(&auth_provider)?;
    let status = codex_state.0.get_status().await;
    Ok(ManagedAuthStatus {
        provider: AUTH_PROVIDER_CODEX_OAUTH.to_string(),
        authenticated: status.authenticated,
        accounts: status.accounts.into_iter().map(map_account).collect(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_remove_account(
    auth_provider: String,
    account_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    ensure_auth_provider(&auth_provider)?;
    remove_codex_oauth_account_with_switch_lock(app_state.inner(), &account_id).await
}

pub(crate) async fn remove_codex_oauth_account_with_switch_lock(
    app_state: &AppState,
    account_id: &str,
) -> Result<(), String> {
    // Serialize Auth Center credential deletion with managed provider
    // add/update/switch. Otherwise a switch that already preflighted a bundle
    // could recreate auth.json after removal.
    let _switch_guard = app_state
        .switch_locks
        .lock_for_app(AppType::Codex.as_str())
        .await;
    app_state
        .codex_oauth_manager
        .remove_account(account_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_logout(
    auth_provider: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    ensure_auth_provider(&auth_provider)?;
    logout_codex_oauth_with_switch_lock(app_state.inner()).await
}

pub(crate) async fn logout_codex_oauth_with_switch_lock(
    app_state: &AppState,
) -> Result<(), String> {
    let _switch_guard = app_state
        .switch_locks
        .lock_for_app(AppType::Codex.as_str())
        .await;
    app_state
        .codex_oauth_manager
        .clear_auth()
        .await
        .map_err(|error| error.to_string())
}
