//! The instruction library: agent instruction files (`AGENTS.md`, `CLAUDE.md`, ...) kept
//! once under `~/.youskill/instructions/<name>.md` and deployed to the file each agent
//! reads. Install records reuse the skill types; only the unit differs (a file, not a
//! directory).

use crate::models::{HubState, InstallRecord, InstallView, LOCK_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where an instruction came from. Unlike skills there is nothing to sync with; the source
/// is shown so the user knows what a library entry is.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstructionSource {
  #[serde(rename_all = "camelCase")]
  Github {
    /// `owner/repo`
    repo: String,
    /// Path of the file inside the repository.
    file_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
  },
  /// A Markdown file that is not an agent location.
  #[serde(rename_all = "camelCase")]
  File { path: String },
  /// The instruction file of an agent, adopted or imported from there.
  #[serde(rename_all = "camelCase")]
  Agent { path: String },
  /// Created in the app.
  #[default]
  None,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionRecord {
  #[serde(default)]
  pub source: InstructionSource,
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
  pub source: InstructionSource,
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
  /// Where the file came from; derived from the path when absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source: Option<InstructionSource>,
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

/// A Markdown file found in a folder, among picked files, or in a downloaded repository.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInstruction {
  /// Suggested library name; the user may change it before importing.
  pub name: String,
  /// Absolute path to copy from (a temp directory for GitHub sources).
  pub path: String,
  /// Path relative to the folder or repository it was found in.
  pub rel_path: String,
  pub file_name: String,
  /// Set for downloaded files, whose temp path says nothing about the origin.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source: Option<InstructionSource>,
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
        source: InstructionSource::Agent {
          path: "/p/AGENTS.md".to_string(),
        },
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
    assert!(json.contains("\"type\": \"agent\""));
    let parsed: InstructionLockFile = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, lock);
  }
}
