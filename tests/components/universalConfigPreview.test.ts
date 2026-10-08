import { describe, expect, it } from "vitest";

import {
  buildUniversalClaudeConfig,
  buildUniversalCodexConfig,
  buildUniversalGeminiConfig,
  universalCodexBaseUrl,
  UNIVERSAL_CLAUDE_DEFAULT_MODEL,
  UNIVERSAL_CODEX_DEFAULT_MODEL,
  UNIVERSAL_GEMINI_DEFAULT_MODEL,
} from "@/components/universal/universalConfigPreview";
import { universalProviderPresets } from "@/config/universalProviderPresets";

// 这些用例和 src-tauri/src/provider.rs 里 universal_provider_* 测试对应：预览要和后端同步时
// 写入的配置一致
describe("universal provider config preview", () => {
  it("uses the default Claude model when nothing is filled in", () => {
    expect(
      buildUniversalClaudeConfig("https://relay", "key", undefined).env,
    ).toMatchObject({
      ANTHROPIC_MODEL: UNIVERSAL_CLAUDE_DEFAULT_MODEL,
      ANTHROPIC_DEFAULT_HAIKU_MODEL: UNIVERSAL_CLAUDE_DEFAULT_MODEL,
      ANTHROPIC_DEFAULT_SONNET_MODEL: UNIVERSAL_CLAUDE_DEFAULT_MODEL,
      ANTHROPIC_DEFAULT_OPUS_MODEL: UNIVERSAL_CLAUDE_DEFAULT_MODEL,
    });
  });

  it("falls back to the main model for blank Haiku, Sonnet and Opus", () => {
    const { env } = buildUniversalClaudeConfig("https://relay", "key", {
      model: " relay-sonnet ",
      haikuModel: "",
      opusModel: "relay-opus",
    });
    expect(env).toMatchObject({
      ANTHROPIC_MODEL: "relay-sonnet",
      ANTHROPIC_DEFAULT_HAIKU_MODEL: "relay-sonnet",
      ANTHROPIC_DEFAULT_SONNET_MODEL: "relay-sonnet",
      ANTHROPIC_DEFAULT_OPUS_MODEL: "relay-opus",
    });
  });

  it("fills blank Codex and Gemini fields with the defaults", () => {
    const codex = buildUniversalCodexConfig(
      "https://relay.example.com",
      "key",
      {
        model: "",
        reasoningEffort: "  ",
      },
    );
    expect(codex.config).toContain(
      `model = "${UNIVERSAL_CODEX_DEFAULT_MODEL}"`,
    );
    expect(codex.config).toContain('model_reasoning_effort = "high"');

    const gemini = buildUniversalGeminiConfig("https://relay", "key", {
      model: "",
    });
    expect(gemini.env.GEMINI_MODEL).toBe(UNIVERSAL_GEMINI_DEFAULT_MODEL);
  });

  it("adds /v1 to the Codex base URL only when it has no path", () => {
    expect(universalCodexBaseUrl("https://api.example.com")).toBe(
      "https://api.example.com/v1",
    );
    expect(universalCodexBaseUrl("https://api.example.com/")).toBe(
      "https://api.example.com/v1",
    );
    expect(universalCodexBaseUrl("https://api.example.com/v1/")).toBe(
      "https://api.example.com/v1",
    );
    expect(universalCodexBaseUrl("https://relay.example.com/openai")).toBe(
      "https://relay.example.com/openai",
    );
  });

  it("matches the NewAPI preset defaults", () => {
    const newApi = universalProviderPresets.find(
      (preset) => preset.providerType === "newapi",
    );
    expect(newApi?.defaultModels.claude?.model).toBe(
      UNIVERSAL_CLAUDE_DEFAULT_MODEL,
    );
    expect(newApi?.defaultModels.codex?.model).toBe(
      UNIVERSAL_CODEX_DEFAULT_MODEL,
    );
    expect(newApi?.defaultModels.gemini?.model).toBe(
      UNIVERSAL_GEMINI_DEFAULT_MODEL,
    );
  });
});
