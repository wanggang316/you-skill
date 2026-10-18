use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Version 2 keys skill records by id; version 1 keyed them by name.
pub const LOCK_VERSION: u32 = 2;

/// A new skill id: the lock key and the first component of the hub path.
pub fn new_skill_id() -> String {
  Uuid::new_v4().to_string()
}

/// Whether a lock key or hub directory name is a skill id (a version 1 key is a name).
pub fn is_skill_id(key: &str) -> bool {
  Uuid::parse_str(key).is_ok()
}

// ---------------------------------------------------------------------------
// Marketplace / detection DTOs (unchanged wire format, snake_case)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteSkill {
  pub id: String,
  pub skill_id: String,
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub star_count: u64,
  pub heat_score: u64,
  pub install_count: u64,
  pub source: String,
  pub url: Option<String>,
  pub path: Option<String>,
  /// GitHub tree SHA for the skill folder (for update detection)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub skill_path_sha: Option<String>,
  /// Git branch name (e.g., "main", "master")
  #[serde(skip_serializing_if = "Option::is_none")]
  pub branch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteSkillsResponse {
  pub skills: Vec<RemoteSkill>,
  pub total: i64,
  pub has_more: bool,
}

/// A skill found by `detect_*`, staged in a temp directory and ready to be imported.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedSkill {
  pub name: String,
  pub tmp_path: String,
  /// Path of `SKILL.md` relative to the detected root (repo root for GitHub sources).
  pub skill_path: String,
  /// Branch the repository archive was downloaded from (GitHub sources only).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub branch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillDirectoryEntry {
  pub path: String,
  pub is_directory: bool,
  /// Absolute target when the entry is a symlink, `None` for a regular file or directory.
  #[serde(default)]
  pub link_target: Option<String>,
  /// A symlink whose target does not exist.
  #[serde(default)]
  pub link_broken: bool,
}

// ---------------------------------------------------------------------------
// Hub domain (lock file + views, camelCase)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum InstallScope {
  #[default]
  User,
  Project,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
  #[default]
  Copy,
  Symlink,
}

impl InstallMode {
  pub fn from_setting(value: &str) -> Self {
    match value.trim().to_ascii_lowercase().as_str() {
      "symlink" => InstallMode::Symlink,
      _ => InstallMode::Copy,
    }
  }
}

/// Where a hub skill originally came from. `None` covers migrated/adopted skills whose
/// provenance is unknown and skills scanned from an agent directory.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SkillSource {
  #[serde(rename_all = "camelCase")]
  Github {
    /// `owner/repo`
    repo: String,
    url: String,
    /// Path of `SKILL.md` inside the repository, e.g. `skills/foo/SKILL.md`.
    skill_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    /// Git tree SHA of the skill folder at import time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remote_sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    marketplace_id: Option<String>,
    /// Latest tree SHA observed by `check_source_updates`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latest_remote_sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    checked_at: Option<String>,
  },
  #[serde(rename_all = "camelCase")]
  Folder { path: String },
  #[serde(rename_all = "camelCase")]
  Zip { path: String },
  #[default]
  None,
}

impl SkillSource {
  pub fn github_repo(&self) -> Option<&str> {
    match self {
      SkillSource::Github { repo, .. } => Some(repo.as_str()),
      _ => None,
    }
  }

  /// The source key: `github:<owner>/<repo>@<branch>:<skillDir>`, `folder:<path>`, or only
  /// the type (`zip`, `none`). A legacy GitHub source without a branch has no `@<branch>`.
  pub fn key(&self) -> String {
    match self {
      SkillSource::Github {
        repo,
        skill_path,
        branch,
        ..
      } => {
        let branch = branch
          .as_deref()
          .filter(|b| !b.is_empty())
          .map(|b| format!("@{}", b))
          .unwrap_or_default();
        format!(
          "github:{}{}:{}",
          repo.trim().to_ascii_lowercase(),
          branch,
          skill_dir_of(skill_path)
        )
      },
      SkillSource::Folder { path } => format!("folder:{}", folder_key_path(path)),
      SkillSource::Zip { .. } => "zip".to_string(),
      SkillSource::None => "none".to_string(),
    }
  }

