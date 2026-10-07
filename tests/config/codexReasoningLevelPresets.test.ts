import { describe, expect, it } from "vitest";
import { codexProviderPresets } from "@/config/codexProviderPresets";

// 预填口径（2026-08-15 官方文档盘点 + Jason 同日拍板"表单可见性优先"）：
// 填厂商官方声明的真实差异化档位子集（含照抄 DeepSeek 官方 catalog 镜像、
// MiniMax/MiMo 与模板默认相同的显式声明——表单显示"未设置"的误导比快照
// 过时/冗余声明的代价更大）。后端按行应用这些覆盖
// （apply_codex_reasoning_level_override）。
// 后端 codex_canonical_efforts 对未知值静默丢弃——预设里的拼写错误不会报错，
// 只会让 Codex 选择器静默少档/错档，所以白名单校验必须在测试层兜住。
const CANONICAL_EFFORTS = [
  "none",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
  "ultra",
];

function catalogModel(presetName: string, modelId: string) {
  const preset = codexProviderPresets.find((item) => item.name === presetName);
  expect(preset, `preset ${presetName}`).toBeDefined();
  const model = (preset?.modelCatalog ?? []).find(
    (item) => item.model === modelId,
  );
  expect(model, `${presetName} catalog model ${modelId}`).toBeDefined();
  return model!;
}

