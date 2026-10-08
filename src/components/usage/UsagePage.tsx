import { useSettings } from "@/hooks/useSettings";
import type { SettingsFormState } from "@/hooks/useSettings";
import type { AppTypeFilter } from "@/types/usage";
import { UsageDashboard } from "./UsageDashboard";

interface UsagePageProps {
  /** 从应用页「查看此应用的用量」进入时带上的应用筛选 */
  initialAppType?: AppTypeFilter;
}

/** 侧栏「用量统计」全局页（v7 S6）：数据来自各应用的会话日志。 */
export function UsagePage({ initialAppType }: UsagePageProps = {}) {
  const { settings, updateSettings, autoSaveSettings } = useSettings();

  const save = (updates: Partial<SettingsFormState>) => {
    updateSettings(updates);
    void autoSaveSettings(updates).catch(() => undefined);
  };

  return (
    <UsageDashboard
      refreshIntervalMs={settings?.usageDashboardRefreshIntervalMs}
      onRefreshIntervalChange={(usageDashboardRefreshIntervalMs) =>
        save({ usageDashboardRefreshIntervalMs })
      }
      sessionAutoSyncEnabled={settings?.sessionAutoSyncEnabled ?? true}
      onSessionAutoSyncEnabledChange={(sessionAutoSyncEnabled) =>
        save({ sessionAutoSyncEnabled })
      }
      initialAppType={initialAppType}
    />
  );
}