  /// Whether the source has a location key: YouSkill reads GitHub and folder sources again
  /// after the import, so their location identifies the skill.
  pub fn has_location(&self) -> bool {
    matches!(
      self,
      SkillSource::Github { .. } | SkillSource::Folder { .. }
    )
  }

  /// Whether two sources are the same location. A GitHub source without a branch (from a
  /// lock that did not record it) matches the same skill on any branch.
  pub fn same_location(&self, other: &SkillSource) -> bool {
    match (self, other) {
      (
        SkillSource::Github {
          repo: a_repo,
          skill_path: a_path,
          branch: a_branch,
          ..
        },
        SkillSource::Github {
          repo: b_repo,
          skill_path: b_path,
          branch: b_branch,
          ..
        },
      ) => {
        let branch_matches = match (a_branch.as_deref(), b_branch.as_deref()) {
          (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => a == b,
          _ => true,
        };
        a_repo.trim().eq_ignore_ascii_case(b_repo.trim())
          && skill_dir_of(a_path) == skill_dir_of(b_path)
          && branch_matches
      },
      (SkillSource::Folder { path: a }, SkillSource::Folder { path: b }) => {
        folder_key_path(a) == folder_key_path(b)
      },
      _ => false,
    }
  }
}

/// Whether `reference` (`[github:]owner/repo[@branch]:<dir or name>` or `folder:<path>`)
/// names a skill with this source and name.
fn source_key_matches(source: &SkillSource, name: &str, reference: &str) -> bool {
  if let Some(path) = reference.strip_prefix("folder:") {
    return source.key() == format!("folder:{}", folder_key_path(path));
  }

  let SkillSource::Github {
    repo,
    skill_path,
    branch,
    ..
  } = source
  else {
    return false;
  };
  let reference = reference.strip_prefix("github:").unwrap_or(reference);
  let Some((location, skill)) = reference.split_once(':') else {
    return false;
  };
  let (wanted_repo, wanted_branch) = match location.split_once('@') {
    Some((repo, branch)) => (repo, Some(branch)),
    None => (location, None),
  };
  let branch_matches = match (wanted_branch, branch.as_deref()) {
    (Some(wanted), Some(actual)) if !actual.is_empty() => wanted == actual,
    _ => true,
  };
  repo.trim().eq_ignore_ascii_case(wanted_repo.trim())
    && branch_matches
    && (skill_dir_of(skill_path) == skill.trim_matches('/') || name.eq_ignore_ascii_case(skill))
}

/// `skills/foo/SKILL.md` -> `skills/foo`; a skill at the repository root is `.`.
fn skill_dir_of(skill_path: &str) -> String {
  let path = skill_path.trim().replace('\\', "/");
  let dir = path
    .strip_suffix("SKILL.md")
    .unwrap_or(&path)
    .trim_matches('/');
  if dir.is_empty() {
    ".".to_string()
  } else {
    dir.to_string()
  }
}

fn folder_key_path(path: &str) -> String {
  let path = path.trim().replace('\\', "/");
  match path.trim_end_matches('/') {
    "" => path,
    trimmed => trimmed.to_string(),
  }
}

/// One deployment of a hub skill. Keyed by `path`; several agent apps may map to the same
/// directory (e.g. every agent that uses `.agents/skills` inside a project).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallRecord {
  pub scope: InstallScope,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub project_path: Option<String>,
  pub path: String,
  pub mode: InstallMode,
  /// Hub hash at the time of the last install/push/adopt (three-way merge base).
  pub hash: String,
  pub installed_at: String,
  #[serde(default)]
  pub agent_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
  /// Install directory name and agent runtime name. Empty only in a version 1 record,
  /// where the lock key is the name.
  #[serde(default)]
  pub name: String,
  #[serde(default)]
  pub source: SkillSource,
  /// Content hash of the hub directory at import or last accepted change.
  pub hash: String,
  pub imported_at: String,
  pub updated_at: String,
  #[serde(default)]
  pub installs: Vec<InstallRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub completed_at: Option<String>,
  #[serde(default)]
  pub imported: Vec<String>,
  #[serde(default)]
  pub installs: usize,
  #[serde(default)]
  pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LockFile {
  #[serde(default = "default_lock_version")]
  pub version: u32,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub migration: Option<MigrationReport>,
  #[serde(default)]
  pub skills: BTreeMap<String, SkillRecord>,
}

fn default_lock_version() -> u32 {
  LOCK_VERSION
}

impl Default for LockFile {
  fn default() -> Self {
    Self {
      version: LOCK_VERSION,
      migration: None,
      skills: BTreeMap::new(),
    }
  }
}

impl LockFile {
  /// Find a skill by id, else by source key, else by name (names compare
  /// case-insensitively). A source key can leave out `github:` and `@<branch>`, and can end
  /// in the skill name instead of its directory: `owner/repo:tdd`. `Ok(None)` when nothing
  /// matches; an error when more than one skill matches.
  pub fn resolve(&self, key: &str) -> Result<Option<(String, SkillRecord)>, String> {
    let key = key.trim();
    if let Some((id, record)) = self
      .skills
      .iter()
      .find(|(id, _)| id.eq_ignore_ascii_case(key))
    {
      return Ok(Some((id.clone(), record.clone())));
    }
    let matches: Vec<(&String, &SkillRecord)> = if key.contains(':') {
      self
        .skills
        .iter()
        .filter(|(_, record)| source_key_matches(&record.source, &record.name, key))
        .collect()
    } else {
      self
        .skills
        .iter()
        .filter(|(_, record)| record.name.eq_ignore_ascii_case(key))
        .collect()
    };
    match matches.as_slice() {
      [] => Ok(None),
      [(id, record)] => Ok(Some(((*id).clone(), (*record).clone()))),
      _ => Err(format!(
        "ambiguous skill '{}': {}",
        key,
        matches
          .iter()
          .map(|(id, record)| format!("{} ({})", id, record.source.key()))
          .collect::<Vec<_>>()
          .join(", ")
      )),
    }
  }

