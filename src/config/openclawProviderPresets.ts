/**
 * OpenClaw provider presets configuration
 * OpenClaw uses models.providers structure with custom provider configs
 * 费用为标准 USD / 百万 tokens 估算，优先使用供应商公布的单价。
 * 套餐和中转参考价不包含订阅费用或账户折扣。
 */
import type {
  ProviderCategory,
  OpenClawProviderConfig,
  OpenClawDefaultModel,
} from "../types";
import type { PresetTheme, TemplateValueConfig } from "./claudeProviderPresets";
import type { PresetFamilyFields } from "./presetFamilies";

/** Suggested default model configuration for a preset */
export interface OpenClawSuggestedDefaults {
  /** Default model config to apply (agents.defaults.model) */
  model?: OpenClawDefaultModel;
  /** Model catalog entries to add (agents.defaults.models) */
  modelCatalog?: Record<string, { alias?: string }>;
}

export interface OpenClawProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string; // i18n key for localized display name
  websiteUrl: string;
  apiKeyUrl?: string;
  /** OpenClaw settings_config structure */
  settingsConfig: OpenClawProviderConfig;
  isOfficial?: boolean;
  partnerPromotionKey?: string;
  category?: ProviderCategory;
  /** Template variable definitions */
  templateValues?: Record<string, TemplateValueConfig>;
  /** Visual theme config */
  theme?: PresetTheme;
  /** Icon name */
  icon?: string;
  /** Icon color */
  iconColor?: string;
  /** Mark as custom template (for UI distinction) */
  isCustomTemplate?: boolean;
  /** Suggested default model configuration */
  suggestedDefaults?: OpenClawSuggestedDefaults;
}

function rebaseOpenClawModelRef(modelRef: string, providerKey: string): string {
  const slashIndex = modelRef.indexOf("/");
  return slashIndex === -1
    ? `${providerKey}/${modelRef}`
    : `${providerKey}${modelRef.slice(slashIndex)}`;
}

/**
 * OpenClaw default model refs are stored as "<provider-key>/<model-id>".
 * Presets carry stable built-in keys for display/tests, but the real key is
 * chosen in the add-provider form, so rewrite refs right before submission.
 */
export function rebaseOpenClawSuggestedDefaults(
  defaults: OpenClawSuggestedDefaults,
  providerKey: string,
): OpenClawSuggestedDefaults {
  const key = providerKey.trim();
  if (!key) return defaults;

  return {
    model: defaults.model
      ? {
          ...defaults.model,
          primary: rebaseOpenClawModelRef(defaults.model.primary, key),
          fallbacks: defaults.model.fallbacks?.map((modelRef) =>
            rebaseOpenClawModelRef(modelRef, key),
          ),
        }
      : undefined,
    modelCatalog: defaults.modelCatalog
      ? Object.fromEntries(
          Object.entries(defaults.modelCatalog).map(([modelRef, entry]) => [
            rebaseOpenClawModelRef(modelRef, key),
            entry,
          ]),
        )
      : undefined,
  };
}

/**
 * OpenClaw API protocol options
 * @see https://github.com/openclaw/openclaw/blob/main/docs/gateway/configuration.md
 */
export const openclawApiProtocols = [
  { value: "openai-completions", label: "OpenAI Completions" },
  { value: "openai-responses", label: "OpenAI Responses" },
  { value: "anthropic-messages", label: "Anthropic Messages" },
  { value: "google-generative-ai", label: "Google Generative AI" },
  { value: "bedrock-converse-stream", label: "AWS Bedrock" },
] as const;

/**
 * OpenClaw provider presets list
 */
