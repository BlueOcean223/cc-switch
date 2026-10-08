import { describe, expect, it } from "vitest";
import {
  getCacheWriteAvailability,
  isUnpricedModelStat,
  isUnpricedUsage,
} from "@/types/usage";

describe("getCacheWriteAvailability", () => {
  it("distinguishes cache-write support across fixed protocols", () => {
    expect(getCacheWriteAvailability(["claude"])).toBe("ok");
    expect(getCacheWriteAvailability(["pi"])).toBe("partial");
    expect(getCacheWriteAvailability(["gemini"])).toBe("na");
    expect(getCacheWriteAvailability(["grokbuild"])).toBe("ok");
    expect(getCacheWriteAvailability(["gemini", "grokbuild"])).toBe("partial");
    expect(getCacheWriteAvailability(["codex"])).toBe("partial");
    expect(getCacheWriteAvailability(["claude", "gemini"])).toBe("partial");
    expect(getCacheWriteAvailability([])).toBe("ok");
  });
});

describe("unpriced usage", () => {
  const log = {
    inputTokens: 10,
    outputTokens: 2,
    cacheReadTokens: 0,
    cacheCreationTokens: 0,
    totalCostUsd: "0",
    statusCode: 200,
  };

  it("marks only models missing from the pricing table", () => {
    expect(isUnpricedUsage({ ...log, hasPricing: false })).toBe(true);
    expect(isUnpricedUsage({ ...log, hasPricing: true })).toBe(false);
    expect(
      isUnpricedUsage({ ...log, totalCostUsd: "0.01", hasPricing: false }),
    ).toBe(false);

    const stat = { totalTokens: 12, totalCost: "0.000000" };
    expect(isUnpricedModelStat({ ...stat, hasPricing: false })).toBe(true);
    expect(isUnpricedModelStat({ ...stat, hasPricing: true })).toBe(false);
    expect(
      isUnpricedModelStat({ ...stat, totalTokens: 0, hasPricing: false }),
    ).toBe(false);
  });
});
