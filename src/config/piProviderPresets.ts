import type { ProviderCategory } from "@/types";
import type { PresetTheme } from "./claudeProviderPresets";
import {
  getPiModelCatalogReference,
  piModel,
  type PiCatalogModel,
} from "./piModelCatalog";
import {
  getPiThinkingProfile,
  resolvePiThinkingProfile,
  type PiThinkingLevelMap,
} from "./piThinkingProfiles";
import type { PresetFamilyFields } from "./presetFamilies";

export type PiApiFormat =
  | "openai-completions"
  | "openai-responses"
  | "anthropic-messages"
  | "google-generative-ai"
  | "bedrock-converse-stream";

export type PiPresetModel = PiCatalogModel & {
  thinkingLevelMap?: PiThinkingLevelMap;
  compat?: Record<string, unknown>;
};

export interface PiProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string;
  providerKey: string;
  /**
   * Pi 已内置这个供应商时的内置 ID 和地址（Pi 1.0.4）。添加到 Pi 时只在
   * models.json 的这个 ID 下写 key，模型和地址由 Pi 维护；baseUrl 只给 CC Switch
   * 判断用量查询用，不写进 models.json。settingsConfig 仍是完整的端点和模型，
   * MiniMax Code 预设从这里派生。
   */
  piBuiltIn?: { provider: string; baseUrl: string };
  websiteUrl: string;
  apiKeyUrl?: string;
  settingsConfig: {
    name: string;
    baseUrl: string;
    api: PiApiFormat;
    apiKey: string;
    headers?: Record<string, string>;
    compat?: Record<string, unknown>;
    models: PiPresetModel[];
  };
  category?: ProviderCategory;
  partnerPromotionKey?: string;
  theme?: PresetTheme;
  icon?: string;
  iconColor?: string;
}

const OPENAI_COMPLETIONS_COMPAT = {
  supportsStore: false,
  supportsDeveloperRole: false,
  maxTokensField: "max_tokens",
} as const;

const DEEPSEEK_THINKING_COMPAT = {
  ...OPENAI_COMPLETIONS_COMPAT,
  requiresReasoningContentOnAssistantMessages: true,
  thinkingFormat: "deepseek",
} as const;

// 1823/132248: thinking.type defaults to "enabled"; disabling requires an explicit
// {"type":"disabled"}. The same doc says reasoning_content need not be echoed back
// on multi-turn, so we don't reuse DEEPSEEK_THINKING_COMPAT wholesale.
const TENCENT_DEEPSEEK_THINKING_COMPAT = {
  ...OPENAI_COMPLETIONS_COMPAT,
  thinkingFormat: "deepseek",
} as const;

const XIAOMI_THINKING_COMPAT = {
  requiresReasoningContentOnAssistantMessages: true,
  thinkingFormat: "deepseek",
} as const;

// DashScope's /compatible-mode/v1 returns reasoning in Qwen's own envelope and
// rejects the `developer` role, so neither OpenAI nor DeepSeek thinking applies.
const QWEN_THINKING_COMPAT = {
  thinkingFormat: "qwen",
  supportsDeveloperRole: false,
} as const;

const KIMI_K3_COMPAT = {
  supportsStore: false,
  supportsDeveloperRole: false,
  supportsReasoningEffort: true,
  maxTokensField: "max_tokens",
  supportsStrictMode: false,
  thinkingFormat: "openai",
  requiresReasoningContentOnAssistantMessages: true,
  deferredToolsMode: "kimi",
} as const;

/**
 * Pi-native provider catalog.
 *
 * This list is independently maintained because provider protocol, endpoint
 * roots and model capabilities are application-specific. It was initially
 * aligned with the OpenCode catalog, but Pi does not import or derive from
 * another application's presets at runtime.
 */
