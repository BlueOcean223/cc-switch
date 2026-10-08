import React from "react";
import { CodexAuthSection, CodexConfigSection } from "./CodexConfigSections";
import type { ProviderEditorInactiveField } from "@/lib/api/providers";

interface CodexConfigEditorProps {
  authValue: string;

  configValue: string;

  providerName?: string;

  showRemoteCompaction?: boolean;

  onAuthChange: (value: string) => void;

  onConfigChange: (value: string) => void;

  onAuthBlur?: () => void;

  authError: string;

  configError: string; // config.toml 错误提示

  /** 行里保存着、但不随切换生效的全局设置。 */
  inactiveFields?: ProviderEditorInactiveField[];
}

const CodexConfigEditor: React.FC<CodexConfigEditorProps> = ({
  authValue,
  configValue,
  providerName,
  showRemoteCompaction,
  onAuthChange,
  onConfigChange,
  onAuthBlur,
  authError,
  configError,
  inactiveFields,
}) => {
  return (
    <div className="space-y-6">
      {/* Auth JSON Section */}
      <CodexAuthSection
        value={authValue}
        onChange={onAuthChange}
        onBlur={onAuthBlur}
        error={authError}
      />

      {/* Config TOML Section */}
      <CodexConfigSection
        value={configValue}
        onChange={onConfigChange}
        providerName={providerName}
        showRemoteCompaction={showRemoteCompaction}
        configError={configError}
        inactiveFields={inactiveFields}
      />
    </div>
  );
};

export default CodexConfigEditor;
