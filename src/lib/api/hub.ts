/**
 * Hub API
 *
 * Central skill repository (~/.youskill): import, install, drift, scan, migration.
 * Payloads use camelCase to match the lock file format.
 */

import { apiCall } from "./index";

// ============ Types ============

export type InstallScope = "user" | "project";
export type InstallMode = "copy" | "symlink";

export type SkillSource =
  | {
      type: "github";
      repo: string;
      url: string;
      skillPath: string;
      branch?: string | null;
      remoteSha?: string | null;
      marketplaceId?: string | null;
      latestRemoteSha?: string | null;
      checkedAt?: string | null;
    }
  | { type: "folder"; path: string }
  | { type: "zip"; path: string }
  | { type: "none" };

export type HubState = "ok" | "modified" | "missing" | "invalid" | "name_mismatch";

export type SourceState =
  | "in_sync"
  | "update_available"
  | "source_modified"
  | "source_missing"
  | "not_checkable"
  | "unchecked";

export type TargetState =
  | "in_sync"
  | "outdated"
  | "modified"
  | "conflict"
  | "missing"
  | "broken_link";

export interface InstallView {
  scope: InstallScope;
  projectPath?: string | null;
  path: string;
  mode: InstallMode;
  hash: string;
  installedAt: string;
  agentIds: string[];
  state: TargetState;
  currentHash?: string | null;
  projectMissing: boolean;
  missingAgentIds: string[];
}

export interface HubSkillView {
  name: string;
  hubPath: string;
  description?: string | null;
  source: SkillSource;
  hash: string;
  hubHash?: string | null;
  hubState: HubState;
  sourceState: SourceState;
  importedAt: string;
  updatedAt: string;
  installs: InstallView[];
  hasDrift: boolean;
}

export interface InstallTargetSpec {
  scope: InstallScope;
  projectPath?: string | null;
  agentId: string;
}

export interface InstallRequest {
  name: string;
  targets: InstallTargetSpec[];
  mode?: InstallMode | null;
  force?: boolean;
}

export interface UninstallRequest {
  name: string;
  targets: InstallTargetSpec[];
  force?: boolean;
}

export interface ActionResult {
  applied: boolean;
  blockers: string[];
  skill?: HubSkillView | null;
}

export type SyncAction =
  | { kind: "pull_source"; force?: boolean }
  | { kind: "push_targets"; targets?: string[] | null; force?: boolean }
  | { kind: "adopt_target"; path: string; force?: boolean }
  | { kind: "accept_hub" }
  | { kind: "push_source"; force?: boolean };

export interface ImportItem {
  name: string;
  tmpPath: string;
  source: SkillSource;
}

export interface ImportOutcome {
  name: string;
  hubPath: string;
  hash: string;
  replaced: boolean;
}

export interface SourceUpdate {
  name: string;
  remoteSha?: string | null;
  latestRemoteSha?: string | null;
  updateAvailable: boolean;
  error?: string | null;
}

export type ScanStatus = "new" | "identical" | "different" | "linked" | "hub" | "invalid_name";

export interface AgentRootMatch {
  scope: InstallScope;
  projectPath?: string | null;
  agentIds: string[];
  registeredProject: boolean;
}

export interface ScanItem {
  name: string;
  path: string;
  hash?: string | null;
  status: ScanStatus;
  hubHash?: string | null;
  inAgentRoot?: AgentRootMatch | null;
  sourceHint?: SkillSource | null;
  error?: string | null;
}

export type ScanResolution = "import" | "adopt_into_hub" | "push_from_hub" | "skip";

export interface ScanDecision {
  name: string;
  path: string;
  resolution: ScanResolution;
  registerInstall: boolean;
  source?: SkillSource | null;
}

export interface MigrationReport {
  completedAt?: string | null;
  imported: string[];
  installs: number;
  errors: string[];
}

// ---- Diff ----

/** What the hub copy is compared against; the hub is always the left (old) side. */
export type DiffAgainst = { kind: "target"; path: string } | { kind: "source" };

