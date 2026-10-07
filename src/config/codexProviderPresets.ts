/**
 * Codex 预设供应商配置模板
 */
import { ProviderCategory } from "../types";
import type { CodexCatalogModel } from "../types";
import type { PresetTheme } from "./claudeProviderPresets";
import type { PresetFamilyFields } from "./presetFamilies";

// MiMo 官方 Codex 目录的系统提示词。
// https://mimo.mi.com/docs/tokenplan/integration/codex-configuration
const MIMO_CODEX_BASE_INSTRUCTIONS =
  "You are MiMo, an AI assistant developed by Xiaomi. Today's date: {date} {week}. Your knowledge cutoff date is December 2024.";

export interface CodexProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string; // i18n key for localized display name
  websiteUrl: string;
  // 第三方供应商可提供单独的获取 API Key 链接
  apiKeyUrl?: string;
  auth: Record<string, any>; // 将写入 ~/.codex/auth.json
  config: string; // 将写入 ~/.codex/config.toml（TOML 字符串）
  isOfficial?: boolean; // 标识是否为官方预设
  partnerPromotionKey?: string; // 预设标识（如 google-official），保存到 meta 供识别
  category?: ProviderCategory; // 新增：分类
  isCustomTemplate?: boolean; // 标识是否为自定义模板
  // 新增：请求地址候选列表（用于地址管理/测速）
  endpointCandidates?: string[];
  // 新增：视觉主题配置
  theme?: PresetTheme;
  // 图标配置
  icon?: string; // 图标名称
  iconColor?: string; // 图标颜色
  // 官方预设：可以绑定授权中心里的 ChatGPT 账号
  providerType?: "codex_oauth";
  // 写进 Codex 模型目录（model-catalogs.json）的模型
  modelCatalog?: CodexCatalogModel[];
}

/**
 * 生成第三方供应商的 auth.json
 */
export function generateThirdPartyAuth(apiKey: string): Record<string, any> {
  return {
    OPENAI_API_KEY: apiKey || "",
  };
}

/**
 * 生成第三方供应商的 config.toml
 */
export function generateThirdPartyConfig(
  providerName: string,
  baseUrl: string,
  modelName = "gpt-5.6-sol",
): string {
  const tomlString = (value: string) => JSON.stringify(value);

  return `model_provider = "custom"
model = ${tomlString(modelName)}
model_reasoning_effort = "high"

[model_providers.custom]
name = ${tomlString(providerName)}
base_url = ${tomlString(baseUrl)}
wire_api = "responses"
requires_openai_auth = true`;
}

function modelCatalog(
  models: Array<
    | string
    | {
        model: string;
        displayName?: string;
        contextWindow?: number;
        // Native Responses (direct) overrides for the generated
        // model-catalogs.json. Omitted input modalities are inferred by the
        // backend: confirmed text-only models stay text-only; everything else
        // defaults to text+image.
        supportsParallelToolCalls?: boolean;
        inputModalities?: string[];
        // Vendor's OFFICIAL base_instructions; omit to inherit the neutral
        // template default. Required by Codex, so the backend always emits one.
        baseInstructions?: string;
        // Reasoning efforts the vendor's endpoint actually accepts (subset of
        // none/minimal/low/medium/high/xhigh/max/ultra). Omit to keep the
        // template's conservative none/high default. Pre-filled from official
        // vendor docs; users can still edit per provider in the form.
        reasoningLevels?: string[];
        defaultReasoningLevel?: string;
      }
  >,
): CodexCatalogModel[] {
  return models.map((entry) =>
    typeof entry === "string"
      ? { model: entry }
      : {
          model: entry.model,
          displayName: entry.displayName,
          contextWindow: entry.contextWindow,
          supportsParallelToolCalls: entry.supportsParallelToolCalls,
          inputModalities: entry.inputModalities,
          baseInstructions: entry.baseInstructions,
          reasoningLevels: entry.reasoningLevels,
          defaultReasoningLevel: entry.defaultReasoningLevel,
        },
  );
}

