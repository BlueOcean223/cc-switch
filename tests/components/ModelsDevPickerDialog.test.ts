import { describe, expect, it } from "vitest";

import {
  flattenModels,
  formatPrice,
} from "@/components/usage/ModelsDevPickerDialog";
import { normalizeModelsDevModelId } from "@/lib/modelsDev";
import {
  getCommonModelKeys,
  longContextTiers,
  resolveModelsDevSelection,
  toModelPricing,
} from "@/lib/modelsDevPricing";

describe("normalizeModelsDevModelId", () => {
  it("keeps already-normalized ids unchanged", () => {
    expect(normalizeModelsDevModelId("claude-opus-4-5")).toBe(
      "claude-opus-4-5",
    );
  });

  it("strips the vendor prefix before the last slash", () => {
    expect(normalizeModelsDevModelId("z-ai/glm-4.7")).toBe("glm-4.7");
    expect(normalizeModelsDevModelId("clarifai/main/models/mm-poly-8b")).toBe(
      "mm-poly-8b",
    );
  });

  it("lowercases the id", () => {
    expect(normalizeModelsDevModelId("MiniMaxAI/MiniMax-M2.1")).toBe(
      "minimax-m2.1",
    );
  });

  it("truncates colon suffixes", () => {
    expect(normalizeModelsDevModelId("claude-sonnet-4-thinking:8192")).toBe(
      "claude-sonnet-4-thinking",
    );
  });

  it("maps @ to -", () => {
    expect(normalizeModelsDevModelId("claude-sonnet-4@20250514")).toBe(
      "claude-sonnet-4-20250514",
    );
  });

  it("strips the [1m] context marker", () => {
    expect(normalizeModelsDevModelId("claude-sonnet-4-5[1m]")).toBe(
      "claude-sonnet-4-5",
    );
  });

  it("combines all rules", () => {
    expect(normalizeModelsDevModelId("Vendor/Claude-Sonnet-4@2025:free")).toBe(
      "claude-sonnet-4-2025",
    );
  });
});

describe("formatPrice", () => {
  it("formats integers without a decimal point", () => {
    expect(formatPrice(5)).toBe("5");
    expect(formatPrice(25)).toBe("25");
  });

  it("trims trailing zeros", () => {
    expect(formatPrice(0.5)).toBe("0.5");
    expect(formatPrice(6.25)).toBe("6.25");
    expect(formatPrice(1.0395)).toBe("1.0395");
  });

  it("keeps up to six decimal places", () => {
    expect(formatPrice(0.000001)).toBe("0.000001");
    expect(formatPrice(0.0000004)).toBe("0");
  });

  it("returns 0 for zero, negative and non-finite values", () => {
    expect(formatPrice(0)).toBe("0");
    expect(formatPrice(-1)).toBe("0");
    expect(formatPrice(NaN)).toBe("0");
    expect(formatPrice(Infinity)).toBe("0");
  });

  it("never produces exponent notation", () => {
    // 后端 Decimal::from_str 不接受科学计数法
    expect(formatPrice(1e-8)).toBe("0");
    expect(formatPrice(1e21)).toBe("0");
    for (const value of [5, 0.5, 0.000123, 123456.789]) {
      expect(formatPrice(value)).toMatch(/^\d+(\.\d+)?$/);
    }
  });
});

