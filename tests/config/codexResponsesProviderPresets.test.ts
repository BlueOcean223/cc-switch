import { describe, expect, it } from "vitest";
import { codexProviderPresets } from "@/config/codexProviderPresets";
import {
  extractCodexBaseUrl,
  extractCodexModelName,
} from "@/utils/providerConfigUtils";

describe("Codex Responses provider presets", () => {
  // cc-switch 只写配置、不做协议转换：第三方预设必须是厂商原生的 Responses 端点
  it("only ships presets that speak Responses directly", () => {
    for (const preset of codexProviderPresets) {
      if (preset.category === "official") continue;
      expect(preset.config, preset.name).toMatch(/^wire_api = "responses"$/m);
    }
  });

  it.each([
    ["StepFun API", "https://api.stepfun.com/v1", "step-3.7-flash"],
    ["StepFun API en", "https://api.stepfun.ai/v1", "step-3.7-flash"],
    ["Baidu Qianfan", "https://qianfan.baidubce.com/v2", "deepseek-v4-pro"],
    [
      "Astron Coding Plan",
      "https://maas-coding-api.cn-huabei-1.xf-yun.com/v1",
      "astron-code-latest",
    ],
  ])(
    "connects %s to its native Responses endpoint",
    (name, baseUrl, modelId) => {
      const preset = codexProviderPresets.find((item) => item.name === name);

      expect(preset, `${name} preset`).toBeDefined();
      expect(extractCodexBaseUrl(preset?.config)).toBe(baseUrl);
      expect(extractCodexModelName(preset?.config)).toBe(modelId);
      expect(preset?.endpointCandidates).toContain(baseUrl);
      expect(preset?.modelCatalog?.[0]?.model).toBe(modelId);
    },
  );

  it("keeps open-weight Qwen models scoped to pay-as-you-go catalogs", () => {
    for (const name of ["千问AI平台", "QwenCloud"]) {
      const preset = codexProviderPresets.find((item) => item.name === name);
      expect(preset, name).toBeDefined();
      expect(preset?.modelCatalog).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            model: "qwen3.8-2.4t-a95b",
            inputModalities: ["text"],
          }),
          expect.objectContaining({
            model: "qwen3.8-27b",
            inputModalities: ["text", "image"],
          }),
        ]),
      );
    }
    for (const name of ["千问AI平台 Token Plan", "QwenCloud Token Plan"]) {
      const preset = codexProviderPresets.find((item) => item.name === name);
      expect(preset, name).toBeDefined();
      const models = preset?.modelCatalog?.map((row) => row.model) ?? [];
      expect(models).not.toContain("qwen3.8-2.4t-a95b");
      expect(models).not.toContain("qwen3.8-27b");
    }
  });

  it("pins the Responses endpoint and model catalog of native presets", () => {
    const nativeResponsesPresets = new Map<
      string,
      { baseUrl?: string; contextWindows: Record<string, number> }
    >([
      // 官方 Codex 文档确认 Agent Plan /api/plan/v3 与 Coding Plan
      // /api/coding/v3 均支持 Responses API；BytePlus 国际站 coding/v3
      // 同（docs.byteplus.com/en/docs/ModelArk/2556056，2026-08-15 核实）
      ["火山 Agent Plan", { contextWindows: { "ark-code-latest": 256000 } }],
      ["火山 Coding Plan", { contextWindows: { "ark-code-latest": 256000 } }],
      ["BytePlus", { contextWindows: { "ark-code-latest": 256000 } }],
      [
        "Volcengine Doubao",
        { contextWindows: { "doubao-seed-2-1-pro-260628": 262144 } },
      ],
      [
        "千问AI平台",
        {
          contextWindows: {
            "qwen3.8-max": 983616,
            "qwen3.8-2.4t-a95b": 983616,
            "qwen3.8-27b": 983616,
          },
        },
      ],
      // 腾讯 TokenHub 官方 Codex 文档确认 hy3 原生 Responses（2026-07-14）
      [
        "Tencent Hunyuan",
        {
          contextWindows: {
            hy3: 256000,
            "hy3-preview": 256000,
            "hy4-preview": 960000,
          },
        },
      ],
      // DeepSeek 官方 Codex 文档的 V4.1 Flash 使用 deepseek-flash；
      // catalog 由后端按 deepseek.com host 镜像官方 models.json 生成
      [
        "DeepSeek",
        {
          contextWindows: {
            "deepseek-flash": 1048576,
            "deepseek-v4-pro": 1048576,
          },
        },
      ],
      ["Longcat", { contextWindows: { "LongCat-2.0": 1048576 } }],
      [
        "MiniMax",
        {
          baseUrl: "https://api.minimax.cn/v1",
          contextWindows: { "MiniMax-M3": 1000000 },
        },
      ],
      ["MiniMax en", { contextWindows: { "MiniMax-M3": 1000000 } }],
      [
        "Xiaomi MiMo",
        {
          contextWindows: {
            "mimo-v2.5-pro": 1048576,
            "mimo-v2.5": 1048576,
            "mimo-v2.6-pro": 1048576,
            "mimo-v2.6-flash": 1048576,
            "mimo-v2.6-pro-ultraspeed": 1048576,
          },
        },
      ],
      [
        "Xiaomi MiMo Token Plan (China)",
        {
          contextWindows: {
            "mimo-v2.5-pro": 1048576,
            "mimo-v2.5": 1048576,
            "mimo-v2.6-pro": 1048576,
            "mimo-v2.6-flash": 1048576,
          },
        },
      ],
      // 智谱三端点分立（Anthropic /api/anthropic、Chat /api/coding/paas/v4、
      // Responses /api/v1），官方明示错误端点无法使用 Coding Plan 套餐额度——
      // 原生 Responses 预设必须锁在 /api/v1（#6944；docs.bigmodel.cn/cn/coding-plan/
      // tool/codex 与 docs.z.ai/devpack/tool/codex 自带 models.json，2026-09-04 核对：
      // 国内站 glm-5.3 + glm-5-turbo，国际站仅 glm-5.3）
      [
        "Zhipu GLM",
        {
          baseUrl: "https://open.bigmodel.cn/api/v1",
          contextWindows: {
            "glm-5.3": 1048576,
            "glm-5.3-flash": 1048576,
            "glm-5-turbo": 204800,
          },
        },
      ],
      [
        "Zhipu GLM en",
        {
          baseUrl: "https://api.z.ai/api/v1",
          contextWindows: { "glm-5.3": 1048576, "glm-5.3-flash": 1048576 },
        },
      ],
      // Kimi 两份官方 Codex 接入文档均要求 wire_api = "responses"，并明写服务
      // 端原生实现 Responses API、无需协议转换（platform.kimi.com/
      // docs/guide/codex-kimi.md 与 kimi.com/code/docs/third-party-tools/
      // codex.html，2026-09-09 真 Key 探针复核）
      [
        "Kimi",
        {
          baseUrl: "https://api.moonshot.cn/v1",
          contextWindows: { "kimi-k3": 1048576, "kimi-k2.7-code": 262144 },
        },
      ],
      [
        "Kimi For Coding",
        {
          baseUrl: "https://api.kimi.com/coding/v1",
          contextWindows: {
            "kimi-for-coding": 1048576,
            "kimi-for-coding-highspeed": 262144,
            k3: 1048576,
            "k3-256k": 262144,
          },
        },
      ],
    ]);

    for (const [name, expected] of nativeResponsesPresets) {
      const preset = codexProviderPresets.find((item) => item.name === name);

      expect(preset, `${name} preset`).toBeDefined();
      if (expected.baseUrl) {
        // 直连预设的 base_url 必须是厂商的 Responses 端点本身（不是同站的
        // Chat 端点）；endpointCandidates 与主地址同路径档
        expect(extractCodexBaseUrl(preset?.config)).toBe(expected.baseUrl);
        expect(preset?.endpointCandidates).toContain(expected.baseUrl);
        expect(extractCodexModelName(preset?.config)).toBe(
          preset?.modelCatalog?.[0]?.model,
        );
      }
      // 原生 Responses 预设带 modelCatalog：切换时据此生成 ~/.codex 的
      // model-catalogs.json（shell_command 编辑、不发 freeform apply_patch）。
      expect((preset?.modelCatalog ?? []).length).toBeGreaterThan(0);
      expect(
        Object.fromEntries(
          (preset?.modelCatalog ?? []).map((model) => [
            model.model,
            model.contextWindow,
          ]),
        ),
      ).toEqual(expected.contextWindows);
    }
  });
});