export const openclawProviderPresets: OpenClawProviderPreset[] = [
  {
    name: "Kimi",
    family: "kimi",
    planKey: "payg",
    regionKey: "cn",
    websiteUrl: "https://platform.kimi.com",
    apiKeyUrl: "https://platform.kimi.com/console/api-keys",
    settingsConfig: {
      baseUrl: "https://api.moonshot.cn/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "kimi-k2.7-code",
          name: "Kimi K2.7 Code",
          contextWindow: 262144,
          cost: { input: 0.95, output: 4, cacheRead: 0.19 },
        },
        {
          id: "kimi-k3",
          name: "Kimi K3",
          contextWindow: 1048576,
          cost: { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 0 },
        },
        {
          id: "kimi-k2.7-code-highspeed",
          name: "Kimi K2.7 Code HighSpeed",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 262144,
          maxTokens: 262144,
          cost: { input: 1.9, output: 8, cacheRead: 0.38 },
        },
        {
          id: "kimi-k2.6",
          name: "Kimi K2.6",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 262144,
          maxTokens: 262144,
          cost: { input: 0.95, output: 4, cacheRead: 0.16 },
        },
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.moonshot.cn/v1",
        defaultValue: "https://api.moonshot.cn/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "kimi/kimi-k2.7-code" },
      modelCatalog: { "kimi/kimi-k2.7-code": { alias: "Kimi" } },
    },
  },
  // API 开放平台海外/Global 变体：platform.kimi.ai + api.moonshot.ai 端点
  {
    name: "Kimi Global",
    family: "kimi",
    planKey: "payg",
    regionKey: "intl",
    websiteUrl: "https://platform.kimi.ai",
    apiKeyUrl: "https://platform.kimi.ai/console/api-keys",
    settingsConfig: {
      baseUrl: "https://api.moonshot.ai/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "kimi-k2.7-code",
          name: "Kimi K2.7 Code",
          contextWindow: 262144,
          cost: { input: 0.95, output: 4, cacheRead: 0.19 },
        },
        {
          id: "kimi-k3",
          name: "Kimi K3",
          contextWindow: 1048576,
          cost: { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 0 },
        },
        {
          id: "kimi-k2.7-code-highspeed",
          name: "Kimi K2.7 Code HighSpeed",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 262144,
          maxTokens: 262144,
          cost: { input: 1.9, output: 8, cacheRead: 0.38 },
        },
        {
          id: "kimi-k2.6",
          name: "Kimi K2.6",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 262144,
          maxTokens: 262144,
          cost: { input: 0.95, output: 4, cacheRead: 0.16 },
        },
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.moonshot.ai/v1",
        defaultValue: "https://api.moonshot.ai/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "kimi/kimi-k2.7-code" },
      modelCatalog: { "kimi/kimi-k2.7-code": { alias: "Kimi" } },
    },
  },
  {
    name: "Kimi For Coding",
    family: "kimi",
    planKey: "coding",
    regionKey: "cn",
    websiteUrl: "https://www.kimi.com/code/",
    apiKeyUrl: "https://platform.kimi.com/console/api-keys",
    settingsConfig: {
      baseUrl: "https://api.kimi.com/coding/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "kimi-for-coding",
          name: "Kimi For Coding",
          contextWindow: 131072,
          cost: { input: 0.95, output: 4, cacheRead: 0.19 },
        },
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.kimi.com/coding/v1",
        defaultValue: "https://api.kimi.com/coding/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "kimi-coding/kimi-for-coding" },
      modelCatalog: { "kimi-coding/kimi-for-coding": { alias: "Kimi" } },
    },
  },
  // 海外/Global 变体：kimi.ai/code + api.kimi.ai 端点，其余与国内版一致
  {
    name: "Kimi For Coding Global",
    family: "kimi",
    planKey: "coding",
    regionKey: "intl",
    websiteUrl: "https://www.kimi.ai/code",
    apiKeyUrl: "https://www.kimi.ai/code",
    settingsConfig: {
      baseUrl: "https://api.kimi.ai/coding/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "kimi-for-coding",
          name: "Kimi For Coding",
          contextWindow: 131072,
          cost: { input: 0.95, output: 4, cacheRead: 0.19 },
        },
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.kimi.ai/coding/v1",
        defaultValue: "https://api.kimi.ai/coding/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "kimi-coding/kimi-for-coding" },
      modelCatalog: { "kimi-coding/kimi-for-coding": { alias: "Kimi" } },
    },
  },

  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    settingsConfig: {
      baseUrl: "https://api.qnaigc.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "openai/gpt-6-astra",
          name: "GPT-6 Astra",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1050000,
          maxTokens: 128000,
          cost: { input: 10, output: 50, cacheRead: 1, cacheWrite: 12.5 },
        },
        {
          id: "moonshotai/kimi-k3",
          name: "Kimi K3",
          contextWindow: 1048576,
          maxTokens: 131072,
          input: ["text", "image"],
          reasoning: true,
        },
        {
          id: "z-ai/glm-5.3",
          name: "GLM-5.3",
          reasoning: true,
          input: ["text"],
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        {
          id: "z-ai/glm-5.3-flash",
          name: "GLM-5.3-Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
        },
      ],
    },
    category: "aggregator",
    icon: "qiniu",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "qiniu/openai/gpt-6-astra",
      },
      modelCatalog: {
        "qiniu/openai/gpt-6-astra": { alias: "GPT-6 Astra" },
      },
    },
  },
  {
    name: "PPIO",
    websiteUrl: "https://ppio.com",
    apiKeyUrl: "https://ppio.com/settings/key-management",
    settingsConfig: {
      baseUrl: "https://api.ppio.com/openai/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "deepseek/deepseek-v4-flash-0731",
          name: "Deepseek V4 Flash 0731",
          reasoning: true,
          input: ["text"],
          contextWindow: 1048576,
          maxTokens: 393216,
          cost: { input: 0.14, output: 0.29, cacheRead: 0.03 },
        },
      ],
    },
    category: "aggregator",
    icon: "ppio",
    iconColor: "#2874FF",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "ppio/deepseek/deepseek-v4-flash-0731" },
      modelCatalog: {
        "ppio/deepseek/deepseek-v4-flash-0731": {
          alias: "Deepseek V4 Flash 0731",
        },
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
      baseUrl: "https://ark.cn-beijing.volces.com/api/plan/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
          contextWindow: 256000,
        },
      ],
    },
    category: "cn_official",
    icon: "huoshan",
    iconColor: "#3370FF",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "ark_agentplan/ark-code-latest" },
      modelCatalog: {
        "ark_agentplan/ark-code-latest": { alias: "Ark Code" },
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
      baseUrl: "https://ark.cn-beijing.volces.com/api/coding/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
          contextWindow: 256000,
        },
      ],
    },
    category: "cn_official",
    icon: "huoshan",
    iconColor: "#3370FF",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "ark_codingplan/ark-code-latest" },
      modelCatalog: {
        "ark_codingplan/ark-code-latest": { alias: "Ark Code" },
      },
    },
  },
  {
    name: "BytePlus",
    websiteUrl: "https://www.byteplus.com/en/product/modelark",
    apiKeyUrl: "https://www.byteplus.com/en/product/modelark",
    settingsConfig: {
      baseUrl: "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
          contextWindow: 256000,
        },
      ],
    },
    category: "cn_official",
    icon: "byteplus",
    iconColor: "#3370FF",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "byteplus/ark-code-latest" },
      modelCatalog: {
        "byteplus/ark-code-latest": { alias: "Ark Code" },
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
      baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "doubao-seed-2-1-pro-260915",
          name: "DouBao Seed 2.1 Pro",
          contextWindow: 1048576,
        },
      ],
    },
    category: "cn_official",
    icon: "doubao",
    iconColor: "#3370FF",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "doubaoseed/doubao-seed-2-1-pro-260915" },
      modelCatalog: {
        "doubaoseed/doubao-seed-2-1-pro-260915": { alias: "DouBao" },
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
      baseUrl: "https://api.siliconflow.cn/v1",
      apiKey: "",
      api: "openai-completions",
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
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "siliconflow/deepseek-ai/DeepSeek-V4-Flash" },
      modelCatalog: {
        "siliconflow/deepseek-ai/DeepSeek-V4-Flash": { alias: "DeepSeek" },
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
      baseUrl: "https://api.siliconflow.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "MiniMaxAI/MiniMax-M3",
          name: "MiniMax M3",
          contextWindow: 1048576,
          cost: { input: 0.3, output: 1.2, cacheRead: 0.06, cacheWrite: 0.375 },
        },
      ],
    },
    category: "aggregator",
    icon: "siliconflow",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "siliconflow-en/MiniMaxAI/MiniMax-M3" },
      modelCatalog: {
        "siliconflow-en/MiniMaxAI/MiniMax-M3": { alias: "MiniMax" },
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
      baseUrl: "https://api.modelverse.cn/v1",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-5",
          name: "Claude Opus 5",
          contextWindow: 1000000,
          cost: { input: 5, output: 25, cacheRead: 0.5, cacheWrite: 6.25 },
        },
      ],
    },
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "compshare/claude-opus-5",
      },
      modelCatalog: {
        "compshare/claude-opus-5": { alias: "Opus" },
      },
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
      baseUrl: "https://cp.compshare.cn/v1",
      apiKey: "",
      api: "anthropic-messages",
      // 套餐只含国产模型，没有 Claude；按官方接入页用 deepseek-v4-pro
      // https://www.compshare.cn/docs/modelverse/package_plan/usecases (2026-09-11)
      models: [
        {
          id: "deepseek-v4-pro",
          name: "DeepSeek V4 Pro",
        },
      ],
    },
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "compshare-coding/deepseek-v4-pro",
      },
      modelCatalog: {
        "compshare-coding/deepseek-v4-pro": { alias: "DeepSeek" },
      },
    },
  },
  {
    name: "AtlasCloud",
    websiteUrl: "https://www.atlascloud.ai/console/coding-plan",
    apiKeyUrl: "https://www.atlascloud.ai/console/coding-plan",
    settingsConfig: {
      baseUrl: "https://api.atlascloud.ai/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "zai-org/glm-5.2",
          name: "GLM 5.2",
        },
      ],
    },
    category: "aggregator",
    icon: "atlascloud",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "atlascloud/zai-org/glm-5.2",
      },
    },
  },
  {
    name: "DeepSeek",
    websiteUrl: "https://platform.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    settingsConfig: {
      baseUrl: "https://api.deepseek.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "deepseek-v4-pro",
          name: "DeepSeek V4 Pro",
          contextWindow: 1000000,
          cost: { input: 1.32, output: 3.96, cacheRead: 0.044, cacheWrite: 0 },
        },
        {
          id: "deepseek-flash",
          name: "DeepSeek V4.1 Flash",
          contextWindow: 1000000,
          cost: { input: 0.3, output: 1.2, cacheRead: 0.006, cacheWrite: 0 },
          input: ["text", "image"],
          reasoning: true,
          maxTokens: 384000,
        },
      ],
    },
    category: "cn_official",
    icon: "deepseek",
    iconColor: "#1E88E5",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "deepseek/deepseek-flash",
        fallbacks: ["deepseek/deepseek-v4-pro"],
      },
      modelCatalog: {
        "deepseek/deepseek-flash": { alias: "Flash" },
        "deepseek/deepseek-v4-pro": { alias: "Pro" },
      },
    },
  },
  {
    name: "Zhipu GLM",
    family: "zhipu",
    regionKey: "cn",
    websiteUrl: "https://open.bigmodel.cn",
    apiKeyUrl: "https://www.bigmodel.cn/claude-code",
    settingsConfig: {
      baseUrl: "https://open.bigmodel.cn/api/coding/paas/v4",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "glm-5.3",
          name: "GLM-5.3",
          contextWindow: 1048576,
          cost: { input: 1.4, output: 4.4, cacheRead: 0.26 },
        },
        {
          id: "glm-5.3-flash",
          name: "GLM-5.3-Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 0.15, output: 0.5, cacheRead: 0.03 },
        },
      ],
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://open.bigmodel.cn/api/coding/paas/v4",
        defaultValue: "https://open.bigmodel.cn/api/coding/paas/v4",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "zhipu/glm-5.3" },
      modelCatalog: { "zhipu/glm-5.3": { alias: "GLM" } },
    },
  },
  {
    name: "Zhipu GLM en",
    family: "zhipu",
    regionKey: "intl",
    websiteUrl: "https://z.ai",
    apiKeyUrl: "https://z.ai/subscribe",
    settingsConfig: {
      baseUrl: "https://api.z.ai/api/coding/paas/v4",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "glm-5.3",
          name: "GLM-5.3",
          contextWindow: 1048576,
          cost: { input: 1.4, output: 4.4, cacheRead: 0.26 },
        },
        {
          id: "glm-5.3-flash",
          name: "GLM-5.3-Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 0.15, output: 0.5, cacheRead: 0.03 },
        },
      ],
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.z.ai/api/coding/paas/v4",
        defaultValue: "https://api.z.ai/api/coding/paas/v4",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "zhipu-en/glm-5.3" },
      modelCatalog: { "zhipu-en/glm-5.3": { alias: "GLM" } },
    },
  },
  {
    // 腾讯云 Token Plan 个人版（1823/130060，2026-08-21 版）：通用 + Hy 两
    // 系列共用同一端点与 API Key；Auto 智能路由调用 ID 是 tc-code-latest。
    // 模型条目照官方 OpenClaw 接入页（1823/130062，2026-08-27 版）原样：
    // cost 全零、ctx/maxTokens 为官方 OpenClaw 口径（≠平台模型列表页，如
    // deepseek 1000000≠1048576，勿按平台口径"修正"）。kimi-k2.5 官方
    // 2026-08-31 下线不收（接入页仍列，随阵容口径弃用）。超出接入页的
    // minimax-m2.7/glm-5.1/glm-5.2/hy3 按平台模型列表页（1300/78934）补
    // maxTokens；hy3 reasoning:true 与 hy3-preview 一致（Preserved Thinking）
    name: "Tencent Token Plan",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan",
    settingsConfig: {
      baseUrl: "https://api.lkeap.cloud.tencent.com/plan/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "tc-code-latest",
          name: "Auto",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 196608,
          maxTokens: 32768,
        },
        {
          id: "deepseek-v4-flash-202605",
          name: "DeepSeek V4 Flash",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1000000,
          maxTokens: 384000,
        },
        {
          id: "deepseek-v4-pro-202606",
          name: "DeepSeek V4 Pro",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1000000,
          maxTokens: 384000,
        },
        // 接入页未列：maxTokens 按平台列表（128k）；reasoning 同族接入页口径
        {
          id: "minimax-m2.7",
          name: "MiniMax M2.7",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 200000,
          maxTokens: 131072,
        },
        // glm-5、glm-5.1 2026-10-09 下线（Token Plan 个人版文档 1823/130060）
        // 接入页未列：maxTokens 按平台列表（128k，与企业版接入页 131072 一致）
        {
          id: "glm-5.2",
          name: "GLM-5.2",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        // 接入页未列：hy3-preview 已下线、调用自动路由至 hy3，reasoning/口径
        // 沿用原 hy3-preview 接入页；maxTokens 按平台列表
        {
          id: "hy3",
          name: "Hy3",
          reasoning: true,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 256000,
          maxTokens: 131072,
        },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.lkeap.cloud.tencent.com/plan/v3",
        defaultValue: "https://api.lkeap.cloud.tencent.com/plan/v3",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "tencent-tokenplan/tc-code-latest" },
      modelCatalog: {
        "tencent-tokenplan/tc-code-latest": { alias: "Auto" },
      },
    },
  },
  {
    // 国际站（新加坡地域）个人版（intl 1300/81315，2026-08-20 版）：Auto
    // 调用 ID 是 auto（≠国内 tc-code-latest），阵容不同（无 GLM-5/5.1/
    // Hy3）。端点用国际站文档钦定的 tencentcloudmaas.com 域；Key 按站独立。
    // 个人版无国际站专属 OpenClaw 接入页：auto/glm-5.2/minimax-m3 取自
    // 企业版接入页（1300/81503）口径、deepseek 取自国内个人版接入页
    //（1823/130062）口径，cost 全零；kimi-k2.6 接入页未列，maxTokens 按
    // 平台模型列表页（1300/78934，256k）
    name: "Tencent Token Plan (Intl)",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan",
    settingsConfig: {
      baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "auto",
          name: "Auto",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 196608,
          maxTokens: 32768,
        },
        {
          id: "glm-5.2",
          name: "GLM-5.2",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        // 接入页未列：maxTokens 按平台列表（256k）；reasoning 同族接入页口径
        {
          id: "kimi-k2.6",
          name: "Kimi K2.6",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 262144,
          maxTokens: 262144,
        },
        {
          id: "deepseek-v4-pro-202606",
          name: "DeepSeek V4 Pro",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1000000,
          maxTokens: 384000,
        },
        {
          id: "deepseek-v4-flash-202605",
          name: "DeepSeek V4 Flash",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1000000,
          maxTokens: 384000,
        },
        {
          id: "minimax-m3",
          name: "MiniMax M3",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 32768,
        },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        defaultValue: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "tencent-tokenplan-intl/auto" },
      modelCatalog: {
        "tencent-tokenplan-intl/auto": { alias: "Auto" },
      },
    },
  },
  {
    // Token Plan 企业版专业套餐（1823/130659，2026-08-25 版，广州地域）：
    // 模型条目照官方企业版 OpenClaw 接入页（1300/81503，Pro 块）原样
    //（cost 全零、ctx/maxTokens 为官方 OpenClaw 口径）；glm-5/deepseek
    // 带日期对取个人版接入页（1823/130062）口径；接入页未列的
    // glm-5.3/glm-5.1/glm-5-turbo/kimi-k2.6/minimax-m2.7/deepseek-*-0731/
    // -0813 按平台模型列表页（1300/78934）补 maxTokens、reasoning 随同族
    // 接入页口径（全 false）。kimi-k2.5 官方 2026-08-31 下线不收；
    // minimax-m2.5 官方已除名且平台计划下线，2026-09-07 从全部 app 移除
    name: "Tencent Token Plan Enterprise Pro",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      baseUrl: "https://tokenhub.tencentmaas.com/plan/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "auto",
          name: "Auto",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 196608,
          maxTokens: 32768,
        },
        // 接入页未列：maxTokens 按平台列表（128k）
        {
          id: "glm-5.3",
          name: "GLM-5.3",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        {
          id: "glm-5.2",
          name: "GLM-5.2",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        {
          id: "glm-5",
          name: "GLM-5",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 202752,
          maxTokens: 16384,
        },
        // 接入页未列：maxTokens 按平台列表（128k）
        {
          id: "glm-5.1",
          name: "GLM-5.1",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 200000,
          maxTokens: 131072,
        },
        // 接入页未列：maxTokens 按平台列表（128k）
        {
          id: "glm-5-turbo",
          name: "GLM-5 Turbo",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 200000,
          maxTokens: 131072,
        },
        {
          id: "kimi-k2.7-code",
          name: "Kimi K2.7 Code",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 262144,
          maxTokens: 262144,
        },
        {
          id: "kimi-k2.7-code-highspeed",
          name: "Kimi K2.7 Code HighSpeed",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 262144,
          maxTokens: 262144,
        },
        // 接入页未列：maxTokens 按平台列表（256k）
        {
          id: "kimi-k2.6",
          name: "Kimi K2.6",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 262144,
          maxTokens: 262144,
        },
        // 接入页未列：maxTokens 按平台列表（128k）
        {
          id: "minimax-m2.7",
          name: "MiniMax M2.7",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 200000,
          maxTokens: 131072,
        },
        {
          id: "minimax-m3",
          name: "MiniMax M3",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 32768,
        },
        {
          id: "deepseek-v4-flash",
          name: "DeepSeek V4 Flash",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        {
          id: "deepseek-v4-pro",
          name: "DeepSeek V4 Pro",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        // 接入页未列：maxTokens 按平台列表（384k，与企业版接入页同口径）
        {
          id: "deepseek-v4-flash-0731",
          name: "DeepSeek V4 Flash 0731 GA",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        // 接入页未列：maxTokens 按平台列表（384k，与企业版接入页同口径）
        {
          id: "deepseek-v4-pro-0813",
          name: "DeepSeek V4 Pro 0813 GA",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        {
          id: "deepseek-v4-flash-202605",
          name: "DeepSeek V4 Flash Official",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        {
          id: "deepseek-v4-pro-202606",
          name: "DeepSeek V4 Pro Official",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://tokenhub.tencentmaas.com/plan/v3",
        defaultValue: "https://tokenhub.tencentmaas.com/plan/v3",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "tencent-tokenplan-epro/auto" },
      modelCatalog: {
        "tencent-tokenplan-epro/auto": { alias: "Auto" },
      },
    },
  },
  {
    // 国际站企业版专业套餐（intl 1300/81489，2026-08-26 版，新加坡地域）：
    // 阵容为广州地域子集（无 GLM-5/5.1/5-Turbo、Kimi-K2.6、MiniMax-M2.7）。
    // 模型条目照官方企业版 OpenClaw 接入页（1300/81503，Pro 块）原样
    //（cost 全零）；接入页未列的 glm-5.3/deepseek-*-0731/-0813 按平台
    // 模型列表页（1300/78934）补 maxTokens、reasoning 随同族口径
    name: "Tencent Token Plan Enterprise Pro (Intl)",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    settingsConfig: {
      baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "auto",
          name: "Auto",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 196608,
          maxTokens: 32768,
        },
        // 接入页未列：maxTokens 按平台列表（128k）
        {
          id: "glm-5.3",
          name: "GLM-5.3",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        {
          id: "glm-5.2",
          name: "GLM-5.2",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        {
          id: "minimax-m3",
          name: "MiniMax M3",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 32768,
        },
        {
          id: "kimi-k2.7-code",
          name: "Kimi K2.7 Code",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 262144,
          maxTokens: 262144,
        },
        {
          id: "kimi-k2.7-code-highspeed",
          name: "Kimi K2.7 Code HighSpeed",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 262144,
          maxTokens: 262144,
        },
        {
          id: "deepseek-v4-flash",
          name: "DeepSeek V4 Flash",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        {
          id: "deepseek-v4-pro",
          name: "DeepSeek V4 Pro",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        // 接入页未列：maxTokens 按平台列表（384k，与企业版接入页同口径）
        {
          id: "deepseek-v4-flash-0731",
          name: "DeepSeek V4 Flash 0731 GA",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        // 接入页未列：maxTokens 按平台列表（384k，与企业版接入页同口径）
        {
          id: "deepseek-v4-pro-0813",
          name: "DeepSeek V4 Pro 0813 GA",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        {
          id: "deepseek-v4-flash-202605",
          name: "DeepSeek V4 Flash Official",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
        {
          id: "deepseek-v4-pro-202606",
          name: "DeepSeek V4 Pro Official",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 1048576,
          maxTokens: 393216,
        },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        defaultValue: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "tencent-tokenplan-epro-intl/auto" },
      modelCatalog: {
        "tencent-tokenplan-epro-intl/auto": { alias: "Auto" },
      },
    },
  },
  {
    // Token Plan 企业版轻享套餐（1823/131173，2026-08-28 版）：仅 Auto 模型。
    // 条目照官方企业版 OpenClaw 接入页（1300/81503，Lite 块）原样
    name: "Tencent Token Plan Enterprise Lite",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      baseUrl: "https://tokenhub.tencentmaas.com/plan/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "auto",
          name: "Auto",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 196608,
          maxTokens: 32768,
        },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://tokenhub.tencentmaas.com/plan/v3",
        defaultValue: "https://tokenhub.tencentmaas.com/plan/v3",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "tencent-tokenplan-elite/auto" },
      modelCatalog: {
        "tencent-tokenplan-elite/auto": { alias: "Auto" },
      },
    },
  },
  {
    // 国际站企业版轻享套餐（intl 1300/81490）：新加坡地域（资源调度范围
    // Global），仅 Auto 模型。条目照官方企业版 OpenClaw 接入页
    //（1300/81503，Lite 块）原样
    name: "Tencent Token Plan Enterprise Lite (Intl)",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    settingsConfig: {
      baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "auto",
          name: "Auto",
          reasoning: false,
          input: ["text"],
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 196608,
          maxTokens: 32768,
        },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        defaultValue: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "tencent-tokenplan-elite-intl/auto" },
      modelCatalog: {
        "tencent-tokenplan-elite-intl/auto": { alias: "Auto" },
      },
    },
  },
  {
    // 千帆 Token Plan 个人版（2026-07-13 起替代 Coding Plan 发售）。模型
    // 条目照官方 OpenClaw 接入页（2026-07-22 版）原样：cost/窗口 98304/
    // maxTokens 65536 均为官方钦定的 OpenClaw 口径（≠平台模型列表页 1M，
    // 与智谱预设 128000≠平台 200K 同款惯例，勿按平台口径"修正"）
    name: "Baidu Qianfan Token Plan",
    websiteUrl: "https://cloud.baidu.com/product/codingplan.html",
    apiKeyUrl: "https://console.bce.baidu.com/qianfan/resource/token-plan",
    settingsConfig: {
      baseUrl: "https://qianfan.baidubce.com/v2/tokenplan/personal",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "deepseek-v4-pro",
          name: "deepseek-v4-pro",
          reasoning: false,
          input: ["text"],
          cost: { input: 0.0025, output: 0.01, cacheRead: 0, cacheWrite: 0 },
          contextWindow: 98304,
          maxTokens: 65536,
        },
      ],
    },
    category: "cn_official",
    icon: "baidu",
    iconColor: "#2932E1",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://qianfan.baidubce.com/v2/tokenplan/personal",
        defaultValue: "https://qianfan.baidubce.com/v2/tokenplan/personal",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "qianfan-tokenplan/deepseek-v4-pro" },
      modelCatalog: {
        "qianfan-tokenplan/deepseek-v4-pro": { alias: "DeepSeek" },
      },
    },
  },
  {
    name: "千问AI平台",
    family: "qianwen",
    planKey: "payg",
    websiteUrl: "https://platform.qianwenai.com/",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    settingsConfig: {
      baseUrl: "https://maas.qianwenaiapi.com/compatible-mode/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "qwen3.8-max",
          name: "Qwen3.8 Max",
          contextWindow: 983616,
        },
      ],
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://maas.qianwenaiapi.com/compatible-mode/v1",
        defaultValue: "https://maas.qianwenaiapi.com/compatible-mode/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "qwen/qwen3.8-max" },
      modelCatalog: { "qwen/qwen3.8-max": { alias: "Qwen" } },
    },
  },
  {
    name: "千问AI平台 Token Plan",
    family: "qianwen",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.qianwenai.com/pricing/token-plan",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    settingsConfig: {
      baseUrl: "https://token-plan.maas.qianwenaiapi.com/apps/anthropic",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "qwen3.8-max",
          name: "Qwen3.8 Max",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 983616,
          maxTokens: 131072,
        },
        {
          id: "qwen3.8-flash",
          name: "Qwen3.8 Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 983616,
          maxTokens: 131072,
        },
      ],
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://token-plan.maas.qianwenaiapi.com/apps/anthropic",
        defaultValue: "https://token-plan.maas.qianwenaiapi.com/apps/anthropic",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "qianwenai-token-plan/qwen3.8-max" },
      modelCatalog: {
        "qianwenai-token-plan/qwen3.8-max": { alias: "Qwen3.8 Max" },
        "qianwenai-token-plan/qwen3.8-flash": { alias: "Qwen3.8 Flash" },
      },
    },
  },
  // ===== QwenCloud（国际站，API 域名 qwencloudapi.com）=====
  // 与上面国内条目是两套独立站点：域名、控制台、密钥互不通用。
  // QwenCloud 与 Token Plan 都走 anthropic-messages，地址比 Claude Code 的
  // 多一段 /v1；国内 Token Plan 是唯一例外，官方文档给的就是不带 /v1 的
  // /apps/anthropic，勿照搬国际站补齐。
  // modelCatalog 必须逐条覆盖 models：OpenClaw 把 agents.defaults.models
  // 当白名单，漏写的模型会在客户端里被隐藏。
  {
    name: "QwenCloud",
    family: "qwencloud",
    planKey: "payg",
    websiteUrl: "https://home.qwencloud.com/",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      baseUrl: "https://maas.qwencloudapi.com/apps/anthropic/v1",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "qwen3.8-max",
          name: "Qwen3.8 Max",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 983616,
          maxTokens: 131072,
        },
        {
          id: "qwen3.8-flash",
          name: "Qwen3.8 Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 983616,
          maxTokens: 131072,
        },
        {
          id: "qwen3.7-max",
          name: "Qwen3.7 Max",
          input: ["text"],
          contextWindow: 1000000,
          maxTokens: 65536,
        },
      ],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://maas.qwencloudapi.com/apps/anthropic/v1",
        defaultValue: "https://maas.qwencloudapi.com/apps/anthropic/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "qwencloud/qwen3.8-max" },
      modelCatalog: {
        "qwencloud/qwen3.8-max": { alias: "Qwen3.8 Max" },
        "qwencloud/qwen3.8-flash": { alias: "Qwen3.8 Flash" },
        "qwencloud/qwen3.7-max": { alias: "Qwen3.7 Max" },
      },
    },
  },
  {
    name: "QwenCloud For Coding",
    family: "qwencloud",
    planKey: "coding",
    websiteUrl: "https://www.qwencloud.com",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      baseUrl: "https://coding-intl.dashscope.aliyuncs.com/apps/anthropic/v1",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "qwen3.7-plus",
          name: "Qwen3.7 Plus",
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 65536,
        },
        {
          id: "qwen3.6-plus",
          name: "Qwen3.6 Plus",
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 65536,
        },
      ],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder:
          "https://coding-intl.dashscope.aliyuncs.com/apps/anthropic/v1",
        defaultValue:
          "https://coding-intl.dashscope.aliyuncs.com/apps/anthropic/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "qwencloud-coding/qwen3.7-plus" },
      modelCatalog: {
        "qwencloud-coding/qwen3.7-plus": { alias: "Qwen3.7 Plus" },
        "qwencloud-coding/qwen3.6-plus": { alias: "Qwen3.6 Plus" },
      },
    },
  },
  {
    name: "QwenCloud Token Plan",
    family: "qwencloud",
    planKey: "tokenPlan",
    websiteUrl: "https://www.qwencloud.com/pricing/token-plan",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      baseUrl: "https://token-plan.maas.qwencloudapi.com/apps/anthropic/v1",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "qwen3.8-max",
          name: "Qwen3.8 Max",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 983616,
          maxTokens: 131072,
        },
        {
          id: "qwen3.8-flash",
          name: "Qwen3.8 Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 983616,
          maxTokens: 131072,
        },
        {
          id: "qwen3.7-max",
          name: "Qwen3.7 Max",
          input: ["text"],
          contextWindow: 1000000,
          maxTokens: 65536,
        },
      ],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder:
          "https://token-plan.maas.qwencloudapi.com/apps/anthropic/v1",
        defaultValue:
          "https://token-plan.maas.qwencloudapi.com/apps/anthropic/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "qwencloud-token-plan/qwen3.8-max" },
      modelCatalog: {
        "qwencloud-token-plan/qwen3.8-max": { alias: "Qwen3.8 Max" },
        "qwencloud-token-plan/qwen3.8-flash": { alias: "Qwen3.8 Flash" },
        "qwencloud-token-plan/qwen3.7-max": { alias: "Qwen3.7 Max" },
      },
    },
  },
  {
    name: "StepFun",
    family: "stepfun",
    regionKey: "cn",
    websiteUrl: "https://platform.stepfun.com/step-plan",
    apiKeyUrl: "https://platform.stepfun.com/interface-key",
    settingsConfig: {
      baseUrl: "https://api.stepfun.com/step_plan/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "step-3.5-flash-2603",
          name: "Step 3.5 Flash 2603",
          contextWindow: 262144,
        },
        {
          id: "step-3.5-flash",
          name: "Step 3.5 Flash",
          contextWindow: 262144,
        },
        {
          id: "step-3.7-flash",
          name: "Step 3.7 Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 256000,
          maxTokens: 256000,
          cost: { input: 0.19, output: 1.13, cacheRead: 0.04, cacheWrite: 0 },
        },
        {
          id: "step-5-preview",
          name: "Step 5 Preview",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 64000,
          cost: {
            input: 0.98,
            output: 2.8,
            cacheRead: 0.05,
            cacheWrite: 0,
          },
        },
      ],
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#16D6D2",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.stepfun.com/step_plan/v1",
        defaultValue: "https://api.stepfun.com/step_plan/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "step-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "stepfun/step-5-preview" },
      modelCatalog: {
        "stepfun/step-5-preview": { alias: "StepFun" },
        "stepfun/step-3.5-flash": { alias: "StepFun Flash" },
      },
    },
  },
  {
    name: "StepFun en",
    family: "stepfun",
    regionKey: "intl",
    websiteUrl: "https://platform.stepfun.ai/step-plan",
    apiKeyUrl: "https://platform.stepfun.ai/interface-key",
    settingsConfig: {
      baseUrl: "https://api.stepfun.ai/step_plan/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "step-3.5-flash-2603",
          name: "Step 3.5 Flash 2603",
          contextWindow: 262144,
        },
        {
          id: "step-3.5-flash",
          name: "Step 3.5 Flash",
          contextWindow: 262144,
        },
        {
          id: "step-3.7-flash",
          name: "Step 3.7 Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 256000,
          maxTokens: 256000,
          cost: { input: 0.19, output: 1.13, cacheRead: 0.04, cacheWrite: 0 },
        },
        {
          id: "step-5-preview",
          name: "Step 5 Preview",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 64000,
          cost: {
            input: 0.98,
            output: 2.8,
            cacheRead: 0.05,
            cacheWrite: 0,
          },
        },
      ],
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#16D6D2",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.stepfun.ai/step_plan/v1",
        defaultValue: "https://api.stepfun.ai/step_plan/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "step-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "stepfun-en/step-5-preview" },
      modelCatalog: {
        "stepfun-en/step-5-preview": { alias: "StepFun" },
        "stepfun-en/step-3.5-flash": { alias: "StepFun Flash" },
      },
    },
  },
  {
    name: "MiniMax",
    family: "minimax",
    regionKey: "cn",
    websiteUrl: "https://platform.minimax.cn",
    apiKeyUrl: "https://platform.minimax.cn/console/plan",
    settingsConfig: {
      baseUrl: "https://api.minimax.cn/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "MiniMax-M3",
          name: "MiniMax M3",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 131072,
          cost: { input: 0.3, output: 1.2, cacheRead: 0.06, cacheWrite: 0 },
        },
      ],
    },
    category: "cn_official",
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
    icon: "minimax",
    iconColor: "#FF6B6B",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "minimax/MiniMax-M3" },
      modelCatalog: { "minimax/MiniMax-M3": { alias: "MiniMax" } },
    },
  },
  {
    name: "MiniMax en",
    family: "minimax",
    regionKey: "intl",
    websiteUrl: "https://platform.minimax.io",
    apiKeyUrl: "https://platform.minimax.io/console/plan",
    settingsConfig: {
      baseUrl: "https://api.minimax.io/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "MiniMax-M3",
          name: "MiniMax M3",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 131072,
          cost: { input: 0.3, output: 1.2, cacheRead: 0.06, cacheWrite: 0 },
        },
      ],
    },
    category: "cn_official",
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
    icon: "minimax",
    iconColor: "#FF6B6B",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "minimax-en/MiniMax-M3" },
      modelCatalog: { "minimax-en/MiniMax-M3": { alias: "MiniMax" } },
    },
  },
  {
    name: "KAT-Coder",
    websiteUrl: "https://console.streamlake.ai",
    apiKeyUrl: "https://console.streamlake.ai/console/api-key",
    settingsConfig: {
      baseUrl:
        "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "KAT-Coder-Pro",
          name: "KAT-Coder Pro",
          contextWindow: 128000,
          cost: { input: 0.3, output: 1.2, cacheRead: 0.06 },
        },
      ],
    },
    category: "cn_official",
    icon: "catcoder",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        defaultValue:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        editorValue: "",
      },
      ENDPOINT_ID: {
        label: "Endpoint ID",
        placeholder: "",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "katcoder/KAT-Coder-Pro" },
      modelCatalog: { "katcoder/KAT-Coder-Pro": { alias: "KAT-Coder" } },
    },
  },
  {
    name: "Longcat",
    websiteUrl: "https://longcat.chat/platform",
    apiKeyUrl: "https://longcat.chat/platform/api_keys",
    settingsConfig: {
      baseUrl: "https://api.longcat.chat/openai/v1",
      apiKey: "",
      api: "openai-completions",
      authHeader: true,
      models: [
        {
          id: "LongCat-2.0",
          name: "LongCat 2.0",
          reasoning: false,
          input: ["text"],
          contextWindow: 1048576,
          maxTokens: 131072,
          compat: { maxTokensField: "max_tokens" },
          cost: { input: 0.75, output: 2.95, cacheRead: 0.015 },
        },
      ],
    },
    category: "cn_official",
    icon: "longcat",
    iconColor: "#29E154",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.longcat.chat/openai/v1",
        defaultValue: "https://api.longcat.chat/openai/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "longcat/LongCat-2.0" },
      modelCatalog: { "longcat/LongCat-2.0": { alias: "LongCat" } },
    },
  },
  {
    name: "BaiLing",
    websiteUrl: "https://developer.ant-ling.com/zh-CN/docs/",
    apiKeyUrl: "https://chat.ant-ling.com/open",
    settingsConfig: {
      baseUrl: "https://api.ant-ling.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "Ling-2.6-1T",
          name: "Ling 2.6 1T",
          contextWindow: 262144,
          cost: { input: 0.63, output: 2.52 },
        },
      ],
    },
    category: "cn_official",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "bailing/Ling-2.6-1T" },
      modelCatalog: { "bailing/Ling-2.6-1T": { alias: "BaiLing" } },
    },
    icon: "bailing",
  },
  {
    name: "Xiaomi MiMo",
    family: "xiaomi-mimo",
    planKey: "payg",
    websiteUrl: "https://platform.xiaomimimo.com",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/api-keys",
    settingsConfig: {
      baseUrl: "https://api.xiaomimimo.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "mimo-v2.6-pro",
          name: "MiMo V2.6 Pro",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 0.435, output: 0.87, cacheRead: 0.0036 },
        },
        {
          id: "mimo-v2.6-flash",
          name: "MiMo V2.6 Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 0.14, output: 0.28, cacheRead: 0.0028 },
        },
        {
          id: "mimo-v2.6-pro-ultraspeed",
          name: "MiMo V2.6 Pro UltraSpeed",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 4.35, output: 8.7, cacheRead: 0.036 },
        },
      ],
    },
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "xiaomimimo/mimo-v2.6-pro" },
      modelCatalog: { "xiaomimimo/mimo-v2.6-pro": { alias: "MiMo" } },
    },
  },
  {
    name: "Xiaomi MiMo Token Plan (China)",
    family: "xiaomi-mimo",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.xiaomimimo.com/#/token-plan",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/plan-manage",
    settingsConfig: {
      baseUrl: "https://token-plan-cn.xiaomimimo.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "mimo-v2.6-pro",
          name: "MiMo V2.6 Pro",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 0.435, output: 0.87, cacheRead: 0.0036 },
        },
        {
          id: "mimo-v2.6-flash",
          name: "MiMo V2.6 Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 0.14, output: 0.28, cacheRead: 0.0028 },
        },
      ],
    },
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "Token Plan API Key",
        placeholder: "tp-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "xiaomi-mimo-token-plan/mimo-v2.6-pro" },
      modelCatalog: {
        "xiaomi-mimo-token-plan/mimo-v2.6-pro": {
          alias: "MiMo Token Plan (China)",
        },
      },
    },
  },

  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    settingsConfig: {
      baseUrl: "https://open.cherryin.net",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "anthropic/claude-opus-5.5",
          name: "Claude Opus 5.5",
          contextWindow: 1000000,
        },
        {
          id: "anthropic/claude-sonnet-5.5",
          name: "Claude Sonnet 5.5",
          contextWindow: 1000000,
        },
        {
          id: "anthropic/claude-fable-5.1",
          name: "Claude Fable 5.1",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 128000,
          cost: { input: 10, output: 50, cacheRead: 0.25, cacheWrite: 12.5 },
        },
      ],
    },
    category: "aggregator",
    icon: "cherryin",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "cherryin/anthropic/claude-opus-5.5",
        fallbacks: ["cherryin/anthropic/claude-sonnet-5.5"],
      },
      modelCatalog: {
        "cherryin/anthropic/claude-opus-5.5": { alias: "Opus" },
        "cherryin/anthropic/claude-sonnet-5.5": { alias: "Sonnet" },
      },
    },
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    settingsConfig: {
      baseUrl: "https://openrouter.ai/api/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "anthropic/claude-sonnet-5.5",
          name: "Claude Sonnet 5.5",
          contextWindow: 1000000,
          cost: {
            input: 2,
            output: 10,
            cacheRead: 0.2,
            cacheWrite: 2.5,
          },
        },
        {
          id: "anthropic/claude-opus-5.5",
          name: "Claude Opus 5.5",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 128000,
          cost: {
            input: 4,
            output: 20,
            cacheRead: 0.2,
            cacheWrite: 5,
          },
        },
        {
          id: "anthropic/claude-fable-5.1",
          name: "Claude Fable 5.1",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1000000,
          maxTokens: 128000,
          cost: { input: 10, output: 50, cacheRead: 0.25, cacheWrite: 12.5 },
        },
      ],
    },
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6566F1",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-or-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "openrouter/anthropic/claude-opus-5.5",
        fallbacks: ["openrouter/anthropic/claude-sonnet-5.5"],
      },
      modelCatalog: {
        "openrouter/anthropic/claude-opus-5.5": { alias: "Opus" },
        "openrouter/anthropic/claude-sonnet-5.5": { alias: "Sonnet" },
      },
    },
  },
  {
    name: "ModelScope",
    websiteUrl: "https://modelscope.cn",
    apiKeyUrl: "https://modelscope.cn/my/myaccesstoken",
    settingsConfig: {
      baseUrl: "https://api-inference.modelscope.cn/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "ZhipuAI/GLM-5.2",
          name: "GLM-5.2",
          contextWindow: 128000,
          cost: { input: 1.4, output: 4.4, cacheRead: 0.26 },
        },
      ],
    },
    category: "aggregator",
    icon: "modelscope",
    iconColor: "#624AFF",
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api-inference.modelscope.cn/v1",
        defaultValue: "https://api-inference.modelscope.cn/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "modelscope/ZhipuAI/GLM-5.2" },
      modelCatalog: { "modelscope/ZhipuAI/GLM-5.2": { alias: "GLM" } },
    },
  },
  {
    name: "Novita AI",
    websiteUrl: "https://novita.ai",
    apiKeyUrl: "https://novita.ai",
    settingsConfig: {
      baseUrl: "https://api.novita.ai/openai",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "zai-org/glm-5.3",
          name: "GLM-5.3",
          reasoning: true,
          input: ["text"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 1.4, output: 4.4, cacheRead: 0.26, cacheWrite: 0 },
        },
        {
          id: "zai-org/glm-5.3-flash",
          name: "GLM-5.3-Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 0.15, output: 0.5, cacheRead: 0.03, cacheWrite: 0 },
        },
        {
          id: "moonshotai/kimi-k3",
          name: "Kimi K3",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 0 },
        },
      ],
    },
    category: "aggregator",
    icon: "novita",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "novita/zai-org/glm-5.3" },
      modelCatalog: {
        "novita/zai-org/glm-5.3": { alias: "GLM-5.3" },
      },
    },
  },
  {
    name: "Nvidia",
    websiteUrl: "https://build.nvidia.com",
    apiKeyUrl: "https://build.nvidia.com/settings/api-keys",
    settingsConfig: {
      baseUrl: "https://integrate.api.nvidia.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "moonshotai/kimi-k3",
          name: "Kimi K3",
          contextWindow: 1048576,
          maxTokens: 131072,
          input: ["text", "image"],
          reasoning: true,
        },
        {
          id: "z-ai/glm-5.3",
          name: "GLM-5.3",
          reasoning: true,
          input: ["text"],
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        {
          id: "z-ai/glm-5.3-flash",
          name: "GLM-5.3-Flash",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
        },
      ],
    },
    category: "aggregator",
    icon: "nvidia",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "nvapi-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "nvidia/moonshotai/kimi-k3" },
      modelCatalog: { "nvidia/moonshotai/kimi-k3": { alias: "Kimi" } },
    },
  },
  {
    name: "AWS Bedrock",
    websiteUrl: "https://aws.amazon.com/bedrock/",
    settingsConfig: {
      // 请将 us-west-2 替换为你的 AWS Region
      baseUrl: "https://bedrock-runtime.us-west-2.amazonaws.com",
      apiKey: "",
      api: "bedrock-converse-stream",
      // global 跨区推理配置：Opus / Sonnet 在 us-west-2 不支持区内调用，
      // Haiku 4.5 不接受不带前缀的 ID（AWS 各模型卡，2026-10）
      models: [
        {
          id: "global.anthropic.claude-opus-5-5",
          name: "Claude Opus 5.5",
          contextWindow: 1000000,
          cost: { input: 4, output: 20, cacheRead: 0.2, cacheWrite: 5 },
        },
        {
          id: "global.anthropic.claude-sonnet-5-5",
          name: "Claude Sonnet 5.5",
          contextWindow: 1000000,
          cost: { input: 2, output: 10, cacheRead: 0.2, cacheWrite: 2.5 },
        },
        {
          id: "global.anthropic.claude-haiku-4-5-20251001-v1:0",
          name: "Claude Haiku 4.5",
          contextWindow: 200000,
          cost: { input: 1, output: 5, cacheRead: 0.1, cacheWrite: 1.25 },
        },
      ],
    },
    category: "cloud_provider",
    icon: "aws",
    iconColor: "#FF9900",
  },
  {
    name: "模力方舟",
    websiteUrl: "https://moark.com",
    apiKeyUrl: "https://moark.com/dashboard/tokens",
    settingsConfig: {
      baseUrl: "https://api.moark.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "deepseek-v4-flash-0731",
          name: "DeepSeek V4 Flash",
          reasoning: true,
          contextWindow: 1000000,
          maxTokens: 384000,
        },
        {
          id: "DeepSeek-V4-Pro",
          name: "DeepSeek V4 Pro",
          reasoning: true,
          contextWindow: 1000000,
          maxTokens: 384000,
        },
        {
          id: "GLM-5.3",
          name: "GLM-5.3",
          reasoning: true,
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        {
          id: "Kimi-K2.7-Code",
          name: "Kimi K2.7 Code",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 262144,
          maxTokens: 262144,
        },
        // qwen3-coder-plus 2026-10-10 下线（阿里云公告 118344）
        {
          id: "qwen3.8-max",
          name: "Qwen3.8 Max",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 983616,
          maxTokens: 131072,
        },
      ],
    },
    category: "aggregator",
    icon: "moark",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "moark/deepseek-v4-flash-0731" },
      modelCatalog: {
        "moark/deepseek-v4-flash-0731": { alias: "DeepSeek V4 Flash" },
      },
    },
  },
];
