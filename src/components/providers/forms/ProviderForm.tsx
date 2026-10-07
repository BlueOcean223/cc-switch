import { useEffect, useMemo, useState, useCallback, useRef } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "@/lib/toast";
import { Button } from "@/components/ui/button";
import { Form, FormField, FormItem, FormMessage } from "@/components/ui/form";
import { ImeSafeInput } from "@/components/ui/ime-safe-input";
import { providerSchema, type ProviderFormData } from "@/lib/schemas/provider";
import { providersApi, type AppId } from "@/lib/api";
import type { ProviderEditorInactiveField } from "@/lib/api/providers";
import {
  overlayClaudeProviderFields,
  withClaudeGatewayDefaults,
} from "@/utils/claudeEditorOverlay";
import { useDarkMode } from "@/hooks/useDarkMode";
import type {
  ProviderCategory,
  ProviderMeta,
  CodexCatalogModel,
  ClaudeApiKeyField,
} from "@/types";
import {
  providerPresets,
  type ProviderPreset,
} from "@/config/claudeProviderPresets";
import {
  codexProviderPresets,
  type CodexProviderPreset,
} from "@/config/codexProviderPresets";
import {
  geminiProviderPresets,
  type GeminiProviderPreset,
} from "@/config/geminiProviderPresets";
import {
  opencodeProviderPresets,
  type OpenCodeProviderPreset,
} from "@/config/opencodeProviderPresets";
import {
  openclawProviderPresets,
  rebaseOpenClawSuggestedDefaults,
  type OpenClawProviderPreset,
  type OpenClawSuggestedDefaults,
} from "@/config/openclawProviderPresets";
import {
  hermesProviderPresets,
  type HermesProviderPreset,
} from "@/config/hermesProviderPresets";
import { OpenCodeFormFields } from "./OpenCodeFormFields";
import { OpenClawFormFields } from "./OpenClawFormFields";
import { HermesFormFields } from "./HermesFormFields";
import type { UniversalProviderPreset } from "@/config/universalProviderPresets";
import {
  applyTemplateValues,
  hasApiKeyField,
} from "@/utils/providerConfigUtils";
import { mergeProviderMeta } from "@/utils/providerMetaUtils";
import {
  setCodexWireApi,
  extractCodexModelName,
  setCodexModelName as setCodexModelNameInConfig,
} from "@/utils/providerConfigUtils";
import { getCodexCustomTemplate } from "@/config/codexTemplates";
import CodexConfigEditor from "./CodexConfigEditor";
import { CommonConfigEditor } from "./CommonConfigEditor";
import GeminiConfigEditor from "./GeminiConfigEditor";
import JsonEditor from "@/components/JsonEditor";
import { Label } from "@/components/ui/label";
import { ProviderPresetSelector } from "./ProviderPresetSelector";
import { BasicFormFields } from "./BasicFormFields";
import { ClaudeFormFields } from "./ClaudeFormFields";
import { GrokBuildProviderForm } from "./GrokBuildProviderForm";
import { CodexFormFields } from "./CodexFormFields";
import { GeminiFormFields } from "./GeminiFormFields";
import { McodeProviderForm } from "./McodeProviderForm";
import { PiProviderForm } from "./PiProviderForm";
import { OmoFormFields } from "./OmoFormFields";
import { parseOmoOtherFieldsObject } from "@/types/omo";
import {
  useProviderCategory,
  useDraftEditorProjection,
  type EditorBaseChange,
  useApiKeyState,
  useBaseUrlState,
  useModelState,
  useCodexConfigState,
  useApiKeyLink,
  useTemplateValues,
  useSpeedTestEndpoints,
  useCodexTomlValidation,
  useGeminiConfigState,
  useOmoModelSource,
  useOpencodeFormState,
  useOmoDraftState,
  useOpenclawFormState,
  useHermesFormState,
  useCodexOauth,
} from "./hooks";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import {
  CLAUDE_DEFAULT_CONFIG,
  CODEX_DEFAULT_CONFIG,
  GEMINI_DEFAULT_CONFIG,
  OPENCODE_DEFAULT_CONFIG,
  OPENCLAW_DEFAULT_CONFIG,
  hasNativeOpencodeDefinition,
  isNativeOpencodeConfig,
} from "./helpers/opencodeFormUtils";
import { HERMES_DEFAULT_CONFIG } from "./hooks/useHermesFormState";
import { resolveManagedAccountId } from "@/lib/authBinding";
import { useOpenClawLiveProviderIds } from "@/hooks/useOpenClaw";
import { useHermesLiveProviderIds } from "@/hooks/useHermes";
import { resolveCodexOfficialIdentity } from "@/utils/providerCapabilities";

type PresetEntry = {
  id: string;
  preset:
    | ProviderPreset
    | CodexProviderPreset
    | GeminiProviderPreset
    | OpenCodeProviderPreset
    | OpenClawProviderPreset
    | HermesProviderPreset;
};

function getPresetProviderType(
  preset: PresetEntry["preset"] | null | undefined,
): "codex_oauth" | undefined {
  if (!preset || !("providerType" in preset)) return undefined;
  return preset.providerType === "codex_oauth" ? "codex_oauth" : undefined;
}

export const normalizeCodexCatalogModelsForSave = (
  models: CodexCatalogModel[],
): CodexCatalogModel[] => {
  const seen = new Set<string>();
  const normalized: CodexCatalogModel[] = [];

  for (const item of models) {
    const model = item.model.trim();
    if (!model || seen.has(model)) continue;
    seen.add(model);

    const displayName = item.displayName?.trim();
    const rawContextWindow = String(item.contextWindow ?? "").replace(
      /[^\d]/g,
      "",
    );
    const contextWindow = rawContextWindow
      ? Number.parseInt(rawContextWindow, 10)
      : undefined;

    const inputModalities = item.inputModalities?.filter(
      (m) => typeof m === "string" && m.trim(),
    );

    const baseInstructions = item.baseInstructions?.trim();
    const reasoningLevels = item.reasoningLevels
      ?.filter((level) => typeof level === "string" && level.trim())
      .map((level) => level.trim());
    const defaultReasoningLevel = item.defaultReasoningLevel?.trim();

    normalized.push({
      model,
      ...(displayName ? { displayName } : {}),
      ...(contextWindow && contextWindow > 0 ? { contextWindow } : {}),
      // Native Responses profile overrides.
      ...(typeof item.supportsParallelToolCalls === "boolean"
        ? { supportsParallelToolCalls: item.supportsParallelToolCalls }
        : {}),
      ...(inputModalities && inputModalities.length > 0
        ? { inputModalities }
        : {}),
      ...(baseInstructions ? { baseInstructions } : {}),
      ...(reasoningLevels && reasoningLevels.length > 0
        ? { reasoningLevels }
        : {}),
      ...(defaultReasoningLevel ? { defaultReasoningLevel } : {}),
    });
  }

  return normalized;
};

const normalizeProviderKey = (value: string) =>
  value.toLowerCase().replace(/[^a-z0-9-]/g, "");

const asRecord = (value: unknown): Record<string, unknown> =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};

export interface ProviderFormProps {
  appId: AppId;
  providerId?: string;
  submitLabel: string;
  onSubmit: (values: ProviderFormValues) => Promise<void> | void;
  onCancel: () => void;
  onUniversalPresetSelect?: (preset: UniversalProviderPreset) => void;
  onManageUniversalProviders?: () => void;
  onManageAuthAccounts?: () => void;
  onSubmittingChange?: (isSubmitting: boolean) => void;
  onSubmitReadyChange?: (isReady: boolean) => void;
  initialData?: {
    name?: string;
    websiteUrl?: string;
    notes?: string;
    settingsConfig?: Record<string, unknown>;
    category?: ProviderCategory;
    meta?: ProviderMeta;
    icon?: string;
    iconColor?: string;
  };
  showButtons?: boolean;
  /** 编辑器里行保存着、但不随切换生效的字段（Claude Code、Codex、Gemini CLI、Grok Build）。 */
  inactiveFields?: ProviderEditorInactiveField[];
  /**
   * Claude 新增：当前 live 去掉当前供应商的关键字段后的样子。预设的关键字段套在它上面
   * 显示，保存时其余部分的改动写进 live。
   */
  claudeLiveBase?: Record<string, unknown>;
  /**
   * Codex、Gemini CLI、Grok Build 新增：预设或模板投影到当前配置文件上之后的内容，保存时
   * 作为三方比较的底；投影进行中或失败时为 `null`。
   */
  onEditorBaseChange?: EditorBaseChange;
}

export function ProviderForm(props: ProviderFormProps) {
  if (props.appId === "mcode") return <McodeProviderForm {...props} />;
  if (props.appId === "pi") {
    return <PiProviderForm {...props} />;
  }
  if (props.appId === "grokbuild") {
    return <GrokBuildProviderForm {...props} />;
  }

  return <ProviderFormFull {...props} />;
}

