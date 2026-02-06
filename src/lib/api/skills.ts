import { invoke } from '@tauri-apps/api/core'

export interface LocalSkill {
  name: string
  description?: string | null
  scope: string
  canonical_path: string
  agents: string[]
  managed_status: 'managed' | 'unmanaged' | 'mixed' | 'unknown'
  name_conflict: boolean
  created_at?: number | null
  conflict_with_managed: boolean
}

export interface RemoteSkill {
  id: string
  skill_id: string
  name: string
  installs: number
  source: string
}

export interface RemoteSkillsResponse {
  skills: RemoteSkill[]
  has_more: boolean
}

export interface InstallRequest {
  source: string
  skill_id: string
  agent: string
  global: boolean
  project_dir?: string | null
}

export interface InstallResult {
  success: boolean
  stdout: string
  stderr: string
  message: string
}

export interface AgentInfo {
  id: string
  display_name: string
  project_path?: string | null
  global_path?: string | null
}

export interface CanonicalCheckResult {
  exists: boolean
  canonical_path: string
}

export interface UnifyRequest {
  name: string
  agent: string
  scope: string
  current_path: string
  prefer: 'canonical' | 'current'
}

export interface UnifyResult {
  success: boolean
  message: string
}

export interface AppSettings {
  language: 'en' | 'zh'
  theme: 'system' | 'light' | 'dark'
  sync_mode: 'symlink' | 'copy'
}

export interface DetectedSkill {
  name: string
  path: string
}

export interface InstallZipRequest {
  zip_path: string
  skill_path: string
  agents: string[]
}

export interface InstallGithubRequest {
  url: string
  skill_path: string
  agents: string[]
}

export interface BackupResult {
  success: boolean
  message: string
  backup_path: string | null
  backup_time: string | null
}

export const api = {
  scanLocalSkills: () => invoke<LocalSkill[]>('scan_local_skills'),
  getScanRoots: () => invoke<string[]>('get_scan_roots'),
  addScanRoot: (path: string) => invoke<string[]>('add_scan_root', { path }),
  removeScanRoot: (path: string) => invoke<string[]>('remove_scan_root', { path }),
  deleteSkill: (path: string) => invoke('delete_skill', { path }),
  fetchRemoteSkills: (page?: number, pageSize?: number, query?: string) =>
    invoke<RemoteSkillsResponse>('fetch_remote_skills', { page, pageSize, query }),
  installSkill: (request: InstallRequest) =>
    invoke<InstallResult>('install_skill', { request }),
  listAgents: () => invoke<AgentInfo[]>('list_agents'),
  checkCanonicalSkill: (name: string, scope: string) =>
    invoke<CanonicalCheckResult>('check_canonical_skill', { name, scope }),
  unifySkill: (request: UnifyRequest) =>
    invoke<UnifyResult>('unify_skill', { request }),
  setAgentLink: (name: string, agent: string, scope: string, linked: boolean) =>
    invoke('set_agent_link', { name, agent, scope, linked }),
  getSettings: () => invoke<AppSettings>('get_settings'),
  updateSettings: (settings: AppSettings) =>
    invoke<AppSettings>('update_settings', { settings }),
  detectGithubSkills: (url: string) =>
    invoke<DetectedSkill[]>('detect_github_skills', { url }),
  detectZipSkills: (zipPath: string) =>
    invoke<DetectedSkill[]>('detect_zip_skills', { zipPath }),
  installZipSkill: (request: InstallZipRequest) =>
    invoke<InstallResult>('install_zip_skill', { request }),
  installGithubSkill: (request: InstallGithubRequest) =>
    invoke<InstallResult>('install_github_skill', { request }),
  getBackupFolder: () => invoke<string | null>('get_backup_folder'),
  setBackupFolder: (path: string) => invoke<string | null>('set_backup_folder', { path }),
  openBackupFolder: (path: string) => invoke<void>('open_backup_folder', { path }),
  backupSkills: (backupFolder: string) => invoke<BackupResult>('backup_skills', { backupFolder }),
  getLastBackupTime: () => invoke<string | null>('get_last_backup_time')
}
