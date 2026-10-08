import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "@/lib/toast";
import { profilesApi, providersApi } from "@/lib/api";
import type { ProfileScope } from "@/lib/api/profiles";
import { extractErrorMessage } from "@/utils/errorUtils";

const updateTrayMenuSafely = async () => {
  try {
    await providersApi.updateTrayMenu();
  } catch (trayError) {
    console.error("Failed to update tray menu after profile change", trayError);
  }
};

export const useProfilesQuery = () => {
  return useQuery({
    queryKey: ["profiles"],
    queryFn: () => profilesApi.list(),
  });
};

export const useCreateProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({ name, scope }: { name: string; scope: ProfileScope }) =>
      profilesApi.create(name, scope),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["profiles"] });
      await updateTrayMenuSafely();
      toast.success(t("profiles.createSuccess"), { closeButton: true });
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.createFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};

export const useUpdateProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({
      id,
      name,
      resnapshot,
      scope,
    }: {
      id: string;
      name?: string;
      resnapshot?: boolean;
      scope?: ProfileScope;
    }) => profilesApi.update(id, { name, resnapshot, scope }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["profiles"] });
      await updateTrayMenuSafely();
      toast.success(t("profiles.updateSuccess"), { closeButton: true });
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.updateFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};

export const useDeleteProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (id: string) => profilesApi.delete(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["profiles"] });
      await updateTrayMenuSafely();
      toast.success(t("profiles.deleteSuccess"), { closeButton: true });
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.deleteFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};

export const useClearProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (scope: ProfileScope) => profilesApi.clearCurrent(scope),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["profiles"] });
      await updateTrayMenuSafely();
      toast.success(t("profiles.clearSuccess"), { closeButton: true });
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.applyFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};

export const useApplyProfileMutation = () => {
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({ id, scope }: { id: string; scope: ProfileScope }) =>
      profilesApi.apply(id, scope),
    // 后端 apply_profile 已重建托盘，并发出 provider-switched / profile-applied，
    // App.tsx 的监听会刷新供应商、项目、MCP 和 Skills 缓存。
    onSuccess: async (warnings) => {
      if (warnings.length > 0) {
        toast.warning(
          t("profiles.applyWarnings", {
            warningCount: warnings.length,
            details: warnings.join("\n"),
          }),
          { closeButton: true, duration: 10000 },
        );
      } else {
        toast.success(t("profiles.applySuccess"), { closeButton: true });
      }
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.applyFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};
