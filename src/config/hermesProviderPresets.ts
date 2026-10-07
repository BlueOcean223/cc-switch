/**
 * Hermes Agent provider presets configuration
 * Hermes uses custom_providers array in config.yaml
 */
import type { ProviderCategory } from "../types";
import type { PresetTheme, TemplateValueConfig } from "./claudeProviderPresets";
import type { PresetFamilyFields } from "./presetFamilies";

/**
 * Marker field and source values that `hermes_config.rs::get_providers`
 * injects onto each settings payload. Kept in sync with the Rust constants
 * `PROVIDER_SOURCE_FIELD` / `PROVIDER_SOURCE_CUSTOM_LIST` / `PROVIDER_SOURCE_DICT`.
 */
export const HERMES_PROVIDER_SOURCE_FIELD = "_cc_source";
export const HERMES_PROVIDER_SOURCE_DICT = "providers_dict";

/**
 * True when the provider was sourced from Hermes' v12+ `providers:` dict —
 * CC Switch renders those read-only and routes edits to Hermes Web UI.
 */
export function isHermesReadOnlyProvider(settingsConfig: unknown): boolean {
  if (!settingsConfig || typeof settingsConfig !== "object") {
    return false;
  }
  const marker = (settingsConfig as Record<string, unknown>)[
    HERMES_PROVIDER_SOURCE_FIELD
  ];
  return marker === HERMES_PROVIDER_SOURCE_DICT;
}

/**
 * A model entry under a Hermes custom_provider.
 *
 * Serialized to YAML as a dict keyed by `id`:
 *
 * ```yaml
 * models:
 *   anthropic/claude-opus-5:
 *     context_length: 200000
 * ```
 *
 * Hermes' `_VALID_CUSTOM_PROVIDER_FIELDS` (hermes_cli/config.py) does not include
 * `max_tokens` at the per-model level — writing it produces an "unknown field"
 * warning on Hermes startup. Max tokens is a per-request parameter, not a
 * provider-level config.
 */
export interface HermesModel {
  /** Model ID — becomes the YAML key and the value written to top-level model.default. */
  id: string;
  /** Optional display label (UI only, not serialized to YAML). */
  name?: string;
  /** Override the auto-detected context window. */
  context_length?: number;
}

/**
 * Top-level `model:` defaults suggested by a preset.
 *
 * Written to the YAML `model:` section when the user switches to this provider.
 * Per-model `context_length` lives on the individual `HermesModel` entries and
 * flows through `custom_providers[].models`, not this object.
 */
export interface HermesSuggestedDefaults {
  model: {
    /** Model ID for `model.default`. Typically equals `models[0].id`. */
    default: string;
    /** Value for `model.provider`. Omit to use the custom_provider name. */
    provider?: string;
  };
}

/** Hermes custom_provider protocol mode. Always written explicitly. */
export type HermesApiMode =
  | "chat_completions"
  | "anthropic_messages"
  | "codex_responses"
  | "bedrock_converse";

/** Default mode used when a provider has no stored value yet. */
export const HERMES_DEFAULT_API_MODE: HermesApiMode = "chat_completions";

/** Dropdown options for the API Mode selector. `labelKey` is looked up in i18n. */
export const hermesApiModes: Array<{
  value: HermesApiMode;
  labelKey: string;
}> = [
  { value: "chat_completions", labelKey: "hermes.form.apiModeChatCompletions" },
  {
    value: "anthropic_messages",
    labelKey: "hermes.form.apiModeAnthropicMessages",
  },
  { value: "codex_responses", labelKey: "hermes.form.apiModeCodexResponses" },
  {
    value: "bedrock_converse",
    labelKey: "hermes.form.apiModeBedrockConverse",
  },
];

export interface HermesProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string;
  websiteUrl: string;
  apiKeyUrl?: string;
  settingsConfig: HermesProviderSettingsConfig;
  isOfficial?: boolean;
  partnerPromotionKey?: string;
  category?: ProviderCategory;
  templateValues?: Record<string, TemplateValueConfig>;
  theme?: PresetTheme;
  icon?: string;
  iconColor?: string;
  isCustomTemplate?: boolean;
  /** Optional top-level `model:` defaults written on switch. */
  suggestedDefaults?: HermesSuggestedDefaults;
}

export interface HermesProviderSettingsConfig {
  name: string;
  base_url?: string;
  api_key?: string;
  api_mode?: HermesApiMode;
  /** UI-side ordered list; serialized to YAML as a dict keyed by id. */
  models?: HermesModel[];
  /** Delay in seconds between consecutive requests to this provider. */
  rate_limit_delay?: number;
  [key: string]: unknown;
}