describe("flattenModels", () => {
  it("flattens providers, fills defaults and sorts by release date desc", () => {
    const entries = flattenModels({
      acme: {
        id: "acme",
        name: "Acme AI",
        models: {
          "old-model": {
            id: "old-model",
            name: "Old Model",
            release_date: "2024-01-01",
            cost: { input: 1, output: 2 },
          },
          "new-model": {
            id: "new-model",
            name: "New Model",
            release_date: "2025-06-01",
            cost: { input: 3, output: 6, cache_read: 0.3, cache_write: 3.75 },
          },
          "free-model": {
            id: "free-model",
            name: "No Cost Model",
          },
        },
      },
      bare: {
        models: {
          "Vendor/Some-Model:free": {
            release_date: "2025-01",
            cost: { input: 0.1 },
          },
        },
      },
    });

    expect(entries.map((e) => e.key)).toEqual([
      "acme/new-model",
      "bare/Vendor/Some-Model:free",
      "acme/old-model",
    ]);

    const newModel = entries[0];
    expect(newModel.normalizedId).toBe("new-model");
    expect(newModel.cacheRead).toBe(0.3);
    expect(newModel.cacheWrite).toBe(3.75);

    // 没给缓存写入价时按输入价算
    expect(entries[2].cacheWrite).toBe(1);

    // 没有 name 的 provider 用 id 兜底；缺失的成本字段补 0
    const bareModel = entries[1];
    expect(bareModel.providerName).toBe("bare");
    expect(bareModel.normalizedId).toBe("some-model");
    expect(bareModel.output).toBe(0);
    expect(bareModel.cacheRead).toBe(0);

    // 完全没有定价的模型被过滤
    expect(entries.some((e) => e.modelId === "free-model")).toBe(false);
  });

  it("filters deprecated and non-text output models while keeping multimodal input models", () => {
    const entries = flattenModels({
      acme: {
        models: {
          "multimodal-chat": {
            name: "Multimodal Chat",
            modalities: {
              input: ["text", "image", "audio", "video"],
              output: ["text"],
            },
            cost: { input: 1, output: 2 },
          },
          "legacy-chat": {
            status: "deprecated",
            modalities: { output: ["text"] },
            cost: { input: 1, output: 2 },
          },
          "speech-model": {
            modalities: { output: ["audio"] },
            cost: { input: 1, output: 2 },
          },
          "mixed-output-model": {
            modalities: { output: ["text", "audio"] },
            cost: { input: 1, output: 2 },
          },
          "movie-generator": {
            modalities: { output: ["video"] },
            cost: { input: 1, output: 2 },
          },
          "fallback-video-model": {
            cost: { input: 1, output: 2 },
          },
        },
      },
    });

    expect(entries.map((entry) => entry.modelId)).toEqual(["multimodal-chat"]);
  });

  it("selects a bounded canonical set of common model families", () => {
    const openAiModels = Object.fromEntries(
      Array.from({ length: 7 }, (_, index) => {
        const version = index + 1;
        return [
          `gpt-${version}`,
          {
            name: `GPT ${version}`,
            release_date: `2025-0${version}-01`,
            cost: { input: version, output: version * 2 },
          },
        ];
      }),
    );
    const entries = flattenModels({
      openai: {
        name: "OpenAI",
        models: {
          ...openAiModels,
          "gpt-image-1": {
            release_date: "2026-01-01",
            cost: { input: 1, output: 2 },
          },
        },
      },
      aggregator: {
        name: "Aggregator",
        models: {
          "gpt-7": {
            release_date: "2026-02-01",
            cost: { input: 9, output: 18 },
          },
        },
      },
      anthropic: {
        name: "Anthropic",
        models: {
          "claude-sonnet-5": {
            release_date: "2026-06-01",
            cost: { input: 3, output: 15 },
          },
        },
      },
      deepseek: {
        name: "DeepSeek",
        models: {
          "deepseek-chat": {
            release_date: "2025-12-01",
            cost: { input: 0.3, output: 1.2 },
          },
        },
      },
      xiaomi: {
        name: "Xiaomi",
        models: {
          "mimo-v2.5": {
            release_date: "2026-04-01",
            cost: { input: 0.2, output: 1 },
          },
          "mimo-v2.5-tts": {
            release_date: "2026-05-01",
            cost: { input: 0.1, output: 0.5 },
          },
        },
      },
      longcat: {
        name: "LongCat",
        models: {
          "LongCat-2.0": {
            release_date: "2026-03-01",
            cost: { input: 0.4, output: 1.6 },
          },
        },
      },
    });

    const common = getCommonModelKeys(entries);
    expect(common.has("openai/gpt-image-1")).toBe(false);
    expect(common.has("aggregator/gpt-7")).toBe(false);
    expect(common.has("openai/gpt-7")).toBe(true);
    expect(common.has("openai/gpt-1")).toBe(false);
    expect(common.has("anthropic/claude-sonnet-5")).toBe(true);
    expect(common.has("deepseek/deepseek-chat")).toBe(true);
    expect(common.has("xiaomi/mimo-v2.5")).toBe(true);
    expect(common.has("xiaomi/mimo-v2.5-tts")).toBe(false);
    expect(common.has("longcat/LongCat-2.0")).toBe(true);
  });

  it("combines common and explicit selections and deduplicates normalized ids", () => {
    const entries = flattenModels({
      openai: {
        models: {
          "gpt-5": {
            name: "GPT-5 Official",
            release_date: "2025-08-01",
            cost: { input: 1, output: 2 },
          },
        },
      },
      relay: {
        models: {
          "vendor/GPT-5": {
            name: "GPT-5 Relay",
            release_date: "2025-07-01",
            cost: { input: 9, output: 18 },
          },
          "custom-model": {
            name: "Custom",
            release_date: "2025-06-01",
            cost: { input: 0.5, output: 1 },
          },
        },
      },
    });
    const selected = resolveModelsDevSelection(entries, {
      autoSyncEnabled: true,
      includeCommonModels: true,
      selectedModelKeys: ["relay/vendor/GPT-5", "relay/custom-model"],
      excludedCommonModelKeys: ["openai/gpt-5"],
      lastSyncAt: null,
      lastSyncError: null,
    });

    expect(selected.map((entry) => entry.key)).toEqual([
      "relay/vendor/GPT-5",
      "relay/custom-model",
    ]);

    const pricing = toModelPricing([
      entries.find((entry) => entry.key === "openai/gpt-5")!,
      entries.find((entry) => entry.key === "relay/vendor/GPT-5")!,
    ]);
    expect(pricing).toHaveLength(1);
    expect(pricing[0]).toMatchObject({
      modelId: "gpt-5",
      displayName: "GPT-5 Official",
      inputCostPerMillion: "1",
    });
  });
});

