/**
 * Coding Plan 供应商的 base_url 路由表。
 *
 * 与后端 `src-tauri/src/services/coding_plan.rs::detect_provider` 保持一致：
 * 后端靠 `url.contains(...)` 做子串判断（MiniMax 例外，按 host 标签匹配），
 * 前端这里用 RegExp 做同效匹配。
 * 新增供应商时改这一处即可（UsageScriptModal 下拉 + useProviderActions
 * 新建自动注入 + 托盘识别全部复用）。
 */
import { createUsageScript } from "@/types";
import { TEMPLATE_TYPES } from "@/config/constants";
import { extractCodexBaseUrl } from "@/utils/providerConfigUtils";
import { piBuiltInBaseUrl } from "@/config/piProviderPresets";

export interface CodingPlanProviderEntry {
  /** 与后端 QuotaTier 的 `codingPlanProvider` 取值对齐 */
  id:
    | "kimi"
    | "zhipu"
    | "zhipu_team"
    | "minimax"
    | "zenmux"
    | "volcengine"
    | "opencode_go"
    | "command_code";
  /** UsageScriptModal 下拉显示用 */
  label: string;
  /** base_url 匹配规则 */
  pattern: RegExp;
}

/**
 * MiniMax 的 base_url。按 host 标签边界匹配（同后端 codex_url_host_matches_any），
 * 不认 api.minimax.cn.example.com 这类伪造后缀或出现在路径里的域名。
 */
