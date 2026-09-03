use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LOCK_VERSION: u32 = 1;

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
  /// `SKILL.md` frontmatter name no longer matches the record key.
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
  /// Agent app path was reconfigured; files live at the recorded path.
  Relocated,
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
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HubSkillView {
  pub name: String,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
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
  Skip,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanDecision {
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
      "foo".to_string(),
      SkillRecord {
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
  }
}
