import { invoke } from "@tauri-apps/api/core";

export type ManagedAuthProvider = "codex_oauth";

export const CODEX_OAUTH_DUPLICATE_ACCOUNT_ERROR =
  "codex_oauth_duplicate_account";

export interface ManagedAuthAccount {
  id: string;
  provider: ManagedAuthProvider;
  login: string;
  authenticated_at: number;
  /** 账号缺少写入 Codex auth.json 所需的身份或 workspace 信息，需要重新登录。 */
  reauth_required?: boolean;
}

export interface ManagedAuthStatus {
  provider: ManagedAuthProvider;
  authenticated: boolean;
  accounts: ManagedAuthAccount[];
}

export interface ManagedAuthDeviceCodeResponse {
  provider: ManagedAuthProvider;
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export async function authStartLogin(
  targetAccountId?: string,
): Promise<ManagedAuthDeviceCodeResponse> {
  return invoke<ManagedAuthDeviceCodeResponse>("auth_start_login", {
    targetAccountId: targetAccountId || null,
  });
}

export async function authPollForAccount(
  deviceCode: string,
): Promise<ManagedAuthAccount | null> {
  return invoke<ManagedAuthAccount | null>("auth_poll_for_account", {
    deviceCode,
  });
}

export async function authCancelLogin(deviceCode: string): Promise<boolean> {
  return invoke<boolean>("auth_cancel_login", { deviceCode });
}

export async function authListAccounts(): Promise<ManagedAuthAccount[]> {
  return invoke<ManagedAuthAccount[]>("auth_list_accounts");
}

export async function authGetStatus(): Promise<ManagedAuthStatus> {
  return invoke<ManagedAuthStatus>("auth_get_status");
}

export async function authRemoveAccount(accountId: string): Promise<void> {
  return invoke("auth_remove_account", { accountId });
}

export async function authLogout(): Promise<void> {
  return invoke("auth_logout");
}

export const authApi = {
  authStartLogin,
  authPollForAccount,
  authCancelLogin,
  authListAccounts,
  authGetStatus,
  authRemoveAccount,
  authLogout,
};
