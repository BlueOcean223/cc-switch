import { parse as parseToml, stringify as stringifyToml } from "smol-toml";

export const GROK_BUILD_DEFAULT_MODEL = "grok-4.7";
/** Grok Build 的 `api_backend` 取值，决定请求上游的哪个接口。 */
export const GROK_BUILD_API_BACKENDS = [
  { value: "responses", label: "OpenAI Responses" },
  { value: "chat_completions", label: "OpenAI Chat Completions" },
  { value: "messages", label: "Anthropic Messages" },
] as const;
export type GrokBuildApiBackend =
  (typeof GROK_BUILD_API_BACKENDS)[number]["value"];
/** 不写 `api_backend` 时 Grok 用的接口。 */
export const GROK_BUILD_IMPLICIT_API_BACKEND: GrokBuildApiBackend =
  "chat_completions";
export const GROK_BUILD_DEFAULT_CONTEXT_WINDOW = 500000;

export interface GrokBuildConfigValues {
  /** Client-visible profile selected by [models].default. */
  model: string;
  /** Real model sent to the upstream provider. */
  upstreamModel?: string;
  baseUrl: string;
  name: string;
  apiKey: string;
  envKey?: string;
  /** config.toml 里的原值；没写时为空，保存时也不补写。 */
  apiBackend?: string;
  contextWindow: number;
}

const asRecord = (value: unknown): Record<string, unknown> | undefined =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;

const asString = (value: unknown, fallback = "") =>
  typeof value === "string" ? value : fallback;

/** `env_key` 可以是一个变量名，也可以是数组（Grok 取第一个有值的）。 */
const envKeyNames = (value: unknown): string[] =>
  (Array.isArray(value) ? value : [value])
    .filter((item): item is string => typeof item === "string")
    .map((item) => item.trim())
    .filter(Boolean);

export function parseGrokBuildConfig(
  configToml: string | undefined,
  fallbackName = "",
): GrokBuildConfigValues {
  const fallback: GrokBuildConfigValues = {
    model: GROK_BUILD_DEFAULT_MODEL,
    upstreamModel: GROK_BUILD_DEFAULT_MODEL,
    baseUrl: "",
    name: fallbackName,
    apiKey: "",
    contextWindow: GROK_BUILD_DEFAULT_CONTEXT_WINDOW,
  };

  if (!configToml?.trim()) return fallback;

  try {
    const root = asRecord(parseToml(configToml));
    const models = asRecord(root?.models);
    const defaultModel = asString(models?.default, GROK_BUILD_DEFAULT_MODEL);
    const modelTables = asRecord(root?.model);
    const selectedModel = asRecord(modelTables?.[defaultModel]);
    const rawContextWindow = selectedModel?.context_window;

    return {
      model: defaultModel,
      upstreamModel: asString(selectedModel?.model, defaultModel),
      baseUrl: asString(selectedModel?.base_url),
      name: asString(selectedModel?.name, fallbackName),
      apiKey: asString(selectedModel?.api_key),
      envKey: envKeyNames(selectedModel?.env_key)[0] ?? "",
      apiBackend: asString(selectedModel?.api_backend).trim() || undefined,
      contextWindow:
        typeof rawContextWindow === "number" &&
        Number.isInteger(rawContextWindow) &&
        rawContextWindow > 0
          ? rawContextWindow
          : GROK_BUILD_DEFAULT_CONTEXT_WINDOW,
    };
  } catch {
    return fallback;
  }
}

export function buildGrokBuildConfig(values: GrokBuildConfigValues): string {
  return updateGrokBuildConfig(undefined, values);
}

export function updateGrokBuildConfig(
  configToml: string | undefined,
  values: GrokBuildConfigValues,
): string {
  const profile = values.model.trim() || GROK_BUILD_DEFAULT_MODEL;
  const upstreamModel = values.upstreamModel?.trim() || profile;
  let config: Record<string, unknown> = {};

  try {
    config = asRecord(configToml?.trim() ? parseToml(configToml) : {}) ?? {};
  } catch {
    config = {};
  }

  const existingModels = asRecord(config.models) ?? {};
  const previousProfile = asString(existingModels.default, profile);
  config.models = { ...existingModels, default: profile };

  const modelTables = asRecord(config.model) ?? {};
  const existingSelected =
    asRecord(modelTables[profile]) ??
    asRecord(modelTables[previousProfile]) ??
    {};
  const apiKey = values.apiKey.trim();
  const apiBackend = values.apiBackend?.trim();
  const envKey = values.envKey?.trim();
  const existingEnvKeys = envKeyNames(existingSelected.env_key);
  const updatedSelected: Record<string, unknown> = {
    ...existingSelected,
    model: upstreamModel,
    base_url: values.baseUrl.trim(),
    name: values.name.trim(),
    ...(apiBackend ? { api_backend: apiBackend } : {}),
    context_window:
      Number.isInteger(values.contextWindow) && values.contextWindow > 0
        ? values.contextWindow
        : GROK_BUILD_DEFAULT_CONTEXT_WINDOW,
  };
  if (apiKey) updatedSelected.api_key = apiKey;
  else delete updatedSelected.api_key;
  // 没传或传的就是现有第一个名字时保留原值，数组形式的 env_key 不会被缩成一个名字
  if (envKey && envKey !== existingEnvKeys[0]) updatedSelected.env_key = envKey;
  else if (existingEnvKeys.length === 0) delete updatedSelected.env_key;

  config.model = {
    ...modelTables,
    [profile]: updatedSelected,
  };

  if (previousProfile !== profile && previousProfile in modelTables) {
    delete (config.model as Record<string, unknown>)[previousProfile];
  }

  return `${stringifyToml(config).trim()}\n`;
}

export function validateGrokBuildConfig(configToml: string): string | null {
  if (!configToml.trim()) return "config.toml must not be empty";
  try {
    const root = asRecord(parseToml(configToml));
    const models = asRecord(root?.models);
    const profile = asString(models?.default).trim();
    const selected = asRecord(asRecord(root?.model)?.[profile]);
    if (!profile || !selected) return "Missing [models] default model table";
    // 与后端 validate_config_toml 一致：name、api_backend、context_window 在 Grok 里可省略
    for (const field of ["model", "base_url"]) {
      if (!asString(selected[field]).trim()) return `Missing ${field}`;
    }
    if (
      !asString(selected.api_key).trim() &&
      envKeyNames(selected.env_key).length === 0
    ) {
      return "Missing api_key or env_key";
    }
    if ("api_backend" in selected && !asString(selected.api_backend).trim()) {
      return "Missing api_backend";
    }
    const contextWindow = selected.context_window;
    if (
      contextWindow !== undefined &&
      (typeof contextWindow !== "number" ||
        !Number.isInteger(contextWindow) ||
        contextWindow <= 0)
    ) {
      return "context_window must be a positive integer";
    }
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : "Invalid TOML";
  }
}

export function extractGrokBuildBaseUrl(configToml: string): string {
  return parseGrokBuildConfig(configToml).baseUrl;
}