describe("Codex preset pre-filled reasoning levels", () => {
  // 每条期望值都对应官方文档证据（见预设文件内注释）；改动任一侧前先核对来源。
  // 第四位=期望的显式 defaultReasoningLevel：官方 catalog 声明的默认值，
  // 或与预设 config.toml 的显式档位保持一致。
  const EXPECTED: Array<[string, string, string[], string?]> = [
    // 火山官方 Codex 接入文档四份一致：low/medium/high
    ["火山 Agent Plan", "ark-code-latest", ["low", "medium", "high"]],
    ["火山 Coding Plan", "ark-code-latest", ["low", "medium", "high"]],
    // 方舟深度思考文档：本模型无限制的通用四档（minimal=关思考直接回答）
    [
      "Volcengine Doubao",
      "doubao-seed-2-1-pro-260628",
      ["minimal", "low", "medium", "high"],
    ],
    // 混元官方枚举 low/high；hy3 开源 chat template 对其他值直接 raise
    ["Tencent Hunyuan", "hy3", ["low", "high"]],
    ["Tencent Hunyuan", "hy3-preview", ["low", "high"]],
    ["Tencent Hunyuan", "hy4-preview", ["none", "high"], "high"],
    // LongCat 无档位可调：全站唯一 effort 证据=官方示例的 high
    ["Longcat", "LongCat-2.0", ["high"]],
    // xAI Reasoning guide 模型级枚举；grok-4.5 不可关思考故无 none
    ["xAI (Grok)", "grok-4.5", ["low", "medium", "high", "xhigh"]],
    // DeepSeek 直连照抄官方 catalog 镜像（Jason 2026-08-15 拍板：表单可见性
    // 优先，接受快照过时风险——官方目录变更时须同步）
    ["DeepSeek", "deepseek-flash", ["low", "high", "max"]],
    ["DeepSeek", "deepseek-v4-pro", ["low", "high", "max"]],
    // MiniMax 官方 catalog=none/high；MiMo 2026-09-23 官方目录为四档、默认 low。
    ["MiniMax", "MiniMax-M3", ["none", "high"]],
    ["MiniMax en", "MiniMax-M3", ["none", "high"]],
    ["Xiaomi MiMo", "mimo-v2.5-pro", ["none", "low", "medium", "high"], "low"],
    ["Xiaomi MiMo", "mimo-v2.5", ["none", "low", "medium", "high"], "low"],
    [
      "Xiaomi MiMo Token Plan (China)",
      "mimo-v2.5-pro",
      ["none", "low", "medium", "high"],
      "low",
    ],
    [
      "Xiaomi MiMo Token Plan (China)",
      "mimo-v2.5",
      ["none", "low", "medium", "high"],
      "low",
    ],
    // 智谱官方 Codex 接入页自带 models.json（docs.bigmodel.cn/cn/coding-plan/tool/
    // codex、docs.z.ai/devpack/tool/codex，2026-09-04 核对）：glm-5.3 档位
    // low/high/max、默认 max（≠ 后端回落的模板默认 high，故显式声明）；
    // glm-5-turbo 官方档位为空、默认 max——cc-switch 表达不了空档位（会回落
    // 到模板 none/high，而 none 在原生直连下没有转换层兜底、会原样发给严格
    // 网关），按官方默认收成单档 max
    ["Zhipu GLM", "glm-5.3", ["low", "high", "max"], "max"],
    ["Zhipu GLM", "glm-5-turbo", ["max"]],
    ["Zhipu GLM en", "glm-5.3", ["low", "high", "max"], "max"],
    // BytePlus 国际站已切原生 Responses，档位=官方 Codex 文档三档（与国内
    // 站火山双 Plan 同源交叉印证）
    ["BytePlus", "ark-code-latest", ["low", "medium", "high"]],
    // Kimi 开放平台（原生 Responses 直连，默认模型 kimi-k3）：k3 三档不声明
    // default——native 模板默认 high ∈ 子集故后端保留 high（= 预设
    // config.toml 的 model_reasoning_effort），官方默认 max 只是 API 侧未显式
    // 传 effort 时的行为；k2.7-code 始终思考且官方标注不支持 effort → 单档。
    // 均关不掉思考无 none
    ["Kimi", "kimi-k3", ["low", "high", "max"]],
    ["Kimi", "kimi-k2.7-code", ["high"]],
    // Kimi Code 端点（原生 Responses 直连）：k3/k3-256k 官方 models.json 明写
    // default_reasoning_level "high"，与 native 模板回落值相同——照抄官方目录
    // 的显式声明（表单可见性优先，MiniMax/MiMo 先例）故仍有第四位期望；
    // 标准 kimi-for-coding 已升级到 K2.8 Preview；highspeed 保留原来的单档目录。
    ["Kimi For Coding", "kimi-for-coding", ["low", "high", "max"], "high"],
    ["Kimi For Coding", "kimi-for-coding-highspeed", ["high"]],
    ["Kimi For Coding", "k3", ["low", "high", "max"], "high"],
    ["Kimi For Coding", "k3-256k", ["low", "high", "max"], "high"],
    // 千问官方 Codex 页只发布一份 model-catalog.local.json，且该元数据段落
    // 位于套餐分页之前（help.aliyun.com/zh/model-studio/codex，2026-09-08
    // 核对）：qwen3.8-max 档位 low/medium/xhigh、默认 xhigh（≠ 模板回落的
    // none/high，故显式声明）；按量付费与 Token Plan 同源同一份
    ["千问AI平台", "qwen3.8-max", ["low", "medium", "xhigh"], "xhigh"],
    ["千问AI平台", "qwen3.8-2.4t-a95b", ["low", "medium", "xhigh"], "xhigh"],
    ["千问AI平台", "qwen3.8-27b", ["low", "medium", "xhigh"], "xhigh"],
    ["QwenCloud", "qwen3.8-2.4t-a95b", ["low", "medium", "xhigh"], "xhigh"],
    ["QwenCloud", "qwen3.8-27b", ["low", "medium", "xhigh"], "xhigh"],
  ];

  it.each(EXPECTED)(
    "%s / %s declares the vendor-documented levels",
    (presetName, modelId, levels, expectedDefault) => {
      const model = catalogModel(presetName, modelId);
      expect(model.reasoningLevels).toEqual(levels);
      // 显式默认来源见各预设注释；未声明时仍保留后端的 fallback 行为。
      expect(model.defaultReasoningLevel).toBe(expectedDefault);
    },
  );

  it("only ever declares canonical Codex efforts", () => {
    for (const preset of codexProviderPresets) {
      for (const model of preset.modelCatalog ?? []) {
        for (const level of model.reasoningLevels ?? []) {
          expect(
            CANONICAL_EFFORTS,
            `${preset.name}/${model.model} level "${level}"`,
          ).toContain(level);
        }
        if (model.defaultReasoningLevel !== undefined) {
          expect(model.reasoningLevels ?? []).toContain(
            model.defaultReasoningLevel,
          );
        }
      }
    }
  });
});
