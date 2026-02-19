/**
 * Settings API
 *
 * 处理应用设置相关的 IPC 调用
 */

import { apiCall } from "./index";

export interface AppSettings {
  language: "en" | "zh";
  theme: "system" | "light" | "dark";
  sync_mode: "symlink" | "copy";
  known_skill_install_permission: boolean;
  backup_folder?: string | null;
  last_backup_time?: string | null;
}

/**
 * 获取应用设置
 */
export async function getSettings(): Promise<AppSettings> {
  return apiCall<AppSettings>("get_settings");
}

/**
 * 更新应用设置
 */
export async function updateSettings(
  settings: Pick<AppSettings, "language" | "theme" | "sync_mode" | "known_skill_install_permission">
): Promise<AppSettings> {
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
 * 备份所有技能
 */
export async function backupSkills(backupFolder: string): Promise<BackupResult> {
  return apiCall<BackupResult>("backup_skills", { backupFolder });
}

export interface BackupResult {
  success: boolean;
  message: string;
  backup_path: string | null;
  backup_time: string | null;
}