  /// The skill with this name (case-insensitive). The oldest import wins when more than
  /// one skill has the name.
  pub fn find_by_name(&self, name: &str) -> Option<(&String, &SkillRecord)> {
    self
      .skills
      .iter()
      .filter(|(_, record)| record.name.eq_ignore_ascii_case(name))
      .min_by(|a, b| a.1.imported_at.cmp(&b.1.imported_at))
  }
}

// ---------------------------------------------------------------------------
// Drift states
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HubState {
  Ok,
  /// Hub directory content differs from the recorded hash.
  Modified,
  Missing,
  /// Hub directory exists but has no readable `SKILL.md`.
  Invalid,
  /// `SKILL.md` frontmatter name no longer matches the record name.
  NameMismatch,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceState {
  InSync,
  UpdateAvailable,
  SourceModified,
  SourceMissing,
  NotCheckable,
  Unchecked,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetState {
  InSync,
  /// Target untouched since install, hub moved on: safe to push.
  Outdated,
  /// Target edited, hub unchanged since install: safe to adopt.
  Modified,
  /// Both target and hub changed since install.
  Conflict,
  Missing,
  BrokenLink,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallView {
  #[serde(flatten)]
  pub record: InstallRecord,
  pub state: TargetState,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub current_hash: Option<String>,
  pub project_missing: bool,
  #[serde(default)]
  pub missing_agent_ids: Vec<String>,
  /// Where the target links to when it is a symlink on disk, whatever `mode` records.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub link_target: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HubSkillView {
  pub id: String,
  pub name: String,
  pub source_key: String,
  pub hub_path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub source: SkillSource,
  pub hash: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hub_hash: Option<String>,
  pub hub_state: HubState,
  pub source_state: SourceState,
  pub imported_at: String,
  pub updated_at: String,
  pub installs: Vec<InstallView>,
  pub has_drift: bool,
}

// ---------------------------------------------------------------------------
// Command payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallTargetSpec {
  pub scope: InstallScope,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub project_path: Option<String>,
  pub agent_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
  pub name: String,
  pub targets: Vec<InstallTargetSpec>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub mode: Option<InstallMode>,
  #[serde(default)]
  pub force: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UninstallRequest {
  pub name: String,
  pub targets: Vec<InstallTargetSpec>,
  /// Install directories to drop as a whole, whichever agents read them.
  #[serde(default)]
  pub paths: Vec<String>,
  #[serde(default)]
  pub force: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportItem {
  pub name: String,
  pub tmp_path: String,
  #[serde(default)]
  pub source: SkillSource,
  /// The record to replace (needs `overwrite`). `None` adds a new record, except that a
  /// source already in the hub always replaces its record.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub replace_id: Option<String>,
}

/// A hub skill that an import item can replace.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillMatch {
  pub id: String,
  pub name: String,
  pub source_key: String,
  /// Same location key: the import is a new version of this skill.
  pub same_source: bool,
  pub updated_at: String,
}

/// What an import item does by default, from `plan_import`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
  pub name: String,
  pub source_key: String,
  /// Records with the same location key, then records with the same name.
  pub matches: Vec<SkillMatch>,
  /// The record replaced by default; `None` adds a new record.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub default_replace_id: Option<String>,
  /// The source is already in the hub, so the import can only replace that record.
  pub replace_required: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
  pub id: String,
  pub name: String,
  pub hub_path: String,
  pub hash: String,
  pub replaced: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncAction {
  PullSource {
    #[serde(default)]
    force: bool,
  },
  PushTargets {
    /// Install paths to push to; `None` means every copy target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    targets: Option<Vec<String>>,
    #[serde(default)]
    force: bool,
  },
  AdoptTarget {
    path: String,
    #[serde(default)]
    force: bool,
  },
  AcceptHub,
  PushSource {
    #[serde(default)]
    force: bool,
  },
}

/// Result of a mutating hub action. `applied = false` with non-empty `blockers` means the
/// action was refused because it would discard local changes; retry with `force`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
  pub applied: bool,
  #[serde(default)]
  pub blockers: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub skill: Option<HubSkillView>,
}

impl ActionResult {
  pub fn blocked(blockers: Vec<String>, skill: Option<HubSkillView>) -> Self {
    Self {
      applied: false,
      blockers,
      skill,
    }
  }

  pub fn applied(skill: HubSkillView) -> Self {
    Self {
      applied: true,
      blockers: Vec::new(),
      skill: Some(skill),
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SourceUpdate {
  pub id: String,
  pub name: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub remote_sha: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub latest_remote_sha: Option<String>,
  pub update_available: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Diff
// ---------------------------------------------------------------------------

/// What the hub copy is compared against. The hub is always the left (old) side.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffAgainst {
  /// An install target, identified by its install path.
  Target { path: String },
  /// The recorded source: a local folder, or a GitHub checkout downloaded on demand.
  Source,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiffStatus {
  /// Present only on the right side.
  Added,
  /// Present only in the hub.
  Removed,
  Modified,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiffLineKind {
  Context,
  Delete,
  Insert,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
  pub kind: DiffLineKind,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub old_line: Option<usize>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub new_line: Option<usize>,
  pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
  pub header: String,
  pub lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
  /// Path relative to the skill root, `/` separated.
  pub path: String,
  pub status: DiffStatus,
  /// Content differs but cannot be shown as text.
  pub binary: bool,
  /// Hunks were cut off (file too large or too many changed lines).
  pub truncated: bool,
  #[serde(default)]
  pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillDiff {
  pub name: String,
  pub left_label: String,
  pub right_label: String,
  #[serde(default)]
  pub files: Vec<FileDiff>,
  /// Number of files that are identical on both sides.
  pub unchanged: usize,
}

// ---------------------------------------------------------------------------
// Scan
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
  /// Not in the hub yet.
  New,
  /// Same name and same content as the hub copy.
  Identical,
  /// Same name, different content.
  Different,
  /// Symlink that already points into the hub.
  Linked,
  /// The directory is inside the hub itself.
  Hub,
  InvalidName,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRootMatch {
  pub scope: InstallScope,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub project_path: Option<String>,
  pub agent_ids: Vec<String>,
  pub registered_project: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanItem {
  /// The hub skill this folder belongs to; `None` when it is new.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub name: String,
  pub path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hash: Option<String>,
  pub status: ScanStatus,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hub_hash: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub in_agent_root: Option<AgentRootMatch>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source_hint: Option<SkillSource>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScanResolution {
  Import,
  AdoptIntoHub,
  PushFromHub,
  /// Leave the hub alone and only record this folder as an install location. The skill has
  /// to be in the hub already, or arrive there through another decision of the same batch.
  RegisterOnly,
  Skip,
}

impl ScanResolution {
  /// Whether the resolution writes to the hub. Those run before the register-only decisions
  /// of the same skill, so the install records get the right merge base.
  pub fn writes_hub(self) -> bool {
    matches!(self, Self::Import | Self::AdoptIntoHub)
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanDecision {
  /// The hub skill from the scan item; `None` selects the oldest skill with the name.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub name: String,
  pub path: String,
  pub resolution: ScanResolution,
  #[serde(default)]
  pub register_install: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source: Option<SkillSource>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lock_file_round_trips_with_camel_case() {
    let mut lock = LockFile::default();
    lock.skills.insert(
      new_skill_id(),
      SkillRecord {
        name: "foo".to_string(),
        source: SkillSource::Github {
          repo: "owner/repo".to_string(),
          url: "https://github.com/owner/repo.git".to_string(),
          skill_path: "skills/foo/SKILL.md".to_string(),
          branch: Some("main".to_string()),
          remote_sha: Some("abc".to_string()),
          marketplace_id: None,
          latest_remote_sha: None,
          checked_at: None,
        },
        hash: "h".to_string(),
        imported_at: "t".to_string(),
        updated_at: "t".to_string(),
        installs: vec![InstallRecord {
          scope: InstallScope::Project,
          project_path: Some("/p".to_string()),
          path: "/p/.agents/skills/foo".to_string(),
          mode: InstallMode::Symlink,
          hash: "h".to_string(),
          installed_at: "t".to_string(),
          agent_ids: vec!["codex".to_string(), "cursor".to_string()],
        }],
      },
    );
    let json = serde_json::to_string_pretty(&lock).unwrap();
    assert!(json.contains("\"skillPath\""));
    assert!(json.contains("\"projectPath\""));
    assert!(json.contains("\"agentIds\""));
    assert!(json.contains("\"type\": \"github\""));
    let parsed: LockFile = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, lock);
  }

  #[test]
  fn missing_source_defaults_to_none() {
    let json = r#"{"version":1,"skills":{"x":{"hash":"h","importedAt":"t","updatedAt":"t"}}}"#;
    let parsed: LockFile = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.skills["x"].source, SkillSource::None);
    assert!(parsed.skills["x"].installs.is_empty());
    assert!(parsed.skills["x"].name.is_empty());
  }

  fn named(name: &str, imported_at: &str) -> SkillRecord {
    SkillRecord {
      name: name.to_string(),
      source: SkillSource::None,
      hash: "h".to_string(),
      imported_at: imported_at.to_string(),
      updated_at: imported_at.to_string(),
      installs: Vec::new(),
    }
  }

  #[test]
  fn resolve_by_id_then_by_unique_name() {
    let mut lock = LockFile::default();
    let foo = new_skill_id();
    let tdd_a = new_skill_id();
    let tdd_b = new_skill_id();
    lock.skills.insert(foo.clone(), named("foo", "1"));
    lock.skills.insert(tdd_a.clone(), named("tdd", "1"));
    lock.skills.insert(tdd_b.clone(), named("tdd", "2"));

    assert_eq!(lock.resolve(&foo).unwrap().unwrap().0, foo);
    assert_eq!(
      lock.resolve(&tdd_b.to_uppercase()).unwrap().unwrap().0,
      tdd_b
    );
    assert_eq!(lock.resolve("FOO").unwrap().unwrap().0, foo);
    assert!(lock.resolve("missing").unwrap().is_none());
    let err = lock.resolve("tdd").unwrap_err();
    assert!(err.contains(&tdd_a) && err.contains(&tdd_b));
    assert!(err.starts_with("ambiguous skill 'tdd'"), "{err}");
    assert_eq!(lock.find_by_name("tdd").unwrap().0, &tdd_a);
  }

  fn github(repo: &str, skill_path: &str, branch: Option<&str>) -> SkillSource {
    SkillSource::Github {
      repo: repo.to_string(),
      url: String::new(),
      skill_path: skill_path.to_string(),
      branch: branch.map(str::to_string),
      remote_sha: None,
      marketplace_id: None,
      latest_remote_sha: None,
      checked_at: None,
    }
  }

  #[test]
  fn source_keys() {
    assert_eq!(
      github("Owner/Repo", "skills/tdd/SKILL.md", Some("main")).key(),
      "github:owner/repo@main:skills/tdd"
    );
    assert_eq!(github("o/r", "SKILL.md", None).key(), "github:o/r:.");
    assert_eq!(
      SkillSource::Folder {
        path: "/a/b/tdd/".to_string()
      }
      .key(),
      "folder:/a/b/tdd"
    );
    assert_eq!(
      SkillSource::Zip {
        path: "/x.zip".to_string()
      }
      .key(),
      "zip"
    );
    assert_eq!(SkillSource::None.key(), "none");
  }

  #[test]
  fn same_location_compares_branch_only_when_both_have_one() {
    let main = github("o/r", "skills/tdd/SKILL.md", Some("main"));
    assert!(main.same_location(&github("O/R", "skills/tdd/SKILL.md", Some("main"))));
    assert!(!main.same_location(&github("o/r", "skills/tdd/SKILL.md", Some("next"))));
    assert!(main.same_location(&github("o/r", "skills/tdd/SKILL.md", None)));
    assert!(!main.same_location(&github("o/r", "skills/other/SKILL.md", Some("main"))));
    let zip = SkillSource::Zip {
      path: "/x.zip".to_string(),
    };
    assert!(!zip.same_location(&zip));
    assert!(!SkillSource::None.same_location(&SkillSource::None));
  }

  #[test]
  fn resolve_by_source_key() {
    let mut lock = LockFile::default();
    let main = new_skill_id();
    let next = new_skill_id();
    let folder = new_skill_id();
    let mut with_source = |id: &String, source: SkillSource| {
      let mut record = named("tdd", "1");
      record.source = source;
      lock.skills.insert(id.clone(), record);
    };
    with_source(&main, github("o/r", "skills/tdd/SKILL.md", Some("main")));
    with_source(&next, github("o/r", "skills/tdd/SKILL.md", Some("next")));
    with_source(
      &folder,
      SkillSource::Folder {
        path: "/src/tdd".to_string(),
      },
    );

    let id_of = |key: &str| lock.resolve(key).unwrap().map(|(id, _)| id);
    assert_eq!(id_of("github:o/r@main:skills/tdd"), Some(main.clone()));
    assert_eq!(id_of("O/R@next:tdd"), Some(next.clone()));
    assert_eq!(id_of("folder:/src/tdd/"), Some(folder.clone()));
    assert_eq!(id_of("o/r@dev:tdd"), None);
    let err = lock.resolve("o/r:tdd").unwrap_err();
    assert!(
      err.contains("github:o/r@main:skills/tdd") && err.contains("github:o/r@next:skills/tdd")
    );
  }

  #[test]
  fn skill_ids_are_uuids() {
    assert!(is_skill_id(&new_skill_id()));
    assert!(!is_skill_id("tdd"));
  }
}
