import type { ProviderCategory } from "@/types";
import type { PresetFamilyFields } from "./presetFamilies";

/**
 * Gemini 预设供应商的视觉主题配置
 */
export interface GeminiPresetTheme {
  /** 图标类型：'gemini' | 'generic' */
  icon?: "gemini" | "generic";
  /** 背景色（选中状态），支持 hex 颜色 */
  backgroundColor?: string;
  /** 文字色（选中状态），支持 hex 颜色 */
  textColor?: string;
}

export interface GeminiProviderPreset extends PresetFamilyFields {
  name: string;
  nameKey?: string; // i18n key for localized display name
  websiteUrl: string;
  apiKeyUrl?: string;
  settingsConfig: object;
  baseURL?: string;
  model?: string;
  description?: string;
  category?: ProviderCategory;
  partnerPromotionKey?: string;
  endpointCandidates?: string[];
  theme?: GeminiPresetTheme;
  // 图标配置
  icon?: string; // 图标名称
  iconColor?: string; // 图标颜色
}

export const geminiProviderPresets: GeminiProviderPreset[] = [
  {
    name: "Google Official",
    websiteUrl: "https://ai.google.dev/",
    apiKeyUrl: "https://aistudio.google.com/apikey",
    settingsConfig: {
      env: {},
    },
    description: "Google 官方 Gemini API (OAuth)",
    category: "official",
    partnerPromotionKey: "google-official",
    theme: {
      icon: "gemini",
      backgroundColor: "#4285F4",
      textColor: "#FFFFFF",
    },
    icon: "gemini",
    iconColor: "#4285F4",
  },
  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://www.qiniu.com/ai",
    apiKeyUrl: "https://portal.qiniu.com/ai-inference/api-key",
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://api.qnaigc.com/bypass/vertex",
        GEMINI_MODEL: "gemini-3.6-flash",
      },
    },
    baseURL: "https://api.qnaigc.com/bypass/vertex",
    model: "gemini-3.6-flash",
    description: "Qiniu",
    category: "aggregator",
    endpointCandidates: [
      "https://api.qnaigc.com/bypass/vertex",
      "https://api.modelink.ai/bypass/vertex",
    ],
    icon: "qiniu",
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://open.cherryin.net",
        GEMINI_API_KEY: "",
        GEMINI_MODEL: "google/gemini-3.8-flash",
      },
    },
    baseURL: "https://open.cherryin.net",
    model: "google/gemini-3.8-flash",
    description: "CherryIN",
    category: "aggregator",
    endpointCandidates: ["https://open.cherryin.net"],
    icon: "cherryin",
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://openrouter.ai/api",
        GEMINI_MODEL: "gemini-3.8-flash",
      },
    },
    baseURL: "https://openrouter.ai/api",
    model: "gemini-3.8-flash",
    description: "OpenRouter",
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6566F1",
  },
];
