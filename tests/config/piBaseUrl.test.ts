import { describe, expect, it } from "vitest";
import { piProviderBaseUrl } from "@/config/piProviderPresets";
import vectors from "../fixtures/pi-base-url-vectors.json";

// 后端 src-tauri/src/pi_config/mod.rs 用同一份向量断言 provider_base_url
describe("piProviderBaseUrl", () => {
  it.each(vectors.vectors)("$name", ({ providerKey, config, expected }) => {
    expect(piProviderBaseUrl(providerKey, config) ?? null).toBe(expected);
  });
});
