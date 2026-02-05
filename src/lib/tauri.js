import { invoke } from '@tauri-apps/api/core'

export const api = {
  scanLocalSkills: () => invoke('scan_local_skills'),
  getScanRoots: () => invoke('get_scan_roots'),
  addScanRoot: (path) => invoke('add_scan_root', { path }),
  removeScanRoot: (path) => invoke('remove_scan_root', { path }),
  deleteSkill: (path) => invoke('delete_skill', { path }),
  moveSkill: (from, to) => invoke('move_skill', { from, to }),
  copySkill: (from, to) => invoke('copy_skill', { from, to }),
  fetchRemoteSkills: (page, pageSize, query) =>
    invoke('fetch_remote_skills', { page, pageSize, query }),
  installSkill: (request) => invoke('install_skill', { request }),
  listAgents: () => invoke('list_agents')
}
