/**
 * Grok Build (Grok CLI) 预设供应商配置模板
 *
 * 独立维护，与 codexProviderPresets.ts 无数据联动（Jason 2026-07-21 定）。
 * 初始条目取自当时的 Codex 预设快照，此后两边各自演进：
 * 合作伙伴链接 / 图标 / endpoint 变更需要在本文件单独修改。
 *
 * 收录规则：
 * - 不含官方 / 托管 OAuth 预设：Grok CLI 自带 xAI 订阅登录，官方态走
 *   独立的 "Grok Official" 条目（对应 providers_seed.rs 的 seed，
 *   空 config = 不写自定义模型表）。
 * - 不含国产模型官方直连（cn_official）与纯开源模型托管站
 *   （SiliconFlow / ModelScope / Novita / Nvidia / AtlasCloud）：
 *   这些上游没有 Grok 模型，无法在 Grok CLI 中使用。
 * - OpenCode Go 上游自 2026-08 起已提供 grok-4.5，但暂仍不收录：
 *   订阅制网关是否纳入 Grok 预设属产品决策，收录前需单独评估。
 * - 只收聚合站与第三方中转站，默认模型统一为 grok-4.5；
 *   用 x-ai/ 命名空间的站点（OpenRouter、七牛）用 "x-ai/grok-4.5"，
 *   CherryIN 没有 4.5，用 "x-ai/grok-4.6"（2026-10 公开定价接口）。
 *
 * config 字段沿用 Codex 风格 TOML 作为载体：Grok 表单只从中提取
 * base_url / model 两个字段（extractCodex* 工具），再重建
 * Grok CLI 自己的 config.toml。api_backend 单独由 apiBackend 给出，
 * 按上游对所选 Grok 模型实际开放的接口填写。
 */
import type { ProviderCategory } from "../types";
import {
  GROK_BUILD_DEFAULT_MODEL,
  type GrokBuildApiBackend,
} from "../utils/grokBuildConfig";
import type { PresetFamilyFields } from "./presetFamilies";

export interface GrokBuildProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string; // i18n key for localized display name
  websiteUrl: string;
  apiKeyUrl?: string;
  auth: Record<string, any>;
  config: string; // Codex 风格 TOML 载体（只消费 base_url / model）
  /** 写进 config.toml 的 api_backend；官方条目没有自定义模型表，不填 */
  apiBackend?: GrokBuildApiBackend;
  isOfficial?: boolean;
  partnerPromotionKey?: string;
  category?: ProviderCategory;
  endpointCandidates?: string[];
  icon?: string;
  iconColor?: string;
}

// 官方条目与后端 seed（providers_seed.rs 的 "Grok Official"）对应：
// 空 config = 不写自定义模型表，Grok CLI 回落到自带的 xAI OAuth 登录。
// 预设 id 复用固定 provider id，AddProviderDialog 据此走 ensure seed 流程。
export const grokBuildOfficialPreset: GrokBuildProviderPreset = {
  name: "Grok Official",
  websiteUrl: "https://x.ai/grok",
  isOfficial: true,
  category: "official",
  auth: {},
  config: "",
  icon: "grok",
  iconColor: "currentColor",
};

/** x-ai/ 命名空间站点的 Grok 模型 id */
const OPENROUTER_STYLE_GROK_MODEL = "x-ai/grok-4.5";

const grokAuth = (): Record<string, any> => ({ OPENAI_API_KEY: "" });

function grokPresetConfig(
  providerName: string,
  baseUrl: string,
  model = GROK_BUILD_DEFAULT_MODEL,
): string {
  const tomlString = (value: string) => JSON.stringify(value);

  return `model_provider = "custom"
model = ${tomlString(model)}

[model_providers.custom]
name = ${tomlString(providerName)}
base_url = ${tomlString(baseUrl)}
requires_openai_auth = true`;
}

export const grokBuildProviderPresets: GrokBuildProviderPreset[] = [
  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    auth: grokAuth(),
    // bypass/openai 是 GPT 的 Responses 直通，没有 Grok。Grok 走通用网关的
    // /v1/chat/completions；海外入口 modelink.ai 的模型列表有 x-ai/grok-4.5，
    // 国内 api.qnaigc.com 是否提供 Grok 未证实。
    apiBackend: "chat_completions",
    config: grokPresetConfig(
      "Qiniu",
      "https://api.modelink.ai/v1",
      OPENROUTER_STYLE_GROK_MODEL,
    ),
    endpointCandidates: [
      "https://api.modelink.ai/v1",
      "https://api.qnaigc.com/v1",
    ],
    category: "aggregator",
    icon: "qiniu",
  },
  {
    name: "Compshare",
    family: "compshare",
    planKey: "payg",
    nameKey: "providerForm.presets.ucloud",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl: "https://www.compshare.cn/coding-plan",
    auth: grokAuth(),
    apiBackend: "responses",
    config: grokPresetConfig("Compshare", "https://api.modelverse.cn/v1"),
    endpointCandidates: ["https://api.modelverse.cn/v1"],
    category: "aggregator",
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
    auth: grokAuth(),
    apiBackend: "responses",
    config: grokPresetConfig(
      "Compshare Coding Plan",
      "https://cp.compshare.cn/v1",
    ),
    endpointCandidates: ["https://cp.compshare.cn/v1"],
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "xAI (Grok)",
    websiteUrl: "https://x.ai/api",
    apiKeyUrl: "https://console.x.ai",
    auth: grokAuth(),
    apiBackend: "responses",
    config: grokPresetConfig("xAI (Grok)", "https://api.x.ai/v1"),
    endpointCandidates: ["https://api.x.ai/v1"],
    category: "third_party",
    icon: "xai",
    iconColor: "#000000",
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    auth: grokAuth(),
    apiBackend: "responses",
    // 定价接口里 x-ai/grok-4.6 支持 openai 与 openai-response
    config: grokPresetConfig(
      "CherryIN",
      "https://open.cherryin.net/v1",
      "x-ai/grok-4.6",
    ),
    endpointCandidates: ["https://open.cherryin.net/v1"],
    category: "aggregator",
    icon: "cherryin",
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    auth: grokAuth(),
    apiBackend: "responses",
    config: grokPresetConfig(
      "OpenRouter",
      "https://openrouter.ai/api/v1",
      OPENROUTER_STYLE_GROK_MODEL,
    ),
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6566F1",
  },
];
