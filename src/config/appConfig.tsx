import type { AppId } from "@/lib/api/types";
import type { VisibleApps } from "@/types";

export const APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "grokbuild",
  "opencode",
  "openclaw",
  "hermes",
  "pi",
  "mcode",
];

export const DEFAULT_VISIBLE_APPS: VisibleApps = {
  claude: true,
  codex: true,
  gemini: true,
  grokbuild: true,
  opencode: true,
  openclaw: true,
  hermes: true,
  pi: true,
  mcode: true,
};

/** App IDs shown in Skills panels. */
export const SKILLS_APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "grokbuild",
  "opencode",
  "hermes",
  "pi",
  "mcode",
];

export type AdditiveAppId = Extract<
  AppId,
  "opencode" | "openclaw" | "hermes" | "pi" | "mcode"
>;

export const ADDITIVE_APP_IDS: AdditiveAppId[] = [
  "mcode",
  "opencode",
  "openclaw",
  "hermes",
  "pi",
];

export function isAdditiveAppId(appId: string): appId is AdditiveAppId {
  return (ADDITIVE_APP_IDS as string[]).includes(appId);
}

/**
 * 切换只替换关键字段的应用：供应商编辑器显示「切到这个供应商之后配置文件的样子」，由后端
 * `ProviderService::editor_view` 投影。
 */
export const EDITOR_VIEW_APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "grokbuild",
];

export function usesEditorView(appId: AppId): boolean {
  return EDITOR_VIEW_APP_IDS.includes(appId);
}

/** OpenClaw 不由 CC Switch 管理 MCP；Pi 1.0 起内置 MCP（`~/.pi/agent/mcp.json`） */
export type McpAppId = Exclude<AppId, "openclaw">;
export const MCP_APP_IDS: McpAppId[] = [
  "claude",
  "codex",
  "gemini",
  "grokbuild",
  "opencode",
  "hermes",
  "pi",
  "mcode",
];

export function isMcpAppId(appId: string): appId is McpAppId {
  return (MCP_APP_IDS as string[]).includes(appId);
}
