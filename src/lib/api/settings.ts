/**
 * Settings API
 *
 * 处理应用设置相关的 IPC 调用
 */

import { apiCall } from "./index";
import type { SyncPolicy } from "./library-sync";

export type TranslateDisplayMode = "bilingual" | "translation";
export type TranslateTextStyle = "none" | "dashed_underline";

export type AutoSyncInterval = "15m" | "1h" | "6h" | "24h";

/** Library auto sync of this computer; not synced to other computers. */
export interface AutoSyncSettings {
  enabled: boolean;
  backup: boolean;
  restore: boolean;
  interval: AutoSyncInterval;
  policy: SyncPolicy;
}

export const DEFAULT_AUTO_SYNC: AutoSyncSettings = {
  enabled: false,
  backup: true,
  restore: true,
  interval: "1h",
  policy: { repositoryChange: "apply", removal: "ask", conflict: "ask" },
};

export interface AppSettings {
  language: "en" | "zh";
  theme: "system" | "light" | "dark";
  /** Default install mode for new targets. */
  sync_mode: "symlink" | "copy";
  openrouter_api_key?: string | null;
  translate_target_language: string;
  translate_model: string;
  translate_display_mode: TranslateDisplayMode;
  translate_text_style: TranslateTextStyle;
  /** Changed key combinations by action; actions not listed use their default. */
  shortcuts: Record<string, string>;
  auto_sync: AutoSyncSettings;
}

export type EditableSettings = Pick<
  AppSettings,
  | "language"
  | "theme"
  | "sync_mode"
  | "openrouter_api_key"
  | "translate_target_language"
  | "translate_model"
  | "translate_display_mode"
  | "translate_text_style"
  | "shortcuts"
  | "auto_sync"
>;

/**
 * 获取应用设置
 */
export async function getSettings(): Promise<AppSettings> {
  return apiCall<AppSettings>("get_settings");
}

/**
 * 更新应用设置
 */
export async function updateSettings(settings: EditableSettings): Promise<AppSettings> {
  return apiCall<AppSettings>("update_settings", { settings });
}

export interface OpenRouterModelOption {
  id: string;
  name: string;
}

export async function listOpenRouterModels(search?: string): Promise<OpenRouterModelOption[]> {
  return apiCall<OpenRouterModelOption[]>("list_openrouter_models", {
    search: search?.trim() || null,
  });
}
