import { describe, expect, it } from "vitest";
import { getCacheWriteAvailability } from "@/types/usage";

describe("getCacheWriteAvailability", () => {
  it("distinguishes cache-write support across fixed protocols", () => {
    expect(getCacheWriteAvailability(["claude"])).toBe("ok");
    expect(getCacheWriteAvailability(["pi"])).toBe("partial");
    expect(getCacheWriteAvailability(["gemini", "grokbuild"])).toBe("na");
    expect(getCacheWriteAvailability(["codex"])).toBe("partial");
    expect(getCacheWriteAvailability(["claude", "gemini"])).toBe("partial");
    expect(getCacheWriteAvailability([])).toBe("ok");
  });
});
