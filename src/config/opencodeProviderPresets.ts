import type { ProviderCategory, OpenCodeProviderConfig } from "../types";
import type { PresetTheme, TemplateValueConfig } from "./claudeProviderPresets";
import type { PresetFamilyFields } from "./presetFamilies";

export interface OpenCodeProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string; // i18n key for localized display name
  websiteUrl: string;
  apiKeyUrl?: string;
  settingsConfig: OpenCodeProviderConfig;
  isOfficial?: boolean;
  partnerPromotionKey?: string;
  category?: ProviderCategory;
  templateValues?: Record<string, TemplateValueConfig>;
  theme?: PresetTheme;
  icon?: string;
  iconColor?: string;
  isCustomTemplate?: boolean;
}

export const opencodeNpmPackages = [
  { value: "@ai-sdk/openai", label: "OpenAI Responses" },
  { value: "@ai-sdk/openai-compatible", label: "OpenAI Compatible" },
  { value: "@ai-sdk/anthropic", label: "Anthropic" },
  { value: "@ai-sdk/amazon-bedrock", label: "Amazon Bedrock" },
  { value: "@ai-sdk/google", label: "Google (Gemini)" },
] as const;

export interface PresetModelVariant {
  id: string;
  name?: string;
  contextLimit?: number;
  outputLimit?: number;
  modalities?: { input: string[]; output: string[] };
  options?: Record<string, unknown>;
  variants?: Record<string, Record<string, unknown>>;
}

export const OPENCODE_PRESET_MODEL_VARIANTS: Record<
  string,
  PresetModelVariant[]
> = {
  "@ai-sdk/openai-compatible": [
    {
      id: "MiniMax-M3",
      name: "MiniMax M3",
      contextLimit: 1000000,
      outputLimit: 131072,
      modalities: { input: ["text", "image"], output: ["text"] },
    },
    {
      id: "MiniMax-M2.7",
      name: "MiniMax M2.7",
      contextLimit: 204800,
      outputLimit: 131072,
      modalities: { input: ["text"], output: ["text"] },
    },
    {
      id: "glm-5.1",
      name: "GLM 5.1",
      contextLimit: 204800,
      outputLimit: 131072,
      modalities: { input: ["text"], output: ["text"] },
    },
    {
      id: "kimi-k2.6",
      name: "Kimi K2.6",
      contextLimit: 262144,
      outputLimit: 262144,
      modalities: { input: ["text", "image", "video"], output: ["text"] },
    },
    {
      id: "kimi-k3",
      name: "Kimi K3",
      contextLimit: 1048576,
      outputLimit: 131072,
      modalities: { input: ["text", "image", "video"], output: ["text"] },
    },
    {
      id: "step-3.5-flash-2603",
      name: "Step 3.5 Flash 2603",
      contextLimit: 262144,
    },
    {
      id: "step-3.5-flash",
      name: "Step 3.5 Flash",
      contextLimit: 262144,
    },
  ],
  "@ai-sdk/google": [
    {
      id: "gemini-2.5-flash-lite",
      name: "Gemini 2.5 Flash Lite",
      contextLimit: 1048576,
      outputLimit: 65536,
      modalities: {
        input: ["text", "image", "pdf", "video", "audio"],
        output: ["text"],
      },
      variants: {
        auto: {
          thinkingConfig: { includeThoughts: true, thinkingBudget: -1 },
        },
        "no-thinking": { thinkingConfig: { thinkingBudget: 0 } },
      },
    },
    {
      id: "gemini-3.6-flash",
      name: "Gemini 3.6 Flash",
      contextLimit: 1048576,
      outputLimit: 65536,
      modalities: {
        input: ["text", "image", "pdf", "video", "audio"],
        output: ["text"],
      },
      variants: {
        minimal: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "minimal" },
        },
        low: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "low" },
        },
        medium: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "medium" },
        },
        high: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "high" },
        },
      },
    },
  ],
  "@ai-sdk/openai": [
    {
      id: "gpt-5.6-sol",
      name: "GPT-5.6 Sol",
      contextLimit: 400000,
      outputLimit: 128000,
      modalities: { input: ["text", "image"], output: ["text"] },
      variants: {
        low: {
          reasoningEffort: "low",
          reasoningSummary: "auto",
          textVerbosity: "medium",
        },
        medium: {
          reasoningEffort: "medium",
          reasoningSummary: "auto",
          textVerbosity: "medium",
        },
        high: {
          reasoningEffort: "high",
          reasoningSummary: "auto",
          textVerbosity: "medium",
        },
        xhigh: {
          reasoningEffort: "xhigh",
          reasoningSummary: "auto",
          textVerbosity: "medium",
        },
      },
    },
  ],
  "@ai-sdk/amazon-bedrock": [
    {
      id: "global.anthropic.claude-opus-5-5",
      name: "Claude Opus 5.5",
      contextLimit: 1000000,
      outputLimit: 128000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
    },
    {
      id: "global.anthropic.claude-sonnet-5-5",
      name: "Claude Sonnet 5.5",
      contextLimit: 1000000,
      outputLimit: 128000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
    },
    {
      id: "global.anthropic.claude-haiku-4-5-20251001-v1:0",
      name: "Claude Haiku 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
    },
    {
      id: "us.amazon.nova-pro-v1:0",
      name: "Amazon Nova Pro",
      contextLimit: 300000,
      outputLimit: 5000,
      modalities: { input: ["text", "image"], output: ["text"] },
    },
    {
      id: "us.meta.llama4-maverick-17b-instruct-v1:0",
      name: "Meta Llama 4 Maverick",
      contextLimit: 131072,
      outputLimit: 131072,
      modalities: { input: ["text"], output: ["text"] },
    },
    {
      id: "us.deepseek.r1-v1:0",
      name: "DeepSeek R1",
      contextLimit: 131072,
      outputLimit: 131072,
      modalities: { input: ["text"], output: ["text"] },
    },
  ],
  "@ai-sdk/anthropic": [
    {
      id: "claude-sonnet-4-5-20250929",
      name: "Claude Sonnet 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { effort: "low" },
        medium: { effort: "medium" },
        high: { effort: "high" },
      },
    },
    {
      id: "claude-opus-4-5-20251101",
      name: "Claude Opus 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { thinking: { budgetTokens: 5000, type: "enabled" } },
        medium: { thinking: { budgetTokens: 13000, type: "enabled" } },
        high: { thinking: { budgetTokens: 18000, type: "enabled" } },
      },
    },
    {
      id: "claude-opus-5-5",
      name: "Claude Opus 5.5",
      contextLimit: 1000000,
      outputLimit: 128000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { effort: "low" },
        medium: { effort: "medium" },
        high: { effort: "high" },
        max: { effort: "max" },
      },
    },
    {
      id: "claude-sonnet-5-5",
      name: "Claude Sonnet 5.5",
      contextLimit: 1000000,
      outputLimit: 128000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { effort: "low" },
        medium: { effort: "medium" },
        high: { effort: "high" },
        max: { effort: "max" },
      },
    },
    {
      id: "claude-opus-5",
      name: "Claude Opus 5",
      contextLimit: 1000000,
      outputLimit: 128000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { effort: "low" },
        medium: { effort: "medium" },
        high: { effort: "high" },
        max: { effort: "max" },
      },
    },
    {
      id: "claude-haiku-4-5-20251001",
      name: "Claude Haiku 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
    },
    {
      id: "gemini-claude-opus-4-5-thinking",
      name: "Antigravity - Claude Opus 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { effort: "low" },
        medium: { effort: "medium" },
        high: { effort: "high" },
      },
    },
    {
      id: "gemini-claude-sonnet-4-5-thinking",
      name: "Antigravity - Claude Sonnet 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { thinking: { budgetTokens: 5000, type: "enabled" } },
        medium: { thinking: { budgetTokens: 13000, type: "enabled" } },
        high: { thinking: { budgetTokens: 18000, type: "enabled" } },
      },
    },
  ],
};

