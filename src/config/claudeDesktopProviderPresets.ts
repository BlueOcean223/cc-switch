/**
 * Claude Desktop 预设供应商配置模板
 *
 * 形态与 Claude Code 预设不同：
 * - baseUrl 是顶级字段，而不是 settingsConfig.env.ANTHROPIC_BASE_URL
 * - 模型信息以"Desktop 可见模型 ID → 上游模型"表达，
 *   对应后端 ClaudeDesktopModelRoute 的 routeId / model
 *
 * 翻译来源：src/config/claudeProviderPresets.ts（排除 OAuth 与不兼容预设）
 */
import { ProviderCategory } from "../types";
import type { PresetTheme } from "./claudeProviderPresets";
import type { PresetFamilyFields } from "./presetFamilies";

export type ClaudeDesktopApiFormat =
  | "anthropic"
  | "openai_chat"
  | "openai_responses"
  | "gemini_native";

export interface ClaudeDesktopRoutePreset {
  routeId: string;
  upstreamModel: string;
  labelOverride?: string;
  supports1m: boolean;
}

/**
 * Claude Desktop 3P fail-all 校验接受的角色名。Desktop 1.12603.1+ 起白名单
 * 纳入 fable（app.asar 内 ["sonnet","opus","haiku","fable","mythos"]，实测
 * 2026-06-13）；此前 1.6259.1 仅接受 sonnet/opus/haiku。mythos 官方未公开
 * 发布，暂不暴露给用户。所有预设工厂、表单角色下拉、后端
 * `next_catalog_safe_route_id` 都从此映射派生 routeId，避免散落硬编码。
 */
export const CLAUDE_DESKTOP_ROLE_ROUTE_IDS = {
  sonnet: "claude-sonnet-5",
  opus: "claude-opus-5",
  fable: "claude-fable-5",
  haiku: "claude-haiku-4-5",
} as const;

export type ClaudeDesktopRoleId = keyof typeof CLAUDE_DESKTOP_ROLE_ROUTE_IDS;

export interface ClaudeDesktopProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string;
  websiteUrl: string;
  apiKeyUrl?: string;
  category?: ProviderCategory;
  isPartner?: boolean;
  primePartner?: boolean; // 旧版的置顶合作伙伴标记；v7 起界面不再读取，新预设不写
  partnerPromotionKey?: string;

  baseUrl: string;
  apiKeyField?: "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY";

  mode: "direct" | "proxy";
  apiFormat?: ClaudeDesktopApiFormat;
  modelRoutes?: ClaudeDesktopRoutePreset[];
  providerType?: "github_copilot" | "codex_oauth" | "xai_oauth";
  requiresOAuth?: boolean;

  endpointCandidates?: string[];
  theme?: PresetTheme;
  icon?: string;
  iconColor?: string;
}

const passthroughRoutes = (supports1m = false): ClaudeDesktopRoutePreset[] => [
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet,
    upstreamModel: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet,
    supports1m,
  },
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus,
    upstreamModel: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus,
    supports1m,
  },
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku,
    upstreamModel: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku,
    supports1m,
  },
];

const mappedRoutes = (
  sonnet: string,
  opus: string,
  haiku: string,
  supports1m = false,
): ClaudeDesktopRoutePreset[] => [
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet,
    upstreamModel: sonnet,
    supports1m,
  },
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus,
    upstreamModel: opus,
    supports1m,
  },
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku,
    upstreamModel: haiku,
    supports1m,
  },
];

/**
 * 非 Claude 上游模型用此工厂：route ID 使用 Claude Desktop 能通过校验的
 * Sonnet/Opus/Haiku 路由，真实品牌名只写入 labelOverride 和 upstreamModel。
 */
const brandedRoutes = (
  sonnet: string,
  opus: string,
  haiku: string,
  supports1m = false,
): ClaudeDesktopRoutePreset[] => {
  const seenUpstream = new Set<string>();
  return [
    { routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet, upstreamModel: sonnet },
    { routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus, upstreamModel: opus },
    { routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku, upstreamModel: haiku },
  ]
    .map(({ routeId, upstreamModel }) => ({
      routeId,
      upstreamModel,
      labelOverride: upstreamModel,
      supports1m,
    }))
    .filter((route) => {
      if (seenUpstream.has(route.upstreamModel)) {
        return false;
      }
      seenUpstream.add(route.upstreamModel);
      return true;
    });
};

