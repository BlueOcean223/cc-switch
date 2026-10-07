import type { AppId } from "@/lib/api";
import { ProviderIcon } from "@/components/ProviderIcon";
import { cn } from "@/lib/utils";

/** 应用名：品牌名不翻译，四种语言都写原文。 */
export const APP_DISPLAY_NAME: Record<AppId, string> = {
  claude: "Claude Code",
  codex: "Codex",
  gemini: "Gemini CLI",
  grokbuild: "Grok Build",
  opencode: "OpenCode",
  openclaw: "OpenClaw",
  hermes: "Hermes",
  pi: "Pi",
  mcode: "MiniMax Code",
};

const APP_ICON_NAME: Record<AppId, string> = {
  claude: "claude",
  codex: "openai",
  gemini: "gemini",
  grokbuild: "grok",
  opencode: "opencode",
  openclaw: "openclaw",
  hermes: "hermes",
  pi: "pi",
  mcode: "minimax",
};

interface AppGlyphProps {
  app: AppId;
  /** 图标边长：侧栏 16，页头 20 */
  size?: number;
  className?: string;
}

/** 应用图标（装饰性，名称由旁边的文字或按钮的 aria-label 提供）。 */
export function AppGlyph({ app, size = 16, className }: AppGlyphProps) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "relative inline-flex shrink-0 items-center justify-center",
        className,
      )}
      style={{ width: size + 2, height: size + 2 }}
    >
      <ProviderIcon
        icon={APP_ICON_NAME[app]}
        name=""
        size={size}
        showFallback={false}
      />
    </span>
  );
}