export const MINIMAX_BASE_URL_PATTERN =
  /^(?:https?:\/\/)?(?:[^/?#@]*@)?(?:[\w-]+\.)*api\.(?:minimaxi\.com|minimax\.(?:io|cn))(?=[:/?#]|$)/i;

/**
 * MiniMax 按量计费的 Key 以 `sk-api-` 开头，没有 Token Plan，只能查账户余额
 * （官方 CLI 同样按前缀区分）。与后端 `is_minimax_pay_as_you_go_key` 一致。
 */
export function isMiniMaxPayAsYouGoKey(apiKey: unknown): boolean {
  return typeof apiKey === "string" && apiKey.startsWith("sk-api-");
}

export const CODING_PLAN_PROVIDERS: readonly CodingPlanProviderEntry[] = [
  {
    id: "kimi",
    label: "Kimi For Coding",
    pattern: /api\.kimi\.(?:com|ai)\/coding/i,
  },
  {
    id: "zhipu",
    label: "Zhipu GLM (智谱)",
    pattern: /bigmodel\.cn|api\.z\.ai/i,
  },
  {
    // 智谱团队套餐（Team Plan）。base_url 与个人版智谱（open.bigmodel.cn）相同，
    // 无法靠 base_url 自动区分——靠显式 codingPlanProvider === "zhipu_team" 路由。
    // 个人版 zhipu 排在前面，detectCodingPlanProvider 首匹配仍命中个人版，
    // 故团队版永不被 injectCodingPlanUsageScript 自动注入（必须用户手动选）。
    // pattern 仅占位（下拉展示用），实际不参与自动检测。
    id: "zhipu_team",
    label: "Zhipu GLM Team (智谱团队)",
    pattern: /bigmodel\.cn/i,
  },
  {
    id: "minimax",
    label: "MiniMax",
    pattern: MINIMAX_BASE_URL_PATTERN,
  },
  {
    id: "zenmux",
    label: "ZenMux",
    pattern: /zenmux\./i,
  },
  {
    // 火山方舟 Agent Plan / Coding Plan。base_url 形如
    // ark.cn-beijing.volces.com/api/plan[/v3]（Agent Plan）或
    // /api/coding[/v3]（Coding Plan）；与后端 detect_provider 的
    // `volces.com/api/plan` / `volces.com/api/coding` 子串判断同效。
    id: "volcengine",
    label: "火山方舟 (Volcengine)",
    pattern: /volces\.com\/api\/(plan|coding)/i,
  },
  {
    // OpenCode Go（$10/月订阅，三时间窗口美元额度）。用量端点
    // GET /zen/go/v1/usage 是官方第一方但未文档化的路由，只认
    // Authorization: Bearer（与推理侧 /messages 只认 x-api-key 相反）。
    // base 分两档：/zen/go（claude 直连 /messages）与
    // /zen/go/v1（opencode/pi 等走 Chat），子串同时覆盖；
    // Zen 按量版（/zen/v1）没有用量 API，刻意不命中。
    id: "opencode_go",
    label: "OpenCode Go",
    pattern: /opencode\.ai\/zen\/go/i,
  },
  {
    // Command Code 的余额与窗口接口由官方 CLI 使用，当前未公开文档化。
    // Claude 使用 /provider，Codex 使用 /provider/v1。
    id: "command_code",
    label: "Command Code",
    pattern: /api\.commandcode\.ai\/provider(?:[/?#]|$)/i,
  },
] as const;

/** 根据 Base URL 自动检测 Coding Plan 供应商；未命中返回 null */
export function detectCodingPlanProvider(
  baseUrl: string | undefined | null,
): CodingPlanProviderEntry["id"] | null {
  if (!baseUrl) return null;
  for (const cp of CODING_PLAN_PROVIDERS) {
    if (cp.pattern.test(baseUrl)) return cp.id;
  }
  return null;
}

/**
 * 按 app 从 settingsConfig 里取出 base_url，供自动注入检测用。
 * 提取路径与后端 `Provider::resolve_usage_credentials` 的各 app 分支对齐
 * （token_plan 查询最终用的就是那份凭据，两边不一致会注入了却查不到）。
 */
export function extractBaseUrlForUsageDetection(
  appId: string,
  settingsConfig: Record<string, any> | undefined,
  providerKey?: string,
): string | null {
  if (!settingsConfig) return null;
  let raw: unknown;
  switch (appId) {
    case "claude":
      raw = settingsConfig.env?.ANTHROPIC_BASE_URL;
      break;
    case "codex":
      raw = extractCodexBaseUrl(
        typeof settingsConfig.config === "string"
          ? settingsConfig.config
          : null,
      );
      break;
    case "opencode":
      raw = settingsConfig.options?.baseURL;
      break;
    case "pi":
      // Pi 内置供应商的条目只写 key，地址按供应商 ID 找
      raw =
        settingsConfig.baseUrl ??
        (providerKey ? piBuiltInBaseUrl(providerKey) : undefined);
      break;
    default:
      return null;
  }
  return typeof raw === "string" ? raw : null;
}

/**
 * 新建供应商时，若 base_url 命中 Coding Plan 路由表，自动把
 * `meta.usage_script` 标记为 token_plan 并启用。
 *
 * - 仅在 `meta.usage_script` 完全缺失时注入，不覆盖用户/UsageScriptModal 已有配置
 * - Claude app 保持既有行为：命中任意 Coding Plan 供应商都注入；
 *   其余 app 仅对已验证预设注入：Codex 支持 Command Code，其他非 Claude
 *   Coding Plan 保持既有行为——
 *   多个 app 各有一份 OpenCode Go 预设、凭据形态后端全部支持，而智谱/Kimi
 *   等在其他 app 的自动注入未逐一验证过，不随手扩大
 * - code 置空：Rust 端走专用 `coding_plan::get_coding_plan_quota`，不执行 JS 脚本
 */
export function injectCodingPlanUsageScript<
  T extends {
    settingsConfig?: Record<string, any>;
    meta?: Record<string, any>;
    providerKey?: string;
  },
>(appId: string, provider: T): T {
  if (provider.meta?.usage_script) return provider;

  const baseUrl = extractBaseUrlForUsageDetection(
    appId,
    provider.settingsConfig,
    provider.providerKey,
  );
  const codingPlanProvider = detectCodingPlanProvider(baseUrl);
  if (!codingPlanProvider) return provider;
  const isCommandCodeForCodex =
    appId === "codex" && codingPlanProvider === "command_code";
  if (
    appId !== "claude" &&
    codingPlanProvider !== "opencode_go" &&
    !isCommandCodeForCodex
  ) {
    return provider;
  }

  // MiniMax 走到这里只可能是 Claude；按量计费的 Key 没有 Token Plan，改查余额
  const env = provider.settingsConfig?.env;
  const usageScript =
    codingPlanProvider === "minimax" &&
    isMiniMaxPayAsYouGoKey(env?.ANTHROPIC_AUTH_TOKEN || env?.ANTHROPIC_API_KEY)
      ? createUsageScript({
          enabled: true,
          templateType: TEMPLATE_TYPES.BALANCE,
        })
      : createUsageScript({
          enabled: true,
          templateType: TEMPLATE_TYPES.TOKEN_PLAN,
          codingPlanProvider,
        });

  return {
    ...provider,
    meta: {
      ...(provider.meta ?? {}),
      usage_script: usageScript,
    },
  };
}