export const claudeDesktopProviderPresets: ClaudeDesktopProviderPreset[] = [
  {
    name: "Claude Desktop Official",
    websiteUrl: "https://claude.ai/download",
    category: "official",
    baseUrl: "",
    mode: "direct",
    apiFormat: "anthropic",
    theme: {
      icon: "claude",
      backgroundColor: "#D97757",
      textColor: "#FFFFFF",
    },
    icon: "anthropic",
    iconColor: "#D4915D",
  },
  {
    name: "Kimi",
    family: "kimi",
    planKey: "payg",
    regionKey: "cn",
    websiteUrl: "https://platform.kimi.com",
    category: "cn_official",
    baseUrl: "https://api.moonshot.cn/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "kimi-k2.7-code",
      "kimi-k2.7-code",
      "kimi-k2.7-code",
    ),
    icon: "kimi",
    iconColor: "#6366F1",
  },
  // API 开放平台海外/Global 变体：platform.kimi.ai + api.moonshot.ai 端点
  {
    name: "Kimi Global",
    family: "kimi",
    planKey: "payg",
    regionKey: "intl",
    websiteUrl: "https://platform.kimi.ai",
    category: "cn_official",
    baseUrl: "https://api.moonshot.ai/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "kimi-k2.7-code",
      "kimi-k2.7-code",
      "kimi-k2.7-code",
    ),
    icon: "kimi",
    iconColor: "#6366F1",
  },
  {
    name: "Kimi For Coding",
    family: "kimi",
    planKey: "coding",
    regionKey: "cn",
    websiteUrl: "https://www.kimi.com/code/",
    category: "cn_official",
    baseUrl: "https://api.kimi.com/coding/",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    icon: "kimi",
    iconColor: "#6366F1",
  },
  // 海外/Global 变体：kimi.ai/code + api.kimi.ai 端点，其余与国内版一致
  {
    name: "Kimi For Coding Global",
    family: "kimi",
    planKey: "coding",
    regionKey: "intl",
    websiteUrl: "https://www.kimi.ai/code",
    category: "cn_official",
    baseUrl: "https://api.kimi.ai/coding/",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    icon: "kimi",
    iconColor: "#6366F1",
  },
  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    category: "aggregator",
    baseUrl: "https://api.qnaigc.com",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://api.qnaigc.com", "https://api.modelink.ai"],
    icon: "qiniu",
  },
  {
    name: "PPIO",
    websiteUrl: "https://ppio.com",
    apiKeyUrl: "https://ppio.com/settings/key-management",
    category: "aggregator",
    baseUrl: "https://api.ppio.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "deepseek/deepseek-v4-flash-0731",
      "deepseek/deepseek-v4-flash-0731",
      "deepseek/deepseek-v4-flash-0731",
      true,
    ),
    endpointCandidates: ["https://api.ppio.com/anthropic"],
    icon: "ppio",
    iconColor: "#2874FF",
  },
  {
    name: "火山 Agent Plan",
    family: "volcengine",
    planKey: "agentPlan",
    websiteUrl: "https://www.volcengine.com/activity/agentplan",
    apiKeyUrl: "https://www.volcengine.com/activity/agentplan",
    category: "cn_official",
    baseUrl: "https://ark.cn-beijing.volces.com/api/plan",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "ark-code-latest",
      "ark-code-latest",
      "ark-code-latest",
    ),
    icon: "huoshan",
    iconColor: "#3370FF",
  },
  {
    name: "火山 Coding Plan",
    family: "volcengine",
    planKey: "codingPlan",
    websiteUrl: "https://www.volcengine.com/activity/codingplan",
    apiKeyUrl: "https://www.volcengine.com/activity/codingplan",
    category: "cn_official",
    baseUrl: "https://ark.cn-beijing.volces.com/api/coding",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "ark-code-latest",
      "ark-code-latest",
      "ark-code-latest",
    ),
    icon: "huoshan",
    iconColor: "#3370FF",
  },
  {
    name: "BytePlus",
    websiteUrl: "https://www.byteplus.com/en/product/modelark",
    apiKeyUrl: "https://www.byteplus.com/en/product/modelark",
    category: "cn_official",
    baseUrl: "https://ark.ap-southeast.bytepluses.com/api/coding",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "ark-code-latest",
      "ark-code-latest",
      "ark-code-latest",
    ),
    icon: "byteplus",
    iconColor: "#3370FF",
  },
  {
    name: "Volcengine Doubao",
    family: "volcengine",
    planKey: "payg",
    nameKey: "providerForm.presets.doubaoseed",
    websiteUrl:
      "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    apiKeyUrl:
      "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    category: "cn_official",
    baseUrl: "https://ark.cn-beijing.volces.com/api/compatible",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "doubao-seed-2-1-pro-260628",
      "doubao-seed-2-1-pro-260628",
      "doubao-seed-2-1-pro-260628",
    ),
    icon: "doubao",
    iconColor: "#3370FF",
  },
  {
    name: "SiliconFlow",
    family: "siliconflow",
    regionKey: "cn",
    websiteUrl: "https://siliconflow.cn",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    category: "aggregator",
    baseUrl: "https://api.siliconflow.cn",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "Pro/MiniMaxAI/MiniMax-M2.5",
      "Pro/MiniMaxAI/MiniMax-M2.5",
      "Pro/MiniMaxAI/MiniMax-M2.5",
    ),
    icon: "siliconflow",
    iconColor: "#6E29F6",
  },
  {
    name: "SiliconFlow en",
    family: "siliconflow",
    regionKey: "intl",
    websiteUrl: "https://siliconflow.com",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    category: "aggregator",
    baseUrl: "https://api.siliconflow.com",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "MiniMaxAI/MiniMax-M3",
      "MiniMaxAI/MiniMax-M3",
      "MiniMaxAI/MiniMax-M3",
    ),
    icon: "siliconflow",
    iconColor: "#000000",
  },
  {
    name: "Compshare",
    family: "compshare",
    planKey: "payg",
    nameKey: "providerForm.presets.ucloud",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl: "https://www.compshare.cn/coding-plan",
    category: "aggregator",
    baseUrl: "https://api.modelverse.cn",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://api.modelverse.cn"],
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "Compshare Coding Plan",
    family: "compshare",
    planKey: "codingPlan",
    nameKey: "providerForm.presets.ucloudCoding",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl: "https://www.compshare.cn/coding-plan",
    category: "aggregator",
    baseUrl: "https://cp.compshare.cn",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://cp.compshare.cn"],
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "AtlasCloud",
    websiteUrl: "https://www.atlascloud.ai/console/coding-plan",
    apiKeyUrl: "https://www.atlascloud.ai/console/coding-plan",
    category: "aggregator",
    baseUrl: "https://api.atlascloud.ai",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://api.atlascloud.ai"],
    icon: "atlascloud",
  },
  {
    name: "Gemini Native",
    websiteUrl: "https://ai.google.dev/gemini-api",
    apiKeyUrl: "https://aistudio.google.com/app/apikey",
    category: "third_party",
    baseUrl: "https://generativelanguage.googleapis.com",
    apiKeyField: "ANTHROPIC_API_KEY",
    mode: "proxy",
    apiFormat: "gemini_native",
    modelRoutes: brandedRoutes(
      "gemini-3.6-flash",
      "gemini-3.6-flash",
      "gemini-3.6-flash",
    ),
    endpointCandidates: ["https://generativelanguage.googleapis.com"],
    icon: "gemini",
    iconColor: "#4285F4",
  },
  {
    name: "GitHub Copilot",
    websiteUrl: "https://github.com/features/copilot",
    category: "third_party",
    baseUrl: "https://api.githubcopilot.com",
    mode: "proxy",
    apiFormat: "openai_chat",
    providerType: "github_copilot",
    requiresOAuth: true,
    modelRoutes: brandedRoutes(
      "claude-sonnet-5",
      "claude-sonnet-5",
      "claude-haiku-4.5",
    ),
    icon: "github",
    iconColor: "#000000",
  },
  {
    name: "Codex",
    websiteUrl: "https://openai.com/chatgpt/pricing",
    category: "third_party",
    baseUrl: "https://chatgpt.com/backend-api/codex",
    mode: "proxy",
    apiFormat: "openai_responses",
    providerType: "codex_oauth",
    requiresOAuth: true,
    modelRoutes: brandedRoutes("gpt-5.6-sol", "gpt-5.6-sol", "gpt-5.6-luna"),
    icon: "openai",
    iconColor: "#000000",
  },
  {
    name: "xAI (Grok)",
    websiteUrl: "https://x.ai/grok",
    category: "third_party",
    baseUrl: "https://api.x.ai/v1",
    mode: "proxy",
    apiFormat: "openai_responses",
    providerType: "xai_oauth",
    requiresOAuth: true,
    modelRoutes: brandedRoutes("grok-4.5", "grok-4.5", "grok-4.5"),
    icon: "xai",
    iconColor: "#000000",
  },
  {
    name: "DeepSeek",
    websiteUrl: "https://platform.deepseek.com",
    category: "cn_official",
    baseUrl: "https://api.deepseek.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    // supports1m：两个档位钉的都是 1M 窗口模型（本仓 Codex catalog 记
    // deepseek-v4-pro / deepseek-flash 均 1048576；haiku 档的
    // deepseek-v4-flash 被官方端点路由到 V4.1 Flash，窗口同档）。[1m] 只是
    // Claude Desktop 本地标记，匹配前会被剥掉，不会随请求发往上游
    modelRoutes: brandedRoutes(
      "deepseek-v4-pro",
      "deepseek-v4-pro",
      "deepseek-flash",
      true,
    ),
    icon: "deepseek",
    iconColor: "#1E88E5",
  },
  {
    name: "OpenCode Go",
    websiteUrl: "https://opencode.ai/go",
    apiKeyUrl: "https://opencode.ai/go",
    category: "third_party",
    baseUrl: "https://opencode.ai/zen/go",
    mode: "proxy",
    // Go 网关 /messages 收除 grok-4.5 外全部模型（Chat 组靠服务端转换），
    // anthropic 透传即可；上游只认 x-api-key，apiKey 直填默认即该头。
    apiFormat: "anthropic",
    // supports1m：deepseek-v4-flash 窗口 1M（本仓 Go 网关 Codex catalog
    // 记 1048576）。[1m] 只是 Claude Desktop 本地标记，匹配前会被剥掉，
    // 不会随请求发往上游
    modelRoutes: brandedRoutes(
      "deepseek-v4-flash",
      "deepseek-v4-flash",
      "deepseek-v4-flash",
      true,
    ),
    endpointCandidates: ["https://opencode.ai/zen/go"],
    icon: "opencode",
    iconColor: "#211E1E",
  },
  {
    // 腾讯云 Token Plan 个人版：通用 + Hy 两系列共用端点与 Key，
    // Auto 智能路由调用 ID 为 tc-code-latest（1823/130060）
    name: "Tencent Token Plan",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan",
    category: "cn_official",
    baseUrl: "https://api.lkeap.cloud.tencent.com/plan/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "tc-code-latest",
      "tc-code-latest",
      "tc-code-latest",
    ),
    endpointCandidates: ["https://api.lkeap.cloud.tencent.com/plan/anthropic"],
    icon: "tencent",
    iconColor: "#0052D9",
  },
  {
    // 国际站（新加坡）个人版（intl 1300/81315）：Auto 调用 ID 是 auto
    name: "Tencent Token Plan (Intl)",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan",
    category: "cn_official",
    baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("auto", "auto", "auto"),
    endpointCandidates: [
      "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
    ],
    icon: "tencent",
    iconColor: "#0052D9",
  },
  {
    // Token Plan 企业版专业套餐（1823/130659，广州地域）
    name: "Tencent Token Plan Enterprise Pro",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    category: "cn_official",
    baseUrl: "https://tokenhub.tencentmaas.com/plan/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("auto", "auto", "auto"),
    // 广州地域为默认端点；国内站企业套餐另可选新加坡地域（1823/130659、
    // 131173 双地域表：tokenhub-intl.tencentmaas.com，需开通新加坡地域，
    // 不支持跨地域调用，故仅作候选端点）
    endpointCandidates: [
      "https://tokenhub.tencentmaas.com/plan/anthropic",
      "https://tokenhub-intl.tencentmaas.com/plan/anthropic",
    ],
    icon: "tencent",
    iconColor: "#0052D9",
  },
  {
    // 国际站企业版专业套餐（intl 1300/81489，新加坡地域）
    name: "Tencent Token Plan Enterprise Pro (Intl)",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    category: "cn_official",
    baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("auto", "auto", "auto"),
    // 新加坡地域为默认端点；国际站企业套餐另可选广州地域（1300/81489、
    // 81490 双地域表：tokenhub.tencentcloudmaas.com，需开通广州地域，
    // 不支持跨地域调用，故仅作候选端点）
    endpointCandidates: [
      "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
      "https://tokenhub.tencentcloudmaas.com/plan/anthropic",
    ],
    icon: "tencent",
    iconColor: "#0052D9",
  },
  {
    // Token Plan 企业版轻享套餐（1823/131173）：仅 Auto 模型
    name: "Tencent Token Plan Enterprise Lite",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    category: "cn_official",
    baseUrl: "https://tokenhub.tencentmaas.com/plan/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("auto", "auto", "auto"),
    // 广州地域为默认端点；国内站企业套餐另可选新加坡地域（1823/130659、
    // 131173 双地域表：tokenhub-intl.tencentmaas.com，需开通新加坡地域，
    // 不支持跨地域调用，故仅作候选端点）
    endpointCandidates: [
      "https://tokenhub.tencentmaas.com/plan/anthropic",
      "https://tokenhub-intl.tencentmaas.com/plan/anthropic",
    ],
    icon: "tencent",
    iconColor: "#0052D9",
  },
  {
    // 国际站企业版轻享套餐（intl 1300/81490，新加坡地域）
    name: "Tencent Token Plan Enterprise Lite (Intl)",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    category: "cn_official",
    baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("auto", "auto", "auto"),
    // 新加坡地域为默认端点；国际站企业套餐另可选广州地域（1300/81489、
    // 81490 双地域表：tokenhub.tencentcloudmaas.com，需开通广州地域，
    // 不支持跨地域调用，故仅作候选端点）
    endpointCandidates: [
      "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
      "https://tokenhub.tencentcloudmaas.com/plan/anthropic",
    ],
    icon: "tencent",
    iconColor: "#0052D9",
  },
  {
    name: "Zhipu GLM",
    family: "zhipu",
    regionKey: "cn",
    websiteUrl: "https://open.bigmodel.cn",
    apiKeyUrl: "https://www.bigmodel.cn/claude-code",
    category: "cn_official",
    baseUrl: "https://open.bigmodel.cn/api/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("glm-5.3", "glm-5.3", "glm-5.3"),
    icon: "zhipu",
    iconColor: "#0F62FE",
  },
  {
    name: "Zhipu GLM en",
    family: "zhipu",
    regionKey: "intl",
    websiteUrl: "https://z.ai",
    apiKeyUrl: "https://z.ai/subscribe",
    category: "cn_official",
    baseUrl: "https://api.z.ai/api/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("glm-5.3", "glm-5.3", "glm-5.3"),
    icon: "zhipu",
    iconColor: "#0F62FE",
  },
  {
    name: "Baidu Qianfan Coding Plan",
    family: "baidu-qianfan",
    planKey: "codingPlan",
    websiteUrl: "https://cloud.baidu.com/product/qianfan_modelbuilder",
    apiKeyUrl:
      "https://console.bce.baidu.com/qianfan/ais/console/applicationConsole/application",
    category: "cn_official",
    baseUrl: "https://qianfan.baidubce.com/anthropic/coding",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "qianfan-code-latest",
      "qianfan-code-latest",
      "qianfan-code-latest",
    ),
    endpointCandidates: ["https://qianfan.baidubce.com/anthropic/coding"],
    icon: "baidu",
    iconColor: "#2932E1",
  },
  {
    // Token Plan 个人版：2026-07-13 起替代 Coding Plan 发售（存量 Coding
    // Plan 可用至到期，旧预设保留）。模型=官方 Claude Code 接入页
    // （2026-07-30 版）全角色 deepseek-v4-pro
    name: "Baidu Qianfan Token Plan",
    family: "baidu-qianfan",
    planKey: "tokenPlan",
    websiteUrl: "https://cloud.baidu.com/product/codingplan.html",
    apiKeyUrl: "https://console.bce.baidu.com/qianfan/resource/token-plan",
    category: "cn_official",
    baseUrl: "https://qianfan.baidubce.com/anthropic/tokenplan/personal",
    mode: "proxy",
    apiFormat: "anthropic",
    // supports1m：DeepSeek V4 Pro 官方窗口 1M（千帆平台模型列表口径，
    // 本仓 Codex catalog 同记 1048576）。[1m] 只是 Claude Desktop 本地
    // 标记，匹配前会被剥掉，不会随请求发往上游
    modelRoutes: brandedRoutes(
      "deepseek-v4-pro",
      "deepseek-v4-pro",
      "deepseek-v4-pro",
      true,
    ),
    endpointCandidates: [
      "https://qianfan.baidubce.com/anthropic/tokenplan/personal",
    ],
    icon: "baidu",
    iconColor: "#2932E1",
  },
  {
    name: "千问AI平台",
    family: "qianwen",
    planKey: "payg",
    websiteUrl: "https://platform.qianwenai.com/",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    category: "cn_official",
    baseUrl: "https://dashscope.aliyuncs.com/apps/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("qwen3.7-plus", "qwen3.8-max", "qwen3.8-flash"),
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  {
    name: "千问AI平台 Coding Plan",
    family: "qianwen",
    planKey: "codingPlan",
    websiteUrl: "https://bailian.console.aliyun.com",
    category: "cn_official",
    baseUrl: "https://coding.dashscope.aliyuncs.com/apps/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  {
    name: "千问AI平台 Token Plan",
    family: "qianwen",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.qianwenai.com/pricing/token-plan",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    category: "cn_official",
    baseUrl: "https://token-plan.cn-beijing.maas.aliyuncs.com/apps/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("qwen3.7-plus", "qwen3.8-max", "qwen3.8-flash"),
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  // ===== QwenCloud（DashScope 国际站）=====
  // 与上面国内条目是两套独立站点：域名、控制台、密钥互不通用。
  // 官方文档要求把 ANTHROPIC_MODEL 显式设成 qwen 模型名，端点不认
  // claude-* 别名，所以走 brandedRoutes 而不是 passthroughRoutes。
  {
    name: "QwenCloud",
    family: "qwencloud",
    planKey: "payg",
    websiteUrl: "https://home.qwencloud.com/",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    category: "cn_official",
    baseUrl: "https://dashscope-intl.aliyuncs.com/apps/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    // 不挂 [1m]：qwen3.8 系官方窗口是 983616，不足 1M，
    // Desktop 对模型能力校验是精确的，误标会被上游拒绝
    modelRoutes: brandedRoutes("qwen3.7-plus", "qwen3.8-max", "qwen3.8-flash"),
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "QwenCloud For Coding",
    family: "qwencloud",
    planKey: "coding",
    websiteUrl: "https://www.qwencloud.com",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    category: "cn_official",
    baseUrl: "https://coding-intl.dashscope.aliyuncs.com/apps/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    // 挂 [1m]：本条钉的是 qwen3.7-plus，官方窗口 1000000
    modelRoutes: brandedRoutes(
      "qwen3.7-plus",
      "qwen3.7-plus",
      "qwen3.7-plus",
      true,
    ),
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "QwenCloud Token Plan",
    family: "qwencloud",
    planKey: "tokenPlan",
    websiteUrl: "https://www.qwencloud.com/pricing/token-plan",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    category: "cn_official",
    baseUrl:
      "https://token-plan.ap-southeast-1.maas.aliyuncs.com/apps/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    // 不挂 [1m]：qwen3.8 系官方窗口是 983616，不足 1M，
    // Desktop 对模型能力校验是精确的，误标会被上游拒绝
    modelRoutes: brandedRoutes("qwen3.7-plus", "qwen3.8-max", "qwen3.8-flash"),
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "StepFun",
    family: "stepfun",
    regionKey: "cn",
    websiteUrl: "https://platform.stepfun.com/step-plan",
    apiKeyUrl: "https://platform.stepfun.com/interface-key",
    category: "cn_official",
    baseUrl: "https://api.stepfun.com/step_plan",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "step-3.5-flash-2603",
      "step-3.5-flash-2603",
      "step-3.5-flash-2603",
    ),
    endpointCandidates: ["https://api.stepfun.com/step_plan"],
    icon: "stepfun",
    iconColor: "#16D6D2",
  },
  {
    name: "StepFun en",
    family: "stepfun",
    regionKey: "intl",
    websiteUrl: "https://platform.stepfun.ai/step-plan",
    apiKeyUrl: "https://platform.stepfun.ai/interface-key",
    category: "cn_official",
    baseUrl: "https://api.stepfun.ai/step_plan",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "step-3.5-flash-2603",
      "step-3.5-flash-2603",
      "step-3.5-flash-2603",
    ),
    endpointCandidates: ["https://api.stepfun.ai/step_plan"],
    icon: "stepfun",
    iconColor: "#16D6D2",
  },
  {
    name: "ModelScope",
    websiteUrl: "https://modelscope.cn",
    category: "aggregator",
    baseUrl: "https://api-inference.modelscope.cn",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "ZhipuAI/GLM-5.2",
      "ZhipuAI/GLM-5.2",
      "ZhipuAI/GLM-5.2",
    ),
    icon: "modelscope",
    iconColor: "#624AFF",
  },
  {
    name: "Longcat",
    websiteUrl: "https://longcat.chat/platform",
    apiKeyUrl: "https://longcat.chat/platform/api_keys",
    category: "cn_official",
    baseUrl: "https://api.longcat.chat/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("LongCat-2.0", "LongCat-2.0", "LongCat-2.0"),
    icon: "longcat",
    iconColor: "#29E154",
  },
  {
    name: "MiniMax",
    family: "minimax",
    regionKey: "cn",
    websiteUrl: "https://platform.minimax.cn",
    apiKeyUrl: "https://platform.minimax.cn/subscribe/token-plan",
    category: "cn_official",
    baseUrl: "https://api.minimax.cn/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("MiniMax-M3", "MiniMax-M3", "MiniMax-M3", true),
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
    icon: "minimax",
    iconColor: "#FF6B6B",
  },
  {
    name: "MiniMax en",
    family: "minimax",
    regionKey: "intl",
    websiteUrl: "https://platform.minimax.io",
    apiKeyUrl: "https://platform.minimax.io/subscribe/coding-plan",
    category: "cn_official",
    baseUrl: "https://api.minimax.io/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("MiniMax-M3", "MiniMax-M3", "MiniMax-M3", true),
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
    icon: "minimax",
    iconColor: "#FF6B6B",
  },
  {
    name: "BaiLing",
    websiteUrl: "https://developer.ant-ling.com/zh-CN/docs/",
    apiKeyUrl: "https://chat.ant-ling.com/open",
    category: "cn_official",
    baseUrl: "https://api.ant-ling.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("Ling-2.6-1T", "Ling-2.6-1T", "Ling-2.6-1T"),
    icon: "bailing",
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    category: "aggregator",
    baseUrl: "https://open.cherryin.net",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: mappedRoutes(
      "anthropic/claude-sonnet-5",
      "anthropic/claude-opus-5",
      "anthropic/claude-haiku-4.5",
    ),
    endpointCandidates: ["https://open.cherryin.net"],
    icon: "cherryin",
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    category: "aggregator",
    baseUrl: "https://openrouter.ai/api",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: mappedRoutes(
      "anthropic/claude-sonnet-5",
      "anthropic/claude-opus-5",
      "anthropic/claude-haiku-4.5",
      true,
    ),
    icon: "openrouter",
    iconColor: "#6566F1",
  },
  {
    name: "Novita AI",
    websiteUrl: "https://novita.ai",
    apiKeyUrl: "https://novita.ai",
    category: "aggregator",
    baseUrl: "https://api.novita.ai/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "zai-org/glm-5.1",
      "zai-org/glm-5.1",
      "zai-org/glm-5.1",
    ),
    endpointCandidates: ["https://api.novita.ai/anthropic"],
    icon: "novita",
    iconColor: "#000000",
  },
  {
    name: "Nvidia",
    websiteUrl: "https://build.nvidia.com",
    apiKeyUrl: "https://build.nvidia.com/settings/api-keys",
    category: "aggregator",
    baseUrl: "https://integrate.api.nvidia.com",
    mode: "proxy",
    apiFormat: "openai_chat",
    modelRoutes: brandedRoutes(
      "moonshotai/kimi-k3",
      "moonshotai/kimi-k3",
      "moonshotai/kimi-k3",
    ),
    icon: "nvidia",
    iconColor: "#000000",
  },
  {
    name: "Xiaomi MiMo",
    family: "xiaomi-mimo",
    planKey: "payg",
    websiteUrl: "https://platform.xiaomimimo.com",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/api-keys",
    category: "cn_official",
    baseUrl: "https://api.xiaomimimo.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "mimo-v2.6-pro",
      "mimo-v2.6-pro",
      "mimo-v2.6-pro",
    ),
    icon: "xiaomimimo",
    iconColor: "#000000",
  },
  {
    name: "Xiaomi MiMo Token Plan (China)",
    family: "xiaomi-mimo",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.xiaomimimo.com/#/token-plan",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/plan-manage",
    category: "cn_official",
    baseUrl: "https://token-plan-cn.xiaomimimo.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "mimo-v2.6-pro",
      "mimo-v2.6-pro",
      "mimo-v2.6-pro",
    ),
    icon: "xiaomimimo",
    iconColor: "#000000",
  },
];
