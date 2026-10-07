import { useQuery } from "@tanstack/react-query";
import { promptsApi, type AppId } from "@/lib/api";

/**
 * 提示词页之外读提示词的地方（应用页页头的「提示词：X ›」、提示词页应用下拉里的条数）。
 * 提示词页自己的列表走 usePromptActions；写完后 invalidate `promptKeys.all`，这里跟着刷新。
 */
export const promptKeys = {
  all: ["prompts"] as const,
  list: (app: AppId) => ["prompts", "list", app] as const,
  location: (app: AppId) => ["prompts", "location", app] as const,
};

/** 支持提示词的应用，按侧栏顺序。OpenClaw 在自己的「工作区」里管理。 */
export const PROMPT_APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "grokbuild",
  "opencode",
  "hermes",
  "pi",
  "mcode",
];

export function usePromptFileLocationQuery(app: AppId) {
  return useQuery({
    queryKey: promptKeys.location(app),
    queryFn: () => promptsApi.getFileLocation(app),
    staleTime: 30_000,
  });
}
