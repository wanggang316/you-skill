use serde::{Deserialize, Serialize};

/// A folder the user keeps scanning for skills. Unlike a one-off scan, it is registered and
/// listed in the UI so the scanned surface is visible and manageable.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanRoot {
  pub name: String,
  pub path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub last_scanned_at: Option<String>,
  /// How many skill folders the last scan of this root reported.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub last_found: Option<usize>,
}

/// A scan root as listed to the UI, with whether its folder still exists.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanRootView {
  #[serde(flatten)]
  pub root: ScanRoot,
  /// The folder was deleted or renamed since the root was registered.
  pub missing: bool,
}

/// An agent's user-level skills directory that exists but is not registered yet, offered as
/// a one-click way to fill an empty list.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanRootSuggestion {
  pub path: String,
  /// Agents that read this directory.
  pub agent_ids: Vec<String>,
}