export type DiffStatus = "added" | "removed" | "modified";

export type DiffLineKind = "context" | "delete" | "insert";

export interface DiffLine {
  kind: DiffLineKind;
  oldLine?: number;
  newLine?: number;
  text: string;
}

export interface DiffHunk {
  header: string;
  lines: DiffLine[];
}

export interface FileDiff {
  path: string;
  status: DiffStatus;
  binary: boolean;
  truncated: boolean;
  hunks: DiffHunk[];
}

export interface SkillDiff {
  name: string;
  leftLabel: string;
  rightLabel: string;
  files: FileDiff[];
  unchanged: number;
}

// ============ Commands ============

export async function listHubSkills(): Promise<HubSkillView[]> {
  return apiCall<HubSkillView[]>("list_hub_skills");
}

export async function getHubSkill(name: string): Promise<HubSkillView | null> {
  return apiCall<HubSkillView | null>("get_hub_skill", { name });
}

export async function importSkills(
  items: ImportItem[],
  overwrite = false
): Promise<ImportOutcome[]> {
  return apiCall<ImportOutcome[]>("import_skills", { items, overwrite });
}

export async function installSkill(request: InstallRequest): Promise<ActionResult> {
  return apiCall<ActionResult>("install_skill", { request });
}

export async function uninstallSkill(request: UninstallRequest): Promise<ActionResult> {
  return apiCall<ActionResult>("uninstall_skill", { request });
}

export async function removeHubSkill(name: string, removeInstalls: boolean): Promise<void> {
  return apiCall<void>("remove_hub_skill", { name, removeInstalls });
}

export async function syncSkill(name: string, action: SyncAction): Promise<ActionResult> {
  return apiCall<ActionResult>("sync_skill", { name, action });
}

export async function checkSourceUpdates(names?: string[] | null): Promise<SourceUpdate[]> {
  return apiCall<SourceUpdate[]>("check_source_updates", { names: names ?? null });
}

export async function scanFolder(path: string, maxDepth?: number): Promise<ScanItem[]> {
  return apiCall<ScanItem[]>("scan_folder", { path, maxDepth: maxDepth ?? null });
}

export async function importScanned(decisions: ScanDecision[]): Promise<ImportOutcome[]> {
  return apiCall<ImportOutcome[]>("import_scanned", { decisions });
}

export async function migrateLegacy(): Promise<MigrationReport> {
  return apiCall<MigrationReport>("migrate_legacy");
}

export async function migrationStatus(): Promise<MigrationReport | null> {
  return apiCall<MigrationReport | null>("migration_status");
}

export async function diffSkill(name: string, against: DiffAgainst): Promise<SkillDiff> {
  return apiCall<SkillDiff>("diff_skill", { name, against });
}

// ============ Helpers ============

export function sourceLabel(source: SkillSource): string {
  switch (source.type) {
    case "github":
      return source.repo;
    case "folder":
    case "zip":
      return source.path;
    default:
      return "";
  }
}

export function sourceRepoUrl(source: SkillSource): string | null {
  if (source.type !== "github") return null;
  const base = source.url.replace(/\.git$/, "");
  const folder = source.skillPath.replace(/\/?SKILL\.md$/, "");
  if (!folder) return base;
  return `${base}/tree/${source.branch || "main"}/${folder}`;
}

export function targetsForProject(skill: HubSkillView, projectPath: string): InstallView[] {
  return skill.installs.filter(
    (install) => install.scope === "project" && install.projectPath === projectPath
  );
}

export function installedAgentIds(
  skill: HubSkillView,
  scope: InstallScope,
  projectPath: string | null
): string[] {
  const ids = new Set<string>();
  for (const install of skill.installs) {
    if (install.scope !== scope) continue;
    if (scope === "project" && install.projectPath !== projectPath) continue;
    for (const id of install.agentIds) ids.add(id);
  }
  return [...ids];
}
