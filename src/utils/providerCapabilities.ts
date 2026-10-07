import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";
import { resolveManagedAccountId } from "@/lib/authBinding";
import {
  extractCodexBaseUrl,
  extractCodexExperimentalBearerToken,
  hasExplicitNonOpenAiCodexModelProvider,
} from "@/utils/providerConfigUtils";

export const CODEX_OFFICIAL_PROVIDER_ID = "codex-official";
export const GROKBUILD_OFFICIAL_PROVIDER_ID = "grokbuild-official";

export type CodexOfficialIdentity =
  | "native_login"
  | "managed_account"
  | "api_key";

const nonEmptyString = (value: unknown): boolean =>
  typeof value === "string" && value.trim().length > 0;

function hasExplicitCodexThirdPartyUpstream(
  settings: Record<string, unknown>,
): boolean {
  const config = typeof settings.config === "string" ? settings.config : "";

  return (
    nonEmptyString(settings.baseUrl) ||
    nonEmptyString(settings.baseURL) ||
    nonEmptyString(settings.base_url) ||
    Boolean(extractCodexExperimentalBearerToken(config)) ||
    Boolean(extractCodexBaseUrl(config)) ||
    hasExplicitNonOpenAiCodexModelProvider(config)
  );
}

function hasStoredCodexApiKey(settings: Record<string, unknown>): boolean {
  const auth = settings.auth as Record<string, unknown> | undefined;
  return nonEmptyString(auth?.OPENAI_API_KEY);
}

export function resolveCodexOfficialIdentity(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
): CodexOfficialIdentity | null {
  if (appId !== "codex") return null;

  const managedAccountId = resolveManagedAccountId(
    provider.meta,
    "codex_oauth",
  )?.trim();
  const hasFixedOfficialId = provider.id === CODEX_OFFICIAL_PROVIDER_ID;
  if (hasFixedOfficialId && provider.category === "official") {
    return managedAccountId ? "managed_account" : "native_login";
  }

  const settings = provider.settingsConfig as Record<string, unknown>;
  const auth = settings?.auth;
  const config = settings?.config;
  if (
    !auth ||
    typeof auth !== "object" ||
    Array.isArray(auth) ||
    (config != null && typeof config !== "string")
  ) {
    return null;
  }

  if (hasExplicitCodexThirdPartyUpstream(settings)) {
    return null;
  }

  if (managedAccountId) {
    return "managed_account";
  }
  if (hasStoredCodexApiKey(settings)) {
    return provider.category === "official" ? "api_key" : null;
  }
  return hasFixedOfficialId || provider.category === "official"
    ? "native_login"
    : null;
}

/**
 * 官方账号卡（对应后端 `codex_direct::is_official`）：Codex 早期绑定托管账号的官方卡
 * 没有 category，按身份认。
 */
export function isOfficialAccount(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
): boolean {
  return (
    provider.category === "official" ||
    resolveCodexOfficialIdentity(appId, provider) !== null
  );
}
