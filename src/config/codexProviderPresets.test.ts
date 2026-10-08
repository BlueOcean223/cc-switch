import { describe, expect, it } from "vitest";
import { generateThirdPartyConfig } from "./codexProviderPresets";

describe("generateThirdPartyConfig", () => {
  it("key-based third-party template keeps the fallback flag by default", () => {
    expect(
      generateThirdPartyConfig("acme", "https://api.acme.dev/v1", "m1"),
    ).toContain("requires_openai_auth = true");
  });
});
