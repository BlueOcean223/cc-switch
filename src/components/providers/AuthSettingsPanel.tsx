import { useTranslation } from "react-i18next";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { AuthCenterPanel } from "@/components/settings/AuthCenterPanel";

interface AuthSettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export function AuthSettingsPanel({ isOpen, onClose }: AuthSettingsPanelProps) {
  const { t } = useTranslation();

  return (
    <FullScreenPanel
      isOpen={isOpen}
      title={t("nav.auth")}
      onClose={onClose}
      motionPreset="slide-from-right"
    >
      {isOpen ? <AuthCenterPanel /> : null}
    </FullScreenPanel>
  );
}
