import { useState } from "react";
import { useTranslation } from "react-i18next";
import { emit } from "@tauri-apps/api/event";
import { Link2, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { deeplinkApi } from "@/lib/api/deeplink";
import { toast } from "@/lib/toast";

/**
 * 粘贴一键导入链接。系统只把 ccslite:// 交给 ccs-lite；供应商网站生成的
 * ccswitch:// 链接会被系统交给上游 CC Switch，用户可以复制后粘贴到这里。
 * 解析成功后发出和系统打开链接时相同的 `deeplink-import` 事件，由
 * DeepLinkImportDialog 接手确认和导入。
 */
export function DeepLinkPasteSection() {
  const { t } = useTranslation();
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);

  const handleImport = async () => {
    const trimmed = url.trim();
    if (!trimmed) return;
    setBusy(true);
    try {
      const request = await deeplinkApi.parseDeeplink(trimmed);
      await emit("deeplink-import", request);
      setUrl("");
    } catch (error) {
      toast.error(t("deeplink.parseError"), {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="space-y-3">
      <p className="text-sm text-fg-2">{t("settings.deeplinkPaste.hint")}</p>
      <div className="flex gap-2">
        <Input
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") void handleImport();
          }}
          placeholder="ccswitch://v1/import?..."
          spellCheck={false}
          className="font-mono text-xs"
        />
        <Button
          type="button"
          onClick={handleImport}
          disabled={busy || !url.trim()}
          className="shrink-0 gap-2"
        >
          {busy ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Link2 className="h-4 w-4" />
          )}
          {t("settings.deeplinkPaste.import")}
        </Button>
      </div>
    </section>
  );
}