export const hermesProviderPresets: HermesProviderPreset[] = [
  {
    name: "Kimi",
    family: "kimi",
    planKey: "payg",
    regionKey: "cn",
    websiteUrl: "https://platform.kimi.com",
    settingsConfig: {
      name: "kimi",
      base_url: "https://api.moonshot.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "kimi-k2.7-code", name: "Kimi K2.7 Code" },
        { id: "kimi-k3", name: "Kimi K3", context_length: 1048576 },
        {
          id: "kimi-k2.7-code-highspeed",
          name: "Kimi K2.7 Code HighSpeed",
          context_length: 262144,
        },
        { id: "kimi-k2.6", name: "Kimi K2.6", context_length: 262144 },
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    suggestedDefaults: {
      model: { default: "kimi-k2.7-code", provider: "kimi" },
    },
  },
  // API 开放平台海外/Global 变体：platform.kimi.ai + api.moonshot.ai 端点
  {
    name: "Kimi Global",
    family: "kimi",
    planKey: "payg",
    regionKey: "intl",
    websiteUrl: "https://platform.kimi.ai",
    settingsConfig: {
      name: "kimi",
      base_url: "https://api.moonshot.ai/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "kimi-k2.7-code", name: "Kimi K2.7 Code" },
        { id: "kimi-k3", name: "Kimi K3", context_length: 1048576 },
        {
          id: "kimi-k2.7-code-highspeed",
          name: "Kimi K2.7 Code HighSpeed",
          context_length: 262144,
        },
        { id: "kimi-k2.6", name: "Kimi K2.6", context_length: 262144 },
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    suggestedDefaults: {
      model: { default: "kimi-k2.7-code", provider: "kimi" },
    },
  },
  {
    name: "Kimi For Coding",
    family: "kimi",
    planKey: "coding",
    regionKey: "cn",
    websiteUrl: "https://www.kimi.com/code/",
    settingsConfig: {
      name: "kimi_coding",
      base_url: "https://api.kimi.com/coding/",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [{ id: "kimi-for-coding", name: "Kimi For Coding" }],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    suggestedDefaults: {
      model: { default: "kimi-for-coding", provider: "kimi_coding" },
    },
  },
  // 海外/Global 变体：kimi.ai/code + api.kimi.ai 端点，其余与国内版一致
  {
    name: "Kimi For Coding Global",
    family: "kimi",
    planKey: "coding",
    regionKey: "intl",
    websiteUrl: "https://www.kimi.ai/code",
    settingsConfig: {
      name: "kimi_coding",
      base_url: "https://api.kimi.ai/coding/",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [{ id: "kimi-for-coding", name: "Kimi For Coding" }],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    suggestedDefaults: {
      model: { default: "kimi-for-coding", provider: "kimi_coding" },
    },
  },
  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    settingsConfig: {
      name: "qiniu",
      base_url: "https://api.qnaigc.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "openai/gpt-6-astra",
          name: "GPT-6 Astra",
          context_length: 1050000,
        },
        { id: "moonshotai/kimi-k3", name: "Kimi K3", context_length: 1048576 },
        { id: "z-ai/glm-5.3", name: "GLM-5.3", context_length: 1048576 },
        {
          id: "z-ai/glm-5.3-flash",
          name: "GLM-5.3-Flash",
          context_length: 1048576,
        },
      ],
    },
    category: "aggregator",
    icon: "qiniu",
    suggestedDefaults: {
      model: { default: "openai/gpt-6-astra", provider: "qiniu" },
    },
  },
  {
    name: "PPIO",
    websiteUrl: "https://ppio.com",
    apiKeyUrl: "https://ppio.com/settings/key-management",
    settingsConfig: {
      name: "ppio",
      base_url: "https://api.ppio.com/openai/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "deepseek/deepseek-v4-flash-0731",
          name: "Deepseek V4 Flash 0731",
          context_length: 1048576,
        },
      ],
    },
    category: "aggregator",
    icon: "ppio",
    iconColor: "#2874FF",
    suggestedDefaults: {
      model: {
        default: "deepseek/deepseek-v4-flash-0731",
        provider: "ppio",
      },
    },
  },
  {
    name: "火山 Agent Plan",
    family: "volcengine",
    planKey: "agentPlan",
    websiteUrl: "https://www.volcengine.com/activity/agentplan",
    apiKeyUrl: "https://www.volcengine.com/activity/agentplan",
    settingsConfig: {
      name: "ark_agentplan",
      base_url: "https://ark.cn-beijing.volces.com/api/plan",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
        },
      ],
    },
    category: "cn_official",
    icon: "huoshan",
    iconColor: "#3370FF",
    suggestedDefaults: {
      model: {
        default: "ark-code-latest",
        provider: "ark_agentplan",
      },
    },
  },
  {
    name: "火山 Coding Plan",
    family: "volcengine",
    planKey: "codingPlan",
    websiteUrl: "https://www.volcengine.com/activity/codingplan",
    apiKeyUrl: "https://www.volcengine.com/activity/codingplan",
    settingsConfig: {
      name: "ark_codingplan",
      base_url: "https://ark.cn-beijing.volces.com/api/coding",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
        },
      ],
    },
    category: "cn_official",
    icon: "huoshan",
    iconColor: "#3370FF",
    suggestedDefaults: {
      model: {
        default: "ark-code-latest",
        provider: "ark_codingplan",
      },
    },
  },
  {
    name: "BytePlus",
    websiteUrl: "https://www.byteplus.com/en/product/modelark",
    apiKeyUrl: "https://www.byteplus.com/en/product/modelark",
    settingsConfig: {
      name: "byteplus",
      base_url: "https://ark.ap-southeast.bytepluses.com/api/coding",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
        },
      ],
    },
    category: "cn_official",
    icon: "byteplus",
    iconColor: "#3370FF",
    suggestedDefaults: {
      model: {
        default: "ark-code-latest",
        provider: "byteplus",
      },
    },
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
    settingsConfig: {
      name: "doubao_seed",
      base_url: "https://ark.cn-beijing.volces.com/api/compatible",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        {
          id: "doubao-seed-2-1-pro-260915",
          name: "Doubao Seed 2.1 Pro",
        },
      ],
    },
    category: "cn_official",
    icon: "doubao",
    iconColor: "#3370FF",
    suggestedDefaults: {
      model: {
        default: "doubao-seed-2-1-pro-260915",
        provider: "doubao_seed",
      },
    },
  },
  {
    name: "SiliconFlow",
    family: "siliconflow",
    regionKey: "cn",
    websiteUrl: "https://siliconflow.cn",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    settingsConfig: {
      name: "siliconflow",
      base_url: "https://api.siliconflow.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        // MiniMax-M2.5 已于 2026-09-11 下线，国内站没有别的 MiniMax 模型
        {
          id: "deepseek-ai/DeepSeek-V4-Flash",
          name: "DeepSeek V4 Flash",
        },
      ],
    },
    category: "aggregator",
    icon: "siliconflow",
    iconColor: "#6E29F6",
    suggestedDefaults: {
      model: {
        default: "deepseek-ai/DeepSeek-V4-Flash",
        provider: "siliconflow",
      },
    },
  },
  {
    name: "SiliconFlow en",
    family: "siliconflow",
    regionKey: "intl",
    websiteUrl: "https://siliconflow.com",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    settingsConfig: {
      name: "siliconflow_en",
      base_url: "https://api.siliconflow.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "MiniMaxAI/MiniMax-M3", name: "MiniMax M3" }],
    },
    category: "aggregator",
    icon: "siliconflow",
    iconColor: "#000000",
    suggestedDefaults: {
      model: {
        default: "MiniMaxAI/MiniMax-M3",
        provider: "siliconflow_en",
      },
    },
  },
  {
    name: "Compshare",
    family: "compshare",
    planKey: "payg",
    nameKey: "providerForm.presets.ucloud",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl: "https://www.compshare.cn/coding-plan",
    settingsConfig: {
      name: "compshare",
      base_url: "https://api.modelverse.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "gpt-6-astra", name: "GPT-6 Astra", context_length: 1050000 },
        { id: "kimi-k3", name: "Kimi K3", context_length: 1048576 },
      ],
    },
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
    suggestedDefaults: {
      model: { default: "gpt-6-astra", provider: "compshare" },
    },
  },
  {
    name: "Compshare Coding Plan",
    family: "compshare",
    planKey: "codingPlan",
    nameKey: "providerForm.presets.ucloudCoding",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl: "https://www.compshare.cn/coding-plan",
    settingsConfig: {
      name: "compshare_coding",
      base_url: "https://cp.compshare.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      // 套餐只含国产模型，没有 GPT；按官方接入页用 deepseek-v4-pro
      // https://www.compshare.cn/docs/modelverse/package_plan/usecases (2026-09-11)
      models: [{ id: "deepseek-v4-pro", name: "DeepSeek V4 Pro" }],
    },
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
    suggestedDefaults: {
      model: { default: "deepseek-v4-pro", provider: "compshare_coding" },
    },
  },
  {
    name: "AtlasCloud",
    websiteUrl: "https://www.atlascloud.ai/console/coding-plan",
    apiKeyUrl: "https://www.atlascloud.ai/console/coding-plan",
    settingsConfig: {
      name: "atlascloud",
      base_url: "https://api.atlascloud.ai/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "zai-org/glm-5.2",
          name: "GLM 5.2",
        },
      ],
    },
    category: "aggregator",
    icon: "atlascloud",
    suggestedDefaults: {
      model: { default: "zai-org/glm-5.2", provider: "atlascloud" },
    },
  },
  {
    name: "OpenRouter",
    nameKey: "providerForm.presets.openrouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    settingsConfig: {
      name: "openrouter",
      base_url: "https://openrouter.ai/api/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "anthropic/claude-opus-5.5",
          name: "Claude Opus 5.5",
          context_length: 1000000,
        },
        {
          id: "anthropic/claude-sonnet-5.5",
          name: "Claude Sonnet 5.5",
          context_length: 1000000,
        },
        {
          id: "anthropic/claude-haiku-4.5",
          name: "Claude Haiku 4.5",
          context_length: 200000,
        },
        {
          id: "openai/gpt-5.6-sol",
          name: "GPT-5.6 Sol",
          context_length: 400000,
        },
        {
          id: "anthropic/claude-fable-5.1",
          name: "Claude Fable 5.1",
          context_length: 1000000,
        },
        {
          id: "openai/gpt-6-astra",
          name: "GPT-6 Astra",
          context_length: 1050000,
        },
        {
          id: "google/gemini-3.8-flash",
          name: "Gemini 3.8 Flash",
          context_length: 1048576,
        },
      ],
    },
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6366F1",
    suggestedDefaults: {
      model: { default: "anthropic/claude-opus-5.5", provider: "openrouter" },
    },
  },
  {
    name: "DeepSeek",
    nameKey: "providerForm.presets.deepseek",
    websiteUrl: "https://platform.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    settingsConfig: {
      name: "deepseek",
      base_url: "https://api.deepseek.com",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "deepseek-v4-pro",
          name: "DeepSeek V4 Pro",
          context_length: 1000000,
        },
        {
          id: "deepseek-flash",
          name: "DeepSeek V4.1 Flash",
          context_length: 1000000,
        },
      ],
    },
    category: "cn_official",
    icon: "deepseek",
    iconColor: "#4D6BFE",
    suggestedDefaults: {
      model: { default: "deepseek-flash", provider: "deepseek" },
    },
  },
  {
    name: "Together AI",
    nameKey: "providerForm.presets.together",
    websiteUrl: "https://together.ai",
    apiKeyUrl: "https://api.together.ai/settings/api-keys",
    settingsConfig: {
      name: "together",
      base_url: "https://api.together.xyz/v1",
      api_key: "",
      api_mode: "chat_completions",
      // 原来的 Qwen3 Coder 480B、DeepSeek V3.2、Llama 4 Maverick 都已不在 Serverless
      // 列表里；下面三个都支持函数调用
      // https://docs.together.ai/docs/serverless-models (2026-10)
      models: [
        {
          id: "zai-org/GLM-5.3",
          name: "GLM-5.3",
          context_length: 1048575,
        },
        {
          id: "moonshotai/Kimi-K3",
          name: "Kimi K3",
          context_length: 1048576,
        },
        {
          id: "deepseek-ai/DeepSeek-V4.1-Flash",
          name: "DeepSeek V4.1 Flash",
          context_length: 1000000,
        },
      ],
    },
    category: "aggregator",
    icon: "together",
    iconColor: "#0F6FFF",
    suggestedDefaults: {
      model: {
        default: "zai-org/GLM-5.3",
        provider: "together",
      },
    },
  },
  {
    name: "Nous Research",
    websiteUrl: "https://nousresearch.com",
    apiKeyUrl: "https://portal.nousresearch.com/",
    settingsConfig: {
      name: "nous",
      base_url: "https://inference-api.nousresearch.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      // Hermes 4 已退役（2026-10-07 请求返回 404 "This model has been retired"），
      // Portal 现在用 OpenRouter 风格的 ID
      models: [
        { id: "z-ai/glm-5.3", name: "GLM-5.3" },
        { id: "deepseek/deepseek-v4-pro", name: "DeepSeek V4 Pro" },
      ],
    },
    isOfficial: true,
    category: "official",
    icon: "hermes",
    iconColor: "#7C3AED",
    suggestedDefaults: {
      model: { default: "z-ai/glm-5.3", provider: "nous" },
    },
  },

  // 字段映射：env.ANTHROPIC_BASE_URL → base_url；env.ANTHROPIC_AUTH_TOKEN → api_key；
  // apiFormat "anthropic"(默认) → api_mode "anthropic_messages"；
  // apiFormat "openai_chat" → api_mode "chat_completions"；
  // ANTHROPIC_MODEL / DEFAULT_HAIKU / SONNET / OPUS_MODEL 去重后塞进 models[]。
  {
    name: "Zhipu GLM",
    family: "zhipu",
    regionKey: "cn",
    websiteUrl: "https://open.bigmodel.cn",
    apiKeyUrl: "https://www.bigmodel.cn/claude-code",
    settingsConfig: {
      name: "zhipu_glm",
      base_url: "https://open.bigmodel.cn/api/coding/paas/v4",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "glm-5.3", name: "GLM-5.3" },
        { id: "glm-5.3-flash", name: "GLM-5.3-Flash", context_length: 1048576 },
      ],
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
    suggestedDefaults: {
      model: { default: "glm-5.3", provider: "zhipu_glm" },
    },
  },
  {
    name: "Zhipu GLM en",
    family: "zhipu",
    regionKey: "intl",
    websiteUrl: "https://z.ai",
    apiKeyUrl: "https://z.ai/subscribe",
    settingsConfig: {
      name: "zhipu_glm_en",
      base_url: "https://api.z.ai/api/coding/paas/v4",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "glm-5.3", name: "GLM-5.3" },
        { id: "glm-5.3-flash", name: "GLM-5.3-Flash", context_length: 1048576 },
      ],
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
    suggestedDefaults: {
      model: { default: "glm-5.3", provider: "zhipu_glm_en" },
    },
  },
  {
    // 腾讯云 Token Plan 个人版（1823/130060，2026-08-21 版）：通用 + Hy 两
    // 系列共用同一端点与 API Key，模型合并两系列；Auto 智能路由调用 ID 是
    // tc-code-latest。官方未发 Hermes 专属接入页，按工具无关的 /plan/v3
    // OpenAI Chat 端点收录（与市场线 1300/80643 的 chat_completions 模式
    // 一致）。kimi-k2.5 官方 2026-08-31 下线不收；minimax-m2.5 真 Key
    // 实测可用（2026-08-31）照实收
    name: "Tencent Token Plan",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan",
    settingsConfig: {
      name: "tencent_token_plan",
      base_url: "https://api.lkeap.cloud.tencent.com/plan/v3",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "tc-code-latest", name: "Auto" },
        { id: "deepseek-v4-flash-202605", name: "DeepSeek V4 Flash" },
        { id: "deepseek-v4-pro-202606", name: "DeepSeek V4 Pro" },
        { id: "minimax-m2.7", name: "MiniMax M2.7" },
        { id: "glm-5.2", name: "GLM-5.2" },
        { id: "hy3", name: "Hy3" },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    suggestedDefaults: {
      model: { default: "tc-code-latest", provider: "tencent_token_plan" },
    },
  },
  {
    // 国际站（新加坡地域）个人版（intl 1300/81315，2026-08-20 版）：Auto
    // 调用 ID 是 auto（≠国内 tc-code-latest），阵容不同（无 GLM-5/5.1/
    // Hy3）。端点用国际站文档钦定的 tencentcloudmaas.com 域；Key 按站独立
    name: "Tencent Token Plan (Intl)",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan",
    settingsConfig: {
      name: "tencent_token_plan_intl",
      base_url: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "auto", name: "Auto" },
        { id: "glm-5.2", name: "GLM-5.2" },
        { id: "kimi-k2.6", name: "Kimi K2.6" },
        { id: "deepseek-v4-pro-202606", name: "DeepSeek V4 Pro" },
        { id: "deepseek-v4-flash-202605", name: "DeepSeek V4 Flash" },
        { id: "minimax-m3", name: "MiniMax M3" },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    suggestedDefaults: {
      model: { default: "auto", provider: "tencent_token_plan_intl" },
    },
  },
  {
    // Token Plan 企业版专业套餐（1823/130659，2026-08-25 版，广州地域）：
    // kimi-k2.5 官方 2026-08-31 下线不收；minimax-m2.5 文档已除名但真 Key
    // 实测可用（2026-08-31），照实收录
    name: "Tencent Token Plan Enterprise Pro",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      name: "tencent_token_plan_enterprise_pro",
      base_url: "https://tokenhub.tencentmaas.com/plan/v3",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "auto", name: "Auto" },
        { id: "glm-5.3", name: "GLM-5.3" },
        { id: "glm-5.2", name: "GLM-5.2" },
        { id: "glm-5", name: "GLM-5" },
        { id: "glm-5.1", name: "GLM-5.1" },
        { id: "glm-5-turbo", name: "GLM-5 Turbo" },
        { id: "kimi-k2.7-code", name: "Kimi K2.7 Code" },
        { id: "kimi-k2.7-code-highspeed", name: "Kimi K2.7 Code HighSpeed" },
        { id: "kimi-k2.6", name: "Kimi K2.6" },
        { id: "minimax-m2.7", name: "MiniMax M2.7" },
        { id: "minimax-m3", name: "MiniMax M3" },
        { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash" },
        { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro" },
        { id: "deepseek-v4-flash-0731", name: "DeepSeek V4 Flash 0731 GA" },
        { id: "deepseek-v4-pro-0813", name: "DeepSeek V4 Pro 0813 GA" },
        { id: "deepseek-v4-flash-202605", name: "DeepSeek V4 Flash Official" },
        { id: "deepseek-v4-pro-202606", name: "DeepSeek V4 Pro Official" },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    suggestedDefaults: {
      model: {
        default: "auto",
        provider: "tencent_token_plan_enterprise_pro",
      },
    },
  },
  {
    // 国际站企业版专业套餐（intl 1300/81489，2026-08-26 版，新加坡地域）：
    // 阵容为广州地域子集（无 GLM-5/5.1/5-Turbo、Kimi-K2.6、MiniMax-M2.7）
    name: "Tencent Token Plan Enterprise Pro (Intl)",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    settingsConfig: {
      name: "tencent_token_plan_enterprise_pro_intl",
      base_url: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "auto", name: "Auto" },
        { id: "glm-5.3", name: "GLM-5.3" },
        { id: "glm-5.2", name: "GLM-5.2" },
        { id: "minimax-m3", name: "MiniMax M3" },
        { id: "kimi-k2.7-code", name: "Kimi K2.7 Code" },
        { id: "kimi-k2.7-code-highspeed", name: "Kimi K2.7 Code HighSpeed" },
        { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash" },
        { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro" },
        { id: "deepseek-v4-flash-0731", name: "DeepSeek V4 Flash 0731 GA" },
        { id: "deepseek-v4-pro-0813", name: "DeepSeek V4 Pro 0813 GA" },
        { id: "deepseek-v4-flash-202605", name: "DeepSeek V4 Flash Official" },
        { id: "deepseek-v4-pro-202606", name: "DeepSeek V4 Pro Official" },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    suggestedDefaults: {
      model: {
        default: "auto",
        provider: "tencent_token_plan_enterprise_pro_intl",
      },
    },
  },
  {
    // Token Plan 企业版轻享套餐（1823/131173，2026-08-28 版）：仅 Auto 模型
    name: "Tencent Token Plan Enterprise Lite",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      name: "tencent_token_plan_enterprise_lite",
      base_url: "https://tokenhub.tencentmaas.com/plan/v3",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "auto", name: "Auto" }],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    suggestedDefaults: {
      model: {
        default: "auto",
        provider: "tencent_token_plan_enterprise_lite",
      },
    },
  },
  {
    // 国际站企业版轻享套餐（intl 1300/81490）：新加坡地域（资源调度范围
    // Global），仅 Auto 模型
    name: "Tencent Token Plan Enterprise Lite (Intl)",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    settingsConfig: {
      name: "tencent_token_plan_enterprise_lite_intl",
      base_url: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "auto", name: "Auto" }],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    suggestedDefaults: {
      model: {
        default: "auto",
        provider: "tencent_token_plan_enterprise_lite_intl",
      },
    },
  },
  {
    // 千帆 Token Plan 个人版（2026-07-13 起替代 Coding Plan 发售）：官方
    // Hermes 接入页确认 /v2/tokenplan/personal、默认 deepseek-v4-pro（其
    // api_mode 写 "openai_messages"，本仓 OpenAI Chat 端点惯例统一映射为
    // chat_completions）；阵容=Token Plan 个人版文档 2026-09-30 版
    // （cloud.baidu.com/doc/qianfan/s/Dmrabu8b6）
    name: "Baidu Qianfan Token Plan",
    websiteUrl: "https://cloud.baidu.com/product/codingplan.html",
    apiKeyUrl: "https://console.bce.baidu.com/qianfan/resource/token-plan",
    settingsConfig: {
      name: "qianfan_tokenplan",
      base_url: "https://qianfan.baidubce.com/v2/tokenplan/personal",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro" },
        { id: "deepseek-v4.1-flash", name: "DeepSeek V4.1 Flash" },
        { id: "deepseek-v4-pro-0813", name: "DeepSeek V4 Pro 0813" },
        { id: "deepseek-v4-flash-0731", name: "DeepSeek V4 Flash 0731" },
        { id: "glm-5.3", name: "GLM-5.3" },
        { id: "glm-5.3-flash", name: "GLM-5.3 Flash" },
        { id: "glm-5.2", name: "GLM-5.2" },
        { id: "glm-5.1", name: "GLM-5.1" },
      ],
    },
    category: "cn_official",
    icon: "baidu",
    iconColor: "#2932E1",
    suggestedDefaults: {
      model: { default: "deepseek-v4-pro", provider: "qianfan_tokenplan" },
    },
  },
  {
    name: "千问AI平台",
    family: "qianwen",
    planKey: "payg",
    websiteUrl: "https://platform.qianwenai.com/",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    settingsConfig: {
      name: "qianwenai",
      base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "qwen3.8-max", name: "Qwen3.8 Max", context_length: 983616 },
        { id: "qwen3.8-flash", name: "Qwen3.8 Flash", context_length: 983616 },
      ],
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
    suggestedDefaults: {
      model: { default: "qwen3.8-max", provider: "qianwenai" },
    },
  },
  {
    name: "千问AI平台 Coding Plan",
    family: "qianwen",
    planKey: "codingPlan",
    websiteUrl: "https://bailian.console.aliyun.com",
    settingsConfig: {
      name: "qianwenai_coding_plan",
      base_url: "https://coding.dashscope.aliyuncs.com/apps/anthropic",
      api_key: "",
      api_mode: "anthropic_messages",
      // qwen3-coder-plus、qwen3-max 2026-10-10 下线；Coding Plan 只认精确版本
      // https://help.aliyun.com/zh/model-studio/coding-plan (2026-09-11)
      models: [
        { id: "qwen3.7-plus", name: "Qwen3.7 Plus", context_length: 1000000 },
        { id: "qwen3.6-plus", name: "Qwen3.6 Plus", context_length: 1000000 },
      ],
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
    suggestedDefaults: {
      model: {
        default: "qwen3.7-plus",
        provider: "qianwenai_coding_plan",
      },
    },
  },
  {
    name: "千问AI平台 Token Plan",
    family: "qianwen",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.qianwenai.com/pricing/token-plan",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    settingsConfig: {
      name: "qianwenai_token_plan",
      base_url:
        "https://token-plan.cn-beijing.maas.aliyuncs.com/apps/anthropic",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "qwen3.8-max", name: "Qwen3.8 Max", context_length: 983616 },
        { id: "qwen3.8-flash", name: "Qwen3.8 Flash", context_length: 983616 },
      ],
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
    suggestedDefaults: {
      model: { default: "qwen3.8-max", provider: "qianwenai_token_plan" },
    },
  },
  // ===== QwenCloud（DashScope 国际站）=====
  // 与上面国内条目是两套独立站点：域名、控制台、密钥互不通用。
  // QwenCloud 与 Token Plan 都用 anthropic_messages，地址不带 /v1（与官方
  // hermes 文档一致，这点和 OpenCode / OpenClaw 不同，勿互相照搬）。
  {
    name: "QwenCloud",
    family: "qwencloud",
    planKey: "payg",
    websiteUrl: "https://home.qwencloud.com/",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      name: "qwencloud",
      base_url: "https://dashscope-intl.aliyuncs.com/apps/anthropic",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "qwen3.8-max", name: "Qwen3.8 Max", context_length: 983616 },
        { id: "qwen3.8-flash", name: "Qwen3.8 Flash", context_length: 983616 },
        { id: "qwen3.7-max", name: "Qwen3.7 Max", context_length: 1000000 },
      ],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    suggestedDefaults: {
      model: { default: "qwen3.8-max", provider: "qwencloud" },
    },
  },
  {
    name: "QwenCloud For Coding",
    family: "qwencloud",
    planKey: "coding",
    websiteUrl: "https://www.qwencloud.com",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      name: "qwencloud_coding",
      base_url: "https://coding-intl.dashscope.aliyuncs.com/apps/anthropic",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "qwen3.7-plus", name: "Qwen3.7 Plus", context_length: 1000000 },
        { id: "qwen3.6-plus", name: "Qwen3.6 Plus", context_length: 1000000 },
      ],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    suggestedDefaults: {
      model: { default: "qwen3.7-plus", provider: "qwencloud_coding" },
    },
  },
  {
    name: "QwenCloud Token Plan",
    family: "qwencloud",
    planKey: "tokenPlan",
    websiteUrl: "https://www.qwencloud.com/pricing/token-plan",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      name: "qwencloud_token_plan",
      base_url:
        "https://token-plan.ap-southeast-1.maas.aliyuncs.com/apps/anthropic",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "qwen3.8-max", name: "Qwen3.8 Max", context_length: 983616 },
        { id: "qwen3.8-flash", name: "Qwen3.8 Flash", context_length: 983616 },
        { id: "qwen3.7-max", name: "Qwen3.7 Max", context_length: 1000000 },
        { id: "qwen3.7-plus", name: "Qwen3.7 Plus", context_length: 1000000 },
        { id: "qwen3.6-plus", name: "Qwen3.6 Plus", context_length: 1000000 },
        { id: "qwen3.6-flash", name: "Qwen3.6 Flash", context_length: 1000000 },
      ],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    suggestedDefaults: {
      model: { default: "qwen3.8-max", provider: "qwencloud_token_plan" },
    },
  },
  {
    name: "StepFun",
    websiteUrl: "https://platform.stepfun.ai",
    apiKeyUrl: "https://platform.stepfun.ai/interface-key",
    settingsConfig: {
      name: "stepfun",
      base_url: "https://api.stepfun.ai/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "step-3.5-flash", name: "Step 3.5 Flash" },
        {
          id: "step-3.7-flash",
          name: "Step 3.7 Flash",
          context_length: 256000,
        },
        {
          id: "step-5-preview",
          name: "Step 5 Preview",
          context_length: 1000000,
        },
      ],
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#005AFF",
    suggestedDefaults: {
      model: { default: "step-5-preview", provider: "stepfun" },
    },
  },
  {
    name: "ModelScope",
    websiteUrl: "https://modelscope.cn",
    settingsConfig: {
      name: "modelscope",
      base_url: "https://api-inference.modelscope.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "ZhipuAI/GLM-5.2", name: "ZhipuAI / GLM-5.2" }],
    },
    category: "aggregator",
    icon: "modelscope",
    iconColor: "#624AFF",
    suggestedDefaults: {
      model: { default: "ZhipuAI/GLM-5.2", provider: "modelscope" },
    },
  },
  {
    name: "KAT-Coder",
    websiteUrl: "https://console.streamlake.ai",
    apiKeyUrl: "https://console.streamlake.ai/console/api-key",
    settingsConfig: {
      name: "kat_coder",
      // 官方接入指南：路径里的 Endpoint ID 换成模型 ID（2026-07-13）
      base_url:
        "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/kat-coder-pro-v2.5/claude-code-proxy",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [{ id: "kat-coder-pro-v2.5", name: "KAT-Coder Pro V2.5" }],
    },
    category: "cn_official",
    icon: "catcoder",
    suggestedDefaults: {
      model: { default: "kat-coder-pro-v2.5", provider: "kat_coder" },
    },
  },
  {
    name: "Longcat",
    websiteUrl: "https://longcat.chat/platform",
    apiKeyUrl: "https://longcat.chat/platform/api_keys",
    settingsConfig: {
      name: "longcat",
      base_url: "https://api.longcat.chat/openai/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "LongCat-2.0", name: "LongCat 2.0" }],
    },
    category: "cn_official",
    icon: "longcat",
    iconColor: "#29E154",
    suggestedDefaults: {
      model: { default: "LongCat-2.0", provider: "longcat" },
    },
  },
  {
    name: "MiniMax",
    family: "minimax",
    regionKey: "cn",
    websiteUrl: "https://platform.minimax.cn",
    apiKeyUrl: "https://platform.minimax.cn/console/plan",
    settingsConfig: {
      name: "minimax",
      base_url: "https://api.minimax.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "MiniMax-M3", name: "MiniMax M3" }],
    },
    category: "cn_official",
    theme: { backgroundColor: "#f64551", textColor: "#FFFFFF" },
    icon: "minimax",
    iconColor: "#FF6B6B",
    suggestedDefaults: {
      model: { default: "MiniMax-M3", provider: "minimax" },
    },
  },
  {
    name: "MiniMax en",
    family: "minimax",
    regionKey: "intl",
    websiteUrl: "https://platform.minimax.io",
    apiKeyUrl: "https://platform.minimax.io/console/plan",
    settingsConfig: {
      name: "minimax_en",
      base_url: "https://api.minimax.io/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "MiniMax-M3", name: "MiniMax M3" }],
    },
    category: "cn_official",
    theme: { backgroundColor: "#f64551", textColor: "#FFFFFF" },
    icon: "minimax",
    iconColor: "#FF6B6B",
    suggestedDefaults: {
      model: { default: "MiniMax-M3", provider: "minimax_en" },
    },
  },
  {
    name: "BaiLing",
    websiteUrl: "https://developer.ant-ling.com/zh-CN/docs/",
    apiKeyUrl: "https://chat.ant-ling.com/open",
    settingsConfig: {
      name: "bailing",
      base_url: "https://api.ant-ling.com/anthropic",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [{ id: "Ling-2.6-1T", name: "Ling 2.6 1T" }],
    },
    category: "cn_official",
    suggestedDefaults: {
      model: { default: "Ling-2.6-1T", provider: "bailing" },
    },
    icon: "bailing",
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    settingsConfig: {
      name: "cherryin",
      base_url: "https://open.cherryin.net",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "anthropic/claude-opus-5.5", name: "Claude Opus 5.5" },
        { id: "anthropic/claude-sonnet-5.5", name: "Claude Sonnet 5.5" },
        {
          id: "anthropic/claude-fable-5.1",
          name: "Claude Fable 5.1",
          context_length: 1000000,
        },
      ],
    },
    category: "aggregator",
    icon: "cherryin",
    suggestedDefaults: {
      model: { default: "anthropic/claude-opus-5.5", provider: "cherryin" },
    },
  },
  {
    name: "Novita AI",
    websiteUrl: "https://novita.ai",
    apiKeyUrl: "https://novita.ai",
    settingsConfig: {
      name: "novita",
      base_url: "https://api.novita.ai/v3/openai",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "zai-org/glm-5.3", name: "GLM-5.3", context_length: 1048576 },
        {
          id: "zai-org/glm-5.3-flash",
          name: "GLM-5.3-Flash",
          context_length: 1048576,
        },
        { id: "moonshotai/kimi-k3", name: "Kimi K3", context_length: 1048576 },
      ],
    },
    category: "aggregator",
    icon: "novita",
    iconColor: "#000000",
    suggestedDefaults: {
      model: { default: "zai-org/glm-5.3", provider: "novita" },
    },
  },
  {
    name: "Nvidia",
    websiteUrl: "https://build.nvidia.com",
    apiKeyUrl: "https://build.nvidia.com/settings/api-keys",
    settingsConfig: {
      name: "nvidia",
      base_url: "https://integrate.api.nvidia.com",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "moonshotai/kimi-k3",
          name: "Moonshot Kimi K3",
          context_length: 1048576,
        },
        { id: "z-ai/glm-5.3", name: "GLM-5.3", context_length: 1048576 },
        {
          id: "z-ai/glm-5.3-flash",
          name: "GLM-5.3-Flash",
          context_length: 1048576,
        },
      ],
    },
    category: "aggregator",
    icon: "nvidia",
    iconColor: "#000000",
    suggestedDefaults: {
      model: { default: "moonshotai/kimi-k3", provider: "nvidia" },
    },
  },
  {
    name: "Xiaomi MiMo",
    family: "xiaomi-mimo",
    planKey: "payg",
    websiteUrl: "https://platform.xiaomimimo.com",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/api-keys",
    settingsConfig: {
      name: "xiaomi_mimo",
      base_url: "https://api.xiaomimimo.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "mimo-v2.6-pro", name: "MiMo V2.6 Pro", context_length: 1048576 },
        {
          id: "mimo-v2.6-flash",
          name: "MiMo V2.6 Flash",
          context_length: 1048576,
        },
        {
          id: "mimo-v2.6-pro-ultraspeed",
          name: "MiMo V2.6 Pro UltraSpeed",
          context_length: 1048576,
        },
      ],
    },
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
    suggestedDefaults: {
      model: { default: "mimo-v2.6-pro", provider: "xiaomi_mimo" },
    },
  },
  {
    name: "Xiaomi MiMo Token Plan (China)",
    family: "xiaomi-mimo",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.xiaomimimo.com/#/token-plan",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/plan-manage",
    settingsConfig: {
      name: "xiaomi_mimo_token_plan",
      base_url: "https://token-plan-cn.xiaomimimo.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "mimo-v2.6-pro", name: "MiMo V2.6 Pro", context_length: 1048576 },
        {
          id: "mimo-v2.6-flash",
          name: "MiMo V2.6 Flash",
          context_length: 1048576,
        },
      ],
    },
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
    suggestedDefaults: {
      model: { default: "mimo-v2.6-pro", provider: "xiaomi_mimo_token_plan" },
    },
  },
];