describe("longContextTiers", () => {
  it("turns context tiers into multipliers", () => {
    // gpt-5.5 on models.dev: prompts over 272K bill input x2, output x1.5
    expect(
      longContextTiers({
        input: 5,
        output: 30,
        cache_read: 0.5,
        tiers: [
          {
            input: 10,
            output: 45,
            cache_read: 1,
            tier: { type: "context", size: 272000 },
          },
        ],
      }),
    ).toEqual([
      {
        thresholdTokens: 272000,
        inputMultiplier: "2",
        outputMultiplier: "1.5",
      },
    ]);
  });

  it("keeps every tier, lowest threshold first", () => {
    // qwen3-max: x2 above 32K, x2.5 above 128K
    expect(
      longContextTiers({
        input: 1.2,
        output: 6,
        tiers: [
          { input: 3, output: 15, tier: { type: "context", size: 128000 } },
          { input: 2.4, output: 12, tier: { type: "context", size: 32000 } },
        ],
      }),
    ).toEqual([
      { thresholdTokens: 32000, inputMultiplier: "2", outputMultiplier: "2" },
      {
        thresholdTokens: 128000,
        inputMultiplier: "2.5",
        outputMultiplier: "2.5",
      },
    ]);
  });

  it("returns an empty list when models.dev has no price-changing tier", () => {
    expect(longContextTiers({ input: 1, output: 2 })).toEqual([]);
    expect(
      longContextTiers({
        input: 1,
        output: 2,
        tiers: [
          { input: 1, output: 2, tier: { type: "context", size: 200000 } },
        ],
      }),
    ).toEqual([]);
  });

  it("keeps the first tier of a repeated threshold", () => {
    expect(
      longContextTiers({
        input: 1,
        output: 2,
        tiers: [
          { input: 2, output: 3, tier: { type: "context", size: 200000 } },
          { input: 4, output: 6, tier: { type: "context", size: 200000 } },
        ],
      }),
    ).toEqual([
      {
        thresholdTokens: 200000,
        inputMultiplier: "2",
        outputMultiplier: "1.5",
      },
    ]);
  });

  it("drops only the tier that has no input price", () => {
    expect(
      longContextTiers({
        input: 1,
        output: 2,
        tiers: [
          { output: 3, tier: { type: "context", size: 128000 } },
          { input: 2, output: 4, tier: { type: "context", size: 200000 } },
        ],
      }),
    ).toEqual([
      { thresholdTokens: 200000, inputMultiplier: "2", outputMultiplier: "2" },
    ]);
    // 基础输入价是 0：一档都算不出倍率，保留现有档位
    expect(
      longContextTiers({
        input: 0,
        output: 2,
        tiers: [
          { input: 0, output: 4, tier: { type: "context", size: 200000 } },
        ],
      }),
    ).toBeUndefined();
  });

  it("prices a missing tier cache write like the tier input", () => {
    // xAI on models.dev: no cache_write at either level
    expect(
      longContextTiers({
        input: 2,
        output: 6,
        cache_read: 0.3,
        tiers: [
          {
            input: 4,
            output: 12,
            cache_read: 0.6,
            tier: { type: "context", size: 200000 },
          },
        ],
      }),
    ).toEqual([
      { thresholdTokens: 200000, inputMultiplier: "2", outputMultiplier: "2" },
    ]);
  });

  it("skips models with a tier one input-side multiplier cannot express", () => {
    // a missing middle tier would misprice prompts above it
    expect(
      longContextTiers({
        input: 1,
        output: 4,
        cache_read: 0.2,
        tiers: [
          {
            input: 2,
            output: 8,
            cache_read: 0.4,
            tier: { type: "context", size: 32000 },
          },
          {
            input: 3,
            output: 12,
            cache_read: 0.4,
            tier: { type: "context", size: 128000 },
          },
        ],
      }),
    ).toBeUndefined();
    expect(
      longContextTiers({
        input: 0.1,
        output: 0.4,
        cache_read: 0.02,
        tiers: [
          {
            input: 0.2,
            output: 0.8,
            cache_read: 0.2,
            tier: { type: "context", size: 128000 },
          },
        ],
      }),
    ).toBeUndefined();
  });

  it("is passed to the synced pricing", () => {
    const [entry] = flattenModels({
      google: {
        models: {
          "gemini-2.5-pro": {
            name: "Gemini 2.5 Pro",
            cost: {
              input: 1.25,
              output: 10,
              tiers: [
                {
                  input: 2.5,
                  output: 15,
                  tier: { type: "context", size: 200000 },
                },
              ],
            },
          },
        },
      },
    });
    expect(toModelPricing([entry])[0].longContextTiers).toEqual([
      {
        thresholdTokens: 200000,
        inputMultiplier: "2",
        outputMultiplier: "1.5",
      },
    ]);
  });
});
