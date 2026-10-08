import type { UniversalProviderModels } from "@/types";

// 统一供应商同步到各应用时写入的配置，用于表单里的预览。默认值和补全规则与后端
// `UniversalProvider::to_*_provider`（src-tauri/src/provider.rs）相同，改一边要改另一边。

export const UNIVERSAL_CLAUDE_DEFAULT_MODEL = "claude-sonnet-5-5";
export const UNIVERSAL_CODEX_DEFAULT_MODEL = "gpt-5.6-sol";
export const UNIVERSAL_CODEX_DEFAULT_REASONING_EFFORT = "high";
export const UNIVERSAL_GEMINI_DEFAULT_MODEL = "gemini-3.8-flash";

/** 去掉首尾空白；空的当作没填（清空输入框后存的是空字符串） */
function filled(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

/** Claude 主模型：没填时用默认值。没填的 Haiku / Sonnet / Opus 用它。 */
export function universalClaudeMainModel(
  claude: UniversalProviderModels["claude"],
): string {
  return filled(claude?.model) ?? UNIVERSAL_CLAUDE_DEFAULT_MODEL;
}

export function buildUniversalClaudeConfig(
  baseUrl: string,
  apiKey: string,
  claude: UniversalProviderModels["claude"],
) {
  const model = universalClaudeMainModel(claude);
  return {
    env: {
      ANTHROPIC_BASE_URL: baseUrl,
      ANTHROPIC_AUTH_TOKEN: apiKey,
      ANTHROPIC_MODEL: model,
      ANTHROPIC_DEFAULT_HAIKU_MODEL: filled(claude?.haikuModel) ?? model,
      ANTHROPIC_DEFAULT_SONNET_MODEL: filled(claude?.sonnetModel) ?? model,
      ANTHROPIC_DEFAULT_OPUS_MODEL: filled(claude?.opusModel) ?? model,
    },
  };
}

/** 只有 origin 的地址补 `/v1`；已经以 `/v1` 结尾或带了其他路径的保持不变 */
export function universalCodexBaseUrl(baseUrl: string): string {
  const trimmed = baseUrl.replace(/\/+$/, "");
  if (trimmed.endsWith("/v1")) return trimmed;
  const schemeIndex = trimmed.indexOf("://");
  const rest = schemeIndex >= 0 ? trimmed.slice(schemeIndex + 3) : trimmed;
  return rest.includes("/") ? trimmed : `${trimmed}/v1`;
}

export function buildUniversalCodexConfig(
  baseUrl: string,
  apiKey: string,
  codex: UniversalProviderModels["codex"],
) {
  const model = filled(codex?.model) ?? UNIVERSAL_CODEX_DEFAULT_MODEL;
  const reasoningEffort =
    filled(codex?.reasoningEffort) ?? UNIVERSAL_CODEX_DEFAULT_REASONING_EFFORT;
  const config = `model_provider = "custom"
model = "${model}"
model_reasoning_effort = "${reasoningEffort}"

[model_providers.custom]
name = "NewAPI"
base_url = "${universalCodexBaseUrl(baseUrl)}"
wire_api = "responses"
requires_openai_auth = true`;
  return {
    auth: {
      OPENAI_API_KEY: apiKey,
    },
    config,
  };
}

export function buildUniversalGeminiConfig(
  baseUrl: string,
  apiKey: string,
  gemini: UniversalProviderModels["gemini"],
) {
  return {
    env: {
      GOOGLE_GEMINI_BASE_URL: baseUrl,
      GEMINI_API_KEY: apiKey,
      GEMINI_MODEL: filled(gemini?.model) ?? UNIVERSAL_GEMINI_DEFAULT_MODEL,
    },
  };
}
