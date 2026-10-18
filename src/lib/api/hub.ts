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
  /** Where the target links to when it is a symlink on disk, whatever `mode` records. */
  linkTarget?: string | null;
}

export interface HubSkillView {
  /** Stable key of the skill in the hub. */
  id: string;
  /** Install directory name and agent runtime name; more than one skill can have it. */
  name: string;
  /** `github:owner/repo@branch:dir`, `folder:<path>`, or only the type (`zip`, `none`). */
  sourceKey: string;
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
  /** Skill id (a unique name also works). */
  name: string;
  targets: InstallTargetSpec[];
  mode?: InstallMode | null;
  force?: boolean;
}

export interface UninstallRequest {
  /** Skill id (a unique name also works). */
  name: string;
  targets: InstallTargetSpec[];
  /** Install directories to drop as a whole, whichever agents read them. */
  paths?: string[];
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
  /** Skill to replace (needs overwrite); omitted adds a new skill. */
  replaceId?: string | null;
}

export interface SkillMatch {
  id: string;
  name: string;
  sourceKey: string;
  /** Same GitHub or folder location: the import is a new version of this skill. */
  sameSource: boolean;
  updatedAt: string;
}

export interface ImportPlan {
  name: string;
  sourceKey: string;
  matches: SkillMatch[];
  /** Skill replaced by default; omitted adds a new skill. */
  defaultReplaceId?: string | null;
  /** The source is already in the hub, so the import can only replace that skill. */
  replaceRequired: boolean;
}

export interface ImportOutcome {
  id: string;
  name: string;
  hubPath: string;
  hash: string;
  replaced: boolean;
}

export interface SourceUpdate {
  id: string;
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
  /** Hub skill the folder belongs to; omitted when it is new. */
  id?: string | null;
  name: string;
  path: string;
  hash?: string | null;
  status: ScanStatus;
  hubHash?: string | null;
  inAgentRoot?: AgentRootMatch | null;
  sourceHint?: SkillSource | null;
  error?: string | null;
}

export type ScanResolution =
  | "import"
  | "adopt_into_hub"
  | "push_from_hub"
  /** Record the folder as an install location, leaving the library copy untouched. */
  | "register_only"
  | "skip";

export interface ScanDecision {
  id?: string | null;
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

export async function getHubSkill(id: string): Promise<HubSkillView | null> {
  return apiCall<HubSkillView | null>("get_hub_skill", { id });
}

export async function planImport(items: ImportItem[]): Promise<ImportPlan[]> {
  return apiCall<ImportPlan[]>("plan_import", { items });
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

export async function removeHubSkill(id: string, removeInstalls: boolean): Promise<void> {
  return apiCall<void>("remove_hub_skill", { id, removeInstalls });
}

export async function syncSkill(id: string, action: SyncAction): Promise<ActionResult> {
  return apiCall<ActionResult>("sync_skill", { id, action });
}

export async function checkSourceUpdates(ids?: string[] | null): Promise<SourceUpdate[]> {
  return apiCall<SourceUpdate[]>("check_source_updates", { ids: ids ?? null });
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

export async function diffSkill(id: string, against: DiffAgainst): Promise<SkillDiff> {
  return apiCall<SkillDiff>("diff_skill", { id, against });
}

// ============ Helpers ============

export function sourceLabel(source: SkillSource): string {
  switch (source.type) {
    case "github":
      return source.branch ? `${source.repo}@${source.branch}` : source.repo;
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

/** One install scope: the user level, or a single project. */
export interface ScopeRef {
  scope: InstallScope;
  projectPath: string | null;
}

/** Anything with install records: a hub skill or a library instruction. */
export interface Installable {
  installs: InstallView[];
}

export function scopedInstalls(item: Installable, ref: ScopeRef): InstallView[] {
  return item.installs.filter(
    (install) =>
      install.scope === ref.scope &&
      (ref.scope === "user" || install.projectPath === ref.projectPath)
  );
}

/** Drift as seen from one scope: hub or source issues, or drifted targets in that scope. */
export function scopedHasDrift(skill: HubSkillView, ref: ScopeRef): boolean {
  if (skill.hubState !== "ok") return true;
  if (
    skill.sourceState === "update_available" ||
    skill.sourceState === "source_modified" ||
    skill.sourceState === "source_missing"
  ) {
    return true;
  }
  return scopedInstalls(skill, ref).some((install) => install.state !== "in_sync");
}

export function installedAgentIds(
  item: Installable,
  scope: InstallScope,
  projectPath: string | null
): string[] {
  const ids = new Set<string>();
  for (const install of item.installs) {
    if (install.scope !== scope) continue;
    if (scope === "project" && install.projectPath !== projectPath) continue;
    for (const id of install.agentIds) ids.add(id);
  }
  return [...ids];
}
