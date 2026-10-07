import type { AppId } from "@/lib/api";
import type { ManagedAuthProvider } from "@/lib/api/auth";
import type { Provider } from "@/types";
import { resolveManagedAccountId } from "@/lib/authBinding";

/** 能用托管账号（授权中心里的账号）的应用：Codex 官方卡可以绑定 ChatGPT 账号。 */
export const MANAGED_ACCOUNT_APPS = ["codex"] as const;

export type ManagedAccountApp = (typeof MANAGED_ACCOUNT_APPS)[number];

export type ProvidersByApp = Partial<
  Record<ManagedAccountApp, Record<string, Provider> | undefined>
>;

/** 在用某个托管账号的一个供应商 */
export interface ManagedAccountUser {
  appId: ManagedAccountApp;
  providerId: string;
  name: string;
}

/**
 * 找出绑定了这些账号的供应商。`accountIds` 传一个就是「删除这个账号」，传全部就是「删除全部账号」。
 */
export function findManagedAccountUsers(
  authProvider: ManagedAuthProvider,
  accountIds: readonly string[],
  providersByApp: ProvidersByApp,
): ManagedAccountUser[] {
  const ids = new Set(accountIds);
  const users: ManagedAccountUser[] = [];
  for (const appId of MANAGED_ACCOUNT_APPS) {
    const providers = providersByApp[appId];
    if (!providers) continue;
    for (const provider of Object.values(providers)) {
      const bound = resolveManagedAccountId(
        provider.meta,
        authProvider,
      )?.trim();
      if (bound && ids.has(bound)) {
        users.push({ appId, providerId: provider.id, name: provider.name });
      }
    }
  }
  return users;
}

/** 按应用分组（保持 MANAGED_ACCOUNT_APPS 的顺序），确认框里一行一个应用 */
export function groupUsersByApp(
  users: readonly ManagedAccountUser[],
): { appId: AppId; names: string[] }[] {
  return MANAGED_ACCOUNT_APPS.map((appId) => ({
    appId,
    names: users.filter((user) => user.appId === appId).map((u) => u.name),
  })).filter((group) => group.names.length > 0);
}