export const codexProviderPresets: CodexProviderPreset[] = [
  {
    name: "OpenAI Official",
    websiteUrl: "https://chatgpt.com/codex",
    isOfficial: true,
    category: "official",
    providerType: "codex_oauth",
    auth: {},
    config: ``,
    theme: {
      icon: "codex",
      backgroundColor: "#1F2937", // gray-800
      textColor: "#FFFFFF",
    },
    icon: "openai",
    iconColor: "#00A67E",
  },
  {
    name: "Kimi",
    family: "kimi",
    planKey: "payg",
    regionKey: "cn",
    websiteUrl: "https://platform.kimi.com",
    apiKeyUrl: "https://platform.kimi.com/console/api-keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "kimi",
      "https://api.moonshot.cn/v1",
      "kimi-k3",
    ),
    endpointCandidates: ["https://api.moonshot.cn/v1"],
    // 原生 Responses 直连：官方 Codex 接入文档
    //（platform.kimi.com/docs/guide/codex-kimi.md，直接以 CC Switch 为例）
    // 给出 base_url = https://api.moonshot.cn/v1 + wire_api = "responses"，
    // 并明写开放平台「原生支持 Codex 使用的 Responses API，无需协议转换或本
    // 地代理」；接口参考 platform.kimi.com/docs/api/responses.md（POST
    // /v1/responses、reasoning.effort 枚举 low/high/max、tool_choice 仅
    // auto、支持 prompt_cache_key，usage 带 cached_tokens）。2026-09-09 真
    // Key 探针：Codex 0.153.4 的全量请求形态（include
    // reasoning.encrypted_content + reasoning.summary + text.verbosity）与
    // 流式事件序列均 200；kimi-k2.7-code 亦 200——文档只列 kimi-k3，属未文
    // 档化能力，厂商若收回从 catalog 删行即可。
    modelCatalog: modelCatalog([
      // 首行 = 默认模型（catalog[0] 须与 config.toml 的 model 一致）：
      // kimi-k3 是官方 Codex 文档与 Responses OpenAPI 唯一列出的模型
      //（Jason 2026-09-10 拍板，推翻 07-17「k3 排在 k2.7-code 之后」的旧序）。
      // 档位照抄官方参数文档（2026-08-15 盘点，2026-09-09 复核）：k3 不可关
      // 思考、reasoning.effort 三档官方默认 max；k2.7-code 始终思考、官方标注
      // 不支持 reasoning_effort → 单档 high（防模板假差异档，LongCat 先例）。
      // 两模型都关不掉思考，none 一律不列。k3 不声明 default：native 模板
      // 默认 high ∈ 子集 → 后端保留 high，与本预设 config.toml 顶层
      // model_reasoning_effort = "high"（实际下发值）一致；catalog 默认只
      // 标记 Codex /model 选择器，声明 max 会展示一个实际不下发的默认值。
      // supportsParallelToolCalls 两行填 true：Kimi Code 官方 catalog 对同一
      // K3 明写 true，且探针里两端点都把 parallel_tool_calls 回显为 true；
      // 不填会让 Codex ≤0.148 用户从 ProxyChat 模板的 true 退化成 native
      // 模板的 false（0.153.4 起该字段已不存在，填了无害）
      {
        model: "kimi-k3",
        displayName: "Kimi K3",
        contextWindow: 1048576,
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
      },
      {
        model: "kimi-k2.7-code",
        displayName: "Kimi K2.7 Code",
        contextWindow: 262144,
        supportsParallelToolCalls: true,
        reasoningLevels: ["high"],
      },
    ]),
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  // API 开放平台海外/Global 变体：platform.kimi.ai + api.moonshot.ai 端点；
  // 接入形态与国内版一致（原生 Responses 直连），依据见上方国内版注释
  {
    name: "Kimi Global",
    family: "kimi",
    planKey: "payg",
    regionKey: "intl",
    websiteUrl: "https://platform.kimi.ai",
    apiKeyUrl: "https://platform.kimi.ai/console/api-keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "kimi",
      "https://api.moonshot.ai/v1",
      "kimi-k3",
    ),
    endpointCandidates: ["https://api.moonshot.ai/v1"],
    modelCatalog: modelCatalog([
      {
        model: "kimi-k3",
        displayName: "Kimi K3",
        contextWindow: 1048576,
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
      },
      {
        model: "kimi-k2.7-code",
        displayName: "Kimi K2.7 Code",
        contextWindow: 262144,
        supportsParallelToolCalls: true,
        reasoningLevels: ["high"],
      },
    ]),
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
    apiKeyUrl: "https://www.kimi.com/code/",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "kimi_coding",
      "https://api.kimi.com/coding/v1",
      "kimi-for-coding",
    ),
    endpointCandidates: ["https://api.kimi.com/coding/v1"],
    // 原生 Responses 直连：官方 Codex 接入文档
    //（kimi.com/code/docs/third-party-tools/codex.html，以 CC Switch 为例）
    // 给出 base_url = https://api.kimi.com/coding/v1 且 wire_api「必须填
    // responses」，并明写「Kimi Code 服务端原生支持 OpenAI Responses API
    //（流式/非流式、reasoning、function calling 均可用），无需任何本地路由
    // 或协议转换工具」。2026-09-09 真 Key 探针：四个模型在 Codex 全量请求
    // 形态下均 200，reasoning item 带真实 encrypted_content；同
    // prompt_cache_key 的二次请求命中 cached_tokens（prompt_cache_key 由
    // Codex 自己发）
    modelCatalog: modelCatalog([
      // 官方 Codex 指南（2026-09-15）：kimi-for-coding 已升级 K2.8 Preview，
      // 最高 1M 上下文、low/high/max。高速版仍为 256K，独立保留其档位。
      // k3 的 1M 权限取决于会员档位，k3-256k 固定 256K。
      {
        model: "kimi-for-coding",
        displayName: "Kimi For Coding (K2.8 Preview)",
        contextWindow: 1048576,
        supportsParallelToolCalls: true,
        inputModalities: ["text", "image"],
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "high",
      },
      {
        model: "kimi-for-coding-highspeed",
        displayName: "Kimi For Coding HighSpeed",
        contextWindow: 262144,
        supportsParallelToolCalls: true,
        reasoningLevels: ["high"],
      },
      {
        model: "k3",
        displayName: "Kimi K3",
        contextWindow: 1048576,
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "high",
      },
      {
        model: "k3-256k",
        displayName: "Kimi K3 256K",
        contextWindow: 262144,
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "high",
      },
    ]),
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  // 海外/Global 变体：kimi.ai/code + api.kimi.ai 端点；接入形态与国内版一致
  //（原生 Responses 直连，wire_api = "responses"），依据见上方国内版注释
  {
    name: "Kimi For Coding Global",
    family: "kimi",
    planKey: "coding",
    regionKey: "intl",
    websiteUrl: "https://www.kimi.ai/code",
    apiKeyUrl: "https://www.kimi.ai/code",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "kimi_coding",
      "https://api.kimi.ai/coding/v1",
      "kimi-for-coding",
    ),
    endpointCandidates: ["https://api.kimi.ai/coding/v1"],
    modelCatalog: modelCatalog([
      {
        model: "kimi-for-coding",
        displayName: "Kimi For Coding",
        contextWindow: 1048576,
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "max",
      },
      {
        model: "kimi-for-coding-highspeed",
        displayName: "Kimi For Coding HighSpeed",
        contextWindow: 262144,
        supportsParallelToolCalls: true,
        reasoningLevels: ["high"],
      },
      {
        model: "k3",
        displayName: "Kimi K3",
        contextWindow: 1048576,
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "high",
      },
      {
        model: "k3-256k",
        displayName: "Kimi K3 256K",
        contextWindow: 262144,
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "high",
      },
    ]),
    category: "cn_official",
    icon: "kimi",
    iconColor: "#6366F1",
  },
  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    category: "aggregator",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "qiniu",
      "https://api.qnaigc.com/bypass/openai/v1",
      "gpt-6-astra",
    ),
    endpointCandidates: [
      "https://api.qnaigc.com/bypass/openai/v1",
      "https://api.modelink.ai/bypass/openai/v1",
    ],
    icon: "qiniu",
  },
  {
    name: "火山 Agent Plan",
    family: "volcengine",
    planKey: "agentPlan",
    websiteUrl: "https://www.volcengine.com/activity/agentplan",
    apiKeyUrl: "https://www.volcengine.com/activity/agentplan",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "ark_agentplan",
      "https://ark.cn-beijing.volces.com/api/plan/v3",
      "ark-code-latest",
    ),
    // ⚠️ 计费红线（官方 warning）：Agent Plan 必须走 /api/plan/v3；
    // 按量端点 /api/v3 不消耗套餐额度、按量另计费，Coding Plan 的
    // /api/coding/v3 是另一份订阅——两者都绝不能混入候选
    endpointCandidates: ["https://ark.cn-beijing.volces.com/api/plan/v3"],
    // 官方 Codex 文档（docs.volcengine.com/docs/82379/2556054，2026-08-24 更新）：
    // Agent Plan /api/plan/v3 与 Coding Plan /api/coding/v3 均已支持
    // Responses API（wire_api=responses）
    modelCatalog: modelCatalog([
      {
        model: "ark-code-latest",
        displayName: "Ark Code Latest",
        contextWindow: 256000,
        // 四份官方 Codex 接入文档（82379/2556054~2556057）一致限定
        // model_reasoning_effort 只能是 low/medium/high；none/xhigh/max 是
        // glm-5-2 专属值（82379/1449737），别名可指向任意后端模型故不可填
        reasoningLevels: ["low", "medium", "high"],
      },
    ]),
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
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "ark_codingplan",
      "https://ark.cn-beijing.volces.com/api/coding/v3",
      "ark-code-latest",
    ),
    // ⚠️ 计费红线（官方 warning）：Coding Plan 必须走 /api/coding/v3；
    // 按量端点 /api/v3 不消耗套餐额度、按量另计费，Agent Plan 的
    // /api/plan/v3 是另一份订阅——两者都绝不能混入候选
    endpointCandidates: ["https://ark.cn-beijing.volces.com/api/coding/v3"],
    // 官方 Codex 文档（volcengine.com/docs/82379/2556056，2026-07 更新）：
    // Coding Plan /api/coding/v3 已支持 Responses API（wire_api=responses）
    modelCatalog: modelCatalog([
      {
        model: "ark-code-latest",
        displayName: "Ark Code Latest",
        contextWindow: 256000,
        // 同 Agent Plan：官方 Codex 文档钉死 low/medium/high 三档
        reasoningLevels: ["low", "medium", "high"],
      },
    ]),
    category: "cn_official",
    icon: "huoshan",
    iconColor: "#3370FF",
  },
  {
    name: "BytePlus",
    websiteUrl: "https://www.byteplus.com/en/product/modelark",
    apiKeyUrl: "https://www.byteplus.com/en/product/modelark",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "byteplus",
      "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
      "ark-code-latest",
    ),
    endpointCandidates: [
      "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
    ],
    // 国际站已核实（2026-08-15 盘点）：BytePlus 官方 Codex 接入文档
    //（docs.byteplus.com/en/docs/ModelArk/2556056）标准 config.toml 的
    // base_url 就是本端点且 wire_api="responses"，OpenCode 文档亦明写
    // Responses 优先——与国内站火山双 Plan 对齐切原生直连
    modelCatalog: modelCatalog([
      {
        model: "ark-code-latest",
        displayName: "Ark Code Latest",
        contextWindow: 256000,
        // 官方 Codex 文档 model_reasoning_effort 限定 low/medium/high，与
        // 国内站同名模型四份文档交叉印证。⚠️auto 路由别名固有不确定性：
        // 路由到 glm-5-2-260617 时官方明载 low/medium 按 high 等价处理
        reasoningLevels: ["low", "medium", "high"],
      },
    ]),
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
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "doubaoseed",
      "https://ark.cn-beijing.volces.com/api/v3",
      "doubao-seed-2-1-pro-260915",
    ),
    endpointCandidates: ["https://ark.cn-beijing.volces.com/api/v3"],
    // 火山方舟主数据面 /api/v3 原生支持 Responses API（/api/v3/responses）
    // 无官方 catalog：合成 MiMo 式（shell_command 编辑、不发 freeform apply_patch），
    // 让 Codex 直连显示模型并避免 custom 工具被网关拒绝
    modelCatalog: modelCatalog([
      // 260628 已移到往期模型；260915 上下文 1024k（方舟模型列表 82379/1330310，2026-09-28）
      {
        model: "doubao-seed-2-1-pro-260915",
        displayName: "Doubao Seed 2.1 Pro",
        contextWindow: 1048576,
        // 方舟深度思考文档（82379/1449737）7 值枚举中本模型无限制的通用四档；
        // none/xhigh 仅 glm-5-2、max 的 deepseek 名单标注 Responses 待支持。
        // minimal=方舟的"关闭思考直接回答"档；官方点名本模型服务端默认 high
        reasoningLevels: ["minimal", "low", "medium", "high"],
      },
    ]),
    category: "cn_official",
    icon: "doubao",
    iconColor: "#3370FF",
  },
  {
    name: "Compshare",
    family: "compshare",
    planKey: "payg",
    nameKey: "providerForm.presets.ucloud",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl: "https://www.compshare.cn/coding-plan",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "compshare",
      "https://api.modelverse.cn/v1",
      "gpt-6-astra",
    ),
    endpointCandidates: ["https://api.modelverse.cn/v1"],
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "Azure OpenAI",
    websiteUrl:
      "https://learn.microsoft.com/en-us/azure/ai-foundry/openai/how-to/codex",
    category: "third_party",
    isOfficial: true,
    auth: generateThirdPartyAuth(""),
    config: `model_provider = "custom"
model = "gpt-6.1-sol"
model_reasoning_effort = "high"

[model_providers.custom]
name = "Azure OpenAI"
base_url = "https://YOUR_RESOURCE_NAME.openai.azure.com/openai"
env_key = "OPENAI_API_KEY"
query_params = { "api-version" = "2025-04-01-preview" }
wire_api = "responses"
requires_openai_auth = true`,
    endpointCandidates: ["https://YOUR_RESOURCE_NAME.openai.azure.com/openai"],
    theme: {
      icon: "codex",
      backgroundColor: "#0078D4",
      textColor: "#FFFFFF",
    },
    icon: "azure",
    iconColor: "#0078D4",
  },
  {
    name: "DeepSeek",
    websiteUrl: "https://platform.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "deepseek",
      "https://api.deepseek.com",
      "deepseek-flash",
    ),
    endpointCandidates: ["https://api.deepseek.com"],
    // DeepSeek 官方 Codex 文档（api-docs.deepseek.com → agent_integrations/codex）：
    // 当前官方推荐 deepseek-flash（V4.1 Flash）；旧 v4-flash 别名继续由后端兼容。
    // deepseek-flash 原生 Responses（wire_api=responses 对自家 base_url）。
    // 后端按 deepseek.com host 直接镜像官方 models.json（freeform apply_patch +
    // GPT-5 harness + low/high/max 思考档，需 codex >= 0.144.0），这里只保留行清单与展示名。
    // 档位照抄官方 catalog（low/high/max 默认 high，2026-08-15 复核 flash/pro
    // 逐字节一致）：per-row 值会覆盖官方镜像，DeepSeek 官方目录变更时须同步这里
    // （Jason 2026-08-15 拍板：表单可见性优先于快照过时风险，"未设置"误导性更大）
    modelCatalog: modelCatalog([
      {
        model: "deepseek-flash",
        displayName: "DeepSeek V4.1 Flash",
        inputModalities: ["text", "image"],
        contextWindow: 1048576,
        reasoningLevels: ["low", "high", "max"],
      },
      // pro 已于 2026-08 开通 Responses/Codex 集成（官方 catalog 条目与 flash 仅差 priority）
      {
        model: "deepseek-v4-pro",
        displayName: "DeepSeek V4 Pro",
        inputModalities: ["text"],
        contextWindow: 1048576,
        reasoningLevels: ["low", "high", "max"],
      },
    ]),
    category: "cn_official",
    icon: "deepseek",
    iconColor: "#1E88E5",
  },
  {
    name: "Zhipu GLM",
    family: "zhipu",
    regionKey: "cn",
    websiteUrl: "https://open.bigmodel.cn",
    apiKeyUrl: "https://www.bigmodel.cn/claude-code",
    auth: generateThirdPartyAuth(""),
    // 智谱三端点分立（docs.bigmodel.cn/cn/coding-plan/tool/others）：Anthropic
    // /api/anthropic、OpenAI Chat /api/coding/paas/v4、OpenAI Responses /api/v1，
    // 并明示「错误配置端点将导致无法使用 GLM Coding Plan 套餐额度」。Codex 直连
    // 发的是 Responses wire，base_url 必须是 /api/v1；Chat 端点上的 /responses
    // 是严格旧网关（拒 type=custom 工具 → #6944 的 400）
    config: generateThirdPartyConfig(
      "zhipu_glm",
      "https://open.bigmodel.cn/api/v1",
      "glm-5.3",
    ),
    endpointCandidates: ["https://open.bigmodel.cn/api/v1"],
    // 官方 Codex 接入页（docs.bigmodel.cn/cn/coding-plan/tool/codex，2026-09-04
    // 核对）：wire_api=responses 对自家 /api/v1，与 MiMo/MiniMax 同为原生直连
    // → NativeResponses profile（shell_command 编辑、不发 freeform apply_patch；
    // 官方目录虽声明 freeform，无真机验证前按保守口径，不引入 400 风险）
    // 档位/上下文/模态照抄官方 models.json：glm-5.3 low/high/max 默认 max；
    // glm-5-turbo 官方档位为空、默认 max——cc-switch 表达不了空档位（回落会得到
    // 模板 none/high，none 会原样发给严格网关），
    // 按官方默认收成单档 max。两模型为纯文本、并行工具调用 true。
    // glm-5.3-flash 例外：官方 Codex models.json 尚未收录（2026-09-26 核对），
    // 依据是模型页 docs.bigmodel.cn/cn/guide/models/vlm/glm-5.3-flash——原生
    // 多模态、1M 上下文、「文本参数与 GLM-5.3 保持一致」、GLM Coding Plan 已全量
    // 开放；档位与并行工具调用据此对齐 glm-5.3。预设自带这一行，用户就不必把
    // glm-5.3 行改名成 flash（改名会连带继承该行隐藏的 ["text"] 声明，Codex
    // 因此拦截图片，见 #7688）。
    modelCatalog: modelCatalog([
      {
        model: "glm-5.3",
        displayName: "GLM-5.3",
        contextWindow: 1048576,
        inputModalities: ["text"],
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "max",
      },
      {
        model: "glm-5.3-flash",
        displayName: "GLM-5.3-Flash",
        contextWindow: 1048576,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "max",
      },
      {
        model: "glm-5-turbo",
        displayName: "GLM-5-Turbo",
        contextWindow: 204800,
        inputModalities: ["text"],
        supportsParallelToolCalls: true,
        reasoningLevels: ["max"],
      },
    ]),
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
    auth: generateThirdPartyAuth(""),
    // 国际站同上（docs.z.ai/devpack/tool/others + devpack/tool/codex，2026-09-04
    // 核对）：Responses 端点 /api/v1，官方 models.json 仅列 glm-5.3。
    // glm-5.3-flash 同国内站：依据国际站模型页 docs.z.ai/guides/vlm/glm-5.3-flash
    // （Coding Plan 同样全量开放），models.json 尚未收录（2026-09-26 核对）
    config: generateThirdPartyConfig(
      "zhipu_glm_en",
      "https://api.z.ai/api/v1",
      "glm-5.3",
    ),
    endpointCandidates: ["https://api.z.ai/api/v1"],
    modelCatalog: modelCatalog([
      {
        model: "glm-5.3",
        displayName: "GLM-5.3",
        contextWindow: 1048576,
        inputModalities: ["text"],
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "max",
      },
      {
        model: "glm-5.3-flash",
        displayName: "GLM-5.3-Flash",
        contextWindow: 1048576,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: true,
        reasoningLevels: ["low", "high", "max"],
        defaultReasoningLevel: "max",
      },
    ]),
    category: "cn_official",
    icon: "zhipu",
    iconColor: "#0F62FE",
  },
  {
    name: "Baidu Qianfan",
    family: "baidu-qianfan",
    planKey: "payg",
    websiteUrl: "https://cloud.baidu.com/product/qianfan_modelbuilder",
    apiKeyUrl:
      "https://console.bce.baidu.com/qianfan/ais/console/applicationConsole/application",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "qianfan",
      "https://qianfan.baidubce.com/v2",
      "deepseek-v4-pro",
    ),
    // 通用按量 Responses：cloud.baidu.com/doc/qianfan-docs/s/4mi400l1m。
    // 与 /v2/coding、/v2/tokenplan/personal 的专属 Key/额度分开，不互作候选。
    endpointCandidates: ["https://qianfan.baidubce.com/v2"],
    modelCatalog: modelCatalog([
      // 先收官方 Responses 支持名单中的 V4 两款，窗口按千帆部署的 1M。
      // 纯文本，不继承 DeepSeek 原厂将旧 Flash 别名升级为视觉模型的行为。
      // high 与生成的配置一致；不把 Chat 的 thinking 开关当成 Responses none。
      {
        model: "deepseek-v4-pro",
        displayName: "DeepSeek V4 Pro",
        contextWindow: 1048576,
        inputModalities: ["text"],
        reasoningLevels: ["high"],
      },
      {
        model: "deepseek-v4-flash",
        displayName: "DeepSeek V4 Flash",
        contextWindow: 1048576,
        inputModalities: ["text"],
        reasoningLevels: ["high"],
      },
    ]),
    category: "cn_official",
    icon: "baidu",
    iconColor: "#2932E1",
  },
  {
    name: "千问AI平台",
    family: "qianwen",
    planKey: "payg",
    websiteUrl: "https://platform.qianwenai.com/",
    apiKeyUrl: "https://platform.qianwenai.com/home/api-keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "qianwenai",
      "https://dashscope.aliyuncs.com/compatible-mode/v1",
      "qwen3.8-max",
    ),
    endpointCandidates: ["https://dashscope.aliyuncs.com/compatible-mode/v1"],
    // DashScope 原生支持 OpenAI Responses API（/compatible-mode/v1/responses，同一 base_url）
    // 档位与窗口照抄官方 Codex model-catalog.local.json——该元数据段落不分
    // 套餐，按量付费与 Token Plan 用同一份（qwen3.8 系只收 low/medium/xhigh，
    // 默认 xhigh；无 high 档，勿按常规四档补齐）
    modelCatalog: modelCatalog([
      {
        model: "qwen3.8-max",
        displayName: "Qwen3.8 Max",
        contextWindow: 983616,
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
      // 官方 Responses 支持清单中的开放权重型号（2026-09-15）。模型页
      // model-studio/qwen3-8-2-4t-a95b、qwen3-8-27b：总窗口 1000000、思考模式
      // 最大输入 983616；2.4T 纯文本，27B 支持图片/视频。HF 模型卡：effort
      // low/medium/xhigh（默认 xhigh），2.4T 不可关思考。
      // 仅加入按量平台；可用模型受地域与套餐约束，不复制到 Token Plan。
      {
        model: "qwen3.8-2.4t-a95b",
        displayName: "Qwen3.8 2.4T A95B",
        contextWindow: 983616,
        inputModalities: ["text"],
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
      {
        model: "qwen3.8-27b",
        displayName: "Qwen3.8 27B",
        contextWindow: 983616,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
    ]),
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
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "qianwenai_token_plan",
      "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
      "qwen3.8-max",
    ),
    endpointCandidates: [
      "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
    ],
    // 档位与窗口照抄官方 Codex model-catalogs.json（qwen3.8 系只收
    // low/medium/xhigh，默认 xhigh；无 high 档，勿按常规四档补齐）
    modelCatalog: modelCatalog([
      {
        model: "qwen3.8-max",
        displayName: "Qwen3.8 Max",
        contextWindow: 983616,
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
      {
        model: "qwen3.8-flash",
        displayName: "Qwen3.8 Flash",
        contextWindow: 983616,
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
    ]),
    category: "cn_official",
    icon: "qianwenai",
    iconColor: "#624AFF",
  },
  // ===== QwenCloud（DashScope 国际站）=====
  // 与上面国内条目是两套独立站点：域名、控制台、密钥互不通用。
  // 按量付费与 Token Plan 走 /compatible-mode/v1 原生 Responses。
  {
    name: "QwenCloud",
    family: "qwencloud",
    planKey: "payg",
    websiteUrl: "https://home.qwencloud.com/",
    apiKeyUrl: "https://home.qwencloud.com/api-keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "qwencloud",
      "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
      "qwen3.8-max",
    ),
    endpointCandidates: [
      "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
    ],
    // 档位与窗口照抄官方 Codex model-catalogs.json（qwen3.8 系只收
    // low/medium/xhigh，默认 xhigh；无 high 档，勿按常规四档补齐）
    modelCatalog: modelCatalog([
      {
        model: "qwen3.8-max",
        displayName: "Qwen3.8 Max",
        contextWindow: 983616,
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
      {
        model: "qwen3.8-flash",
        displayName: "Qwen3.8 Flash",
        contextWindow: 983616,
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
      {
        model: "qwen3.7-max",
        displayName: "Qwen3.7 Max",
        contextWindow: 1000000,
        inputModalities: ["text"],
      },
      // 官方 Responses 支持清单中的开放权重型号（2026-09-15）。模型页
      // model-studio/qwen3-8-2-4t-a95b、qwen3-8-27b：总窗口 1000000、思考模式
      // 最大输入 983616；2.4T 纯文本，27B 支持图片/视频。HF 模型卡：effort
      // low/medium/xhigh（默认 xhigh），2.4T 不可关思考。
      // 仅加入按量平台；可用模型受地域与套餐约束，不复制到 Token Plan。
      {
        model: "qwen3.8-2.4t-a95b",
        displayName: "Qwen3.8 2.4T A95B",
        contextWindow: 983616,
        inputModalities: ["text"],
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
      {
        model: "qwen3.8-27b",
        displayName: "Qwen3.8 27B",
        contextWindow: 983616,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
    ]),
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
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "qwencloud_token_plan",
      "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
      "qwen3.8-max",
    ),
    endpointCandidates: [
      "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
    ],
    // 档位与窗口照抄官方 Codex model-catalogs.json（qwen3.8 系只收
    // low/medium/xhigh，默认 xhigh；无 high 档，勿按常规四档补齐）
    modelCatalog: modelCatalog([
      {
        model: "qwen3.8-max",
        displayName: "Qwen3.8 Max",
        contextWindow: 983616,
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
      {
        model: "qwen3.8-flash",
        displayName: "Qwen3.8 Flash",
        contextWindow: 983616,
        supportsParallelToolCalls: false,
        reasoningLevels: ["low", "medium", "xhigh"],
        defaultReasoningLevel: "xhigh",
      },
      {
        model: "qwen3.7-max",
        displayName: "Qwen3.7 Max",
        contextWindow: 1000000,
        inputModalities: ["text"],
      },
    ]),
    category: "cn_official",
    icon: "qwencloud",
    iconColor: "#6336E7",
  },
  {
    name: "Tencent Hunyuan",
    family: "tencent",
    planKey: "payg",
    regionKey: "cn",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/apikey",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "hy3_tokenhub",
      "https://tokenhub.tencentmaas.com/v1",
      "hy3",
    ),
    // 官方备用域名 tencentmaas.cn（文档 1823/130078）；国际站 tokenhub-intl
    // 属不同地域，API Key 不跨站通用，不作候选
    endpointCandidates: [
      "https://tokenhub.tencentmaas.com/v1",
      "https://tokenhub.tencentmaas.cn/v1",
    ],
    // 腾讯 TokenHub 官方 Codex 文档（cloud.tencent.com/document/product/1823/133532）：
    // hy3 原生 Responses（wire_api=responses）。文档要求的
    // disable_response_storage=true 现已不需要：Codex 的请求固定带 store=false。
    // ⚠️ 须用 TokenHub API Key（创建时范围需勾选 Hy3）；Coding Plan / Token Plan
    // 订阅 Key 只能走各自 /plan 端点，对本预设的 /v1 不通。
    // hy3 在带 tools 的请求里会把 reasoning_effort=low 服务端自动升为 high
    // （Codex 恒带 tools），默认 high 即真实行为。
    // 无官方 catalog：合成 MiMo 式（shell_command 编辑、不发 freeform apply_patch）
    modelCatalog: modelCatalog([
      {
        model: "hy3",
        displayName: "Hy3",
        contextWindow: 256000,
        // hy3 不在官方图片理解模型名单（1823/136956），纯文本
        inputModalities: ["text"],
        // 官方档位枚举只有 low/high（1823/131208 + 开源权重 chat template
        // 对其他 effort 值直接 raise）；带 tools 时 low 被服务端升为 high
        reasoningLevels: ["low", "high"],
      },
      {
        // 混元指南（1300/80695）：总窗口 1M、最大输入 960k、最大输出 64k，
        // 按最大输入口径填（同千问 983616）；深度思考文档（1300/80637）：
        // reasoning_effort 默认 high，支持 none/high。保留 Hy3 为默认模型。
        model: "hy4-preview",
        displayName: "Hy4 Preview",
        contextWindow: 960000,
        inputModalities: ["text"],
        reasoningLevels: ["none", "high"],
        defaultReasoningLevel: "high",
      },
    ]),
    category: "cn_official",
    icon: "hunyuan",
    iconColor: "#0055E9",
  },
  {
    name: "StepFun API",
    family: "stepfun",
    planKey: "payg",
    regionKey: "cn",
    websiteUrl: "https://platform.stepfun.com",
    apiKeyUrl: "https://platform.stepfun.com/interface-key",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "stepfun_api",
      "https://api.stepfun.com/v1",
      "step-3.7-flash",
    ),
    // 常规 API Responses 官方仅列 step-3.7-flash（2026-09-15）：
    // platform.stepfun.com/docs/zh/api-reference/responses/responses-create。
    // Step Plan 的 /step_plan/v1 仍单列 Chat，不与按量接口混用。
    endpointCandidates: ["https://api.stepfun.com/v1"],
    modelCatalog: modelCatalog([
      {
        model: "step-3.7-flash",
        displayName: "Step 3.7 Flash",
        contextWindow: 262144,
        inputModalities: ["text", "image"],
        reasoningLevels: ["low", "medium", "high"],
        defaultReasoningLevel: "high",
      },
    ]),
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#16D6D2",
  },
  {
    name: "StepFun API en",
    family: "stepfun",
    planKey: "payg",
    regionKey: "intl",
    websiteUrl: "https://platform.stepfun.ai",
    apiKeyUrl: "https://platform.stepfun.ai/interface-key",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "stepfun_api_en",
      "https://api.stepfun.ai/v1",
      "step-3.7-flash",
    ),
    // 国际站使用独立域名/Key；与 Step Plan 国际预设分别计费。
    endpointCandidates: ["https://api.stepfun.ai/v1"],
    modelCatalog: modelCatalog([
      {
        model: "step-3.7-flash",
        displayName: "Step 3.7 Flash",
        contextWindow: 262144,
        inputModalities: ["text", "image"],
        reasoningLevels: ["low", "medium", "high"],
        defaultReasoningLevel: "high",
      },
    ]),
    category: "cn_official",
    icon: "stepfun",
    iconColor: "#16D6D2",
  },
  {
    name: "Longcat",
    websiteUrl: "https://longcat.chat/platform",
    apiKeyUrl: "https://longcat.chat/platform/api_keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "longcat",
      "https://api.longcat.chat/openai/v1",
      "LongCat-2.0",
    ),
    endpointCandidates: ["https://api.longcat.chat/openai/v1"],
    // 美团 LongCat 官方 Codex 文档用 wire_api=responses 对自家 base_url，原生 Responses
    // 无官方 catalog：合成 MiMo 式（shell_command 编辑、不发 freeform apply_patch）。
    // 注：LongCat 的 /responses 工具类型契约文档化程度最低，建议真机冒烟一次
    modelCatalog: modelCatalog([
      {
        model: "LongCat-2.0",
        displayName: "LongCat 2.0",
        contextWindow: 1048576,
        // LongCat 无档位可调：全站唯一 effort 证据=官方 Codex 示例的 high；
        // 关思考走另一字段 thinking:{type:disabled}（effort 拼写无文档），
        // models API 的 supported_parameters 也不含 reasoning，故不提供 none 假开关
        reasoningLevels: ["high"],
      },
    ]),
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
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "minimax",
      "https://api.minimax.cn/v1",
      "MiniMax-M3",
    ),
    endpointCandidates: ["https://api.minimax.cn/v1"],
    // MiniMax 官方 API 参考已列 /v1/responses 为正式端点（CN/intl 双区，POST /v1/responses），原生 Responses
    // 官方 Codex catalog（platform.minimax.cn/docs/token-plan/codex）：
    // shell_command 编辑、并行工具、文本+图像，不声明 freeform apply_patch。
    // 档位照抄官方 catalog：none/high（M3 的 effort 是思考开关，minimal/low/medium
    // 端点接受但与 high 行为完全等价，不给假差异档）。与模板默认一致故 Codex 侧
    // 零行为变化，显式声明只为表单可见（"未设置"误导性更大，Jason 2026-08-15 拍板）
    modelCatalog: modelCatalog([
      {
        model: "MiniMax-M3",
        displayName: "MiniMax-M3",
        contextWindow: 1000000,
        reasoningLevels: ["none", "high"],
        supportsParallelToolCalls: true,
        inputModalities: ["text", "image"],
        baseInstructions:
          "You are Codex, a coding agent based on MiniMax-M3. You and the user share the same workspace and collaborate to achieve the user's goals.",
      },
    ]),
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
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "minimax_en",
      "https://api.minimax.io/v1",
      "MiniMax-M3",
    ),
    endpointCandidates: ["https://api.minimax.io/v1"],
    // MiniMax 官方 API 参考已列 /v1/responses 为正式端点（CN/intl 双区，POST /v1/responses），原生 Responses
    // 官方 Codex catalog（platform.minimax.io/docs/token-plan/codex）：
    // shell_command 编辑、并行工具、文本+图像，不声明 freeform apply_patch。
    // 档位照抄官方 catalog：none/high（M3 的 effort 是思考开关，minimal/low/medium
    // 端点接受但与 high 行为完全等价，不给假差异档）。与模板默认一致故 Codex 侧
    // 零行为变化，显式声明只为表单可见（"未设置"误导性更大，Jason 2026-08-15 拍板）
    modelCatalog: modelCatalog([
      {
        model: "MiniMax-M3",
        displayName: "MiniMax-M3",
        contextWindow: 1000000,
        reasoningLevels: ["none", "high"],
        supportsParallelToolCalls: true,
        inputModalities: ["text", "image"],
        baseInstructions:
          "You are Codex, a coding agent based on MiniMax-M3. You and the user share the same workspace and collaborate to achieve the user's goals.",
      },
    ]),
    category: "cn_official",
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
    icon: "minimax",
    iconColor: "#FF6B6B",
  },
  {
    name: "Astron Coding Plan",
    websiteUrl: "https://maas.xfyun.cn/packageSubscription",
    apiKeyUrl: "https://maas.xfyun.cn/packageSubscription",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "astron_coding_plan",
      "https://maas-coding-api.cn-huabei-1.xf-yun.com/v1",
      "astron-code-latest",
    ),
    // 讯飞官方 Codex 配置：www.xfyun.cn/doc/spark/CodingPlan.html。
    // Responses 用 /v1；Chat 的 /v2 以及常规 maas-api 服务均不能混用。
    endpointCandidates: ["https://maas-coding-api.cn-huabei-1.xf-yun.com/v1"],
    modelCatalog: modelCatalog([
      {
        // 别名在控制台切换底层模型；窗口/模态沿用官方保守接入示例。
        model: "astron-code-latest",
        displayName: "Astron Code Latest",
        contextWindow: 92160,
        inputModalities: ["text"],
        reasoningLevels: ["high"],
        defaultReasoningLevel: "high",
      },
    ]),
    category: "cn_official",
    icon: "astron",
  },
  {
    name: "Xiaomi MiMo",
    family: "xiaomi-mimo",
    planKey: "payg",
    websiteUrl: "https://platform.xiaomimimo.com",
    apiKeyUrl: "https://platform.xiaomimimo.com/#/console/api-keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "xiaomi_mimo",
      "https://api.xiaomimimo.com/v1",
      "mimo-v2.6-pro",
    ),
    endpointCandidates: ["https://api.xiaomimimo.com/v1"],
    // 小米 MiMo 官方 Codex 文档已声明原生支持 Responses API（wire_api=responses 对自家 base_url）
    // 官方目录 2026-09-23 起改为四档（none/low/medium/high）、默认 low。
    // https://mimo.mi.com/docs/tokenplan/integration/codex-configuration
    // 仅同步模型元数据，沿用现有工具传输配置。
    modelCatalog: modelCatalog([
      {
        model: "mimo-v2.6-pro",
        displayName: "MiMo V2.6 Pro",
        contextWindow: 1048576,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: false,
        baseInstructions: MIMO_CODEX_BASE_INSTRUCTIONS,
        reasoningLevels: ["none", "low", "medium", "high"],
        defaultReasoningLevel: "low",
      },
      {
        model: "mimo-v2.6-flash",
        displayName: "MiMo V2.6 Flash",
        contextWindow: 1048576,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: false,
        baseInstructions: MIMO_CODEX_BASE_INSTRUCTIONS,
        reasoningLevels: ["none", "low", "medium", "high"],
        defaultReasoningLevel: "low",
      },
      {
        model: "mimo-v2.6-pro-ultraspeed",
        displayName: "MiMo V2.6 Pro UltraSpeed",
        contextWindow: 1048576,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: false,
        baseInstructions: MIMO_CODEX_BASE_INSTRUCTIONS,
        reasoningLevels: ["none", "low", "medium", "high"],
        defaultReasoningLevel: "low",
      },
    ]),
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
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "xiaomi_mimo_token_plan",
      "https://token-plan-cn.xiaomimimo.com/v1",
      "mimo-v2.6-pro",
    ),
    endpointCandidates: ["https://token-plan-cn.xiaomimimo.com/v1"],
    // 小米 MiMo 官方 Codex 文档已声明原生支持 Responses API（wire_api=responses 对自家 base_url）
    // 官方目录 2026-09-23 起改为四档（none/low/medium/high）、默认 low。
    // https://mimo.mi.com/docs/tokenplan/integration/codex-configuration
    // 仅同步模型元数据，沿用现有工具传输配置。
    modelCatalog: modelCatalog([
      {
        model: "mimo-v2.6-pro",
        displayName: "MiMo V2.6 Pro",
        contextWindow: 1048576,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: false,
        baseInstructions: MIMO_CODEX_BASE_INSTRUCTIONS,
        reasoningLevels: ["none", "low", "medium", "high"],
        defaultReasoningLevel: "low",
      },
      {
        model: "mimo-v2.6-flash",
        displayName: "MiMo V2.6 Flash",
        contextWindow: 1048576,
        inputModalities: ["text", "image"],
        supportsParallelToolCalls: false,
        baseInstructions: MIMO_CODEX_BASE_INSTRUCTIONS,
        reasoningLevels: ["none", "low", "medium", "high"],
        defaultReasoningLevel: "low",
      },
    ]),
    category: "cn_official",
    icon: "xiaomimimo",
    iconColor: "#000000",
  },
  {
    name: "xAI (Grok)",
    websiteUrl: "https://x.ai/api",
    apiKeyUrl: "https://console.x.ai",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig("xai", "https://api.x.ai/v1", "grok-4.7"),
    endpointCandidates: ["https://api.x.ai/v1"],
    // xAI 官方以 /v1/responses 为一等端点（docs.x.ai api-reference）：Codex 硬依赖的
    // store:false / include=["reasoning.encrypted_content"] / reasoning effort 均支持，
    // 原生 Responses
    modelCatalog: modelCatalog([
      // https://docs.x.ai/developers/models/grok-4.7 (2026-09-23)
      {
        model: "grok-4.7",
        displayName: "Grok 4.7",
        contextWindow: 500000,
        supportsParallelToolCalls: true,
        inputModalities: ["text", "image"],
        reasoningLevels: ["low", "medium", "high", "xhigh"],
      },
      {
        model: "grok-4.5",
        displayName: "Grok 4.5",
        contextWindow: 500000,
        supportsParallelToolCalls: true,
        inputModalities: ["text", "image"],
        // 实测（2026-08-30，native /v1/responses 逐档探测）：grok-4.5 接受
        // low/medium/high/xhigh，拒绝 max（HTTP 400 "Invalid reasoning
        // effort"）；"Reasoning cannot be disabled" 故无 none 档。Codex 不按
        // catalog clamp 越界档位（Desktop UI 选出的 max 会原样发出），此列表
        // 必须与上游实收集合一致，勿凭文档增删。
        // ⚠️ docs.x.ai/developers/grok-4-5 页面实际渲染的是 grok-4.6 内容勿引
        reasoningLevels: ["low", "medium", "high", "xhigh"],
      },
    ]),
    category: "third_party",
    icon: "xai",
    iconColor: "#000000",
  },
  {
    // Zen 按量网关：GPT 模型原生走 /v1/responses，直连不需要路由；
    // 免费模型只能在 OpenCode 里用（外部调用 403 FreeTierError），默认不用。
    // 默认不用全仓通用的 gpt-5.6-sol：Zen 上游对它返回 403「Model access is
    // disabled」（2026-10-06 真 Key 实测），gpt-6-sol 经 Codex 跑通工具调用。
    name: "OpenCode Zen",
    family: "opencode",
    planKey: "payg",
    websiteUrl: "https://opencode.ai/zen",
    apiKeyUrl: "https://opencode.ai/auth",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "opencode_zen",
      "https://opencode.ai/zen/v1",
      "gpt-6-sol",
    ),
    endpointCandidates: ["https://opencode.ai/zen/v1"],
    category: "aggregator",
    icon: "opencode",
    iconColor: "#211E1E",
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "cherryin",
      "https://open.cherryin.net/v1",
      "openai/gpt-6.1-sol",
    ),
    endpointCandidates: ["https://open.cherryin.net/v1"],
    category: "aggregator",
    icon: "cherryin",
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "openrouter",
      "https://openrouter.ai/api/v1",
      "openai/gpt-6.1-sol",
    ),
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6566F1",
  },
  {
    name: "Command Code",
    websiteUrl: "https://commandcode.ai",
    apiKeyUrl: "https://commandcode.ai/settings/keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "command_code",
      "https://api.commandcode.ai/provider/v1",
      "deepseek/deepseek-v4.1-flash",
    ),
    endpointCandidates: ["https://api.commandcode.ai/provider/v1"],
    // Claude models are only available on /provider/v1/messages. Keep the
    // Codex Responses catalog limited to models accepted by
    // /provider/v1/responses.
    modelCatalog: modelCatalog([
      {
        model: "deepseek/deepseek-v4.1-flash",
        displayName: "DeepSeek V4.1 Flash",
        contextWindow: 1000000,
      },
      {
        model: "z-ai/glm-5.3-flash",
        displayName: "GLM-5.3 Flash",
        contextWindow: 1048576,
      },
      // Qwen3.8-Flash 只开放 /chat/completions，换成支持 /responses 的 Max
      {
        model: "Qwen/Qwen3.8-Max",
        displayName: "Qwen 3.8 Max",
        contextWindow: 1000000,
      },
    ]),
    category: "third_party",
    icon: "commandcode",
  },
  {
    name: "模力方舟",
    websiteUrl: "https://moark.com",
    apiKeyUrl: "https://moark.com/dashboard/tokens",
    auth: generateThirdPartyAuth(""),
    // 官方文档（CC Switch 快速配置）：Codex 走 /v1 的 Responses 原生协议
    config: generateThirdPartyConfig(
      "moark",
      "https://moark.com/v1",
      "deepseek-v4-flash-0731",
    ),
    endpointCandidates: ["https://moark.com/v1"],
    category: "aggregator",
    icon: "moark",
  },
];
