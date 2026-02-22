use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const AGENTS_DIR: &str = ".agents";
const LOCK_FILE: &str = ".skill-lock.json";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillLockEntry {
  pub source: String,
  pub source_type: String,
  pub source_url: String,
  pub skill_path: Option<String>,
  pub skill_folder_hash: Option<String>,
  pub installed_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillLockFile {
  #[serde(default = "default_version")]
  pub version: i64,
  #[serde(default)]
  pub skills: HashMap<String, SkillLockEntry>,
  #[serde(default)]
  pub dismissed: HashMap<String, serde_json::Value>,
  #[serde(default, rename = "lastSelectedAgents")]
  pub last_selected_agents: Vec<String>,
}

impl Default for SkillLockFile {
  fn default() -> Self {
    Self {
      version: 3,
      skills: HashMap::new(),
      dismissed: HashMap::new(),
      last_selected_agents: Vec::new(),
    }
  }
}

fn default_version() -> i64 {
  3
}

fn skill_lock_path() -> Result<PathBuf, String> {
  let home_dir = dirs_next::home_dir().ok_or("Unable to get user home directory")?;
  Ok(home_dir.join(AGENTS_DIR).join(LOCK_FILE))
}

pub fn read_skill_lock_internal() -> Result<SkillLockFile, String> {
  let lock_path = skill_lock_path()?;
  if !lock_path.exists() {
    return Ok(SkillLockFile::default());
  }
  let content =
    fs::read_to_string(&lock_path).map_err(|e| format!("Failed to read lock file: {}", e))?;
  serde_json::from_str(&content).map_err(|e| format!("Failed to parse lock file: {}", e))
}

pub fn write_skill_lock_internal(lock: &SkillLockFile) -> Result<(), String> {
  let lock_path = skill_lock_path()?;
  if let Some(parent) = lock_path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("Failed to create lock file directory: {}", e))?;
  }
  let content = serde_json::to_string_pretty(lock)
    .map_err(|e| format!("Failed to serialize lock file: {}", e))?;
  fs::write(&lock_path, content).map_err(|e| format!("Failed to write lock file: {}", e))?;
  Ok(())
}

pub fn add_skill_to_lock(skill_name: String, entry: SkillLockEntry) -> Result<(), String> {
  let mut lock = read_skill_lock_internal()?;
  lock.version = 3;
  let now = chrono::Utc::now().to_rfc3339();
  let existing_entry = lock.skills.get(&skill_name);

  lock.skills.insert(
    skill_name.clone(),
    SkillLockEntry {
      source: entry.source.clone(),
      source_type: entry.source_type.clone(),
      source_url: entry.source_url.clone(),
      skill_path: entry.skill_path.clone(),
      skill_folder_hash: entry.skill_folder_hash,
      installed_at: existing_entry
        .map(|e| e.installed_at.clone())
        .unwrap_or_else(|| now.clone()),
      updated_at: now,
    },
  );
  write_skill_lock_internal(&lock)
}

pub fn remove_skill_from_lock(skill_name: String) -> Result<bool, String> {
  let mut lock = read_skill_lock_internal()?;
  if !lock.skills.contains_key(&skill_name) {
    return Ok(false);
  }
  lock.skills.remove(&skill_name);
  write_skill_lock_internal(&lock)?;
  Ok(true)
}
