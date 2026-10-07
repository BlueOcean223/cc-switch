/**
 * 预设供应商配置模板
 */
import { ProviderCategory } from "../types";
import type { PresetFamilyFields } from "./presetFamilies";

export interface TemplateValueConfig {
  label: string;
  placeholder: string;
  defaultValue?: string;
  editorValue: string;
}

/**
 * 预设供应商的视觉主题配置
 */
export interface PresetTheme {
  /** 图标类型：'claude' | 'codex' | 'gemini' | 'generic' */
  icon?: "claude" | "codex" | "gemini" | "generic";
  /** 背景色（选中状态），支持 Tailwind 类名或 hex 颜色 */
  backgroundColor?: string;
  /** 文字色（选中状态），支持 Tailwind 类名或 hex 颜色 */
  textColor?: string;
}

export interface ProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string; // i18n key for localized display name
  websiteUrl: string;
  // 新增：第三方/聚合等可单独配置获取 API Key 的链接
  apiKeyUrl?: string;
  settingsConfig: object;
  isOfficial?: boolean; // 标识是否为官方预设
  partnerPromotionKey?: string; // 预设标识（如 google-official），保存到 meta 供识别
  category?: ProviderCategory; // 新增：分类
  // 新增：指定该预设所使用的 API Key 字段名（默认 ANTHROPIC_AUTH_TOKEN）
  apiKeyField?: "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY";
  // 新增：模板变量定义，用于动态替换配置中的值
  templateValues?: Record<string, TemplateValueConfig>; // editorValue 存储编辑器中的实时输入值
  // 新增：请求地址候选列表（用于地址管理/测速）
  endpointCandidates?: string[];
  // 新增：视觉主题配置
  theme?: PresetTheme;
  // 图标配置
  icon?: string; // 图标名称
  iconColor?: string; // 图标颜色

  // 是否在 UI 中隐藏该预设（预设仍存在，仅不在列表中显示）
  hidden?: boolean;

  // 获取模型列表使用的完整 URL（覆写自动候选逻辑）
  // 缺省时后端基于 baseURL 自动尝试 /v1/models、/models 以及剥离已知兼容子路径后的变体。
  modelsUrl?: string;
}