export const opencodeProviderPresets: OpenCodeProviderPreset[] = [
  {
    name: "Kimi",
    family: "kimi",
    planKey: "payg",
    regionKey: "cn",
    websiteUrl: "https://platform.kimi.com",
    apiKeyUrl: "https://platform.kimi.com/console/api-keys",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Kimi",
      options: {
        baseURL: "https://api.moonshot.cn/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "kimi-k2.7-code": { name: "Kimi K2.7 Code", reasoning: true },
        "kimi-k3": { name: "Kimi K3", reasoning: true },
        "kimi-k2.7-code-highspeed": {
          name: "Kimi K2.7 Code HighSpeed",
          reasoning: true,
          limit: { context: 262144, output: 262144 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "kimi-k2.6": {
          name: "Kimi K2.6",
          reasoning: true,
          limit: { context: 262144, output: 262144 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    templateValues: {
      baseURL: {
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
      npm: "@ai-sdk/openai-compatible",
      name: "Kimi",
      options: {
        baseURL: "https://api.moonshot.ai/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "kimi-k2.7-code": { name: "Kimi K2.7 Code", reasoning: true },
        "kimi-k3": { name: "Kimi K3", reasoning: true },
        "kimi-k2.7-code-highspeed": {
          name: "Kimi K2.7 Code HighSpeed",
          reasoning: true,
          limit: { context: 262144, output: 262144 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "kimi-k2.6": {
          name: "Kimi K2.6",
          reasoning: true,
          limit: { context: 262144, output: 262144 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    templateValues: {
      baseURL: {
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
  },
  {
    name: "Kimi For Coding",
    family: "kimi",
    planKey: "coding",
    regionKey: "cn",
    websiteUrl: "https://www.kimi.com/code/",
    apiKeyUrl: "https://platform.kimi.com/console/api-keys",
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "Kimi For Coding",
      options: {
        baseURL: "https://api.kimi.com/coding/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "kimi-for-coding": { name: "Kimi For Coding", reasoning: true },
      },
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    templateValues: {
      baseURL: {
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
      npm: "@ai-sdk/anthropic",
      name: "Kimi For Coding",
      options: {
        baseURL: "https://api.kimi.ai/coding/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "kimi-for-coding": { name: "Kimi For Coding", reasoning: true },
      },
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
    templateValues: {
      baseURL: {
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
  },

  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Qiniu",
      options: {
        baseURL: "https://api.qnaigc.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "gpt-6-astra": {
          name: "GPT-6 Astra",
          reasoning: true,
          limit: { context: 1050000, output: 128000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "moonshotai/kimi-k3": {
          name: "Kimi K3",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "z-ai/glm-5.3": {
          name: "GLM-5.3",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text"], output: ["text"] },
        },
        "z-ai/glm-5.3-flash": {
          name: "GLM-5.3-Flash",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
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
  },
  {
    name: "PPIO",
    websiteUrl: "https://ppio.com",
    apiKeyUrl: "https://ppio.com/settings/key-management",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "PPIO",
      options: {
        baseURL: "https://api.ppio.com/openai/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "deepseek/deepseek-v4-flash-0731": {
          name: "Deepseek V4 Flash 0731",
          reasoning: true,
        },
      },
    },
    category: "aggregator",
    icon: "ppio",
    iconColor: "#2874FF",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
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
      npm: "@ai-sdk/openai-compatible",
      name: "火山 Agent Plan",
      options: {
        baseURL: "https://ark.cn-beijing.volces.com/api/plan/v3",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "ark-code-latest": {
          name: "Ark Code Latest",
        },
      },
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
  },
  {
    name: "火山 Coding Plan",
    family: "volcengine",
    planKey: "codingPlan",
    websiteUrl: "https://www.volcengine.com/activity/codingplan",
    apiKeyUrl: "https://www.volcengine.com/activity/codingplan",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "火山 Coding Plan",
      options: {
        baseURL: "https://ark.cn-beijing.volces.com/api/coding/v3",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "ark-code-latest": {
          name: "Ark Code Latest",
        },
      },
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
  },
  {
    name: "BytePlus",
    websiteUrl: "https://www.byteplus.com/en/product/modelark",
    apiKeyUrl: "https://www.byteplus.com/en/product/modelark",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "BytePlus",
      options: {
        baseURL: "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "ark-code-latest": {
          name: "Ark Code Latest",
        },
      },
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
      npm: "@ai-sdk/openai-compatible",
      name: "Volcengine Doubao",
      options: {
        baseURL: "https://ark.cn-beijing.volces.com/api/v3",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "doubao-seed-2-1-pro-260915": {
          name: "Doubao Seed 2.1 Pro",
          reasoning: true,
        },
      },
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
  },
  {
    name: "AtlasCloud",
    websiteUrl: "https://www.atlascloud.ai/console/coding-plan",
    apiKeyUrl: "https://www.atlascloud.ai/console/coding-plan",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "AtlasCloud",
      options: {
        baseURL: "https://api.atlascloud.ai/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "zai-org/glm-5.2": { name: "GLM 5.2", reasoning: true },
      },
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
  },
  {
    name: "DeepSeek",
    websiteUrl: "https://platform.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      options: {
        baseURL: "https://api.deepseek.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "deepseek-v4-pro": {
          name: "DeepSeek V4 Pro",
          reasoning: true,
          limit: { context: 1000000, output: 384000 },
          modalities: { input: ["text"], output: ["text"] },
        },
        "deepseek-flash": {
          name: "DeepSeek V4.1 Flash",
          reasoning: true,
          limit: { context: 1000000, output: 384000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
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
  },
  {
    name: "Zhipu GLM",
    family: "zhipu",
    regionKey: "cn",
    websiteUrl: "https://open.bigmodel.cn",
    apiKeyUrl: "https://www.bigmodel.cn/claude-code",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Zhipu GLM",
      options: {
        baseURL: "https://open.bigmodel.cn/api/coding/paas/v4",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "glm-5.3": { name: "GLM-5.3", reasoning: true },
        "glm-5.3-flash": {
          name: "GLM-5.3-Flash",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
    templateValues: {
      baseURL: {
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
  },
  {
    name: "Zhipu GLM en",
    family: "zhipu",
    regionKey: "intl",
    websiteUrl: "https://z.ai",
    apiKeyUrl: "https://z.ai/subscribe",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Zhipu GLM en",
      options: {
        baseURL: "https://api.z.ai/api/coding/paas/v4",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "glm-5.3": { name: "GLM-5.3", reasoning: true },
        "glm-5.3-flash": {
          name: "GLM-5.3-Flash",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
    templateValues: {
      baseURL: {
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
  },
  {
    // 腾讯云 Token Plan 个人版（1823/130060，2026-08-21 版）：通用 + Hy 两
    // 系列共用同一端点与 API Key，模型合并两系列；Auto 智能路由的调用 ID
    // 是 tc-code-latest。端点 OpenAI 兼容（官方快速入门 1823/130119 未发
    // OpenCode 专属接入页，按工具无关的 /plan/v3 + 阵容照文档收录）。
    // kimi-k2.5 官方标注 2026-08-31 下线不收；minimax-m2.5 不在套餐文档
    // 表内、但 /plan/v3/models 收录且真 Key 实测可用（2026-08-31），照实收
    name: "Tencent Token Plan",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Tencent Token Plan",
      options: {
        baseURL: "https://api.lkeap.cloud.tencent.com/plan/v3",
        apiKey: "",
      },
      models: {
        "tc-code-latest": { name: "Auto" },
        "deepseek-v4-flash-202605": {
          name: "DeepSeek V4 Flash",
          reasoning: true,
        },
        "deepseek-v4-pro-202606": { name: "DeepSeek V4 Pro", reasoning: true },
        "minimax-m2.7": { name: "MiniMax M2.7", reasoning: true },
        "glm-5.2": { name: "GLM-5.2", reasoning: true },
        hy3: { name: "Hy3", reasoning: true },
      },
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseURL: {
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
  },
  {
    // 国际站（新加坡地域）个人版（intl 1300/81315，2026-08-20 版）：Auto
    // 调用 ID 是 auto（≠国内个人版 tc-code-latest），阵容与国内不同（无
    // GLM-5/5.1/Hy3，多 GLM-5.2/MiniMax-M3）。端点用国际站文档钦定的
    // tencentcloudmaas.com 域；Key 按站独立不跨站通用
    name: "Tencent Token Plan (Intl)",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Tencent Token Plan (Intl)",
      options: {
        baseURL: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        apiKey: "",
      },
      models: {
        auto: { name: "Auto" },
        "glm-5.2": { name: "GLM-5.2", reasoning: true },
        "kimi-k2.6": { name: "Kimi K2.6", reasoning: true },
        "deepseek-v4-pro-202606": { name: "DeepSeek V4 Pro", reasoning: true },
        "deepseek-v4-flash-202605": {
          name: "DeepSeek V4 Flash",
          reasoning: true,
        },
        "minimax-m3": { name: "MiniMax M3", reasoning: true },
      },
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseURL: {
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
  },
  {
    // Token Plan 企业版专业套餐（1823/130659，2026-08-25 版，广州地域）：
    // kimi-k2.5 官方标注 2026-08-31 下线不收；minimax-m2.5 型号列表已除名
    // 但真 Key 实测仍可用（2026-08-31），照实收录
    name: "Tencent Token Plan Enterprise Pro",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Tencent Token Plan Enterprise Pro",
      options: {
        baseURL: "https://tokenhub.tencentmaas.com/plan/v3",
        apiKey: "",
      },
      models: {
        auto: { name: "Auto" },
        "glm-5.3": { name: "GLM-5.3", reasoning: true },
        "glm-5.2": { name: "GLM-5.2", reasoning: true },
        "glm-5": { name: "GLM-5", reasoning: true },
        "glm-5.1": { name: "GLM-5.1", reasoning: true },
        "glm-5-turbo": { name: "GLM-5 Turbo", reasoning: true },
        "kimi-k2.7-code": { name: "Kimi K2.7 Code", reasoning: true },
        "kimi-k2.7-code-highspeed": {
          name: "Kimi K2.7 Code HighSpeed",
          reasoning: true,
        },
        "kimi-k2.6": { name: "Kimi K2.6", reasoning: true },
        "minimax-m2.7": { name: "MiniMax M2.7", reasoning: true },
        "minimax-m3": { name: "MiniMax M3", reasoning: true },
        "deepseek-v4-flash": { name: "DeepSeek V4 Flash", reasoning: true },
        "deepseek-v4-pro": { name: "DeepSeek V4 Pro", reasoning: true },
        "deepseek-v4-flash-0731": {
          name: "DeepSeek V4 Flash 0731 GA",
          reasoning: true,
        },
        "deepseek-v4-pro-0813": {
          name: "DeepSeek V4 Pro 0813 GA",
          reasoning: true,
        },
        "deepseek-v4-flash-202605": {
          name: "DeepSeek V4 Flash Official",
          reasoning: true,
        },
        "deepseek-v4-pro-202606": {
          name: "DeepSeek V4 Pro Official",
          reasoning: true,
        },
      },
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseURL: {
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
      npm: "@ai-sdk/openai-compatible",
      name: "Tencent Token Plan Enterprise Pro (Intl)",
      options: {
        baseURL: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        apiKey: "",
      },
      models: {
        auto: { name: "Auto" },
        "glm-5.3": { name: "GLM-5.3", reasoning: true },
        "glm-5.2": { name: "GLM-5.2", reasoning: true },
        "minimax-m3": { name: "MiniMax M3", reasoning: true },
        "kimi-k2.7-code": { name: "Kimi K2.7 Code", reasoning: true },
        "kimi-k2.7-code-highspeed": {
          name: "Kimi K2.7 Code HighSpeed",
          reasoning: true,
        },
        "deepseek-v4-flash": { name: "DeepSeek V4 Flash", reasoning: true },
        "deepseek-v4-pro": { name: "DeepSeek V4 Pro", reasoning: true },
        "deepseek-v4-flash-0731": {
          name: "DeepSeek V4 Flash 0731 GA",
          reasoning: true,
        },
        "deepseek-v4-pro-0813": {
          name: "DeepSeek V4 Pro 0813 GA",
          reasoning: true,
        },
        "deepseek-v4-flash-202605": {
          name: "DeepSeek V4 Flash Official",
          reasoning: true,
        },
        "deepseek-v4-pro-202606": {
          name: "DeepSeek V4 Pro Official",
          reasoning: true,
        },
      },
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseURL: {
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
      npm: "@ai-sdk/openai-compatible",
      name: "Tencent Token Plan Enterprise Lite",
      options: {
        baseURL: "https://tokenhub.tencentmaas.com/plan/v3",
        apiKey: "",
      },
      models: {
        auto: { name: "Auto" },
      },
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseURL: {
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
      npm: "@ai-sdk/openai-compatible",
      name: "Tencent Token Plan Enterprise Lite (Intl)",
      options: {
        baseURL: "https://tokenhub-intl.tencentcloudmaas.com/plan/v3",
        apiKey: "",
      },
      models: {
        auto: { name: "Auto" },
      },
    },
    category: "cn_official",
    icon: "tencent",
    iconColor: "#0052D9",
    templateValues: {
      baseURL: {
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
  },
  {
    // 千帆 Token Plan 个人版（2026-07-13 起替代 Coding Plan 发售）：官方
    // OpenCode 接入页确认 /v2/tokenplan/personal + @ai-sdk/openai-compatible；
    // 阵容=Token Plan 个人版文档 2026-09-30 版（cloud.baidu.com/doc/qianfan/s/Dmrabu8b6）
    name: "Baidu Qianfan Token Plan",
    websiteUrl: "https://cloud.baidu.com/product/codingplan.html",
    apiKeyUrl: "https://console.bce.baidu.com/qianfan/resource/token-plan",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Baidu Qianfan Token Plan",
      options: {
        baseURL: "https://qianfan.baidubce.com/v2/tokenplan/personal",
        apiKey: "",
      },
      models: {
        "deepseek-v4-pro": { name: "DeepSeek V4 Pro", reasoning: true },
        "deepseek-v4.1-flash": {
          name: "DeepSeek V4.1 Flash",
          reasoning: true,
        },
        "deepseek-v4-pro-0813": {
          name: "DeepSeek V4 Pro 0813",
          reasoning: true,
        },
        "deepseek-v4-flash-0731": {
          name: "DeepSeek V4 Flash 0731",
          reasoning: true,
        },
        "glm-5.3": { name: "GLM-5.3", reasoning: true },
        "glm-5.3-flash": { name: "GLM-5.3 Flash", reasoning: true },
        "glm-5.2": { name: "GLM-5.2", reasoning: true },
        "glm-5.1": { name: "GLM-5.1", reasoning: true },
      },
    },
    category: "cn_official",
    icon: "baidu",
    iconColor: "#2932E1",
    templateValues: {
      baseURL: {
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
  },
  {
    name: "千问AI平台",
    family: "qianwen",
    planKey: "payg",
    websiteUrl: "https://platform.qianwenai.com/",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "千问AI平台",
      options: {
        baseURL: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "qwen3.8-max": {
          name: "Qwen3.8 Max",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
          modalities: { input: ["text", "image", "video"], output: ["text"] },
        },
        "qwen3.8-flash": {
          name: "Qwen3.8 Flash",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
          modalities: { input: ["text", "image", "video"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
    templateValues: {
      baseURL: {
        label: "Base URL",
        placeholder: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        defaultValue: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
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
      npm: "@ai-sdk/anthropic",
      name: "千问AI平台 Token Plan",
      options: {
        baseURL:
          "https://token-plan.cn-beijing.maas.aliyuncs.com/apps/anthropic/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "qwen3.8-max": {
          name: "Qwen3.8 Max",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
          modalities: { input: ["text", "image", "video"], output: ["text"] },
        },
        "qwen3.8-flash": {
          name: "Qwen3.8 Flash",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
          modalities: { input: ["text", "image", "video"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
    templateValues: {
      baseURL: {
        label: "Base URL",
        placeholder:
          "https://token-plan.cn-beijing.maas.aliyuncs.com/apps/anthropic/v1",
        defaultValue:
          "https://token-plan.cn-beijing.maas.aliyuncs.com/apps/anthropic/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
  },
  // ===== QwenCloud（DashScope 国际站）=====
  // 与上面国内条目是两套独立站点：域名、控制台、密钥互不通用。
  // 按量付费走 OpenAI 兼容层（/compatible-mode/v1）；Token Plan 官方给的是
  // Anthropic 协议地址，且比 Claude Code 的多一段 /v1（AI SDK anthropic 惯例）。
  {
    name: "QwenCloud",
    family: "qwencloud",
    planKey: "payg",
    websiteUrl: "https://home.qwencloud.com/",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "QwenCloud",
      options: {
        baseURL: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "qwen3.8-max": {
          name: "Qwen3.8 Max",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
          modalities: { input: ["text", "image", "video"], output: ["text"] },
        },
        "qwen3.8-flash": {
          name: "Qwen3.8 Flash",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
          modalities: { input: ["text", "image", "video"], output: ["text"] },
        },
        "qwen3.7-max": { name: "Qwen3.7 Max", reasoning: true },
      },
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    templateValues: {
      baseURL: {
        label: "Base URL",
        placeholder: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        defaultValue: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
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
      npm: "@ai-sdk/anthropic",
      name: "QwenCloud For Coding",
      options: {
        baseURL: "https://coding-intl.dashscope.aliyuncs.com/apps/anthropic/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "qwen3.7-plus": { name: "Qwen3.7 Plus", reasoning: true },
        "qwen3.6-plus": { name: "Qwen3.6 Plus", reasoning: true },
      },
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    templateValues: {
      baseURL: {
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
  },
  {
    name: "QwenCloud Token Plan",
    family: "qwencloud",
    planKey: "tokenPlan",
    websiteUrl: "https://www.qwencloud.com/pricing/token-plan",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "QwenCloud Token Plan",
      options: {
        baseURL:
          "https://token-plan.ap-southeast-1.maas.aliyuncs.com/apps/anthropic/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "qwen3.8-max": {
          name: "Qwen3.8 Max",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
          modalities: { input: ["text", "image", "video"], output: ["text"] },
        },
        "qwen3.8-flash": {
          name: "Qwen3.8 Flash",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
          modalities: { input: ["text", "image", "video"], output: ["text"] },
        },
        "qwen3.7-max": { name: "Qwen3.7 Max", reasoning: true },
      },
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
    templateValues: {
      baseURL: {
        label: "Base URL",
        placeholder:
          "https://token-plan.ap-southeast-1.maas.aliyuncs.com/apps/anthropic/v1",
        defaultValue:
          "https://token-plan.ap-southeast-1.maas.aliyuncs.com/apps/anthropic/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
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
      npm: "@ai-sdk/openai-compatible",
      name: "StepFun",
      options: {
        baseURL: "https://api.stepfun.com/step_plan/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "step-3.5-flash-2603": { name: "Step 3.5 Flash 2603", reasoning: true },
        "step-3.5-flash": { name: "Step 3.5 Flash", reasoning: true },
        "step-3.7-flash": {
          name: "Step 3.7 Flash",
          reasoning: true,
          limit: { context: 256000, output: 256000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "step-5-preview": {
          name: "Step 5 Preview",
          reasoning: true,
          limit: { context: 1000000, output: 64000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#16D6D2",
    templateValues: {
      baseURL: {
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
  },
  {
    name: "StepFun en",
    family: "stepfun",
    regionKey: "intl",
    websiteUrl: "https://platform.stepfun.ai/step-plan",
    apiKeyUrl: "https://platform.stepfun.ai/interface-key",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "StepFun en",
      options: {
        baseURL: "https://api.stepfun.ai/step_plan/v1",
        apiKey: "",
      },
      models: {
        "step-3.5-flash-2603": { name: "Step 3.5 Flash 2603", reasoning: true },
        "step-3.5-flash": { name: "Step 3.5 Flash", reasoning: true },
        "step-3.7-flash": {
          name: "Step 3.7 Flash",
          reasoning: true,
          limit: { context: 256000, output: 256000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "step-5-preview": {
          name: "Step 5 Preview",
          reasoning: true,
          limit: { context: 1000000, output: 64000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#16D6D2",
    templateValues: {
      baseURL: {
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
  },
  {
    name: "StepFun Step Plan",
    websiteUrl: "https://platform.stepfun.com/docs/zh/step-plan/overview",
    apiKeyUrl: "https://platform.stepfun.com/interface-key",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "StepFun Step Plan",
      options: {
        baseURL: "https://api.stepfun.com/step_plan/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "step-3.5-flash": { name: "Step 3.5 Flash", reasoning: true },
        "step-3.7-flash": {
          name: "Step 3.7 Flash",
          reasoning: true,
          limit: { context: 256000, output: 256000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "step-5-preview": {
          name: "Step 5 Preview",
          reasoning: true,
          limit: { context: 1000000, output: 64000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#005AFF",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "step-...",
        editorValue: "",
      },
    },
  },
  {
    name: "ModelScope",
    websiteUrl: "https://modelscope.cn",
    apiKeyUrl: "https://modelscope.cn/my/myaccesstoken",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "ModelScope",
      options: {
        baseURL: "https://api-inference.modelscope.cn/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "ZhipuAI/GLM-5.2": { name: "GLM-5.2", reasoning: true },
      },
    },
    category: "aggregator",
    icon: "modelscope",
    iconColor: "#624AFF",
    templateValues: {
      baseURL: {
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
  },
  {
    name: "KAT-Coder",
    websiteUrl: "https://console.streamlake.ai",
    apiKeyUrl: "https://console.streamlake.ai/console/api-key",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "KAT-Coder",
      options: {
        baseURL:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "KAT-Coder-Pro": { name: "KAT-Coder Pro" },
      },
    },
    category: "cn_official",
    templateValues: {
      baseURL: {
        label: "Base URL",
        placeholder:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        defaultValue:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        editorValue: "",
      },
      ENDPOINT_ID: {
        label: "Vanchin Endpoint ID",
        placeholder: "ep-xxx-xxx",
        defaultValue: "",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    icon: "catcoder",
  },
  {
    name: "Longcat",
    websiteUrl: "https://longcat.chat/platform",
    apiKeyUrl: "https://longcat.chat/platform/api_keys",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Longcat",
      options: {
        baseURL: "https://api.longcat.chat/openai/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "LongCat-2.0": {
          name: "LongCat 2.0",
          options: { thinking: { type: "disabled" } },
        },
      },
    },
    category: "cn_official",
    icon: "longcat",
    iconColor: "#29E154",
    templateValues: {
      baseURL: {
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
  },
  {
    name: "MiniMax",
    family: "minimax",
    regionKey: "cn",
    websiteUrl: "https://platform.minimax.cn",
    apiKeyUrl: "https://platform.minimax.cn/console/plan",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "MiniMax",
      options: {
        baseURL: "https://api.minimax.cn/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "MiniMax-M3": {
          name: "MiniMax M3",
          reasoning: true,
          limit: { context: 1000000, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
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
  },
  {
    name: "MiniMax en",
    family: "minimax",
    regionKey: "intl",
    websiteUrl: "https://platform.minimax.io",
    apiKeyUrl: "https://platform.minimax.io/console/plan",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "MiniMax en",
      options: {
        baseURL: "https://api.minimax.io/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "MiniMax-M3": {
          name: "MiniMax M3",
          reasoning: true,
          limit: { context: 1000000, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
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
  },
  {
    name: "BaiLing",
    websiteUrl: "https://developer.ant-ling.com/zh-CN/docs/",
    apiKeyUrl: "https://chat.ant-ling.com/open",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "BaiLing",
      options: {
        baseURL: "https://api.ant-ling.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "Ling-2.6-1T": { name: "Ling 2.6-1T" },
      },
    },
    category: "cn_official",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
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
      npm: "@ai-sdk/openai-compatible",
      name: "Xiaomi MiMo",
      options: {
        baseURL: "https://api.xiaomimimo.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "mimo-v2.6-pro": {
          name: "MiMo V2.6 Pro",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "mimo-v2.6-flash": {
          name: "MiMo V2.6 Flash",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "mimo-v2.6-pro-ultraspeed": {
          name: "MiMo V2.6 Pro UltraSpeed",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
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
  },
  {
    name: "Xiaomi MiMo Token Plan (China)",
    family: "xiaomi-mimo",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.xiaomimimo.com/#/token-plan",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/plan-manage",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Xiaomi MiMo Token Plan (China)",
      options: {
        baseURL: "https://token-plan-cn.xiaomimimo.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "mimo-v2.6-pro": {
          name: "MiMo V2.6 Pro",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "mimo-v2.6-flash": {
          name: "MiMo V2.6 Flash",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
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
  },

  {
    name: "OpenCode Go",
    websiteUrl: "https://opencode.ai/go",
    apiKeyUrl: "https://opencode.ai/go",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "OpenCode Go",
      options: {
        baseURL: "https://opencode.ai/zen/go/v1",
        apiKey: "",
        setCacheKey: true,
      },
      // 都在 /v1/chat/completions 上（https://opencode.ai/docs/go/ ，2026-10-06）
      models: {
        "glm-5.3": { name: "GLM-5.3", reasoning: true },
        "kimi-k3": { name: "Kimi K3", reasoning: true },
        "kimi-k2.7-code": { name: "Kimi K2.7 Code", reasoning: true },
        "deepseek-v4-pro": { name: "DeepSeek V4 Pro", reasoning: true },
        "deepseek-v4.1-flash": {
          name: "DeepSeek V4.1 Flash",
          reasoning: true,
        },
        "mimo-v2.6-pro": { name: "MiMo-V2.6-Pro", reasoning: true },
      },
    },
    category: "third_party",
    icon: "opencode",
    iconColor: "#211E1E",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "CherryIN",
      options: {
        baseURL: "https://open.cherryin.net/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "anthropic/claude-sonnet-5.5": {
          name: "Claude Sonnet 5.5",
          reasoning: true,
        },
        "anthropic/claude-opus-5.5": {
          name: "Claude Opus 5.5",
          reasoning: true,
        },
        "anthropic/claude-fable-5.1": {
          name: "Claude Fable 5.1",
          reasoning: true,
          limit: { context: 1000000, output: 128000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
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
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "OpenRouter",
      options: {
        baseURL: "https://openrouter.ai/api/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "anthropic/claude-sonnet-5.5": {
          name: "Claude Sonnet 5.5",
          reasoning: true,
        },
        "anthropic/claude-opus-5.5": {
          name: "Claude Opus 5.5",
          reasoning: true,
          limit: { context: 1000000, output: 128000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "anthropic/claude-fable-5.1": {
          name: "Claude Fable 5.1",
          reasoning: true,
          limit: { context: 1000000, output: 128000 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
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
  },
  {
    name: "Novita AI",
    websiteUrl: "https://novita.ai",
    apiKeyUrl: "https://novita.ai",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Novita AI",
      options: {
        baseURL: "https://api.novita.ai/openai",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "zai-org/glm-5.3": {
          name: "GLM-5.3",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text"], output: ["text"] },
        },
        "zai-org/glm-5.3-flash": {
          name: "GLM-5.3-Flash",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "moonshotai/kimi-k3": {
          name: "Kimi K3",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "aggregator",
    icon: "novita",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    name: "Nvidia",
    websiteUrl: "https://build.nvidia.com",
    apiKeyUrl: "https://build.nvidia.com/settings/api-keys",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Nvidia",
      options: {
        baseURL: "https://integrate.api.nvidia.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "moonshotai/kimi-k3": {
          name: "Kimi K3",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
        "z-ai/glm-5.3": {
          name: "GLM-5.3",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text"], output: ["text"] },
        },
        "z-ai/glm-5.3-flash": {
          name: "GLM-5.3-Flash",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    category: "aggregator",
    icon: "nvidia",
    iconColor: "#000000",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    name: "AWS Bedrock",
    websiteUrl: "https://aws.amazon.com/bedrock/",
    settingsConfig: {
      npm: "@ai-sdk/amazon-bedrock",
      name: "AWS Bedrock",
      options: {
        region: "${region}",
        accessKeyId: "${accessKeyId}",
        secretAccessKey: "${secretAccessKey}",
        setCacheKey: true,
      },
      models: {
        "global.anthropic.claude-opus-5-5": {
          name: "Claude Opus 5.5",
          reasoning: true,
        },
        "global.anthropic.claude-sonnet-5-5": {
          name: "Claude Sonnet 5.5",
          reasoning: true,
        },
        "global.anthropic.claude-haiku-4-5-20251001-v1:0": {
          name: "Claude Haiku 4.5",
          reasoning: true,
        },
        "us.amazon.nova-pro-v1:0": { name: "Amazon Nova Pro" },
        "us.meta.llama4-maverick-17b-instruct-v1:0": {
          name: "Meta Llama 4 Maverick",
        },
        "us.deepseek.r1-v1:0": { name: "DeepSeek R1", reasoning: true },
      },
    },
    category: "cloud_provider",
    icon: "aws",
    iconColor: "#FF9900",
    templateValues: {
      region: {
        label: "AWS Region",
        placeholder: "us-west-2",
        defaultValue: "us-west-2",
        editorValue: "us-west-2",
      },
      accessKeyId: {
        label: "Access Key ID",
        placeholder: "AKIA...",
        editorValue: "",
      },
      secretAccessKey: {
        label: "Secret Access Key",
        placeholder: "your-secret-key",
        editorValue: "",
      },
    },
  },
  {
    name: "Oh My OpenCode",
    websiteUrl: "https://github.com/code-yeongyu/oh-my-openagent",
    settingsConfig: {
      npm: "",
      options: {},
      models: {},
    },
    category: "omo" as ProviderCategory,
    icon: "opencode",
    iconColor: "#8B5CF6",
    isCustomTemplate: true,
  },
  {
    name: "Oh My OpenCode Slim",
    websiteUrl: "https://github.com/alvinunreal/oh-my-opencode-slim",
    settingsConfig: {
      npm: "",
      options: {},
      models: {},
    },
    category: "omo-slim" as ProviderCategory,
    icon: "opencode",
    iconColor: "#6366F1",
    isCustomTemplate: true,
  },
  {
    name: "模力方舟",
    websiteUrl: "https://moark.com",
    apiKeyUrl: "https://moark.com/dashboard/tokens",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "模力方舟",
      options: {
        baseURL: "https://api.moark.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        // OpenCode 以 limit.context 判断自动压缩；缺失会被当成 0 而跳过压缩，
        // 故显式声明容量，取值与本 PR 的 Pi / OpenClaw 预设一致
        "deepseek-v4-flash-0731": {
          name: "DeepSeek V4 Flash",
          reasoning: true,
          limit: { context: 1000000, output: 384000 },
        },
        "DeepSeek-V4-Pro": {
          name: "DeepSeek V4 Pro",
          reasoning: true,
          limit: { context: 1000000, output: 384000 },
        },
        "GLM-5.3": {
          name: "GLM-5.3",
          reasoning: true,
          limit: { context: 1048576, output: 131072 },
        },
        "Kimi-K2.7-Code": {
          name: "Kimi K2.7 Code",
          reasoning: true,
          limit: { context: 262144, output: 262144 },
        },
        "qwen3.8-max": {
          name: "Qwen3.8 Max",
          reasoning: true,
          limit: { context: 983616, output: 131072 },
        },
      },
    },
    category: "aggregator",
    icon: "moark",
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
];
