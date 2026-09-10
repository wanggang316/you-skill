/**
 * Settings API
 *
 * 处理应用设置相关的 IPC 调用
 */

import { apiCall } from "./index";

export interface AppSettings {
  language: "en" | "zh";
  theme: "system" | "light" | "dark";
  /** Default install mode for new targets. */
  sync_mode: "symlink" | "copy";
  backup_folder?: string | null;
  last_backup_time?: string | null;
  openrouter_api_key?: string | null;
  translate_target_language: string;
  translate_model: string;
}

export type EditableSettings = Pick<
  AppSettings,
  | "language"
  | "theme"
  | "sync_mode"
  | "openrouter_api_key"
  | "translate_target_language"
  | "translate_model"
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

/**
 * 设置备份文件夹路径
 */
export async function setBackupFolder(path: string): Promise<string | null> {
  return apiCall<string | null>("set_backup_folder", { path });
}

/**
 * 打开备份文件夹
 */
export async function openBackupFolder(path: string): Promise<void> {
  return apiCall<void>("open_backup_folder", { path });
}

/**
 * 备份中心库（skills + lock 文件）
 */
export async function backupSkills(backupFolder: string): Promise<BackupResult> {
  return apiCall<BackupResult>("backup_skills", { backupFolder });
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

export interface BackupResult {
  success: boolean;
  message: string;
  backup_path: string | null;
  backup_time: string | null;
}
