import { describe, expect, it } from "vitest";
import {
  grokBuildOfficialPreset,
  grokBuildProviderPresets,
} from "./grokBuildProviderPresets";
import {
  extractCodexBaseUrl,
  extractCodexModelName,
} from "../utils/providerConfigUtils";
import { GROK_BUILD_API_BACKENDS } from "../utils/grokBuildConfig";

describe("grokBuildProviderPresets", () => {
  it("has unique preset names", () => {
    const names = grokBuildProviderPresets.map((p) => p.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it("contains no official, managed-OAuth, or cn_official providers", () => {
    for (const preset of grokBuildProviderPresets) {
      expect(preset.category, preset.name).not.toBe("official");
      expect(preset.category, preset.name).not.toBe("cn_official");
      expect(preset.isOfficial, preset.name).toBeFalsy();
    }
  });

  it("excludes providers deliberately kept off the roster", () => {
    const names = new Set(grokBuildProviderPresets.map((p) => p.name));
    const excluded = [
      "OpenAI Official",
      "Azure OpenAI",
      "xAI (Grok) OAuth",
      "DeepSeek",
      "Kimi",
      "Kimi For Coding",
      "Zhipu GLM",
      "MiniMax",
      "SiliconFlow",
      "SiliconFlow en",
      "ModelScope",
      "Novita AI",
      "Nvidia",
      "AtlasCloud",
      // 上游已有 grok-4.5（2026-08 起），但订阅制网关是否收录待产品决策，
      // 目前按刻意排除锁定（理由不再是"上游无 Grok"）。
      "OpenCode Go",
    ];
    for (const name of excluded) {
      expect(names.has(name), name).toBe(false);
    }
  });

  it("uses a Grok default model on every preset", () => {
    for (const preset of grokBuildProviderPresets) {
      const model = extractCodexModelName(preset.config);
      expect(model, preset.name).toMatch(/^(x-ai\/)?grok-/);
    }
  });

  it("carries a valid config carrier and empty API key slot", () => {
    for (const preset of grokBuildProviderPresets) {
      expect(extractCodexBaseUrl(preset.config), preset.name).toMatch(
        /^https:\/\//,
      );
      expect(preset.auth, preset.name).toEqual({ OPENAI_API_KEY: "" });
    }
  });

  it("names an api_backend Grok Build supports on every preset", () => {
    const supported = GROK_BUILD_API_BACKENDS.map((option) => option.value);
    for (const preset of grokBuildProviderPresets) {
      expect(supported, preset.name).toContain(preset.apiBackend);
    }
  });

  it("keeps the official preset as an empty-config seed entry", () => {
    expect(grokBuildOfficialPreset.category).toBe("official");
    expect(grokBuildOfficialPreset.isOfficial).toBe(true);
    expect(grokBuildOfficialPreset.config).toBe("");
    expect(grokBuildOfficialPreset.auth).toEqual({});
  });
});
