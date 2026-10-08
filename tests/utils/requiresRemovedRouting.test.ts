import { describe, expect, it } from "vitest";
import type { AppId } from "@/lib/api";
import type { Provider, ProviderMeta } from "@/types";
import { requiresRemovedRouting } from "@/utils/providerCapabilities";

const card = (
  settingsConfig: Record<string, unknown>,
  meta: ProviderMeta = {},
  category?: string,
): Provider => ({
  id: "p",
  name: "P",
  settingsConfig,
  meta,
  category: category as Provider["category"],
});

const claude = { env: { ANTHROPIC_BASE_URL: "https://x.example" } };
const codex = (wireApi: string) => ({
  auth: {},
  config: `model_provider = "x"\n[model_providers.x]\nbase_url = "https://x.example/v1"\nwire_api = "${wireApi}"\n`,
});
const grok = (backend: string) => ({
  config: `[models]\ndefault = "g"\n\n[model.g]\nmodel = "m"\nbase_url = "https://x.example/v1"\napi_key = "k"\napi_backend = "${backend}"\n`,
});

// 和后端 legacy_routing::tests::cards_that_only_worked_through_routing_are_recognized 同一张表
const cases: [AppId, Provider, boolean][] = [
  ["claude", card(claude), false],
  ["claude", card(claude, { apiFormat: "anthropic" }), false],
  ["claude", card(claude, { apiFormat: "openai_chat" }), true],
  ["claude", card({ env: {}, api_format: "openai_responses" }), true],
  [
    "claude",
    card({ env: {}, api_format: "openai_chat" }, { apiFormat: "anthropic" }),
    false,
  ],
  ["claude", card({ env: {}, openrouter_compat_mode: "1" }), true],
  ["claude", card({ env: {}, openrouter_compat_mode: false }), false],
  ["claude", card(claude, { isFullUrl: true }), true],
  ["claude", card(claude, { providerType: "github_copilot" }), true],
  ["claude", card(claude, { providerType: "codex_oauth" }), true],
  ["codex", card(codex("responses")), false],
  ["codex", card(codex("chat")), true],
  ["codex", card(codex("responses"), { apiFormat: "openai_chat" }), true],
  ["codex", card(codex("responses"), { apiFormat: "openai_responses" }), false],
  ["grokbuild", card(grok("responses"), { apiFormat: "openai_chat" }), true],
  [
    "grokbuild",
    card(grok("chat_completions"), { apiFormat: "openai_chat" }),
    false,
  ],
  ["grokbuild", card(grok("messages"), { apiFormat: "anthropic" }), false],
  ["grokbuild", card(grok("responses"), { providerType: "xai_oauth" }), true],
  ["gemini", card({ env: {} }, { isFullUrl: true }), false],
];

describe("requiresRemovedRouting", () => {
  it.each(cases)("%s %#", (app, provider, expected) => {
    expect(requiresRemovedRouting(app, provider)).toBe(expected);
  });

  it("does not flag official accounts", () => {
    const official = card(
      codex("responses"),
      { providerType: "codex_oauth" },
      "official",
    );
    expect(requiresRemovedRouting("codex", official)).toBe(false);
  });
});
