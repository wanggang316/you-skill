/**
 * Skills API
 *
 * 处理技能管理相关的 IPC 调用
 */

import { apiCall } from "./index";

// ============ Types ============

export type SourceType = "github" | "native" | "unknown";

export interface LocalSkill {
  name: string;
  source?: string | null;
  root_folder?: string | null;
  installed_agent_apps: InstalledAgentApp[];
  source_type: SourceType;
}

export interface InstalledAgentApp {
  id: string;
  skill_folder: string;
  method: "symlink" | "copy";
}

export interface RemoteSkill {
  id: string;
  skill_id: string;
  name: string;
  description?: string | null;
  star_count: number;
  heat_score: number;
  install_count: number;
  source: string;
  url?: string | null;
  path?: string | null;
  /** GitHub tree SHA for the skill folder (for update detection) */
  skill_path_sha?: string | null;
  /** Git branch name (e.g., "main", "master") */
  branch?: string | null;
}

export interface RemoteSkillsResponse {
  skills: RemoteSkill[];
  total: number;
  has_more: boolean;
}

export interface InstallResult {
  success: boolean;
  stdout: string;
  stderr: string;
  message: string;
}

export interface AgentInfo {
  id: string;
  display_name: string;
  project_path?: string | null;
  global_path?: string | null;
  is_user_custom: boolean;
}

export interface DetectedSkill {
  name: string;
  tmp_path: string;
  skill_path: string;
}

function normalizeDetectedSkills(
  value: DetectedSkill | DetectedSkill[] | null | undefined
): DetectedSkill[] {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
}

export type InstallMethod = "symlink" | "copy";
export type InstallScope = "global" | "project";

export interface InstallNativeRequest {
  name: string;
  tmp_path: string;
  skill_path: string;
  agent_apps: string[];
  method: InstallMethod;
  scope: InstallScope;
  project_path?: string | null;
}

export interface InstallGithubRequest {
  name: string;
  tmp_path: string;
  skill_path: string;
  source_url: string;
  skill_folder_hash?: string | null;
  agent_apps: string[];
  method: InstallMethod;
  scope: InstallScope;
  project_path?: string | null;
}

export interface InstallUnknownRequest {
  name: string;
  source_path: string;
  agent_apps: string[];
  method: InstallMethod;
  scope: InstallScope;
  project_path?: string | null;
}

export interface SourceCheckResult {
  source_path?: string | null;
  version_groups: SourceVersionGroup[];
  requires_selection: boolean;
}

export interface SourceVersionGroup {
  version: string;
  source_path: string;
  paths: string[];
}

export interface SkillUpdateCheckItem {
  name: string;
  source: string;
  remote_sha: string;
}

export interface ManageSkillAgentAppsRequest {
  name: string;
  source_type: SourceType;
  agent_apps: string[];
  method: InstallMethod;
  source_path: string;
  scope: InstallScope;
  project_path?: string | null;
}

// ============ Local Skills ============

/**
 * 查询本地技能
 */
export async function listSkills(
  scope: InstallScope,
  projectPath?: string | null
): Promise<LocalSkill[]> {
  return apiCall<LocalSkill[]>("list_skills", { scope, projectPath });
}

/**
 * 删除技能（按名称自动解析路径并清理关联）
 */
export async function deleteSkill(
  name: string,
  scope: InstallScope,
  projectPath?: string | null
): Promise<void> {
  return apiCall<void>("delete_skill", { name, scope, projectPath });
}

// ============ Remote Skills ============

/**
 * 获取远程技能列表
 */
export async function fetchRemoteSkills(params?: {
  skip?: number;
  limit?: number;
  search?: string;
  sortBy?: string;
  sortOrder?: string;
}): Promise<RemoteSkillsResponse> {
  return apiCall<RemoteSkillsResponse>("fetch_remote_skills", params || {});
}

/**
 * 批量获取指定名称的技能（用于更新检测）
 */
export async function fetchSkillsByNames(names: string[]): Promise<RemoteSkill[]> {
  return apiCall<RemoteSkill[]>("fetch_skills_by_names", { names });
}

// ============ Skill Management ============

// ============ Detection ============

/**
 * 从 GitHub URL 检测技能
 */
export async function detectGithubManual(githubPath: string): Promise<DetectedSkill[]> {
  return apiCall<DetectedSkill[]>("detect_github_manual", { githubPath });
}

/**
 * 从 ZIP 文件检测技能
 */
export async function detectZip(zipPath: string): Promise<DetectedSkill[]> {
  const result = await apiCall<DetectedSkill | DetectedSkill[]>("detect_zip", { zipPath });
  return normalizeDetectedSkills(result);
}

// ============ Installation ============

/**
 * 从 ZIP 文件安装技能
 */
export async function detectFolder(folderPath: string): Promise<DetectedSkill[]> {
  const result = await apiCall<DetectedSkill | DetectedSkill[]>("detect_folder", { folderPath });
  return normalizeDetectedSkills(result);
}

/**
 * 从 GitHub 安装技能
 */
export async function detectGithubAuto(
  githubPath: string,
  skillName: string
): Promise<DetectedSkill> {
  return apiCall<DetectedSkill>("detect_github_auto", { githubPath, skillName });
}

export async function installFromNative(request: InstallNativeRequest): Promise<InstallResult> {
  return apiCall<InstallResult>("install_from_native", { request });
}

export async function installFromGithub(request: InstallGithubRequest): Promise<InstallResult> {
  return apiCall<InstallResult>("install_from_github", { request });
}

export async function checkSkillVersion(
  name: string,
  rootFolder: string | null | undefined,
  skillPaths: string[]
): Promise<SourceCheckResult> {
  return apiCall<SourceCheckResult>("check_skill_version", {
    name,
    rootFolder,
    skillPaths,
  });
}

export async function installFromUnknown(request: InstallUnknownRequest): Promise<InstallResult> {
  return apiCall<InstallResult>("install_from_unknown", { request });
}

export async function manageSkillAgentApps(
  request: ManageSkillAgentAppsRequest
): Promise<InstallResult> {
  return apiCall<InstallResult>("manage_skill_agent_apps", { request });
}

// ============ Other ============

/**
 * 读取技能 SKILL.md 文件
 */
export async function readSkillFile(skillPath: string): Promise<string> {
  return apiCall<string>("read_skill_file", { skillPath });
}

export async function checkSkillsUpdates(checks: SkillUpdateCheckItem[]): Promise<string[]> {
  if (checks.length === 0) return [];
  try {
    return apiCall<string[]>("check_skills_updates", { checks });
  } catch {
    return [];
  }
}

/**
 * 在文件管理器中打开文件
 */
export async function openInFileManager(filePath: string): Promise<void> {
  return apiCall<void>("open_in_file_manager", { filePath });
}

/**
 * 记录技能安装
 */
export async function recordInstall(skillId: string): Promise<void> {
  return apiCall<void>("record_skill_install", { skillId: skillId });
}
