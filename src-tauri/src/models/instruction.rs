//! The instruction library: agent instruction files (`AGENTS.md`, `CLAUDE.md`, ...) kept
//! once under `~/.youskill/instructions/<name>.md` and deployed to the file each agent
//! reads. Install records reuse the skill types; only the unit differs (a file, not a
//! directory).

use crate::models::{
  AgentRootMatch, HubState, InstallRecord, InstallView, ScanStatus, LOCK_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionRecord {
  /// Content hash of the library file at import or last accepted change.
  pub hash: String,
  pub imported_at: String,
  pub updated_at: String,
  #[serde(default)]
  pub installs: Vec<InstallRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionLockFile {
  #[serde(default = "default_lock_version")]
  pub version: u32,
  #[serde(default)]
  pub instructions: BTreeMap<String, InstructionRecord>,
}

fn default_lock_version() -> u32 {
  LOCK_VERSION
}

impl Default for InstructionLockFile {
  fn default() -> Self {
    Self {
      version: LOCK_VERSION,
      instructions: BTreeMap::new(),
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionView {
  pub name: String,
  pub hub_path: String,
  /// First line of the file, for the list.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub hash: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hub_hash: Option<String>,
  pub hub_state: HubState,
  pub imported_at: String,
  pub updated_at: String,
  pub installs: Vec<InstallView>,
  pub has_drift: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstructionImportItem {
  pub name: String,
  /// Markdown file to copy into the library.
  pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstructionImportOutcome {
  pub name: String,
  pub hub_path: String,
  pub hash: String,
  pub replaced: bool,
}

/// Result of a mutating instruction action; see `ActionResult`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstructionActionResult {
  pub applied: bool,
  #[serde(default)]
  pub blockers: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub instruction: Option<InstructionView>,
}

impl InstructionActionResult {
  pub fn blocked(blockers: Vec<String>, instruction: Option<InstructionView>) -> Self {
    Self {
      applied: false,
      blockers,
      instruction,
    }
  }

  pub fn applied(instruction: InstructionView) -> Self {
    Self {
      applied: true,
      blockers: Vec::new(),
      instruction: Some(instruction),
    }
  }
}

/// An instruction file found at an agent location (user level or a registered project).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstructionScanItem {
  /// Suggested library name; the user may change it before importing.
  pub name: String,
  pub path: String,
  /// `AGENTS.md`, `CLAUDE.md`, ...
  pub file_name: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hash: Option<String>,
  pub status: ScanStatus,
  /// Library entry this file belongs to: the one whose install record covers the path, or
  /// the one with identical content when the path is not registered yet.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hub_name: Option<String>,
  /// The path is already an install record of `hub_name`.
  pub registered: bool,
  pub location: AgentRootMatch,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstructionScanDecision {
  pub name: String,
  pub path: String,
  pub resolution: crate::models::ScanResolution,
  #[serde(default)]
  pub register_install: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hub_name: Option<String>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{InstallMode, InstallScope};

  #[test]
  fn lock_file_round_trips_with_camel_case() {
    let mut lock = InstructionLockFile::default();
    lock.instructions.insert(
      "team-rules".to_string(),
      InstructionRecord {
        hash: "h".to_string(),
        imported_at: "t".to_string(),
        updated_at: "t".to_string(),
        installs: vec![InstallRecord {
          scope: InstallScope::Project,
          project_path: Some("/p".to_string()),
          path: "/p/AGENTS.md".to_string(),
          mode: InstallMode::Copy,
          hash: "h".to_string(),
          installed_at: "t".to_string(),
          agent_ids: vec!["codex".to_string()],
        }],
      },
    );
    let json = serde_json::to_string_pretty(&lock).unwrap();
    assert!(json.contains("\"instructions\""));
    assert!(json.contains("\"projectPath\""));
    let parsed: InstructionLockFile = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, lock);
  }
}
