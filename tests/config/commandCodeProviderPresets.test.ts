import { describe, expect, it } from "vitest";
import { codexProviderPresets } from "@/config/codexProviderPresets";

const CODEX_BASE_URL = "https://api.commandcode.ai/provider/v1";

describe("Command Code provider presets", () => {
  it("adds the Codex preset with a native Responses non-Claude catalog", () => {
    const preset = codexProviderPresets.find(
      (item) => item.name === "Command Code",
    );

    expect(preset).toBeDefined();
    expect(preset?.endpointCandidates).toEqual([CODEX_BASE_URL]);
    expect(preset?.config).toContain(`base_url = "${CODEX_BASE_URL}"`);
    expect(preset?.config).toContain('model = "deepseek/deepseek-v4.1-flash"');
    expect(preset?.config).toContain('wire_api = "responses"');
    expect(preset?.modelCatalog?.map((model) => model.model)).toEqual([
      "deepseek/deepseek-v4.1-flash",
      "z-ai/glm-5.3-flash",
      "Qwen/Qwen3.8-Max",
    ]);
    expect(
      preset?.modelCatalog?.some((model) => model.model.includes("claude")),
    ).toBe(false);
  });
});
