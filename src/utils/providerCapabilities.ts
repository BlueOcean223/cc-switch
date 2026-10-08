import { parse as parseToml } from "smol-toml";
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

/** 上游这几种托管登录的凭据由本地代理按请求注入，没有路由就用不了。 */
const MANAGED_OAUTH_PROVIDER_TYPES = [
  "github_copilot",
  "codex_oauth",
  "xai_oauth",
] as const;

const CHAT_OR_ANTHROPIC_WIRE_APIS = new Set([
  "chat",
  "chat_completions",
  "chat-completions",
  "openai_chat",
  "openai-chat",
  "openai_chat_completions",
  "anthropic",
  "anthropic_messages",
  "anthropic-messages",
  "messages",
  "claude",
]);

const asTable = (value: unknown): Record<string, unknown> | undefined =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;

function parseConfig(config: unknown): Record<string, unknown> | undefined {
  if (typeof config !== "string") return undefined;
  try {
    return asTable(parseToml(config));
  } catch {
    return undefined;
  }
}

/** Codex 配置里生效 provider 的 `wire_api`，没有时取顶层的。 */
function codexWireApi(config: unknown): string | undefined {
  const root = parseConfig(config);
  if (!root) return undefined;
  const active = root.model_provider;
  const table =
    typeof active === "string"
      ? asTable(asTable(root.model_providers)?.[active])
      : undefined;
  const value = table?.wire_api ?? root.wire_api;
  return typeof value === "string" ? value : undefined;
}

/** Grok Build 选中模型表的 `api_backend`（不写时 Grok 用 chat_completions）。 */
function grokApiBackend(config: unknown): string | undefined {
  const root = parseConfig(config);
  const selected = asTable(root?.models)?.default;
  if (typeof selected !== "string") return undefined;
  const table = asTable(asTable(root?.model)?.[selected.trim()]);
  // 和后端 `extract_model_config` 一样，缺 model 或 base_url 的表读不出来
  if (typeof table?.model !== "string" || typeof table.base_url !== "string") {
    return undefined;
  }
  const backend = table.api_backend;
  return typeof backend === "string" && backend.trim()
    ? backend.trim()
    : "chat_completions";
}

/** 上游代理要转换成别的协议的 Claude 接口格式 */
const CLAUDE_TRANSFORMED_FORMATS = new Set([
  "openai_chat",
  "openai_responses",
  "gemini_native",
]);

/**
 * Claude 卡的接口格式要不要上游代理转换，优先级照上游代理：`meta.apiFormat` > 旧版写在
 * settings 里的 `api_format` > 旧版的 `openrouter_compat_mode`（开着就是 openai_chat），
 * 认不出的值按 anthropic。
 */
function claudeNeedsTransform(
  metaFormat: string | undefined,
  settings: Record<string, unknown> | undefined,
): boolean {
  if (metaFormat !== undefined)
    return CLAUDE_TRANSFORMED_FORMATS.has(metaFormat);
  const legacyFormat = settings?.api_format;
  if (typeof legacyFormat === "string") {
    return CLAUDE_TRANSFORMED_FORMATS.has(legacyFormat);
  }
  const compat = settings?.openrouter_compat_mode;
  if (typeof compat === "boolean") return compat;
  if (typeof compat === "number") return Math.trunc(compat) !== 0;
  if (typeof compat === "string") {
    return ["true", "1"].includes(compat.trim().toLowerCase());
  }
  return false;
}

/**
 * 这个供应商只能经过上游 CC Switch 已移除的本地路由使用（托管登录、需要格式转换的接口、
 * 完整 URL），切换过去客户端用不了。和后端 `legacy_routing::requires_removed_routing`
 * 是同一条规则，改一边要改另一边。
 */
export function requiresRemovedRouting(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
): boolean {
  if (isOfficialAccount(appId, provider)) return false;
  const meta = provider.meta;
  const managedOAuth = MANAGED_OAUTH_PROVIDER_TYPES.some(
    (kind) => kind === meta?.providerType,
  );
  const fullUrl = meta?.isFullUrl === true;
  const apiFormat =
    typeof meta?.apiFormat === "string" && meta.apiFormat.trim()
      ? meta.apiFormat.trim()
      : undefined;
  const config = (provider.settingsConfig as Record<string, unknown>)?.config;

  switch (appId) {
    case "claude":
      return (
        managedOAuth ||
        fullUrl ||
        claudeNeedsTransform(
          apiFormat,
          provider.settingsConfig as Record<string, unknown>,
        )
      );
    case "codex": {
      if (managedOAuth || fullUrl) return true;
      if (apiFormat === "openai_chat" || apiFormat === "anthropic") return true;
      const wireApi = codexWireApi(config);
      return (
        wireApi !== undefined &&
        CHAT_OR_ANTHROPIC_WIRE_APIS.has(wireApi.trim().toLowerCase())
      );
    }
    case "grokbuild": {
      if (managedOAuth || fullUrl) return true;
      // Grok Build 自己能接 Chat Completions 和 Anthropic Messages：表里的 `api_backend`
      // 改成对应的接口后就能直连。
      const wanted =
        apiFormat === "openai_chat"
          ? "chat_completions"
          : apiFormat === "anthropic"
            ? "messages"
            : undefined;
      return wanted !== undefined && grokApiBackend(config) !== wanted;
    }
    default:
      return false;
  }
}