function ProviderFormFull({
  appId,
  providerId,
  submitLabel,
  onSubmit,
  onCancel,
  onUniversalPresetSelect,
  onManageUniversalProviders,
  onManageAuthAccounts,
  onSubmittingChange,
  initialData,
  showButtons = true,
  inactiveFields,
  claudeLiveBase,
  onEditorBaseChange,
}: ProviderFormProps) {
  const { t } = useTranslation();
  const isEditMode = Boolean(initialData);
  const initialCodexOfficialIdentity =
    appId === "codex" && initialData
      ? resolveCodexOfficialIdentity(appId, {
          id: providerId ?? "",
          category: initialData.category,
          meta: initialData.meta,
          settingsConfig: initialData.settingsConfig ?? {},
        })
      : null;
  const hasExistingCodexOfficialIdentity =
    initialCodexOfficialIdentity !== null &&
    initialCodexOfficialIdentity !== "api_key";
  const isDarkMode = useDarkMode();

  const [selectedPresetId, setSelectedPresetId] = useState<string | null>(
    initialData ? null : "custom",
  );
  const [activePreset, setActivePreset] = useState<{
    id: string;
    category?: ProviderCategory;
    partnerPromotionKey?: string;
    suggestedDefaults?: OpenClawSuggestedDefaults;
  } | null>(null);
  const [isEndpointModalOpen, setIsEndpointModalOpen] = useState(false);
  const [isCodexEndpointModalOpen, setIsCodexEndpointModalOpen] =
    useState(false);

  const [draftCustomEndpoints, setDraftCustomEndpoints] = useState<string[]>(
    () => {
      if (initialData) return [];
      return [];
    },
  );
  const [endpointAutoSelect, setEndpointAutoSelect] = useState<boolean>(
    () => initialData?.meta?.endpointAutoSelect ?? true,
  );

  const { category } = useProviderCategory({
    appId,
    selectedPresetId,
    isEditMode,
    initialCategory:
      initialData?.category ??
      (hasExistingCodexOfficialIdentity ? "official" : undefined),
  });
  const isOmoCategory = appId === "opencode" && category === "omo";
  const isOmoSlimCategory = appId === "opencode" && category === "omo-slim";
  const isAnyOmoCategory = isOmoCategory || isOmoSlimCategory;

  useEffect(() => {
    setSelectedPresetId(initialData ? null : "custom");
    setActivePreset(null);

    if (!initialData) {
      setDraftCustomEndpoints([]);
    }
    setEndpointAutoSelect(initialData?.meta?.endpointAutoSelect ?? true);
    setSelectedCodexAccountId(
      resolveManagedAccountId(initialData?.meta, "codex_oauth"),
    );
    setHasValidCodexOfficialSelection(true);
  }, [appId, initialData]);

  const defaultValues: ProviderFormData = useMemo(
    () => ({
      name: initialData?.name ?? "",
      websiteUrl: initialData?.websiteUrl ?? "",
      notes: initialData?.notes ?? "",
      settingsConfig: initialData?.settingsConfig
        ? JSON.stringify(initialData.settingsConfig, null, 2)
        : appId === "claude"
          ? JSON.stringify(
              overlayClaudeProviderFields(
                claudeLiveBase ?? {},
                withClaudeGatewayDefaults(
                  JSON.parse(CLAUDE_DEFAULT_CONFIG) as Record<string, unknown>,
                  "custom",
                ),
              ),
              null,
              2,
            )
          : appId === "codex"
            ? CODEX_DEFAULT_CONFIG
            : appId === "gemini"
              ? GEMINI_DEFAULT_CONFIG
              : appId === "opencode"
                ? OPENCODE_DEFAULT_CONFIG
                : appId === "openclaw"
                  ? OPENCLAW_DEFAULT_CONFIG
                  : appId === "hermes"
                    ? HERMES_DEFAULT_CONFIG
                    : CLAUDE_DEFAULT_CONFIG,
      icon: initialData?.icon ?? "",
      iconColor: initialData?.iconColor ?? "",
    }),
    [initialData, appId, claudeLiveBase],
  );

  const form = useForm<ProviderFormData>({
    resolver: zodResolver(providerSchema),
    defaultValues,
    mode: "onSubmit",
  });
  const { isSubmitting } = form.formState;

  const handleSettingsConfigChange = useCallback(
    (config: string) => {
      form.setValue("settingsConfig", config);
    },
    [form],
  );

  const [localApiKeyField, setLocalApiKeyField] = useState<ClaudeApiKeyField>(
    () => {
      if (appId !== "claude") return "ANTHROPIC_AUTH_TOKEN";
      if (initialData?.meta?.apiKeyField) return initialData.meta.apiKeyField;
      // Infer from existing config env
      const env = (initialData?.settingsConfig as Record<string, unknown>)
        ?.env as Record<string, unknown> | undefined;
      if (env?.ANTHROPIC_API_KEY !== undefined) return "ANTHROPIC_API_KEY";
      return "ANTHROPIC_AUTH_TOKEN";
    },
  );

  // 软校验：收集"业务约束"类问题（空值/缺项），由用户决定是否仍要保存
  const [softIssues, setSoftIssues] = useState<string[] | null>(null);
  const [pendingFormValues, setPendingFormValues] =
    useState<ProviderFormData | null>(null);
  // 确认框走的提交路径绕过了 react-hook-form 的 isSubmitting，单独追踪
  const [isConfirmSubmitting, setIsConfirmSubmitting] = useState(false);

  useEffect(() => {
    onSubmittingChange?.(isSubmitting || isConfirmSubmitting);
  }, [isSubmitting, isConfirmSubmitting, onSubmittingChange]);

  const {
    apiKey,
    handleApiKeyChange,
    showApiKey: shouldShowApiKey,
  } = useApiKeyState({
    initialConfig: form.getValues("settingsConfig"),
    onConfigChange: handleSettingsConfigChange,
    selectedPresetId,
    category,
    appType: appId,
    apiKeyField: appId === "claude" ? localApiKeyField : undefined,
  });

  const { baseUrl, handleClaudeBaseUrlChange } = useBaseUrlState({
    appType: appId,
    category,
    settingsConfig: form.getValues("settingsConfig"),
    codexConfig: "",
    onSettingsConfigChange: handleSettingsConfigChange,
    onCodexConfigChange: () => {},
  });

  const {
    claudeModel,
    defaultHaikuModel,
    defaultHaikuModelName,
    defaultSonnetModel,
    defaultSonnetModelName,
    defaultOpusModel,
    defaultOpusModelName,
    defaultFableModel,
    defaultFableModelName,
    subagentModel,
    handleModelChange,
  } = useModelState({
    settingsConfig: form.getValues("settingsConfig"),
    onConfigChange: handleSettingsConfigChange,
  });

  const handleApiKeyFieldChange = useCallback(
    (field: ClaudeApiKeyField) => {
      const prev = localApiKeyField;
      setLocalApiKeyField(field);

      // Swap the env key name in settingsConfig
      try {
        const raw = form.getValues("settingsConfig");
        const config = JSON.parse(raw || "{}");
        if (config?.env && prev in config.env) {
          const value = config.env[prev];
          delete config.env[prev];
          config.env[field] = value;
          const updated = JSON.stringify(config, null, 2);
          form.setValue("settingsConfig", updated);
          handleSettingsConfigChange(updated);
        }
      } catch {
        // ignore parse errors during editing
      }
    },
    [localApiKeyField, form, handleSettingsConfigChange],
  );

  // ChatGPT 账号状态（Codex 官方卡绑定账号时用）
  const {
    isAuthenticated: isCodexOauthAuthenticated,
    isStatusSuccess: isCodexOauthStatusSuccess,
    isStatusError: isCodexOauthStatusError,
    accounts: codexOauthAccounts,
  } = useCodexOauth();

  // 选中的 ChatGPT 账号 ID（Codex 官方卡多账号支持）
  const [selectedCodexAccountId, setSelectedCodexAccountId] = useState<
    string | null
  >(() => resolveManagedAccountId(initialData?.meta, "codex_oauth"));
  const [hasValidCodexOfficialSelection, setHasValidCodexOfficialSelection] =
    useState(true);

  const {
    codexAuth,
    codexConfig,
    codexApiKey,
    codexBaseUrl,
    codexModel,
    codexCatalogModels,
    codexAuthError,
    setCodexAuth,
    setCodexConfig,
    setCodexCatalogModels,
    handleCodexApiKeyChange,
    handleCodexBaseUrlChange,
    handleCodexModelChange,
    handleCodexConfigChange: originalHandleCodexConfigChange,
    resetCodexConfig,
  } = useCodexConfigState({ initialData });

  const { configError: codexConfigError, debouncedValidate } =
    useCodexTomlValidation();

  const handleCodexConfigChange = useCallback(
    (value: string) => {
      originalHandleCodexConfigChange(value);
      debouncedValidate(value);
    },
    [originalHandleCodexConfigChange, debouncedValidate],
  );

  // 新增：预设或模板投影到当前配置文件上显示。每次重置显示内容都要重新投影，否则保存时
  // 三方比较的底和显示内容对不上。
  const { projectDraft } = useDraftEditorProjection(appId, onEditorBaseChange);
  const projectCodexDraft = useCallback(
    (auth: Record<string, unknown>, config: string, category?: string) =>
      projectDraft({ auth, config }, category, (shown) =>
        setCodexConfig(typeof shown.config === "string" ? shown.config : ""),
      ),
    [projectDraft, setCodexConfig],
  );

  useEffect(() => {
    if (appId === "codex" && !initialData && selectedPresetId === "custom") {
      const template = getCodexCustomTemplate();
      resetCodexConfig(template.auth, template.config);
      projectCodexDraft(template.auth, template.config);
    }
  }, [
    appId,
    initialData,
    selectedPresetId,
    resetCodexConfig,
    projectCodexDraft,
  ]);

  useEffect(() => {
    form.reset(defaultValues);
  }, [defaultValues, form]);

  const presetCategoryLabels: Record<string, string> = useMemo(
    () => ({
      official: t("providerForm.categoryOfficial", {
        defaultValue: "官方",
      }),
      cn_official: t("providerForm.categoryCnOfficial", {
        defaultValue: "国内官方",
      }),
      aggregator: t("providerForm.categoryAggregation", {
        defaultValue: "聚合服务",
      }),
      third_party: t("providerForm.categoryThirdParty", {
        defaultValue: "第三方",
      }),
      omo: "OMO",
    }),
    [t],
  );

  const presetEntries = useMemo(() => {
    if (appId === "codex") {
      return codexProviderPresets.map<PresetEntry>((preset, index) => ({
        id: `codex-${index}`,
        preset,
      }));
    } else if (appId === "gemini") {
      return geminiProviderPresets.map<PresetEntry>((preset, index) => ({
        id: `gemini-${index}`,
        preset,
      }));
    } else if (appId === "opencode") {
      return opencodeProviderPresets.map<PresetEntry>((preset, index) => ({
        id: `opencode-${index}`,
        preset,
      }));
    } else if (appId === "openclaw") {
      return openclawProviderPresets.map<PresetEntry>((preset, index) => ({
        id: `openclaw-${index}`,
        preset,
      }));
    } else if (appId === "hermes") {
      return hermesProviderPresets.map<PresetEntry>((preset, index) => ({
        id: `hermes-${index}`,
        preset,
      }));
    }
    return providerPresets
      .filter((p) => !p.hidden)
      .map<PresetEntry>((preset, index) => ({
        id: `claude-${index}`,
        preset,
      }));
  }, [appId]);

  const selectedPresetEntry = useMemo(
    () =>
      selectedPresetId && selectedPresetId !== "custom"
        ? (presetEntries.find((entry) => entry.id === selectedPresetId) ?? null)
        : null,
    [presetEntries, selectedPresetId],
  );
  const presetProviderType = getPresetProviderType(selectedPresetEntry?.preset);
  const wasCodexOfficialManagedOauthBound =
    appId === "codex" &&
    Boolean(resolveManagedAccountId(initialData?.meta, "codex_oauth"));
  const isCodexOfficialProvider =
    appId === "codex" &&
    (hasExistingCodexOfficialIdentity ||
      wasCodexOfficialManagedOauthBound ||
      (presetProviderType === "codex_oauth" &&
        selectedPresetEntry?.preset.category === "official"));
  const isCodexOfficialManagedOauthBound =
    isCodexOfficialProvider && Boolean(selectedCodexAccountId);
  const requiresExplicitCodexOfficialSelection =
    isCodexOfficialProvider && !hasValidCodexOfficialSelection;

  const {
    templateValues,
    templateValueEntries,
    selectedPreset: templatePreset,
    handleTemplateValueChange,
    validateTemplateValues,
  } = useTemplateValues({
    selectedPresetId: appId === "claude" ? selectedPresetId : null,
    presetEntries: appId === "claude" ? presetEntries : [],
    settingsConfig: form.getValues("settingsConfig"),
    onConfigChange: handleSettingsConfigChange,
  });

  const {
    geminiEnv,
    geminiConfig,
    geminiApiKey,
    geminiBaseUrl,
    geminiModel,
    envError,
    configError: geminiConfigError,
    handleGeminiApiKeyChange: originalHandleGeminiApiKeyChange,
    handleGeminiBaseUrlChange: originalHandleGeminiBaseUrlChange,
    handleGeminiModelChange: originalHandleGeminiModelChange,
    handleGeminiEnvChange,
    handleGeminiConfigChange,
    resetGeminiConfig,
    envStringToObj,
  } = useGeminiConfigState({
    initialData: appId === "gemini" ? initialData : undefined,
  });

  const projectGeminiDraft = useCallback(
    (
      env: Record<string, unknown>,
      config: Record<string, unknown>,
      category?: string,
    ) =>
      projectDraft({ env, config }, category, (shown) =>
        resetGeminiConfig(asRecord(shown.env), asRecord(shown.config)),
      ),
    [projectDraft, resetGeminiConfig],
  );
  // resetGeminiConfig 随编辑内容变，不能放进下面的依赖，否则每次编辑都会重新投影、冲掉
  // 输入；只在打开和切回「自定义」时投影。
  const projectGeminiDraftRef = useRef(projectGeminiDraft);
  projectGeminiDraftRef.current = projectGeminiDraft;

  useEffect(() => {
    if (appId === "gemini" && !initialData && selectedPresetId === "custom") {
      projectGeminiDraftRef.current({}, {});
    }
  }, [appId, initialData, selectedPresetId]);

  const updateGeminiEnvField = useCallback(
    (
      key: "GEMINI_API_KEY" | "GOOGLE_GEMINI_BASE_URL" | "GEMINI_MODEL",
      value: string,
    ) => {
      try {
        const config = JSON.parse(form.getValues("settingsConfig") || "{}") as {
          env?: Record<string, unknown>;
        };
        if (!config.env || typeof config.env !== "object") {
          config.env = {};
        }
        config.env[key] = value;
        form.setValue("settingsConfig", JSON.stringify(config, null, 2));
      } catch {}
    },
    [form],
  );

  const handleGeminiApiKeyChange = useCallback(
    (key: string) => {
      originalHandleGeminiApiKeyChange(key);
      updateGeminiEnvField("GEMINI_API_KEY", key.trim());
    },
    [originalHandleGeminiApiKeyChange, updateGeminiEnvField],
  );

  const handleGeminiBaseUrlChange = useCallback(
    (url: string) => {
      originalHandleGeminiBaseUrlChange(url);
      updateGeminiEnvField(
        "GOOGLE_GEMINI_BASE_URL",
        url.trim().replace(/\/+$/, ""),
      );
    },
    [originalHandleGeminiBaseUrlChange, updateGeminiEnvField],
  );

  const handleGeminiModelChange = useCallback(
    (model: string) => {
      originalHandleGeminiModelChange(model);
      updateGeminiEnvField("GEMINI_MODEL", model.trim());
    },
    [originalHandleGeminiModelChange, updateGeminiEnvField],
  );

  // ── Extracted hooks: OpenCode / OMO / OpenClaw ─────────────────────

  const {
    omoModelOptions,
    omoModelVariantsMap,
    omoPresetMetaMap,
    existingOpencodeKeys,
  } = useOmoModelSource({ isOmoCategory: isAnyOmoCategory, providerId });

  const {
    data: opencodeLiveProviderIds = [],
    isLoading: isOpencodeLiveProviderIdsLoading,
    isSuccess: isOpencodeLiveProviderIdsSuccess,
  } = useQuery({
    queryKey: ["opencodeLiveProviderIds"],
    queryFn: () => providersApi.getOpenCodeLiveProviderIds(),
    enabled: appId === "opencode" && !isAnyOmoCategory,
  });

  const opencodeForm = useOpencodeFormState({
    initialData,
    appId,
    providerId,
    onSettingsConfigChange: (config) => form.setValue("settingsConfig", config),
    getSettingsConfig: () => form.getValues("settingsConfig"),
  });
  const isNativeOpencode =
    appId === "opencode" &&
    !isAnyOmoCategory &&
    isNativeOpencodeConfig(
      form.watch("settingsConfig"),
      initialData?.meta?.opencodeConfigFormat,
    );
  const isExistingNativeOpencodeKey =
    isNativeOpencode && providerId === opencodeForm.opencodeProviderKey;
  // Existing native IDs are kept exactly as OpenCode accepted them.
  const isOpencodeProviderKeyInvalid =
    opencodeForm.opencodeProviderKey.trim() !== "" &&
    !isExistingNativeOpencodeKey &&
    !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(opencodeForm.opencodeProviderKey);

  const keepsOpencodeProviderId =
    isEditMode &&
    !!providerId &&
    opencodeForm.opencodeProviderKey === providerId;
  const canKeepExistingOpencodeOverride =
    keepsOpencodeProviderId &&
    isOpencodeLiveProviderIdsSuccess &&
    opencodeLiveProviderIds.includes(opencodeForm.opencodeProviderKey);
  // Unlike V1, no older version copied native rows without their definition
  // (copies now require one), so a stored native row keeping its ID may stay
  // package-less after removal from the live config.
  const canInheritOpencodeDefinition =
    canKeepExistingOpencodeOverride ||
    (keepsOpencodeProviderId &&
      initialData?.meta?.opencodeConfigFormat === "v2");

  const initialOmoSettings =
    appId === "opencode" &&
    (initialData?.category === "omo" || initialData?.category === "omo-slim")
      ? (initialData.settingsConfig as Record<string, unknown> | undefined)
      : undefined;

  const omoDraft = useOmoDraftState({
    initialOmoSettings,
    isEditMode,
    appId,
    category,
  });

  const openclawForm = useOpenclawFormState({
    initialData,
    appId,
    providerId,
    onSettingsConfigChange: (config) => form.setValue("settingsConfig", config),
    getSettingsConfig: () => form.getValues("settingsConfig"),
  });
  const {
    data: openclawLiveProviderIds = [],
    isLoading: isOpenclawLiveProviderIdsLoading,
  } = useOpenClawLiveProviderIds(appId === "openclaw");

  const hermesForm = useHermesFormState({
    initialData,
    appId,
    providerId,
    onSettingsConfigChange: (config) => form.setValue("settingsConfig", config),
    getSettingsConfig: () => form.getValues("settingsConfig"),
  });
  const {
    data: hermesLiveProviderIds = [],
    isLoading: isHermesLiveProviderIdsLoading,
  } = useHermesLiveProviderIds(appId === "hermes");

  const additiveExistingProviderKeys = useMemo(() => {
    if (appId === "opencode" && !isAnyOmoCategory) {
      return Array.from(
        new Set(
          [...existingOpencodeKeys, ...opencodeLiveProviderIds].filter(
            (key) => key !== providerId,
          ),
        ),
      );
    }

    if (appId === "openclaw") {
      return Array.from(
        new Set(
          [
            ...openclawForm.existingOpenclawKeys,
            ...openclawLiveProviderIds,
          ].filter((key) => key !== providerId),
        ),
      );
    }

    if (appId === "hermes") {
      return Array.from(
        new Set(
          [...hermesForm.existingHermesKeys, ...hermesLiveProviderIds].filter(
            (key) => key !== providerId,
          ),
        ),
      );
    }

    return [];
  }, [
    appId,
    existingOpencodeKeys,
    hermesForm.existingHermesKeys,
    hermesLiveProviderIds,
    isAnyOmoCategory,
    openclawForm.existingOpenclawKeys,
    openclawLiveProviderIds,
    opencodeLiveProviderIds,
    providerId,
  ]);

  const isProviderKeyLockStateLoading = useMemo(() => {
    if (!isEditMode) return false;
    if (appId === "opencode" && !isAnyOmoCategory) {
      return isOpencodeLiveProviderIdsLoading;
    }
    if (appId === "openclaw") {
      return isOpenclawLiveProviderIdsLoading;
    }
    if (appId === "hermes") {
      return isHermesLiveProviderIdsLoading;
    }
    return false;
  }, [
    appId,
    isAnyOmoCategory,
    isEditMode,
    isHermesLiveProviderIdsLoading,
    isOpenclawLiveProviderIdsLoading,
    isOpencodeLiveProviderIdsLoading,
  ]);

  const isProviderKeyLocked = useMemo(() => {
    if (!isEditMode || !providerId) return false;
    if (appId === "opencode" && !isAnyOmoCategory) {
      return opencodeLiveProviderIds.includes(providerId);
    }
    if (appId === "openclaw") {
      return openclawLiveProviderIds.includes(providerId);
    }
    if (appId === "hermes") {
      return hermesLiveProviderIds.includes(providerId);
    }
    return false;
  }, [
    appId,
    hermesLiveProviderIds,
    isAnyOmoCategory,
    isEditMode,
    openclawLiveProviderIds,
    opencodeLiveProviderIds,
    providerId,
  ]);

  const handleSubmit = async (values: ProviderFormData) => {
    // 软性问题（业务约束，用户可选择仍要保存）
    const issues: string[] = [];

    // 模板变量未填：A 类（空值）
    if (appId === "claude" && templateValueEntries.length > 0) {
      const validation = validateTemplateValues();
      if (!validation.isValid && validation.missingField) {
        issues.push(
          t("providerForm.fillParameter", {
            label: validation.missingField.label,
            defaultValue: `请填写 ${validation.missingField.label}`,
          }),
        );
      }
    }

    // 供应商名空：A 类
    if (!values.name.trim()) {
      issues.push(
        t("providerForm.fillSupplierName", {
          defaultValue: "请填写供应商名称",
        }),
      );
    }

    // opencode / openclaw / hermes: providerKey 相关
    // A 类（空）归到 issues；B 类（正则不合法 / 重复 / 状态加载中）仍硬拒绝
    const keyPattern = /^[a-z0-9]+(-[a-z0-9]+)*$/;

    if (appId === "opencode" && !isAnyOmoCategory) {
      // providerKey 是 opencode / openclaw / hermes 的主键 ID，空或格式不合法
      // 都属于完整性约束，保留硬拒绝（mutations 层也会 throw，软化只会让错误更晦涩）
      if (!opencodeForm.opencodeProviderKey.trim()) {
        toast.error(t("opencode.providerKeyRequired"));
        return;
      }
      if (isOpencodeProviderKeyInvalid) {
        toast.error(t("opencode.providerKeyInvalid"));
        return;
      }
      if (isProviderKeyLockStateLoading) {
        toast.error(
          t("providerForm.providerKeyStatusLoading", {
            defaultValue: "正在加载供应商标识状态，请稍后再试",
          }),
        );
        return;
      }
      if (
        !isProviderKeyLocked &&
        additiveExistingProviderKeys.includes(opencodeForm.opencodeProviderKey)
      ) {
        toast.error(t("opencode.providerKeyDuplicate"));
        return;
      }
      // Only an existing override keeping its ID may inherit defaults.
      // Native V2 declarations name their package in `package`, not `npm`.
      if (!canInheritOpencodeDefinition) {
        const hasDefinition = isNativeOpencode
          ? hasNativeOpencodeDefinition(
              form.getValues("settingsConfig"),
              opencodeForm.opencodeProviderKey,
            )
          : !!opencodeForm.opencodeNpm.trim() &&
            Object.keys(opencodeForm.opencodeModels).length > 0;
        if (!hasDefinition) {
          toast.error(
            t(
              isNativeOpencode
                ? "opencode.nativeCustomProviderRequired"
                : "opencode.customProviderRequired",
            ),
          );
          return;
        }
      }
    }

    if (appId === "openclaw") {
      if (!openclawForm.openclawProviderKey.trim()) {
        toast.error(t("openclaw.providerKeyRequired"));
        return;
      }
      if (!keyPattern.test(openclawForm.openclawProviderKey)) {
        toast.error(t("openclaw.providerKeyInvalid"));
        return;
      }
      if (isProviderKeyLockStateLoading) {
        toast.error(
          t("providerForm.providerKeyStatusLoading", {
            defaultValue: "正在加载供应商标识状态，请稍后再试",
          }),
        );
        return;
      }
      if (
        !isProviderKeyLocked &&
        additiveExistingProviderKeys.includes(openclawForm.openclawProviderKey)
      ) {
        toast.error(t("openclaw.providerKeyDuplicate"));
        return;
      }
    }

    if (appId === "hermes") {
      if (!hermesForm.hermesProviderKey.trim()) {
        toast.error(t("hermes.form.providerKeyRequired"));
        return;
      }
      if (!keyPattern.test(hermesForm.hermesProviderKey)) {
        toast.error(t("hermes.form.providerKeyInvalid"));
        return;
      }
      if (isProviderKeyLockStateLoading) {
        toast.error(
          t("providerForm.providerKeyStatusLoading", {
            defaultValue: "正在加载供应商标识状态，请稍后再试",
          }),
        );
        return;
      }
      if (
        !isProviderKeyLocked &&
        additiveExistingProviderKeys.includes(hermesForm.hermesProviderKey)
      ) {
        toast.error(t("hermes.form.providerKeyDuplicate"));
        return;
      }
    }

    // ChatGPT 账号未登录：B 类（token 根本不存在，保存了也没法建立）
    if (requiresExplicitCodexOfficialSelection) {
      toast.error(
        t("codexOauth.explicitSelectionRequired", {
          defaultValue: "请先选择登录方式",
        }),
      );
      return;
    }
    if (isCodexOfficialManagedOauthBound && isCodexOauthStatusError) {
      toast.error(
        t("codexOauth.statusLoadFailed", {
          defaultValue: "无法加载 ChatGPT 账号状态，请重试。",
        }),
      );
      return;
    }
    if (isCodexOfficialManagedOauthBound && !isCodexOauthStatusSuccess) {
      toast.error(
        t("codexOauth.statusLoading", {
          defaultValue: "正在加载 ChatGPT 账号状态，请稍后再试。",
        }),
      );
      return;
    }
    if (isCodexOfficialManagedOauthBound && !isCodexOauthAuthenticated) {
      toast.error(
        t("codexOauth.loginRequired", {
          defaultValue: "请先登录 ChatGPT 账号",
        }),
      );
      return;
    }
    if (
      isCodexOfficialManagedOauthBound &&
      !codexOauthAccounts.some(
        (account) =>
          account.id === selectedCodexAccountId && !account.reauth_required,
      )
    ) {
      toast.error(
        t("managedAuth.selectedAccountNeedsReauth", {
          defaultValue: "已绑定账号不存在或需要重新登录",
        }),
      );
      return;
    }

    // OMO Other Fields JSON：B 类（格式错了保存下去数据就坏了）
    if (
      appId === "opencode" &&
      isAnyOmoCategory &&
      omoDraft.omoOtherFieldsStr.trim()
    ) {
      try {
        const otherFields = parseOmoOtherFieldsObject(
          omoDraft.omoOtherFieldsStr,
        );
        if (!otherFields) {
          toast.error(
            t("omo.jsonMustBeObject", {
              field: t("omo.otherFields", {
                defaultValue: "Other Config",
              }),
              defaultValue: "{{field}} must be a JSON object",
            }),
          );
          return;
        }
      } catch {
        toast.error(
          t("omo.invalidJson", {
            defaultValue: "Other Fields contains invalid JSON",
          }),
        );
        return;
      }
    }

    // 非官方供应商端点 / API Key 空：A 类
    // cloud_provider（如 Bedrock）通过模板变量处理认证，跳过通用校验
    if (category !== "official" && category !== "cloud_provider") {
      if (appId === "claude") {
        if (!baseUrl.trim()) {
          issues.push(
            t("providerForm.endpointRequired", {
              defaultValue: "非官方供应商请填写 API 端点",
            }),
          );
        }
        if (!apiKey.trim()) {
          issues.push(
            t("providerForm.apiKeyRequired", {
              defaultValue: "非官方供应商请填写 API Key",
            }),
          );
        }
      } else if (appId === "codex") {
        if (!codexBaseUrl.trim()) {
          issues.push(
            t("providerForm.endpointRequired", {
              defaultValue: "非官方供应商请填写 API 端点",
            }),
          );
        }
        if (!codexApiKey.trim()) {
          issues.push(
            t("providerForm.apiKeyRequired", {
              defaultValue: "非官方供应商请填写 API Key",
            }),
          );
        }
      }
      if (appId === "gemini") {
        if (!geminiBaseUrl.trim()) {
          issues.push(
            t("providerForm.endpointRequired", {
              defaultValue: "非官方供应商请填写 API 端点",
            }),
          );
        }
        if (!geminiApiKey.trim()) {
          issues.push(
            t("providerForm.apiKeyRequired", {
              defaultValue: "非官方供应商请填写 API Key",
            }),
          );
        }
      }
    }

    if (issues.length > 0) {
      // 弹确认框让用户决定是否仍要保存
      setSoftIssues(issues);
      setPendingFormValues(values);
      return;
    }

    await performSubmit(values);
  };

  const performSubmit = async (values: ProviderFormData) => {
    let settingsConfig: string;

    if (appId === "codex") {
      try {
        const shouldStripCodexOfficialAuth =
          isCodexOfficialManagedOauthBound || wasCodexOfficialManagedOauthBound;
        const authJson = shouldStripCodexOfficialAuth
          ? {}
          : JSON.parse(codexAuth);
        const codexConfigForSave = codexConfig ?? "";
        let normalizedCodexConfig =
          category !== "official" && codexConfigForSave.trim()
            ? setCodexWireApi(codexConfigForSave, "responses")
            : codexConfigForSave;
        // 模型映射：对所有非官方供应商，填了就持久化（生成 model-catalogs.json），
        // 留空归一化为 [] 即不写。后端只看 modelCatalog.models 是否非空。
        const normalizedCatalogModels =
          category !== "official"
            ? normalizeCodexCatalogModelsForSave(codexCatalogModels)
            : [];
        // The default-model field writes the top-level `model` into the TOML
        // as the user types; only when it was left empty fall back to the
        // first catalog row so "fill mapping only" keeps its old behavior.
        if (
          normalizedCatalogModels.length > 0 &&
          !extractCodexModelName(normalizedCodexConfig)
        ) {
          normalizedCodexConfig = setCodexModelNameInConfig(
            normalizedCodexConfig,
            normalizedCatalogModels[0].model,
          );
        }
        const configObj = {
          auth: authJson,
          config: normalizedCodexConfig,
        } as {
          auth: unknown;
          config: string;
          modelCatalog?: { models: CodexCatalogModel[] };
        };
        if (normalizedCatalogModels.length > 0) {
          configObj.modelCatalog = { models: normalizedCatalogModels };
        }
        settingsConfig = JSON.stringify(configObj);
      } catch (err) {
        settingsConfig = values.settingsConfig.trim();
      }
    } else if (appId === "gemini") {
      try {
        const envObj = envStringToObj(geminiEnv);
        const configObj = geminiConfig.trim() ? JSON.parse(geminiConfig) : {};
        const combined = {
          env: envObj,
          config: configObj,
        };
        settingsConfig = JSON.stringify(combined);
      } catch (err) {
        settingsConfig = values.settingsConfig.trim();
      }
    } else if (
      appId === "opencode" &&
      (category === "omo" || category === "omo-slim")
    ) {
      const omoConfig: Record<string, unknown> = {};
      if (Object.keys(omoDraft.omoAgents).length > 0) {
        omoConfig.agents = omoDraft.omoAgents;
      }
      if (
        category === "omo" &&
        Object.keys(omoDraft.omoCategories).length > 0
      ) {
        omoConfig.categories = omoDraft.omoCategories;
      }
      if (omoDraft.omoOtherFieldsStr.trim()) {
        // 格式已在 handleSubmit 前置校验中验证过，此处可以安全解析
        const otherFields = parseOmoOtherFieldsObject(
          omoDraft.omoOtherFieldsStr,
        );
        if (otherFields) {
          omoConfig.otherFields = otherFields;
        }
      }
      settingsConfig = JSON.stringify(omoConfig);
    } else {
      settingsConfig = values.settingsConfig.trim();
    }

    const payload: ProviderFormValues = {
      ...values,
      name: values.name.trim(),
      websiteUrl: values.websiteUrl?.trim() ?? "",
      settingsConfig,
    };

    if (isCodexOfficialProvider) {
      payload.presetCategory = "official";
    }

    if (appId === "opencode") {
      if (isAnyOmoCategory) {
        if (!isEditMode) {
          const prefix = category === "omo" ? "omo" : "omo-slim";
          payload.providerKey = `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
        }
      } else {
        payload.providerKey = opencodeForm.opencodeProviderKey;
      }
    } else if (appId === "openclaw") {
      payload.providerKey = openclawForm.openclawProviderKey;
    } else if (appId === "hermes") {
      payload.providerKey = hermesForm.hermesProviderKey;
    }

    if (isAnyOmoCategory && !payload.presetCategory) {
      payload.presetCategory = category;
    }

    if (activePreset) {
      payload.presetId = activePreset.id;
      if (activePreset.category) {
        payload.presetCategory = activePreset.category;
      }
      // OpenClaw: align preset model refs with the actual submitted provider key.
      if (activePreset.suggestedDefaults) {
        payload.suggestedDefaults =
          appId === "openclaw" && payload.providerKey
            ? rebaseOpenClawSuggestedDefaults(
                activePreset.suggestedDefaults,
                payload.providerKey,
              )
            : activePreset.suggestedDefaults;
      }
    }

    if (!isEditMode && isCodexOfficialManagedOauthBound) {
      const selectedAccountLogin = codexOauthAccounts.find(
        (account) => account.id === selectedCodexAccountId,
      )?.login;
      const presetName = selectedPresetEntry
        ? "nameKey" in selectedPresetEntry.preset &&
          selectedPresetEntry.preset.nameKey
          ? t(selectedPresetEntry.preset.nameKey)
          : selectedPresetEntry.preset.name
        : null;
      if (selectedAccountLogin && presetName && payload.name === presetName) {
        payload.name = `${presetName} (${selectedAccountLogin})`;
      }
    }

    if (!isEditMode && draftCustomEndpoints.length > 0) {
      const customEndpointsToSave: Record<
        string,
        import("@/types").CustomEndpoint
      > = draftCustomEndpoints.reduce(
        (acc, url) => {
          const now = Date.now();
          acc[url] = { url, addedAt: now, lastUsed: undefined };
          return acc;
        },
        {} as Record<string, import("@/types").CustomEndpoint>,
      );

      const hadEndpoints =
        initialData?.meta?.custom_endpoints &&
        Object.keys(initialData.meta.custom_endpoints).length > 0;
      const needsClearEndpoints =
        hadEndpoints && draftCustomEndpoints.length === 0;

      let mergedMeta = needsClearEndpoints
        ? mergeProviderMeta(initialData?.meta, {})
        : mergeProviderMeta(initialData?.meta, customEndpointsToSave);

      if (activePreset?.partnerPromotionKey) {
        mergedMeta = {
          ...(mergedMeta ?? {}),
          partnerPromotionKey: activePreset.partnerPromotionKey,
        };
      }

      if (mergedMeta !== undefined) {
        payload.meta = mergedMeta;
      }
    }

    const metaSource = payload.meta ?? initialData?.meta;
    const baseMeta: ProviderMeta | undefined = metaSource
      ? { ...metaSource }
      : undefined;
    // Existing-provider edits never own endpoint membership. The backend
    // rejects endpoint-bearing update payloads; add/remove/touch use their
    // dedicated commands and remain safe from stale form snapshots.
    if (isEditMode && baseMeta) {
      delete baseMeta.custom_endpoints;
    }

    const nextMeta: ProviderMeta = {
      ...(baseMeta ?? {}),
      opencodeConfigFormat: isNativeOpencode ? "v2" : undefined,
      // Claude Code、Codex、Gemini CLI 的通用配置片段已冻结：沿用行里原有的标记，新增时
      // 由后端写 true（兼容旧版）。
      commonConfigEnabled:
        appId === "claude" || appId === "codex" || appId === "gemini"
          ? initialData?.meta?.commonConfigEnabled
          : undefined,
      endpointAutoSelect,
      // Codex 官方卡绑定了 ChatGPT 账号：切换时把这个账号的登录写进 auth.json
      providerType: isCodexOfficialManagedOauthBound
        ? "codex_oauth"
        : undefined,
      authBinding: isCodexOfficialManagedOauthBound
        ? {
            source: "managed_account",
            authProvider: "codex_oauth",
            accountId: selectedCodexAccountId ?? undefined,
          }
        : undefined,
      apiKeyField:
        appId === "claude" &&
        category !== "official" &&
        localApiKeyField !== "ANTHROPIC_AUTH_TOKEN"
          ? localApiKeyField
          : undefined,
    };

    if (!nextMeta.providerType && "providerType" in nextMeta) {
      delete nextMeta.providerType;
    }
    if (!nextMeta.authBinding && "authBinding" in nextMeta) {
      delete nextMeta.authBinding;
    }

    payload.meta = nextMeta;

    await onSubmit(payload);
  };

  const shouldShowSpeedTest =
    category !== "official" && category !== "cloud_provider";

  const {
    shouldShowApiKeyLink: shouldShowClaudeApiKeyLink,
    websiteUrl: claudeWebsiteUrl,
  } = useApiKeyLink({
    appId: "claude",
    category,
    selectedPresetId,
    presetEntries,
    formWebsiteUrl: form.watch("websiteUrl") || "",
  });

  const {
    shouldShowApiKeyLink: shouldShowCodexApiKeyLink,
    websiteUrl: codexWebsiteUrl,
  } = useApiKeyLink({
    appId: "codex",
    category,
    selectedPresetId,
    presetEntries,
    formWebsiteUrl: form.watch("websiteUrl") || "",
  });

  const {
    shouldShowApiKeyLink: shouldShowGeminiApiKeyLink,
    websiteUrl: geminiWebsiteUrl,
    partnerPromotionKey: geminiPartnerPromotionKey,
  } = useApiKeyLink({
    appId: "gemini",
    category,
    selectedPresetId,
    presetEntries,
    formWebsiteUrl: form.watch("websiteUrl") || "",
  });

  const {
    shouldShowApiKeyLink: shouldShowOpencodeApiKeyLink,
    websiteUrl: opencodeWebsiteUrl,
  } = useApiKeyLink({
    appId: "opencode",
    category,
    selectedPresetId,
    presetEntries,
    formWebsiteUrl: form.watch("websiteUrl") || "",
  });

  // 使用 API Key 链接 hook (OpenClaw)
  const {
    shouldShowApiKeyLink: shouldShowOpenclawApiKeyLink,
    websiteUrl: openclawWebsiteUrl,
  } = useApiKeyLink({
    appId: "openclaw",
    category,
    selectedPresetId,
    presetEntries,
    formWebsiteUrl: form.watch("websiteUrl") || "",
  });

  // 使用 API Key 链接 hook (Hermes)
  const {
    shouldShowApiKeyLink: shouldShowHermesApiKeyLink,
    websiteUrl: hermesWebsiteUrl,
  } = useApiKeyLink({
    appId: "hermes",
    category,
    selectedPresetId,
    presetEntries,
    formWebsiteUrl: form.watch("websiteUrl") || "",
  });

  // 使用端点测速候选 hook
  const speedTestEndpoints = useSpeedTestEndpoints({
    appId,
    selectedPresetId,
    presetEntries,
    baseUrl,
    codexBaseUrl,
    initialData,
  });

  const handlePresetChange = (value: string) => {
    setSelectedPresetId(value);
    if (value === "custom") {
      setActivePreset(null);
      form.reset(defaultValues);

      if (appId === "codex") {
        const template = getCodexCustomTemplate();
        resetCodexConfig(template.auth, template.config);
        projectCodexDraft(template.auth, template.config);
      }
      if (appId === "gemini") {
        resetGeminiConfig({}, {});
        projectGeminiDraft({}, {});
      }
      if (appId === "opencode") {
        opencodeForm.resetOpencodeState();
        omoDraft.resetOmoDraftState();
      }
      // OpenClaw 自定义模式：重置为空配置
      if (appId === "openclaw") {
        openclawForm.resetOpenclawState();
      }
      if (appId === "hermes") {
        hermesForm.resetHermesState();
      }
      return;
    }

    const entry = presetEntries.find((item) => item.id === value);
    if (!entry) {
      return;
    }

    setActivePreset({
      id: value,
      category: entry.preset.category,
      partnerPromotionKey: entry.preset.partnerPromotionKey,
    });

    if (appId === "codex") {
      const preset = entry.preset as CodexProviderPreset;
      const auth = preset.auth ?? {};
      const config = preset.config ?? "";

      resetCodexConfig(auth, config, preset.modelCatalog ?? []);

      form.reset({
        name: preset.nameKey ? t(preset.nameKey) : preset.name,
        websiteUrl: preset.websiteUrl ?? "",
        settingsConfig: JSON.stringify({ auth, config }, null, 2),
        icon: preset.icon ?? "",
        iconColor: preset.iconColor ?? "",
      });
      projectCodexDraft(auth, config, preset.category);
      return;
    }

    if (appId === "gemini") {
      const preset = entry.preset as GeminiProviderPreset;
      const env = (preset.settingsConfig as any)?.env ?? {};
      const config = (preset.settingsConfig as any)?.config ?? {};

      resetGeminiConfig(env, config);

      form.reset({
        name: preset.nameKey ? t(preset.nameKey) : preset.name,
        websiteUrl: preset.websiteUrl ?? "",
        settingsConfig: JSON.stringify(preset.settingsConfig, null, 2),
        icon: preset.icon ?? "",
        iconColor: preset.iconColor ?? "",
      });
      projectGeminiDraft(env, config, preset.category);
      return;
    }

    if (appId === "opencode") {
      const preset = entry.preset as OpenCodeProviderPreset;
      const config = preset.settingsConfig;

      if (preset.category === "omo" || preset.category === "omo-slim") {
        omoDraft.resetOmoDraftState();
        form.reset({
          name: preset.category === "omo" ? "OMO" : "OMO Slim",
          websiteUrl: preset.websiteUrl ?? "",
          settingsConfig: JSON.stringify({}, null, 2),
          icon: preset.icon ?? "",
          iconColor: preset.iconColor ?? "",
        });
        return;
      }

      opencodeForm.resetOpencodeState(config);

      form.reset({
        name: preset.nameKey ? t(preset.nameKey) : preset.name,
        websiteUrl: preset.websiteUrl ?? "",
        settingsConfig: JSON.stringify(config, null, 2),
        icon: preset.icon ?? "",
        iconColor: preset.iconColor ?? "",
      });
      return;
    }

    // OpenClaw preset handling
    if (appId === "openclaw") {
      const preset = entry.preset as OpenClawProviderPreset;
      const config = preset.settingsConfig;

      // Update activePreset with suggestedDefaults for OpenClaw
      setActivePreset({
        id: value,
        category: preset.category,
        partnerPromotionKey: preset.partnerPromotionKey,
        suggestedDefaults: preset.suggestedDefaults,
      });

      openclawForm.resetOpenclawState(config);

      // Update form fields
      form.reset({
        name: preset.nameKey ? t(preset.nameKey) : preset.name,
        websiteUrl: preset.websiteUrl ?? "",
        settingsConfig: JSON.stringify(config, null, 2),
        icon: preset.icon ?? "",
        iconColor: preset.iconColor ?? "",
      });
      return;
    }

    // Hermes preset handling
    if (appId === "hermes") {
      const preset = entry.preset as HermesProviderPreset;
      const config = preset.settingsConfig;

      hermesForm.resetHermesState(config);

      form.reset({
        name: preset.nameKey ? t(preset.nameKey) : preset.name,
        websiteUrl: preset.websiteUrl ?? "",
        settingsConfig: JSON.stringify(config, null, 2),
        icon: preset.icon ?? "",
        iconColor: preset.iconColor ?? "",
      });
      return;
    }

    const preset = entry.preset as ProviderPreset;
    const templated = applyTemplateValues(
      preset.settingsConfig,
      preset.templateValues,
    );
    // 预设只带关键字段和独有字段，套在当前 live 上显示（和切换的结果一致）。
    const config =
      appId === "claude"
        ? overlayClaudeProviderFields(
            claudeLiveBase ?? {},
            withClaudeGatewayDefaults(
              templated as Record<string, unknown>,
              preset.category,
            ),
          )
        : templated;

    setLocalApiKeyField(preset.apiKeyField ?? "ANTHROPIC_AUTH_TOKEN");

    form.reset({
      name: preset.nameKey ? t(preset.nameKey) : preset.name,
      websiteUrl: preset.websiteUrl ?? "",
      settingsConfig: JSON.stringify(config, null, 2),
      icon: preset.icon ?? "",
      iconColor: preset.iconColor ?? "",
    });
  };

  const settingsConfigErrorField = (
    <FormField
      control={form.control}
      name="settingsConfig"
      render={() => (
        <FormItem className="space-y-0">
          <FormMessage />
        </FormItem>
      )}
    />
  );

  return (
    <>
      <Form {...form}>
        <form
          id="provider-form"
          onSubmit={form.handleSubmit(handleSubmit)}
          className="space-y-6"
        >
          {!initialData && (
            <ProviderPresetSelector
              selectedPresetId={selectedPresetId}
              presetEntries={presetEntries}
              presetCategoryLabels={presetCategoryLabels}
              onPresetChange={handlePresetChange}
              onUniversalPresetSelect={onUniversalPresetSelect}
              onManageUniversalProviders={onManageUniversalProviders}
              category={category}
            />
          )}

          <BasicFormFields
            form={form}
            beforeNameSlot={
              appId === "opencode" && !isAnyOmoCategory ? (
                <div className="space-y-2">
                  <Label htmlFor="opencode-key">
                    {t("opencode.providerKey")}
                    <span className="text-destructive ml-1">*</span>
                  </Label>
                  <ImeSafeInput
                    id="opencode-key"
                    value={opencodeForm.opencodeProviderKey}
                    onValueChange={opencodeForm.setOpencodeProviderKey}
                    normalize={normalizeProviderKey}
                    placeholder={t("opencode.providerKeyPlaceholder")}
                    disabled={
                      isProviderKeyLocked || isProviderKeyLockStateLoading
                    }
                    className={
                      (additiveExistingProviderKeys.includes(
                        opencodeForm.opencodeProviderKey,
                      ) &&
                        !isProviderKeyLocked) ||
                      isOpencodeProviderKeyInvalid
                        ? "border-destructive"
                        : ""
                    }
                  />
                  {additiveExistingProviderKeys.includes(
                    opencodeForm.opencodeProviderKey,
                  ) &&
                    !isProviderKeyLocked && (
                      <p className="text-xs text-destructive">
                        {t("opencode.providerKeyDuplicate")}
                      </p>
                    )}
                  {isOpencodeProviderKeyInvalid && (
                    <p className="text-xs text-destructive">
                      {t("opencode.providerKeyInvalid")}
                    </p>
                  )}
                  {!(
                    additiveExistingProviderKeys.includes(
                      opencodeForm.opencodeProviderKey,
                    ) && !isProviderKeyLocked
                  ) &&
                    !isOpencodeProviderKeyInvalid && (
                      <p className="text-xs text-fg-2">
                        {isProviderKeyLocked
                          ? t("opencode.providerKeyLockedHint", {
                              defaultValue:
                                "该供应商已添加到应用配置中，供应商标识不可修改",
                            })
                          : t("opencode.providerKeyHint")}
                      </p>
                    )}
                </div>
              ) : appId === "openclaw" ? (
                <div className="space-y-2">
                  <Label htmlFor="openclaw-key">
                    {t("openclaw.providerKey")}
                    <span className="text-destructive ml-1">*</span>
                  </Label>
                  <ImeSafeInput
                    id="openclaw-key"
                    value={openclawForm.openclawProviderKey}
                    onValueChange={openclawForm.setOpenclawProviderKey}
                    normalize={normalizeProviderKey}
                    placeholder={t("openclaw.providerKeyPlaceholder")}
                    disabled={
                      isProviderKeyLocked || isProviderKeyLockStateLoading
                    }
                    className={
                      (additiveExistingProviderKeys.includes(
                        openclawForm.openclawProviderKey,
                      ) &&
                        !isProviderKeyLocked) ||
                      (openclawForm.openclawProviderKey.trim() !== "" &&
                        !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(
                          openclawForm.openclawProviderKey,
                        ))
                        ? "border-destructive"
                        : ""
                    }
                  />
                  {additiveExistingProviderKeys.includes(
                    openclawForm.openclawProviderKey,
                  ) &&
                    !isProviderKeyLocked && (
                      <p className="text-xs text-destructive">
                        {t("openclaw.providerKeyDuplicate")}
                      </p>
                    )}
                  {openclawForm.openclawProviderKey.trim() !== "" &&
                    !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(
                      openclawForm.openclawProviderKey,
                    ) && (
                      <p className="text-xs text-destructive">
                        {t("openclaw.providerKeyInvalid")}
                      </p>
                    )}
                  {!(
                    additiveExistingProviderKeys.includes(
                      openclawForm.openclawProviderKey,
                    ) && !isProviderKeyLocked
                  ) &&
                    (openclawForm.openclawProviderKey.trim() === "" ||
                      /^[a-z0-9]+(-[a-z0-9]+)*$/.test(
                        openclawForm.openclawProviderKey,
                      )) && (
                      <p className="text-xs text-fg-2">
                        {isProviderKeyLocked
                          ? t("openclaw.providerKeyLockedHint", {
                              defaultValue:
                                "该供应商已添加到应用配置中，供应商标识不可修改",
                            })
                          : t("openclaw.providerKeyHint")}
                      </p>
                    )}
                </div>
              ) : appId === "hermes" ? (
                <div className="space-y-2">
                  <Label htmlFor="hermes-key">
                    {t("hermes.form.providerKey", {
                      defaultValue: "Provider Key",
                    })}
                    <span className="text-destructive ml-1">*</span>
                  </Label>
                  <ImeSafeInput
                    id="hermes-key"
                    value={hermesForm.hermesProviderKey}
                    onValueChange={hermesForm.setHermesProviderKey}
                    normalize={normalizeProviderKey}
                    placeholder={t("hermes.form.providerKeyPlaceholder", {
                      defaultValue: "my-provider",
                    })}
                    disabled={
                      isProviderKeyLocked || isProviderKeyLockStateLoading
                    }
                    className={
                      (additiveExistingProviderKeys.includes(
                        hermesForm.hermesProviderKey,
                      ) &&
                        !isProviderKeyLocked) ||
                      (hermesForm.hermesProviderKey.trim() !== "" &&
                        !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(
                          hermesForm.hermesProviderKey,
                        ))
                        ? "border-destructive"
                        : ""
                    }
                  />
                  {additiveExistingProviderKeys.includes(
                    hermesForm.hermesProviderKey,
                  ) &&
                    !isProviderKeyLocked && (
                      <p className="text-xs text-destructive">
                        {t("hermes.form.providerKeyDuplicate")}
                      </p>
                    )}
                  {hermesForm.hermesProviderKey.trim() !== "" &&
                    !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(
                      hermesForm.hermesProviderKey,
                    ) && (
                      <p className="text-xs text-destructive">
                        {t("hermes.form.providerKeyInvalid")}
                      </p>
                    )}
                  {!(
                    additiveExistingProviderKeys.includes(
                      hermesForm.hermesProviderKey,
                    ) && !isProviderKeyLocked
                  ) &&
                    (hermesForm.hermesProviderKey.trim() === "" ||
                      /^[a-z0-9]+(-[a-z0-9]+)*$/.test(
                        hermesForm.hermesProviderKey,
                      )) && (
                      <p className="text-xs text-fg-2">
                        {isProviderKeyLocked
                          ? t("hermes.form.providerKeyLockedHint", {
                              defaultValue:
                                "This provider is in Hermes config; key is locked.",
                            })
                          : t("hermes.form.providerKeyHint", {
                              defaultValue:
                                "Lowercase letters, numbers, and hyphens only. Used as the provider name in config.yaml.",
                            })}
                      </p>
                    )}
                </div>
              ) : undefined
            }
          />

          {appId === "claude" && (
            <ClaudeFormFields
              providerId={providerId}
              shouldShowApiKey={
                (category !== "cloud_provider" ||
                  hasApiKeyField(form.getValues("settingsConfig"), "claude")) &&
                shouldShowApiKey(form.getValues("settingsConfig"), isEditMode)
              }
              apiKey={apiKey}
              onApiKeyChange={handleApiKeyChange}
              category={category}
              shouldShowApiKeyLink={shouldShowClaudeApiKeyLink}
              websiteUrl={claudeWebsiteUrl}
              templateValueEntries={templateValueEntries}
              templateValues={templateValues}
              templatePresetName={templatePreset?.name || ""}
              onTemplateValueChange={handleTemplateValueChange}
              shouldShowSpeedTest={shouldShowSpeedTest}
              baseUrl={baseUrl}
              onBaseUrlChange={handleClaudeBaseUrlChange}
              isEndpointModalOpen={isEndpointModalOpen}
              onEndpointModalToggle={setIsEndpointModalOpen}
              onCustomEndpointsChange={
                isEditMode ? undefined : setDraftCustomEndpoints
              }
              autoSelect={endpointAutoSelect}
              onAutoSelectChange={setEndpointAutoSelect}
              showEndpointTools
              shouldShowModelSelector={category !== "official"}
              claudeModel={claudeModel}
              defaultHaikuModel={defaultHaikuModel}
              defaultHaikuModelName={defaultHaikuModelName}
              defaultSonnetModel={defaultSonnetModel}
              defaultSonnetModelName={defaultSonnetModelName}
              defaultOpusModel={defaultOpusModel}
              defaultOpusModelName={defaultOpusModelName}
              defaultFableModel={defaultFableModel}
              defaultFableModelName={defaultFableModelName}
              subagentModel={subagentModel}
              onModelChange={handleModelChange}
              speedTestEndpoints={speedTestEndpoints}
              apiKeyField={localApiKeyField}
              onApiKeyFieldChange={handleApiKeyFieldChange}
            />
          )}

          {appId === "codex" && (
            <CodexFormFields
              providerId={providerId}
              codexApiKey={codexApiKey}
              onApiKeyChange={handleCodexApiKeyChange}
              category={category}
              shouldShowApiKeyLink={shouldShowCodexApiKeyLink}
              websiteUrl={codexWebsiteUrl}
              isCodexOauthPreset={isCodexOfficialProvider}
              selectedCodexAccountId={selectedCodexAccountId}
              onCodexAccountSelect={setSelectedCodexAccountId}
              onCodexAuthSelectionConfirmed={() =>
                setHasValidCodexOfficialSelection(true)
              }
              onCodexAuthSelectionInvalidated={() =>
                setHasValidCodexOfficialSelection(false)
              }
              onManageAuthAccounts={onManageAuthAccounts}
              codexOauthSelectionLabel={t("codexOauth.signInMethod")}
              codexOauthNoneOptionLabel={t("codexOauth.noneOptionLabel")}
              codexOauthNoneOptionDescription={t(
                "codex.followCodexLoginDescription",
              )}
              codexOauthAllowUnboundSelection
              codexOauthAllowUnboundSelectionWithoutStatus
              codexOauthRequireExplicitSelection={
                requiresExplicitCodexOfficialSelection
              }
              shouldShowSpeedTest={shouldShowSpeedTest}
              codexBaseUrl={codexBaseUrl}
              onBaseUrlChange={handleCodexBaseUrlChange}
              isEndpointModalOpen={isCodexEndpointModalOpen}
              onEndpointModalToggle={setIsCodexEndpointModalOpen}
              onCustomEndpointsChange={
                isEditMode ? undefined : setDraftCustomEndpoints
              }
              autoSelect={endpointAutoSelect}
              onAutoSelectChange={setEndpointAutoSelect}
              codexModel={codexModel}
              onModelChange={handleCodexModelChange}
              catalogModels={codexCatalogModels}
              onCatalogModelsChange={setCodexCatalogModels}
              speedTestEndpoints={speedTestEndpoints}
            />
          )}

          {appId === "gemini" && (
            <GeminiFormFields
              providerId={providerId}
              shouldShowApiKey={shouldShowApiKey(
                form.getValues("settingsConfig"),
                isEditMode,
              )}
              apiKey={geminiApiKey}
              onApiKeyChange={handleGeminiApiKeyChange}
              category={category}
              shouldShowApiKeyLink={shouldShowGeminiApiKeyLink}
              websiteUrl={geminiWebsiteUrl}
              partnerPromotionKey={geminiPartnerPromotionKey}
              shouldShowSpeedTest={shouldShowSpeedTest}
              baseUrl={geminiBaseUrl}
              onBaseUrlChange={handleGeminiBaseUrlChange}
              isEndpointModalOpen={isEndpointModalOpen}
              onEndpointModalToggle={setIsEndpointModalOpen}
              onCustomEndpointsChange={setDraftCustomEndpoints}
              autoSelect={endpointAutoSelect}
              onAutoSelectChange={setEndpointAutoSelect}
              shouldShowModelField={true}
              model={geminiModel}
              onModelChange={handleGeminiModelChange}
              speedTestEndpoints={speedTestEndpoints}
            />
          )}

          {appId === "opencode" && !isAnyOmoCategory && !isNativeOpencode && (
            <OpenCodeFormFields
              allowBuiltinDefaults={canKeepExistingOpencodeOverride}
              npm={opencodeForm.opencodeNpm}
              onNpmChange={opencodeForm.handleOpencodeNpmChange}
              apiKey={opencodeForm.opencodeApiKey}
              onApiKeyChange={opencodeForm.handleOpencodeApiKeyChange}
              category={category}
              shouldShowApiKeyLink={shouldShowOpencodeApiKeyLink}
              websiteUrl={opencodeWebsiteUrl}
              baseUrl={opencodeForm.opencodeBaseUrl}
              onBaseUrlChange={opencodeForm.handleOpencodeBaseUrlChange}
              headers={opencodeForm.opencodeHeaders}
              onHeadersChange={opencodeForm.handleOpencodeHeadersChange}
              models={opencodeForm.opencodeModels}
              onModelsChange={opencodeForm.handleOpencodeModelsChange}
              extraOptions={opencodeForm.opencodeExtraOptions}
              onExtraOptionsChange={
                opencodeForm.handleOpencodeExtraOptionsChange
              }
            />
          )}

          {appId === "opencode" &&
            (category === "omo" || category === "omo-slim") && (
              <OmoFormFields
                modelOptions={omoModelOptions}
                modelVariantsMap={omoModelVariantsMap}
                presetMetaMap={omoPresetMetaMap}
                agents={omoDraft.omoAgents}
                onAgentsChange={omoDraft.setOmoAgents}
                categories={
                  category === "omo" ? omoDraft.omoCategories : undefined
                }
                onCategoriesChange={
                  category === "omo" ? omoDraft.setOmoCategories : undefined
                }
                otherFieldsStr={omoDraft.omoOtherFieldsStr}
                onOtherFieldsStrChange={omoDraft.setOmoOtherFieldsStr}
                isSlim={category === "omo-slim"}
              />
            )}

          {/* OpenClaw 专属字段 */}
          {appId === "openclaw" && (
            <OpenClawFormFields
              baseUrl={openclawForm.openclawBaseUrl}
              onBaseUrlChange={openclawForm.handleOpenclawBaseUrlChange}
              apiKey={openclawForm.openclawApiKey}
              onApiKeyChange={openclawForm.handleOpenclawApiKeyChange}
              category={category}
              shouldShowApiKeyLink={shouldShowOpenclawApiKeyLink}
              websiteUrl={openclawWebsiteUrl}
              api={openclawForm.openclawApi}
              onApiChange={openclawForm.handleOpenclawApiChange}
              models={openclawForm.openclawModels}
              onModelsChange={openclawForm.handleOpenclawModelsChange}
              userAgent={openclawForm.openclawUserAgent}
              onUserAgentChange={openclawForm.handleOpenclawUserAgentChange}
            />
          )}

          {/* Hermes 专属字段 */}
          {appId === "hermes" && (
            <HermesFormFields
              baseUrl={hermesForm.hermesBaseUrl}
              onBaseUrlChange={hermesForm.handleHermesBaseUrlChange}
              apiKey={hermesForm.hermesApiKey}
              onApiKeyChange={hermesForm.handleHermesApiKeyChange}
              category={category}
              shouldShowApiKeyLink={shouldShowHermesApiKeyLink}
              websiteUrl={hermesWebsiteUrl}
              apiMode={hermesForm.hermesApiMode}
              onApiModeChange={hermesForm.handleHermesApiModeChange}
              models={hermesForm.hermesModels}
              onModelsChange={hermesForm.handleHermesModelsChange}
              rateLimitDelay={hermesForm.hermesRateLimitDelay}
              onRateLimitDelayChange={
                hermesForm.handleHermesRateLimitDelayChange
              }
            />
          )}

          {/* 配置编辑器：Codex、Claude、Gemini 分别使用不同的编辑器 */}
          {appId === "codex" ? (
            <>
              <CodexConfigEditor
                authValue={codexAuth}
                configValue={codexConfig}
                providerName={form.watch("name")}
                showRemoteCompaction={category !== "official"}
                onAuthChange={setCodexAuth}
                onConfigChange={handleCodexConfigChange}
                authError={codexAuthError}
                configError={codexConfigError}
                inactiveFields={inactiveFields}
              />
              {settingsConfigErrorField}
            </>
          ) : appId === "gemini" ? (
            <>
              <GeminiConfigEditor
                envValue={geminiEnv}
                configValue={geminiConfig}
                onEnvChange={handleGeminiEnvChange}
                onConfigChange={handleGeminiConfigChange}
                envError={envError}
                configError={geminiConfigError}
                inactiveFields={inactiveFields}
              />
              {settingsConfigErrorField}
            </>
          ) : appId === "opencode" &&
            (category === "omo" || category === "omo-slim") ? (
            <div className="space-y-2">
              <Label>{t("provider.configJson")}</Label>
              <JsonEditor
                value={omoDraft.mergedOmoJsonPreview}
                onChange={() => {}}
                rows={3}
                showValidation={false}
                language="json"
                darkMode={isDarkMode}
              />
            </div>
          ) : appId === "opencode" &&
            category !== "omo" &&
            category !== "omo-slim" ? (
            <>
              <div className="space-y-2">
                <Label htmlFor="settingsConfig">
                  {t("provider.configJson")}
                </Label>
                {isNativeOpencode && (
                  <p className="text-sm text-fg-2">
                    {t("opencode.nativeConfigHint")}
                  </p>
                )}
                <JsonEditor
                  value={form.getValues("settingsConfig")}
                  onChange={(config) => form.setValue("settingsConfig", config)}
                  placeholder={
                    isNativeOpencode
                      ? "{}"
                      : `{
  "npm": "@ai-sdk/openai-compatible",
  "options": {
    "baseURL": "https://your-api-endpoint.com",
    "apiKey": "your-api-key-here"
  },
  "models": {}
}`
                  }
                  rows={3}
                  showValidation={true}
                  language="json"
                  darkMode={isDarkMode}
                />
              </div>
              {settingsConfigErrorField}
            </>
          ) : appId === "openclaw" || appId === "hermes" ? (
            <>
              <div className="space-y-2">
                <Label htmlFor="settingsConfig">
                  {t("provider.configJson")}
                </Label>
                <JsonEditor
                  value={form.getValues("settingsConfig")}
                  onChange={(config) => form.setValue("settingsConfig", config)}
                  placeholder={
                    appId === "hermes"
                      ? `{
  "name": "my-provider",
  "base_url": "https://api.example.com/v1",
  "api_key": ""
}`
                      : `{
  "baseUrl": "https://api.example.com/v1",
  "apiKey": "your-api-key-here",
  "api": "openai-completions",
  "models": []
}`
                  }
                  rows={3}
                  showValidation={true}
                  language="json"
                  darkMode={isDarkMode}
                />
              </div>
              <FormField
                control={form.control}
                name="settingsConfig"
                render={() => (
                  <FormItem className="space-y-0">
                    <FormMessage />
                  </FormItem>
                )}
              />
            </>
          ) : (
            <>
              <CommonConfigEditor
                value={form.getValues("settingsConfig")}
                onChange={(value) => form.setValue("settingsConfig", value)}
                inactiveFields={inactiveFields}
              />
              {settingsConfigErrorField}
            </>
          )}

          {showButtons && (
            <div className="flex justify-end gap-2">
              <Button variant="outline" type="button" onClick={onCancel}>
                {t("common.cancel")}
              </Button>
              <Button
                type="submit"
                disabled={isSubmitting || isConfirmSubmitting}
              >
                {submitLabel}
              </Button>
            </div>
          )}
        </form>
      </Form>

      <ConfirmDialog
        isOpen={softIssues !== null && softIssues.length > 0}
        variant="info"
        title={t("providerForm.softValidation.title", {
          defaultValue: "配置存在以下问题",
        })}
        message={
          (softIssues ?? []).map((issue) => `• ${issue}`).join("\n") +
          "\n\n" +
          t("providerForm.softValidation.hint", {
            defaultValue:
              "仍要保存吗？保存后切换此供应商时可能失败，可以之后再补全。",
          })
        }
        confirmText={t("providerForm.softValidation.saveAnyway", {
          defaultValue: "仍要保存",
        })}
        cancelText={t("common.cancel")}
        onConfirm={async () => {
          if (isConfirmSubmitting) return;
          const values = pendingFormValues;
          if (!values) {
            setSoftIssues(null);
            setPendingFormValues(null);
            return;
          }
          setIsConfirmSubmitting(true);
          try {
            await performSubmit(values);
            setSoftIssues(null);
            setPendingFormValues(null);
          } catch (error) {
            console.error("[ProviderForm] soft-confirm submit failed:", error);
            // 保留确认框和 pending values，让用户可以重试或取消
          } finally {
            setIsConfirmSubmitting(false);
          }
        }}
        onCancel={() => {
          if (isConfirmSubmitting) return;
          setSoftIssues(null);
          setPendingFormValues(null);
        }}
      />
    </>
  );
}

export type ProviderFormValues = ProviderFormData & {
  presetId?: string;
  presetCategory?: ProviderCategory;
  meta?: ProviderMeta;
  providerKey?: string; // OpenCode/OpenClaw: user-defined provider key
  suggestedDefaults?: OpenClawSuggestedDefaults; // OpenClaw: suggested default model configuration
};