export const providerPresets: ProviderPreset[] = [
  {
    name: "Claude Official",
    websiteUrl: "https://www.anthropic.com/claude-code",
    settingsConfig: {
      env: {},
    },
    isOfficial: true, // 明确标识为官方预设
    category: "official",
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
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.moonshot.cn/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        // 官方 Claude Code 指南：主模型用 K3，Haiku 用 K2.7 Code
        // https://platform.kimi.com/docs/guide/claude-code-kimi (2026-10)
        ANTHROPIC_MODEL: "kimi-k3",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "kimi-k2.7-code",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "kimi-k3",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "kimi-k3",
      },
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
    websiteUrl: "https://platform.kimi.ai",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.moonshot.ai/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        // 官方 Claude Code 指南：主模型用 K3，Haiku 用 K2.7 Code
        // https://platform.kimi.com/docs/guide/claude-code-kimi (2026-10)
        ANTHROPIC_MODEL: "kimi-k3",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "kimi-k2.7-code",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "kimi-k3",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "kimi-k3",
      },
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
    websiteUrl: "https://www.kimi.com/code/",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.kimi.com/coding/",
        ANTHROPIC_AUTH_TOKEN: "",
        // CLAUDE_CODE_MAX_CONTEXT_TOKENS 只对非 claude- 前缀模型 id 生效，
        // 必须显式路由端点别名 kimi-for-coding（与 codex/hermes/opencode 预设一致）
        ANTHROPIC_MODEL: "kimi-for-coding",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "kimi-for-coding",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "kimi-for-coding",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "kimi-for-coding",
        // 双键钉 256K：压缩窗口=min(模型窗口,值)，与窗口同值时行为等价于不设，
        // 但显式钉住可屏蔽远程实验下发的更小压缩点；调整直接改 JSON，不出表单字段
        CLAUDE_CODE_MAX_CONTEXT_TOKENS: "262144",
        CLAUDE_CODE_AUTO_COMPACT_WINDOW: "262144",
      },
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  {
    name: "Kimi For Coding Global",
    family: "kimi",
    planKey: "coding",
    regionKey: "intl",
    websiteUrl: "https://www.kimi.ai/code",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.kimi.ai/coding/",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "kimi-for-coding",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "kimi-for-coding",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "kimi-for-coding",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "kimi-for-coding",
        CLAUDE_CODE_MAX_CONTEXT_TOKENS: "262144",
        CLAUDE_CODE_AUTO_COMPACT_WINDOW: "262144",
      },
    },
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.qnaigc.com",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: ["https://api.qnaigc.com", "https://api.modelink.ai"],
    category: "aggregator",
    icon: "qiniu",
  },
  {
    name: "PPIO",
    websiteUrl: "https://ppio.com",
    apiKeyUrl: "https://ppio.com/settings/key-management",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.ppio.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "deepseek/deepseek-v4-flash-0731",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek/deepseek-v4-flash-0731",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek/deepseek-v4-flash-0731",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek/deepseek-v4-flash-0731",
      },
    },
    category: "aggregator",
    endpointCandidates: ["https://api.ppio.com/anthropic"],
    modelsUrl: "https://api.ppio.com/openai/v1/models",
    icon: "ppio",
    iconColor: "#2874FF",
  },
  {
    name: "火山 Agent Plan",
    family: "volcengine",
    planKey: "agentPlan",
    websiteUrl: "https://www.volcengine.com/activity/agentplan",
    apiKeyUrl: "https://www.volcengine.com/activity/agentplan",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://ark.cn-beijing.volces.com/api/plan",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "ark-code-latest",
      },
    },
    category: "cn_official",
    icon: "huoshan",
    iconColor: "#3370FF",
  },
  {
    name: "火山 Coding Plan",
    family: "volcengine",
    planKey: "codingPlan",
    websiteUrl: "https://www.volcengine.com/activity/codingplan",
    apiKeyUrl: "https://www.volcengine.com/activity/codingplan",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://ark.cn-beijing.volces.com/api/coding",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "ark-code-latest",
      },
    },
    category: "cn_official",
    icon: "huoshan",
    iconColor: "#3370FF",
  },
  {
    name: "BytePlus",
    websiteUrl: "https://www.byteplus.com/en/product/modelark",
    apiKeyUrl: "https://www.byteplus.com/en/product/modelark",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://ark.ap-southeast.bytepluses.com/api/coding",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "ark-code-latest",
      },
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
    websiteUrl:
      "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    apiKeyUrl:
      "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://ark.cn-beijing.volces.com/api/compatible",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "doubao-seed-2-1-pro-260915",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "doubao-seed-2-1-pro-260915",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "doubao-seed-2-1-pro-260915",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "doubao-seed-2-1-pro-260915",
      },
    },
    category: "cn_official",
    icon: "doubao",
    iconColor: "#3370FF",
  },
  {
    name: "SiliconFlow",
    family: "siliconflow",
    regionKey: "cn",
    websiteUrl: "https://siliconflow.cn",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.siliconflow.cn",
        ANTHROPIC_AUTH_TOKEN: "",
        // MiniMax-M2.5 已于 2026-09-11 下线；换成 Anthropic 接口文档里的示例模型
        // https://docs.siliconflow.cn/docs/api/messages-post
        ANTHROPIC_MODEL: "deepseek-ai/DeepSeek-V4-Flash",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-ai/DeepSeek-V4-Flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-ai/DeepSeek-V4-Flash",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-ai/DeepSeek-V4-Flash",
      },
    },
    category: "aggregator",
    icon: "siliconflow",
    iconColor: "#6E29F6",
  },
  {
    name: "SiliconFlow en",
    family: "siliconflow",
    regionKey: "intl",
    websiteUrl: "https://siliconflow.com",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.siliconflow.com",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "MiniMaxAI/MiniMax-M3",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "MiniMaxAI/MiniMax-M3",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "MiniMaxAI/MiniMax-M3",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "MiniMaxAI/MiniMax-M3",
      },
    },
    category: "aggregator",
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
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.modelverse.cn",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: ["https://api.modelverse.cn"],
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
    // 套餐只含国产模型（DeepSeek / Kimi / GLM / MiniMax），官方接入页各档都用
    // deepseek-v4-pro：https://www.compshare.cn/docs/modelverse/package_plan/usecases (2026-09-11)
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://cp.compshare.cn",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-v4-pro",
      },
    },
    endpointCandidates: ["https://cp.compshare.cn"],
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "AtlasCloud",
    websiteUrl: "https://www.atlascloud.ai/console/coding-plan",
    apiKeyUrl: "https://www.atlascloud.ai/console/coding-plan",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.atlascloud.ai",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "zai-org/glm-5.2",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "zai-org/glm-5.2",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "zai-org/glm-5.2",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "zai-org/glm-5.2",
        CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS: "1",
      },
    },
    endpointCandidates: ["https://api.atlascloud.ai"],
    category: "aggregator",
    icon: "atlascloud",
  },
  {
    name: "DeepSeek",
    websiteUrl: "https://platform.deepseek.com",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.deepseek.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-v4-pro",
      },
    },
    category: "cn_official",
    // Anthropic 兼容层挂在 /anthropic 子路径；/models 是根上独立端点
    modelsUrl: "https://api.deepseek.com/models",
    icon: "deepseek",
    iconColor: "#1E88E5",
  },
  {
    name: "OpenCode Go",
    family: "opencode",
    planKey: "coding",
    websiteUrl: "https://opencode.ai/go",
    apiKeyUrl: "https://opencode.ai/go",
    // Go 网关 /v1/messages 只认 x-api-key（Bearer 被静默忽略），
    // 必须用 ANTHROPIC_API_KEY，不能换回 ANTHROPIC_AUTH_TOKEN。
    // 直连 Anthropic 端点可用除 grok-4.5 外的全部 Go 模型；
    // Chat 组模型（DeepSeek/GLM/Kimi 等）依赖网关服务端格式转换（未见文档承诺）。
    apiKeyField: "ANTHROPIC_API_KEY",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://opencode.ai/zen/go",
        ANTHROPIC_API_KEY: "",
        ANTHROPIC_MODEL: "deepseek-v4-flash",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-v4-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-v4-flash",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-v4-flash",
      },
    },
    category: "third_party",
    endpointCandidates: ["https://opencode.ai/zen/go"],
    icon: "opencode",
    iconColor: "#211E1E",
  },
  {
    // Zen 按量网关：Claude 模型原生走 /v1/messages，直连不需要路由。
    // 和 Go 一样只认 x-api-key（Bearer 报 Missing API key）。
    // 免费模型只能在 OpenCode 里用（外部调用 403 FreeTierError），默认不用。
    name: "OpenCode Zen",
    family: "opencode",
    planKey: "payg",
    websiteUrl: "https://opencode.ai/zen",
    apiKeyUrl: "https://opencode.ai/auth",
    apiKeyField: "ANTHROPIC_API_KEY",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://opencode.ai/zen",
        ANTHROPIC_API_KEY: "",
        ANTHROPIC_MODEL: "claude-sonnet-5-5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "claude-haiku-4-5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "claude-sonnet-5-5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "claude-opus-5-5",
      },
    },
    category: "aggregator",
    endpointCandidates: ["https://opencode.ai/zen"],
    icon: "opencode",
    iconColor: "#211E1E",
  },
  {
    // 腾讯云 Token Plan 个人版（1823/130060，2026-08-21 版）：通用 + Hy 两
    // 系列共用同一端点与 API Key；Auto 智能路由的调用 ID 是 tc-code-latest。
    // 注意与 TokenHub 按量 API 市场（1823 线，如 Hunyuan 预设的 /v1 端点）
    // 是两条产品线，订阅 Key 只能走 /plan 端点
    name: "Tencent Token Plan",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://api.lkeap.cloud.tencent.com/plan/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "tc-code-latest",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "tc-code-latest",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "tc-code-latest",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "tc-code-latest",
      },
    },
    category: "cn_official",
    endpointCandidates: ["https://api.lkeap.cloud.tencent.com/plan/anthropic"],
    // /plan/v3/models 真 Key 实测可用（2026-08-31，返回套餐内 19 个模型含
    // 别名）；企业/国际端点无 /models（404），故仅此预设覆写
    modelsUrl: "https://api.lkeap.cloud.tencent.com/plan/v3/models",
    icon: "tencent",
    iconColor: "#0052D9",
  },
  {
    // 国际站（新加坡地域）个人版（intl 1300/81315，2026-08-20 版）：
    // Auto 的调用 ID 是 auto（与国内个人版 tc-code-latest 不同），模型阵容
    // 也与国内不同（无 GLM-5/5.1/Hy3，多 GLM-5.2/MiniMax-M3）。端点用国际站
    // 文档钦定的 tencentcloudmaas.com 域（DNS 实测解析新加坡节点）；国内站
    // 文档对新加坡地域给的是 tokenhub-intl.tencentmaas.com，Key 按站独立、
    // 不跨站通用，故互不作候选
    name: "Tencent Token Plan (Intl)",
    family: "tencent",
    planKey: "tokenPlan",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "auto",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "auto",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "auto",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "auto",
      },
    },
    category: "cn_official",
    endpointCandidates: [
      "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
    ],
    icon: "tencent",
    iconColor: "#0052D9",
  },
  {
    // Token Plan 企业版专业套餐（1823/130659，2026-08-25 版）：广州地域端点
    // 为国内站文档钦定的 tencentmaas.com；新加坡地域模型阵容不同且 Key 不
    // 跨站，见 (Intl) 预设
    name: "Tencent Token Plan Enterprise Pro",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://tokenhub.tencentmaas.com/plan/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "auto",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "auto",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "auto",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "auto",
      },
    },
    category: "cn_official",
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
    // 国际站企业版专业套餐（intl 1300/81489，2026-08-26 版）：新加坡地域，
    // 模型阵容为广州地域的子集（无 GLM-5/5.1/5-Turbo、Kimi-K2.6、
    // MiniMax-M2.7）
    name: "Tencent Token Plan Enterprise Pro (Intl)",
    family: "tencent",
    planKey: "enterprisePro",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "auto",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "auto",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "auto",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "auto",
      },
    },
    category: "cn_official",
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
    // Token Plan 企业版轻享套餐（1823/131173，2026-08-28 版）：仅 Auto 模型
    name: "Tencent Token Plan Enterprise Lite",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/tokenplan-e",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://tokenhub.tencentmaas.com/plan/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "auto",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "auto",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "auto",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "auto",
      },
    },
    category: "cn_official",
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
    // 国际站企业版轻享套餐（intl 1300/81490）：新加坡地域（资源调度范围为
    // Global），仅 Auto 模型
    name: "Tencent Token Plan Enterprise Lite (Intl)",
    family: "tencent",
    planKey: "enterpriseLite",
    regionKey: "intl",
    websiteUrl: "https://www.tencentcloud.com/products/tokenhub",
    apiKeyUrl: "https://console.tencentcloud.com/tokenhub/tokenplan-e",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://tokenhub-intl.tencentcloudmaas.com/plan/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "auto",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "auto",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "auto",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "auto",
      },
    },
    category: "cn_official",
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
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://open.bigmodel.cn/api/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "glm-5.3",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "glm-5.3",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "glm-5.3",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "glm-5.3",
      },
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
  },
  {
    name: "Zhipu GLM en",
    family: "zhipu",
    regionKey: "intl",
    websiteUrl: "https://z.ai",
    apiKeyUrl: "https://z.ai/subscribe",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.z.ai/api/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "glm-5.3",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "glm-5.3",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "glm-5.3",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "glm-5.3",
      },
    },
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
  },
  {
    // Token Plan 个人版：2026-07-13 起替代 Coding Plan 发售（Coding Plan
    // 停售，预设已删）。模型=官方 Claude Code 接入页
    // （2026-07-30 版）全角色 deepseek-v4-pro；Key 是订阅页专属 Key
    name: "Baidu Qianfan Token Plan",
    family: "baidu-qianfan",
    planKey: "tokenPlan",
    websiteUrl: "https://cloud.baidu.com/product/codingplan.html",
    apiKeyUrl: "https://console.bce.baidu.com/qianfan/resource/token-plan",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://qianfan.baidubce.com/anthropic/tokenplan/personal",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-v4-pro",
      },
    },
    category: "cn_official",
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
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://dashscope.aliyuncs.com/apps/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "qwen3.8-max",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "qwen3.8-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "qwen3.7-plus",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "qwen3.8-max",
        // 模型 id 非 claude-* 时 Claude Code 按 200K 默认窗口处理，必须显式
        // 钉住官方值：qwen3.8 系 context_window = 983616
        CLAUDE_CODE_MAX_CONTEXT_TOKENS: "983616",
      },
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  {
    name: "千问AI平台 Coding Plan",
    family: "qianwen",
    planKey: "codingPlan",
    websiteUrl: "https://bailian.console.aliyun.com",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://coding.dashscope.aliyuncs.com/apps/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  {
    name: "千问AI平台 Token Plan",
    family: "qianwen",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.qianwenai.com/pricing/token-plan",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://token-plan.cn-beijing.maas.aliyuncs.com/apps/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "qwen3.8-max",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "qwen3.8-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "qwen3.7-plus",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "qwen3.8-max",
        // 官方 Token Plan 配置同样钉窗口：qwen3.8 系 context_window = 983616
        CLAUDE_CODE_MAX_CONTEXT_TOKENS: "983616",
      },
    },
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  // ===== QwenCloud（DashScope 国际站）=====
  // 与上面国内百炼是两套独立站点：域名、控制台、密钥互不通用。
  // 三条线各有专属 base_url 与专属 API Key，官方文档明示密钥类型与
  // base_url 不匹配会 401，因此拆成三个预设而非共用一条加候选地址。
  {
    name: "QwenCloud",
    family: "qwencloud",
    planKey: "payg",
    websiteUrl: "https://home.qwencloud.com/",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://dashscope-intl.aliyuncs.com/apps/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "qwen3.8-max",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "qwen3.8-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "qwen3.7-plus",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "qwen3.8-max",
      },
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "QwenCloud For Coding",
    family: "qwencloud",
    planKey: "coding",
    websiteUrl: "https://www.qwencloud.com",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://coding-intl.dashscope.aliyuncs.com/apps/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "qwen3.7-plus",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "qwen3.7-plus",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "qwen3.7-plus",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "qwen3.7-plus",
      },
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "QwenCloud Token Plan",
    family: "qwencloud",
    planKey: "tokenPlan",
    websiteUrl: "https://www.qwencloud.com/pricing/token-plan",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://token-plan.ap-southeast-1.maas.aliyuncs.com/apps/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "qwen3.8-max",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "qwen3.8-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "qwen3.7-plus",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "qwen3.8-max",
        // 官方 Claude Code 配置钉的窗口：qwen3.8 系 context_window = 983616
        CLAUDE_CODE_MAX_CONTEXT_TOKENS: "983616",
      },
    },
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "StepFun",
    family: "stepfun",
    regionKey: "cn",
    websiteUrl: "https://platform.stepfun.com/step-plan",
    apiKeyUrl: "https://platform.stepfun.com/interface-key",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.stepfun.com/step_plan",
        ANTHROPIC_AUTH_TOKEN: "",
        // Step Plan 的 Claude Code 指南默认 step-5-preview
        // https://platform.stepfun.com/docs/zh/step-plan/integrations/claude-code (2026-10)
        ANTHROPIC_MODEL: "step-5-preview",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "step-3.5-flash-2603",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "step-5-preview",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "step-5-preview",
      },
    },
    category: "cn_official",
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
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.stepfun.ai/step_plan",
        ANTHROPIC_AUTH_TOKEN: "",
        // Step Plan 的 Claude Code 指南默认 step-5-preview
        // https://platform.stepfun.com/docs/zh/step-plan/integrations/claude-code (2026-10)
        ANTHROPIC_MODEL: "step-5-preview",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "step-3.5-flash-2603",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "step-5-preview",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "step-5-preview",
      },
    },
    category: "cn_official",
    endpointCandidates: ["https://api.stepfun.ai/step_plan"],
    icon: "stepfun",
    iconColor: "#16D6D2",
  },
  {
    name: "ModelScope",
    websiteUrl: "https://modelscope.cn",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api-inference.modelscope.cn",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "ZhipuAI/GLM-5.2",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "ZhipuAI/GLM-5.2",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "ZhipuAI/GLM-5.2",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "ZhipuAI/GLM-5.2",
      },
    },
    category: "aggregator",
    icon: "modelscope",
    iconColor: "#624AFF",
  },
  {
    name: "KAT-Coder",
    websiteUrl: "https://console.streamlake.ai",
    apiKeyUrl: "https://console.streamlake.ai/console/api-key",
    // 官方接入指南（按量付费）：路径里原来的 Endpoint ID 换成模型 ID，
    // 各档都用 kat-coder-pro-v2.5；Air V1 已退役
    // https://www.streamlake.ai/document/DOC/mg6k6nlp8j6qxicx4c9 (2026-07-13)
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/kat-coder-pro-v2.5/claude-code-proxy",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "kat-coder-pro-v2.5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "kat-coder-pro-v2.5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "kat-coder-pro-v2.5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "kat-coder-pro-v2.5",
      },
    },
    category: "cn_official",
    icon: "catcoder",
  },
  {
    name: "Longcat",
    websiteUrl: "https://longcat.chat/platform",
    apiKeyUrl: "https://longcat.chat/platform/api_keys",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.longcat.chat/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "LongCat-2.0",
        ANTHROPIC_SMALL_FAST_MODEL: "LongCat-2.0",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "LongCat-2.0",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "LongCat-2.0",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "LongCat-2.0",
        CLAUDE_CODE_MAX_OUTPUT_TOKENS: "131072",
      },
    },
    category: "cn_official",
    icon: "longcat",
    iconColor: "#29E154",
  },
  {
    name: "MiniMax",
    family: "minimax",
    regionKey: "cn",
    websiteUrl: "https://platform.minimax.cn",
    apiKeyUrl: "https://platform.minimax.cn/console/plan",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.minimax.cn/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        CLAUDE_CODE_AUTO_COMPACT_WINDOW: "1000000",
        ANTHROPIC_MODEL: "MiniMax-M3[1M]",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "MiniMax-M3[1M]",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "MiniMax-M3[1M]",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "MiniMax-M3[1M]",
      },
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
    websiteUrl: "https://platform.minimax.io",
    apiKeyUrl: "https://platform.minimax.io/console/plan",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.minimax.io/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        CLAUDE_CODE_AUTO_COMPACT_WINDOW: "1000000",
        ANTHROPIC_MODEL: "MiniMax-M3[1M]",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "MiniMax-M3[1M]",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "MiniMax-M3[1M]",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "MiniMax-M3[1M]",
      },
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
    websiteUrl: "https://developer.ant-ling.com/zh-CN/docs/",
    apiKeyUrl: "https://chat.ant-ling.com/open",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.ant-ling.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "Ling-2.6-1T",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "Ling-2.6-1T",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "Ling-2.6-1T",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "Ling-2.6-1T",
      },
    },
    category: "cn_official",
    icon: "bailing",
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://open.cherryin.net",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "anthropic/claude-sonnet-5.5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "anthropic/claude-haiku-4.5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "anthropic/claude-sonnet-5.5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "anthropic/claude-opus-5.5",
      },
    },
    category: "aggregator",
    endpointCandidates: ["https://open.cherryin.net"],
    icon: "cherryin",
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://openrouter.ai/api",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "anthropic/claude-sonnet-5.5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "anthropic/claude-haiku-4.5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "anthropic/claude-sonnet-5.5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "anthropic/claude-opus-5.5",
      },
    },
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6566F1",
  },
  {
    name: "Novita AI",
    websiteUrl: "https://novita.ai",
    apiKeyUrl: "https://novita.ai",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.novita.ai/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "zai-org/glm-5.3",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "zai-org/glm-5.3",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "zai-org/glm-5.3",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "zai-org/glm-5.3",
      },
    },
    category: "aggregator",
    endpointCandidates: ["https://api.novita.ai/anthropic"],
    // Anthropic 兼容层在 /anthropic 子路径，OpenAI 侧却在 /openai/v1；剥后缀
    // 后的根路径没有 /models（实测 404），通用候选够不到，故覆写
    modelsUrl: "https://api.novita.ai/openai/v1/models",
    icon: "novita",
    iconColor: "#000000",
  },
  {
    name: "Xiaomi MiMo",
    family: "xiaomi-mimo",
    planKey: "payg",
    websiteUrl: "https://platform.xiaomimimo.com",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/api-keys",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.xiaomimimo.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "mimo-v2.6-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "mimo-v2.6-pro",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "mimo-v2.6-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "mimo-v2.6-pro",
      },
    },
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
  },
  {
    name: "Xiaomi MiMo Token Plan (China)",
    family: "xiaomi-mimo",
    planKey: "tokenPlan",
    websiteUrl: "https://platform.xiaomimimo.com/#/token-plan",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/plan-manage",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://token-plan-cn.xiaomimimo.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "mimo-v2.6-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "mimo-v2.6-pro",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "mimo-v2.6-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "mimo-v2.6-pro",
      },
    },
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
  },
  {
    name: "AWS Bedrock (AKSK)",
    family: "aws-bedrock",
    planKey: "aksk",
    websiteUrl: "https://aws.amazon.com/bedrock/",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://bedrock-runtime.${AWS_REGION}.amazonaws.com",
        AWS_ACCESS_KEY_ID: "${AWS_ACCESS_KEY_ID}",
        AWS_SECRET_ACCESS_KEY: "${AWS_SECRET_ACCESS_KEY}",
        AWS_REGION: "${AWS_REGION}",
        ANTHROPIC_MODEL: "global.anthropic.claude-opus-5-5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL:
          "global.anthropic.claude-haiku-4-5-20251001-v1:0",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "global.anthropic.claude-sonnet-5-5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "global.anthropic.claude-opus-5-5",
        CLAUDE_CODE_USE_BEDROCK: "1",
      },
    },
    category: "cloud_provider",
    templateValues: {
      AWS_REGION: {
        label: "AWS Region",
        placeholder: "us-west-2",
        editorValue: "us-west-2",
      },
      AWS_ACCESS_KEY_ID: {
        label: "Access Key ID",
        placeholder: "AKIA...",
        editorValue: "",
      },
      AWS_SECRET_ACCESS_KEY: {
        label: "Secret Access Key",
        placeholder: "your-secret-key",
        editorValue: "",
      },
    },
    icon: "aws",
    iconColor: "#FF9900",
  },
  {
    name: "AWS Bedrock (API Key)",
    family: "aws-bedrock",
    planKey: "apiKey",
    websiteUrl: "https://aws.amazon.com/bedrock/",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://bedrock-runtime.${AWS_REGION}.amazonaws.com",
        // Claude Code 只从这个变量读 Bedrock API Key，顶层 apiKey 它不认
        AWS_BEARER_TOKEN_BEDROCK: "",
        AWS_REGION: "${AWS_REGION}",
        ANTHROPIC_MODEL: "global.anthropic.claude-opus-5-5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL:
          "global.anthropic.claude-haiku-4-5-20251001-v1:0",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "global.anthropic.claude-sonnet-5-5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "global.anthropic.claude-opus-5-5",
        CLAUDE_CODE_USE_BEDROCK: "1",
      },
    },
    category: "cloud_provider",
    templateValues: {
      AWS_REGION: {
        label: "AWS Region",
        placeholder: "us-west-2",
        editorValue: "us-west-2",
      },
    },
    icon: "aws",
    iconColor: "#FF9900",
  },
  {
    name: "模力方舟",
    websiteUrl: "https://moark.com",
    apiKeyUrl: "https://moark.com/dashboard/tokens",
    settingsConfig: {
      env: {
        // 官方文档（CC Switch 快速配置）指定 Claude Code 走 Anthropic 原生入口
        ANTHROPIC_BASE_URL: "https://moark.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "deepseek-v4-flash-0731",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-v4-flash-0731",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-v4-flash-0731",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-v4-flash-0731",
      },
    },
    category: "aggregator",
    endpointCandidates: ["https://moark.com/anthropic"],
    icon: "moark",
  },
];