const piProviderPresetDefinitions: PiProviderPreset[] = [
  {
    name: "Kimi",
    family: "kimi",
    planKey: "payg",
    regionKey: "cn",
    providerKey: "cc-switch-kimi",
    piBuiltIn: {
      provider: "moonshotai-cn",
      baseUrl: "https://api.moonshot.cn/v1",
    },
    websiteUrl: "https://platform.kimi.com",
    apiKeyUrl: "https://platform.kimi.com/console/api-keys",
    settingsConfig: {
      name: "Kimi",
      baseUrl: "https://api.moonshot.cn/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("moonshotai/kimi-k2.7-code", {
          id: "kimi-k2.7-code",
          thinkingProfile: "offUnsupported",
        }),
        {
          ...piModel("moonshotai/kimi-k3", {
            id: "kimi-k3",
            thinkingProfile: "kimi3",
          }),
          compat: { ...KIMI_K3_COMPAT },
        },
        piModel("moonshotai/kimi-k2.7-code-highspeed", {
          id: "kimi-k2.7-code-highspeed",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k2.6", { id: "kimi-k2.6" }),
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  // API 开放平台海外/Global 变体：platform.kimi.ai + api.moonshot.ai 端点
  {
    name: "Kimi Global",
    family: "kimi",
    planKey: "payg",
    regionKey: "intl",
    providerKey: "cc-switch-kimi-global",
    piBuiltIn: {
      provider: "moonshotai",
      baseUrl: "https://api.moonshot.ai/v1",
    },
    websiteUrl: "https://platform.kimi.ai",
    apiKeyUrl: "https://platform.kimi.ai/console/api-keys",
    settingsConfig: {
      name: "Kimi",
      baseUrl: "https://api.moonshot.ai/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("moonshotai/kimi-k2.7-code", {
          id: "kimi-k2.7-code",
          thinkingProfile: "offUnsupported",
        }),
        {
          ...piModel("moonshotai/kimi-k3", {
            id: "kimi-k3",
            thinkingProfile: "kimi3",
          }),
          compat: { ...KIMI_K3_COMPAT },
        },
        piModel("moonshotai/kimi-k2.7-code-highspeed", {
          id: "kimi-k2.7-code-highspeed",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k2.6", { id: "kimi-k2.6" }),
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  {
    name: "Kimi For Coding",
    family: "kimi",
    planKey: "coding",
    regionKey: "cn",
    providerKey: "cc-switch-kimi-for-coding",
    piBuiltIn: {
      provider: "kimi-coding",
      baseUrl: "https://api.kimi.com/coding",
    },
    websiteUrl: "https://www.kimi.com/code/",
    apiKeyUrl: "https://platform.kimi.com/console/api-keys",
    settingsConfig: {
      name: "Kimi For Coding",
      baseUrl: "https://api.kimi.com/coding",
      api: "anthropic-messages",
      apiKey: "",
      models: [
        piModel("moonshotai/kimi-k2.7-code", {
          id: "kimi-for-coding",
          name: "Kimi For Coding",
          maxTokens: 32768,
        }),
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  // 海外/Global 变体：kimi.ai/code + api.kimi.ai 端点，其余与国内版一致
  {
    name: "Kimi For Coding Global",
    family: "kimi",
    planKey: "coding",
    regionKey: "intl",
    providerKey: "cc-switch-kimi-for-coding-global",
    websiteUrl: "https://www.kimi.ai/code",
    apiKeyUrl: "https://www.kimi.ai/code",
    settingsConfig: {
      name: "Kimi For Coding",
      baseUrl: "https://api.kimi.ai/coding",
      api: "anthropic-messages",
      apiKey: "",
      models: [
        piModel("moonshotai/kimi-k2.7-code", {
          id: "kimi-for-coding",
          name: "Kimi For Coding",
          maxTokens: 32768,
        }),
      ],
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    providerKey: "cc-switch-qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    settingsConfig: {
      name: "Qiniu",
      baseUrl: "https://api.qnaigc.com/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("openai/gpt-6-astra", { id: "openai/gpt-6-astra" }),
        piModel("moonshotai/kimi-k3", {
          id: "moonshotai/kimi-k3",
        }),
        piModel("zai/glm-5.3", { id: "z-ai/glm-5.3" }),
        piModel("zai/glm-5.3-flash", { id: "z-ai/glm-5.3-flash" }),
      ],
    },
    category: "aggregator",
    icon: "qiniu",
  },
  {
    name: "PPIO",
    providerKey: "cc-switch-ppio",
    websiteUrl: "https://ppio.com",
    apiKeyUrl: "https://ppio.com/settings/key-management",
    settingsConfig: {
      name: "PPIO",
      baseUrl: "https://api.ppio.com/openai/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek/deepseek-v4-flash-0731",
            name: "Deepseek V4 Flash 0731",
            contextWindow: 1_048_576,
            maxTokens: 393_216,
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...DEEPSEEK_THINKING_COMPAT },
        },
      ],
    },
    category: "aggregator",
    icon: "ppio",
    iconColor: "#2874FF",
  },
  {
    name: "火山Agentplan",
    family: "volcengine",
    planKey: "agentPlan",
    providerKey: "cc-switch-agentplan",
    websiteUrl: "https://www.volcengine.com/activity/codingplan",
    apiKeyUrl: "https://www.volcengine.com/activity/codingplan",
    settingsConfig: {
      name: "火山Agentplan",
      baseUrl: "https://ark.cn-beijing.volces.com/api/coding/v3",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("volcengine/ark-code-latest", {
          id: "ark-code-latest",
        }),
      ],
    },
    category: "cn_official",
    icon: "huoshan",
    iconColor: "#3370FF",
  },
  {
    name: "BytePlus",
    providerKey: "cc-switch-byte-plus",
    websiteUrl: "https://www.byteplus.com/en/product/modelark",
    apiKeyUrl: "https://www.byteplus.com/en/product/modelark",
    settingsConfig: {
      name: "BytePlus",
      baseUrl: "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("volcengine/ark-code-latest", {
          id: "ark-code-latest",
        }),
      ],
    },
    category: "cn_official",
    icon: "byteplus",
    iconColor: "#3370FF",
  },
  {
    name: "Volcengine Doubao",
    family: "volcengine",
    planKey: "payg",
    nameKey: "providerForm.presets.doubaoseed",
    providerKey: "cc-switch-dou-bao-seed",
    websiteUrl:
      "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    apiKeyUrl:
      "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    settingsConfig: {
      name: "Volcengine Doubao",
      baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
      api: "openai-completions",
      apiKey: "",
      models: [
        // 方舟模型列表：上下文 1024k，最大回答 256k（2026-09-28）
        piModel("volcengine/doubao-seed-2.1-pro", {
          id: "doubao-seed-2-1-pro-260915",
          contextWindow: 1_048_576,
          maxTokens: 262_144,
        }),
      ],
    },
    category: "cn_official",
    icon: "doubao",
    iconColor: "#3370FF",
  },
  {
    name: "AtlasCloud",
    providerKey: "cc-switch-atlas-cloud",
    websiteUrl: "https://www.atlascloud.ai/console/coding-plan",
    apiKeyUrl: "https://www.atlascloud.ai/console/coding-plan",
    settingsConfig: {
      name: "AtlasCloud",
      baseUrl: "https://api.atlascloud.ai/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("zai/glm-5.2", {
          id: "zai-org/glm-5.2",
          name: "GLM 5.2",
        }),
      ],
    },
    category: "aggregator",
    icon: "atlascloud",
  },
  {
    name: "DeepSeek",
    providerKey: "cc-switch-deep-seek",
    piBuiltIn: { provider: "deepseek", baseUrl: "https://api.deepseek.com" },
    websiteUrl: "https://platform.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    settingsConfig: {
      name: "DeepSeek",
      baseUrl: "https://api.deepseek.com/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("deepseek/deepseek-v4-pro", {
          id: "deepseek-v4-pro",
          thinkingProfile: "deepseekV4",
        }),
        piModel("deepseek/deepseek-flash", {
          id: "deepseek-flash",
          thinkingProfile: "deepseekV4",
        }),
      ],
    },
    category: "cn_official",
    icon: "deepseek",
    iconColor: "#1E88E5",
  },
  {
    name: "Zhipu GLM",
    family: "zhipu",
    regionKey: "cn",
    providerKey: "cc-switch-zhipu-glm",
    piBuiltIn: {
      provider: "zai-coding-cn",
      baseUrl: "https://open.bigmodel.cn/api/coding/paas/v4",
    },
    websiteUrl: "https://open.bigmodel.cn",
    apiKeyUrl: "https://www.bigmodel.cn/claude-code",
    settingsConfig: {
      name: "Zhipu GLM",
      baseUrl: "https://open.bigmodel.cn/api/coding/paas/v4",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("zai/glm-5.3", {
          id: "glm-5.3",
        }),
        piModel("zai/glm-5.3-flash", { id: "glm-5.3-flash" }),
      ],
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
  },
  {
    name: "Zhipu GLM en",
    family: "zhipu",
    regionKey: "intl",
    providerKey: "cc-switch-zhipu-glm-en",
    piBuiltIn: {
      provider: "zai",
      baseUrl: "https://api.z.ai/api/coding/paas/v4",
    },
    websiteUrl: "https://z.ai",
    apiKeyUrl: "https://z.ai/subscribe",
    settingsConfig: {
      name: "Zhipu GLM en",
      baseUrl: "https://api.z.ai/api/coding/paas/v4",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("zai/glm-5.3", {
          id: "glm-5.3",
        }),
        piModel("zai/glm-5.3-flash", { id: "glm-5.3-flash" }),
      ],
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
  },
  {
    name: "千问AI平台",
    family: "qianwen",
    planKey: "payg",
    providerKey: "cc-switch-qianwenai",
    websiteUrl: "https://platform.qianwenai.com/",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    settingsConfig: {
      name: "千问AI平台",
      baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        {
          ...piModel("qwen/qwen3.8-max", {
            id: "qwen3.8-max",
          }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
      ],
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  {
    name: "千问AI平台 Token Plan",
    family: "qianwen",
    planKey: "tokenPlan",
    providerKey: "cc-switch-qianwenai-token-plan",
    websiteUrl: "https://platform.qianwenai.com/pricing/token-plan",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    settingsConfig: {
      name: "千问AI平台 Token Plan",
      baseUrl:
        "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        {
          ...piModel("qwen/qwen3.8-max", { id: "qwen3.8-max" }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
        {
          ...piModel("qwen/qwen3.8-flash", { id: "qwen3.8-flash" }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
      ],
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  // ===== QwenCloud（DashScope 国际站）=====
  // 与上面国内条目是两套独立站点：域名、控制台、密钥互不通用。
  // 按量付费与 Token Plan 都走 OpenAI 兼容层（/compatible-mode/v1）。
  {
    name: "QwenCloud",
    family: "qwencloud",
    planKey: "payg",
    providerKey: "cc-switch-qwencloud",
    websiteUrl: "https://home.qwencloud.com/",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      name: "QwenCloud",
      baseUrl: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        {
          ...piModel("qwen/qwen3.8-max", { id: "qwen3.8-max" }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
        {
          ...piModel("qwen/qwen3.8-flash", { id: "qwen3.8-flash" }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
        {
          ...piModel("qwen/qwen3.7-max", { id: "qwen3.7-max" }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
      ],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "QwenCloud For Coding",
    family: "qwencloud",
    planKey: "coding",
    providerKey: "cc-switch-qwencloud-coding",
    websiteUrl: "https://www.qwencloud.com",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      name: "QwenCloud For Coding",
      baseUrl: "https://coding-intl.dashscope.aliyuncs.com/apps/anthropic",
      api: "anthropic-messages",
      apiKey: "",
      models: [piModel("qwen/qwen3.7-plus", { id: "qwen3.7-plus" })],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "QwenCloud Token Plan",
    family: "qwencloud",
    planKey: "tokenPlan",
    providerKey: "cc-switch-qwencloud-token-plan",
    websiteUrl: "https://www.qwencloud.com/pricing/token-plan",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      name: "QwenCloud Token Plan",
      baseUrl:
        "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        {
          ...piModel("qwen/qwen3.8-max", { id: "qwen3.8-max" }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
        {
          ...piModel("qwen/qwen3.8-flash", { id: "qwen3.8-flash" }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
        {
          ...piModel("qwen/qwen3.7-max", { id: "qwen3.7-max" }),
          compat: { ...QWEN_THINKING_COMPAT },
        },
      ],
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "StepFun",
    family: "stepfun",
    regionKey: "cn",
    providerKey: "cc-switch-step-fun",
    websiteUrl: "https://platform.stepfun.com/step-plan",
    apiKeyUrl: "https://platform.stepfun.com/interface-key",
    settingsConfig: {
      name: "StepFun",
      baseUrl: "https://api.stepfun.com/step_plan/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("stepfun/step-3.5-flash", {
          id: "step-3.5-flash-2603",
          name: "Step 3.5 Flash 2603",
        }),
        piModel("stepfun/step-3.5-flash", {
          id: "step-3.5-flash",
        }),
        piModel("stepfun/step-3.7-flash", { id: "step-3.7-flash" }),
        piModel("stepfun/step-5-preview", { id: "step-5-preview" }),
      ],
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#16D6D2",
  },
  {
    name: "StepFun en",
    family: "stepfun",
    regionKey: "intl",
    providerKey: "cc-switch-step-fun-en",
    websiteUrl: "https://platform.stepfun.ai/step-plan",
    apiKeyUrl: "https://platform.stepfun.ai/interface-key",
    settingsConfig: {
      name: "StepFun en",
      baseUrl: "https://api.stepfun.ai/step_plan/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("stepfun/step-3.5-flash", {
          id: "step-3.5-flash-2603",
          name: "Step 3.5 Flash 2603",
        }),
        piModel("stepfun/step-3.5-flash", {
          id: "step-3.5-flash",
        }),
        piModel("stepfun/step-3.7-flash", { id: "step-3.7-flash" }),
        piModel("stepfun/step-5-preview", { id: "step-5-preview" }),
      ],
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#16D6D2",
  },
  {
    name: "StepFun Step Plan",
    providerKey: "cc-switch-step-fun-step-plan",
    websiteUrl: "https://platform.stepfun.com/docs/zh/step-plan/overview",
    apiKeyUrl: "https://platform.stepfun.com/interface-key",
    settingsConfig: {
      name: "StepFun Step Plan",
      baseUrl: "https://api.stepfun.com/step_plan/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("stepfun/step-3.5-flash", {
          id: "step-3.5-flash",
        }),
        piModel("stepfun/step-3.7-flash", { id: "step-3.7-flash" }),
        piModel("stepfun/step-5-preview", { id: "step-5-preview" }),
      ],
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#005AFF",
  },
  {
    name: "ModelScope",
    providerKey: "cc-switch-model-scope",
    websiteUrl: "https://modelscope.cn",
    apiKeyUrl: "https://modelscope.cn/my/myaccesstoken",
    settingsConfig: {
      name: "ModelScope",
      baseUrl: "https://api-inference.modelscope.cn/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("zai/glm-5.2", {
          id: "ZhipuAI/GLM-5.2",
        }),
      ],
    },
    category: "aggregator",
    icon: "modelscope",
    iconColor: "#624AFF",
  },
  {
    name: "KAT-Coder",
    providerKey: "cc-switch-kat-coder",
    websiteUrl: "https://console.streamlake.ai",
    apiKeyUrl: "https://console.streamlake.ai/console/api-key",
    settingsConfig: {
      name: "KAT-Coder",
      baseUrl:
        "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("streamlake/kat-coder-pro", {
          id: "KAT-Coder-Pro",
        }),
      ],
    },
    category: "cn_official",
    icon: "catcoder",
  },
  {
    name: "Longcat",
    providerKey: "cc-switch-longcat",
    websiteUrl: "https://longcat.chat/platform",
    apiKeyUrl: "https://longcat.chat/platform/api_keys",
    settingsConfig: {
      name: "Longcat",
      baseUrl: "https://api.longcat.chat/openai/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("longcat/longcat-2.0", {
          id: "LongCat-2.0",
        }),
      ],
    },
    category: "cn_official",
    icon: "longcat",
    iconColor: "#29E154",
  },
  {
    name: "MiniMax",
    family: "minimax",
    regionKey: "cn",
    providerKey: "cc-switch-mini-max",
    piBuiltIn: {
      provider: "minimax-cn",
      baseUrl: "https://api.minimaxi.com/anthropic",
    },
    websiteUrl: "https://platform.minimax.cn",
    apiKeyUrl: "https://platform.minimax.cn/subscribe/token-plan",
    settingsConfig: {
      name: "MiniMax",
      baseUrl: "https://api.minimax.cn/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("minimax/minimax-m3", {
          id: "MiniMax-M3",
          maxTokens: 131072,
        }),
      ],
    },
    category: "cn_official",
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
    providerKey: "cc-switch-mini-max-en",
    piBuiltIn: {
      provider: "minimax",
      baseUrl: "https://api.minimax.io/anthropic",
    },
    websiteUrl: "https://platform.minimax.io",
    apiKeyUrl: "https://platform.minimax.io/subscribe/coding-plan",
    settingsConfig: {
      name: "MiniMax en",
      baseUrl: "https://api.minimax.io/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("minimax/minimax-m3", {
          id: "MiniMax-M3",
          maxTokens: 131072,
        }),
      ],
    },
    category: "cn_official",
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
    icon: "minimax",
    iconColor: "#FF6B6B",
  },
  {
    name: "BaiLing",
    providerKey: "cc-switch-bai-ling",
    piBuiltIn: { provider: "ant-ling", baseUrl: "https://api.ant-ling.com/v1" },
    websiteUrl: "https://developer.ant-ling.com/zh-CN/docs/",
    apiKeyUrl: "https://chat.ant-ling.com/open",
    settingsConfig: {
      name: "BaiLing",
      baseUrl: "https://api.ant-ling.com/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("inclusionai/ling-2.6-1t", {
          id: "Ling-2.6-1T",
        }),
      ],
    },
    category: "cn_official",
    icon: "bailing",
  },
  {
    name: "Xiaomi MiMo",
    family: "xiaomi-mimo",
    planKey: "payg",
    providerKey: "cc-switch-xiaomi-mi-mo",
    piBuiltIn: { provider: "xiaomi", baseUrl: "https://api.xiaomimimo.com/v1" },
    websiteUrl: "https://platform.xiaomimimo.com",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/api-keys",
    settingsConfig: {
      name: "Xiaomi MiMo",
      baseUrl: "https://api.xiaomimimo.com/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        {
          ...piModel("xiaomi/mimo-v2.6-pro", { id: "mimo-v2.6-pro" }),
          compat: XIAOMI_THINKING_COMPAT,
        },
        {
          ...piModel("xiaomi/mimo-v2.6-flash", { id: "mimo-v2.6-flash" }),
          compat: XIAOMI_THINKING_COMPAT,
        },
        {
          ...piModel("xiaomi/mimo-v2.6-pro-ultraspeed", {
            id: "mimo-v2.6-pro-ultraspeed",
          }),
          compat: XIAOMI_THINKING_COMPAT,
        },
      ],
    },
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
  },
  {
    name: "Xiaomi MiMo Token Plan (China)",
    family: "xiaomi-mimo",
    planKey: "tokenPlan",
    providerKey: "cc-switch-xiaomi-mi-mo-token-plan-china",
    piBuiltIn: {
      provider: "xiaomi-token-plan-cn",
      baseUrl: "https://token-plan-cn.xiaomimimo.com/v1",
    },
    websiteUrl: "https://platform.xiaomimimo.com/#/token-plan",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/plan-manage",
    settingsConfig: {
      name: "Xiaomi MiMo Token Plan (China)",
      baseUrl: "https://token-plan-cn.xiaomimimo.com/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        {
          ...piModel("xiaomi/mimo-v2.6-pro", { id: "mimo-v2.6-pro" }),
          compat: XIAOMI_THINKING_COMPAT,
        },
        {
          ...piModel("xiaomi/mimo-v2.6-flash", { id: "mimo-v2.6-flash" }),
          compat: XIAOMI_THINKING_COMPAT,
        },
      ],
    },
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
  },
  {
    name: "OpenCode Go",
    providerKey: "cc-switch-open-code-go",
    piBuiltIn: {
      provider: "opencode-go",
      baseUrl: "https://opencode.ai/zen/go/v1",
    },
    websiteUrl: "https://opencode.ai/go",
    apiKeyUrl: "https://opencode.ai/go",
    settingsConfig: {
      name: "OpenCode Go",
      baseUrl: "https://opencode.ai/zen/go/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        {
          ...piModel("zai/glm-5.2", {
            id: "glm-5.2",
            name: "GLM 5.2",
            thinkingProfile: "openCodeGoGlm52",
          }),
          compat: { ...OPENAI_COMPLETIONS_COMPAT },
        },
        {
          ...piModel("moonshotai/kimi-k2.7-code", {
            id: "kimi-k2.7-code",
          }),
          compat: { ...OPENAI_COMPLETIONS_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("xiaomi/mimo-v2.5-pro", {
            id: "mimo-v2.5-pro",
          }),
          compat: { ...OPENAI_COMPLETIONS_COMPAT },
        },
      ],
    },
    category: "third_party",
    icon: "opencode",
    iconColor: "#211E1E",
  },
  {
    name: "CherryIN",
    providerKey: "cc-switch-cherry-in",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    settingsConfig: {
      name: "CherryIN",
      baseUrl: "https://open.cherryin.net",
      api: "anthropic-messages",
      apiKey: "",
      models: [
        piModel("anthropic/claude-sonnet-5.5", {
          id: "anthropic/claude-sonnet-5.5",
        }),
        piModel("anthropic/claude-opus-5.5", {
          id: "anthropic/claude-opus-5.5",
        }),
        piModel("anthropic/claude-fable-5.1", {
          id: "anthropic/claude-fable-5.1",
        }),
      ],
    },
    category: "aggregator",
    icon: "cherryin",
  },
  {
    name: "OpenRouter",
    providerKey: "cc-switch-open-router",
    piBuiltIn: {
      provider: "openrouter",
      baseUrl: "https://openrouter.ai/api/v1",
    },
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    settingsConfig: {
      name: "OpenRouter",
      baseUrl: "https://openrouter.ai/api",
      api: "anthropic-messages",
      apiKey: "",
      models: [
        piModel("anthropic/claude-sonnet-5.5", {
          id: "anthropic/claude-sonnet-5.5",
        }),
        piModel("anthropic/claude-opus-5.5", {
          id: "anthropic/claude-opus-5.5",
        }),
        piModel("anthropic/claude-fable-5.1", {
          id: "anthropic/claude-fable-5.1",
        }),
      ],
    },
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6566F1",
  },
  {
    name: "Novita AI",
    providerKey: "cc-switch-novita-ai",
    websiteUrl: "https://novita.ai",
    apiKeyUrl: "https://novita.ai",
    settingsConfig: {
      name: "Novita AI",
      baseUrl: "https://api.novita.ai/openai",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("zai/glm-5.3", { id: "zai-org/glm-5.3" }),
        piModel("zai/glm-5.3-flash", { id: "zai-org/glm-5.3-flash" }),
        piModel("moonshotai/kimi-k3", { id: "moonshotai/kimi-k3" }),
      ],
    },
    category: "aggregator",
    icon: "novita",
    iconColor: "#000000",
  },
  {
    name: "Nvidia",
    providerKey: "cc-switch-nvidia",
    piBuiltIn: {
      provider: "nvidia",
      baseUrl: "https://integrate.api.nvidia.com/v1",
    },
    websiteUrl: "https://build.nvidia.com",
    apiKeyUrl: "https://build.nvidia.com/settings/api-keys",
    settingsConfig: {
      name: "Nvidia",
      baseUrl: "https://integrate.api.nvidia.com/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("moonshotai/kimi-k3", {
          id: "moonshotai/kimi-k3",
        }),
        piModel("zai/glm-5.3", { id: "z-ai/glm-5.3" }),
        piModel("zai/glm-5.3-flash", { id: "z-ai/glm-5.3-flash" }),
      ],
    },
    category: "aggregator",
    icon: "nvidia",
    iconColor: "#000000",
  },
  {
    name: "AWS Bedrock",
    providerKey: "cc-switch-aws-bedrock",
    websiteUrl: "https://aws.amazon.com/bedrock/",
    settingsConfig: {
      name: "AWS Bedrock",
      baseUrl: "https://bedrock-runtime.us-east-1.amazonaws.com",
      api: "bedrock-converse-stream",
      apiKey: "",
      models: [
        // 5.5 不能关闭思考（Anthropic effort 文档，2026-10）
        piModel("anthropic/claude-opus-5.5", {
          id: "global.anthropic.claude-opus-5-5",
          thinkingProfile: "offUnsupportedXhighAndMax",
        }),
        piModel("anthropic/claude-sonnet-5.5", {
          id: "global.anthropic.claude-sonnet-5-5",
          thinkingProfile: "offUnsupportedXhighAndMax",
        }),
        piModel("anthropic/claude-haiku-4.5-20251001", {
          id: "global.anthropic.claude-haiku-4-5-20251001-v1:0",
        }),
        piModel("amazon/nova-pro", {
          id: "us.amazon.nova-pro-v1:0",
        }),
        piModel("meta/llama-4-maverick", {
          id: "us.meta.llama4-maverick-17b-instruct-v1:0",
        }),
        piModel("deepseek/deepseek-r1", {
          id: "us.deepseek.r1-v1:0",
        }),
      ],
    },
    category: "cloud_provider",
    icon: "aws",
    iconColor: "#FF9900",
  },
  // ===== 腾讯云 Token Plan（订阅套餐，产品线 1823/1300）=====
  // 与 claude/codex/opencode/hermes/openclaw 六 app 的腾讯预设同源：
  // 个人版国内走 api.lkeap.cloud.tencent.com/plan，其余走 tencentmaas.com/plan
  // 域族；两站 Key 不互通，双地域候选（国内→新加坡、国际→广州）仅在企业
  // 套餐存在。Pi 预设只携带默认地域 baseUrl（PiProviderPreset 无
  // endpointCandidates 字段），需要第二地域的用户在 UI 中手动改 baseUrl。
  {
    name: "Tencent Token Plan",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "cn",
    providerKey: "cc-switch-tencent-token-plan",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan",
    settingsConfig: {
      name: "Tencent Token Plan",
      baseUrl: "https://api.lkeap.cloud.tencent.com/plan/v3",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("tencent/tokenplan-auto", { id: "tc-code-latest" }),
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-202605",
            name: "DeepSeek V4 Flash Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-202606",
            name: "DeepSeek V4 Pro Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        piModel("minimax/minimax-m2.7", { id: "minimax-m2.7" }),
        piModel("zai/glm-5.2", { id: "glm-5.2" }),
        piModel("tencent/hy3", { id: "hy3" }),
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#00A4FF",
  },
  {
    name: "Tencent Token Plan (Intl)",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "intl",
    providerKey: "cc-switch-tencent-token-plan-intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan",
    settingsConfig: {
      name: "Tencent Token Plan (Intl)",
      baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("tencent/tokenplan-auto", { id: "auto" }),
        piModel("zai/glm-5.2", { id: "glm-5.2" }),
        piModel("moonshotai/kimi-k2.6", { id: "kimi-k2.6" }),
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-202606",
            name: "DeepSeek V4 Pro Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-202605",
            name: "DeepSeek V4 Flash Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        piModel("minimax/minimax-m3", { id: "minimax-m3" }),
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#00A4FF",
  },
  {
    name: "Tencent Token Plan Enterprise Pro",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "cn",
    providerKey: "cc-switch-tencent-token-plan-enterprise-pro",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      name: "Tencent Token Plan Enterprise Pro",
      baseUrl: "https://tokenhub.tencentmaas.com/plan/v3",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("tencent/tokenplan-auto", { id: "auto" }),
        piModel("zai/glm-5.3", { id: "glm-5.3" }),
        piModel("zai/glm-5.2", { id: "glm-5.2" }),
        piModel("zai/glm-5", { id: "glm-5" }),
        piModel("zai/glm-5.1", { id: "glm-5.1" }),
        piModel("zai/glm-5-turbo", { id: "glm-5-turbo" }),
        piModel("moonshotai/kimi-k2.7-code", {
          id: "kimi-k2.7-code",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k2.7-code-highspeed", {
          id: "kimi-k2.7-code-highspeed",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k2.6", { id: "kimi-k2.6" }),
        piModel("minimax/minimax-m2.7", { id: "minimax-m2.7" }),
        piModel("minimax/minimax-m3", { id: "minimax-m3" }),
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-0731",
            name: "DeepSeek V4 Flash 0731 GA",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-0813",
            name: "DeepSeek V4 Pro 0813 GA",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-202605",
            name: "DeepSeek V4 Flash Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-202606",
            name: "DeepSeek V4 Pro Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#00A4FF",
  },
  {
    name: "Tencent Token Plan Enterprise Pro (Intl)",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "intl",
    providerKey: "cc-switch-tencent-token-plan-enterprise-pro-intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    settingsConfig: {
      name: "Tencent Token Plan Enterprise Pro (Intl)",
      baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("tencent/tokenplan-auto", { id: "auto" }),
        piModel("zai/glm-5.3", { id: "glm-5.3" }),
        piModel("zai/glm-5.2", { id: "glm-5.2" }),
        piModel("minimax/minimax-m3", { id: "minimax-m3" }),
        piModel("moonshotai/kimi-k2.7-code", {
          id: "kimi-k2.7-code",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k2.7-code-highspeed", {
          id: "kimi-k2.7-code-highspeed",
          thinkingProfile: "offUnsupported",
        }),
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-0731",
            name: "DeepSeek V4 Flash 0731 GA",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-0813",
            name: "DeepSeek V4 Pro 0813 GA",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-202605",
            name: "DeepSeek V4 Flash Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-202606",
            name: "DeepSeek V4 Pro Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#00A4FF",
  },
  {
    name: "Tencent Token Plan Enterprise Lite",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "cn",
    providerKey: "cc-switch-tencent-token-plan-enterprise-lite",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      name: "Tencent Token Plan Enterprise Lite",
      baseUrl: "https://tokenhub.tencentmaas.com/plan/v3",
      api: "openai-completions",
      apiKey: "",
      models: [piModel("tencent/tokenplan-auto", { id: "auto" })],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#00A4FF",
  },
  {
    name: "Tencent Token Plan Enterprise Lite (Intl)",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "intl",
    providerKey: "cc-switch-tencent-token-plan-enterprise-lite-intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    settingsConfig: {
      name: "Tencent Token Plan Enterprise Lite (Intl)",
      baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
      api: "openai-completions",
      apiKey: "",
      models: [piModel("tencent/tokenplan-auto", { id: "auto" })],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#00A4FF",
  },
  // ===== 腾讯云 TokenHub（按量付费 API 市场，产品线 1823/1300，/v1 端点）=====
  // 与 Token Plan 订阅线（/plan 端点）是两条独立产品线：Key 不互通、端点不同。
  // 模型清单取自官方「语言模型」列表（国内 130051 / 国际 78934），只收录支持
  // Function Calling 的通用语言模型；翻译（hy-mt2）、角色扮演（hy-role）等
  // 专用模型不收录（无法用于编码 agent）。
  {
    name: "Tencent TokenHub",
    family: "tencent",
    planKey: "payg",
    regionKey: "cn",
    providerKey: "cc-switch-tencent-tokenhub",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/apikey",
    settingsConfig: {
      name: "Tencent TokenHub",
      baseUrl: "https://tokenhub.tencentmaas.com/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("tencent/hy4-preview", { id: "hy4-preview" }),
        piModel("tencent/hy3", { id: "hy3" }),
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-202605",
            name: "DeepSeek V4 Flash Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-202606",
            name: "DeepSeek V4 Pro Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash-vision-exp", {
            id: "deepseek/deepseek-v4-flash-vision-exp",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-0731",
            name: "DeepSeek V4 Flash 0731 GA",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-0813",
            name: "DeepSeek V4 Pro 0813 GA",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        piModel("zai/glm-5.3-flash", { id: "glm-5.3-flash" }),
        piModel("zai/glm-5.3", { id: "glm-5.3" }),
        piModel("zai/glm-5.2", { id: "glm-5.2" }),
        piModel("zai/glm-5.1", { id: "glm-5.1" }),
        piModel("zai/glm-5v-turbo", { id: "glm-5v-turbo" }),
        piModel("zai/glm-5-turbo", { id: "glm-5-turbo" }),
        piModel("zai/glm-5", { id: "glm-5" }),
        piModel("moonshotai/kimi-k2.7-code-highspeed", {
          id: "kimi-k2.7-code-highspeed",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k3", { id: "kimi-k3" }),
        piModel("moonshotai/kimi-k2.7-code", {
          id: "kimi-k2.7-code",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k2.6", { id: "kimi-k2.6" }),
        piModel("moonshotai/kimi-k2.5", { id: "kimi-k2.5" }),
        piModel("minimax/minimax-m3", { id: "minimax-m3" }),
        piModel("minimax/minimax-m2.7", { id: "minimax-m2.7" }),
        piModel("xiaomi/mimo-v2.5-pro", { id: "mimo-v2.5-pro" }),
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#00A4FF",
  },
  {
    name: "Tencent TokenHub (Intl)",
    family: "tencent",
    planKey: "payg",
    regionKey: "intl",
    providerKey: "cc-switch-tencent-tokenhub-intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/apikey",
    settingsConfig: {
      name: "Tencent TokenHub (Intl)",
      baseUrl: "https://tokenhub-intl.tencentcloudmaas.com/v1",
      api: "openai-completions",
      apiKey: "",
      models: [
        piModel("tencent/hy4-preview", { id: "hy4-preview" }),
        piModel("tencent/hy3", { id: "hy3" }),
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-202605",
            name: "DeepSeek V4 Flash Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-202606",
            name: "DeepSeek V4 Pro Official",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash-vision-exp", {
            id: "deepseek/deepseek-v4-flash-vision-exp",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash-0731",
            name: "DeepSeek V4 Flash 0731 GA",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro-0813",
            name: "DeepSeek V4 Pro 0813 GA",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-flash", {
            id: "deepseek-v4-flash",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v4-pro", {
            id: "deepseek-v4-pro",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        {
          ...piModel("deepseek/deepseek-v3.2", {
            id: "deepseek-v3.2",
            thinkingProfile: "deepseekV4",
          }),
          compat: { ...TENCENT_DEEPSEEK_THINKING_COMPAT },
        },
        piModel("zai/glm-5.3", { id: "glm-5.3" }),
        piModel("zai/glm-5.3-flash", { id: "glm-5.3-flash" }),
        piModel("zai/glm-5.2", { id: "glm-5.2" }),
        piModel("zai/glm-5", { id: "glm-5" }),
        piModel("zai/glm-5-turbo", { id: "glm-5-turbo" }),
        piModel("zai/glm-5v-turbo", { id: "glm-5v-turbo" }),
        piModel("zai/glm-5.1", { id: "glm-5.1" }),
        piModel("moonshotai/kimi-k3", { id: "kimi-k3" }),
        piModel("moonshotai/kimi-k2.7-code-highspeed", {
          id: "kimi-k2.7-code-highspeed",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k2.7-code", {
          id: "kimi-k2.7-code",
          thinkingProfile: "offUnsupported",
        }),
        piModel("moonshotai/kimi-k2.6", { id: "kimi-k2.6" }),
        piModel("moonshotai/kimi-k2.5", { id: "kimi-k2.5" }),
        piModel("minimax/minimax-m3", { id: "minimax-m3" }),
        piModel("minimax/minimax-m2.7", { id: "minimax-m2.7" }),
        piModel("xiaomi/mimo-v2.5-pro", { id: "mimo-v2.5-pro" }),
      ],
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#00A4FF",
  },
  {
    name: "模力方舟",
    providerKey: "cc-switch-moark",
    websiteUrl: "https://moark.com",
    apiKeyUrl: "https://moark.com/dashboard/tokens",
    settingsConfig: {
      name: "模力方舟",
      baseUrl: "https://api.moark.com/v1",
      api: "openai-completions",
      apiKey: "",
      // 聚合网关，上游思考档位语义未逐一核对，交给 Pi 原生默认行为（{}）
      models: [
        piModel("deepseek/deepseek-v4-flash", {
          id: "deepseek-v4-flash-0731",
        }),
        piModel("deepseek/deepseek-v4-pro", { id: "DeepSeek-V4-Pro" }),
        piModel("zai/glm-5.3", { id: "GLM-5.3" }),
        piModel("moonshotai/kimi-k2.7-code", { id: "Kimi-K2.7-Code" }),
        piModel("qwen/qwen3.8-max", { id: "qwen3.8-max" }),
      ],
    },
    category: "aggregator",
    icon: "moark",
  },
];

function materializeVerifiedThinkingProfiles(
  preset: PiProviderPreset,
): PiProviderPreset {
  return {
    ...preset,
    settingsConfig: {
      ...preset.settingsConfig,
      models: preset.settingsConfig.models.map((model) => {
        const reference = getPiModelCatalogReference(model);
        if (!reference) return model;
        const resolved = reference.presetThinkingProfileId
          ? getPiThinkingProfile(reference.presetThinkingProfileId)
          : resolvePiThinkingProfile({
              catalogKey: reference.catalogKey,
              api: preset.settingsConfig.api,
            });
        return {
          ...model,
          ...(model.reasoning ? { thinkingLevelMap: resolved?.map ?? {} } : {}),
          ...(resolved?.modelCompat
            ? {
                compat: {
                  ...model.compat,
                  ...resolved.modelCompat,
                },
              }
            : {}),
        };
      }),
    },
  };
}

export const piProviderPresets = piProviderPresetDefinitions.map(
  materializeVerifiedThinkingProfiles,
);

/** 只写 key 的内置供应商条目没有 baseUrl，按 ID 找回地址，给用量判断用。 */
export function piBuiltInBaseUrl(providerKey: string): string | undefined {
  return piProviderPresets.find(
    (preset) => preset.piBuiltIn?.provider === providerKey,
  )?.piBuiltIn?.baseUrl;
}
