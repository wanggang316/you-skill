import { invoke } from '@tauri-apps/api/core'

export interface LocalSkill {
  name: string
  description?: string | null
  scope: string
  canonical_path: string
  agents: string[]
  managed_status: 'managed' | 'unmanaged' | 'mixed' | 'unknown'
  name_conflict: boolean
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

export const api = {
  scanLocalSkills: () => invoke<LocalSkill[]>('scan_local_skills'),
  getScanRoots: () => invoke<string[]>('get_scan_roots'),
  addScanRoot: (path: string) => invoke<string[]>('add_scan_root', { path }),
  removeScanRoot: (path: string) => invoke<string[]>('remove_scan_root', { path }),
  deleteSkill: (path: string) => invoke('delete_skill', { path }),
  moveSkill: (from: string, to: string) => invoke('move_skill', { from, to }),
  copySkill: (from: string, to: string) => invoke('copy_skill', { from, to }),
  fetchRemoteSkills: (page?: number, pageSize?: number, query?: string) =>
    invoke<RemoteSkillsResponse>('fetch_remote_skills', { page, pageSize, query }),
  installSkill: (request: InstallRequest) => invoke<InstallResult>('install_skill', { request }),
  listAgents: () => invoke<AgentInfo[]>('list_agents'),
  checkCanonicalSkill: (name: string, scope: string) =>
    invoke<CanonicalCheckResult>('check_canonical_skill', { name, scope }),
  unifySkill: (request: UnifyRequest) => invoke<UnifyResult>('unify_skill', { request })
}
