import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { HelpTip } from "@/components/ui/help-tip";
import { Switch } from "@/components/ui/switch";
import { APP_DISPLAY_NAME } from "@/components/shell/AppGlyph";
import { KNOWN_APP_TYPES } from "@/types/usage";
import { getResolvedLang, joinNames } from "./format";

interface UsageDataSourcesSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessionAutoSyncEnabled: boolean;
  onSessionAutoSyncEnabledChange?: (next: boolean) => void;
  /** 「刚刚同步」「N 分钟前同步」；还没手动同步过时为空 */
  syncedLabel?: string;
  syncing: boolean;
  onSyncNow: () => void;
  rebuildingUsage: boolean;
  /** 只负责打开确认框；确认框里写清后果 */
  onRebuildUsage: () => void;
}

function SourceCard({
  title,
  help,
  trailing,
  children,
}: {
  title: string;
  help: { title: string; body: string };
  trailing?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-3 rounded-panel border border-border px-4 py-3.5">
      <div className="flex items-center gap-2">
        <div className="flex min-w-0 flex-1 items-center gap-0.5">
          <h3 className="m-0 truncate text-strong font-semibold text-fg-1">
            {title}
          </h3>
          <HelpTip title={help.title}>{help.body}</HelpTip>
        </div>
        {trailing}
      </div>
      {children}
    </section>
  );
}

/** 「数据来源」抽屉（v7 S6）：会话日志扫描、用量重建。 */
export function UsageDataSourcesSheet({
  open,
  onOpenChange,
  sessionAutoSyncEnabled,
  onSessionAutoSyncEnabledChange,
  syncedLabel,
  syncing,
  onSyncNow,
  rebuildingUsage,
  onRebuildUsage,
}: UsageDataSourcesSheetProps) {
  const { t, i18n } = useTranslation();
  const coveredApps = joinNames(
    KNOWN_APP_TYPES.map((app) => APP_DISPLAY_NAME[app]),
    getResolvedLang(i18n),
  );
  const cadence = sessionAutoSyncEnabled
    ? t("usage.sources.cadenceOn")
    : t("usage.sources.cadenceOff");

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        width={400}
        closeLabel={t("common.close")}
        dismissOnOutsideClick
      >
        <SheetHeader className="pb-3">
          <SheetTitle>{t("usage.dataSources")}</SheetTitle>
          <SheetDescription className="sr-only">
            {t("usage.dataSources")}
          </SheetDescription>
        </SheetHeader>
        <SheetBody className="flex flex-col gap-3 border-t border-border">
          <SourceCard
            title={t("usage.sources.scanTitle")}
            help={{
              title: t("usage.sources.scanHelpTitle"),
              body: t("usage.sources.scanHelp"),
            }}
            trailing={
              <Switch
                checked={sessionAutoSyncEnabled}
                onCheckedChange={(value) =>
                  onSessionAutoSyncEnabledChange?.(value)
                }
                aria-label={t("usage.sources.scanTitle")}
              />
            }
          >
            <ul className="m-0 flex list-none flex-col gap-1 rounded-control bg-subtle px-3 py-2.5 text-caption text-fg-2">
              <li>{t("usage.sources.coverage", { apps: coveredApps })}</li>
              <li>{syncedLabel ? `${cadence} · ${syncedLabel}` : cadence}</li>
              <li>{t("usage.sources.unsupported")}</li>
            </ul>
            <div className="flex justify-end">
              <Button
                type="button"
                variant="neutral"
                size="compact"
                disabled={syncing}
                onClick={onSyncNow}
              >
                {syncing && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
                {t("usage.sessionSync.syncNow")}
              </Button>
            </div>
          </SourceCard>

          <SourceCard
            title={t("usage.rebuildUsage.title")}
            help={{
              title: t("usage.sources.codexHelpTitle"),
              body: t("usage.rebuildUsage.description"),
            }}
          >
            <p className="m-0 text-caption text-fg-2">
              {t("usage.rebuildUsage.warning")}
            </p>
            <div className="flex justify-end">
              <Button
                type="button"
                variant="neutral"
                size="compact"
                disabled={rebuildingUsage}
                onClick={onRebuildUsage}
              >
                {rebuildingUsage && (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                )}
                {t("usage.rebuildUsage.actionEllipsis")}
              </Button>
            </div>
          </SourceCard>
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}
