use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const LOCK_FILE: &str = "native-skill-lock.json";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSkillLockEntry {
  pub installed_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NativeSkillLockFile {
  #[serde(default = "default_version")]
  pub version: i64,
  #[serde(default)]
  pub skills: HashMap<String, NativeSkillLockEntry>,
}

impl Default for NativeSkillLockFile {
  fn default() -> Self {
    Self {
      version: 1,
      skills: HashMap::new(),
    }
  }
}

fn default_version() -> i64 {
  1
}

fn native_skill_lock_path() -> Result<PathBuf, String> {
  let config_dir = dirs_next::config_dir().ok_or("Unable to get config directory")?;
  Ok(config_dir.join("youskill").join(LOCK_FILE))
}

pub fn read_native_skill_lock_internal() -> Result<NativeSkillLockFile, String> {
  let lock_path = native_skill_lock_path()?;
  if !lock_path.exists() {
    return Ok(NativeSkillLockFile::default());
  }
  let content =
    fs::read_to_string(&lock_path).map_err(|e| format!("Failed to read lock file: {}", e))?;
  serde_json::from_str(&content).map_err(|e| format!("Failed to parse lock file: {}", e))
}

fn write_native_skill_lock_internal(lock: &NativeSkillLockFile) -> Result<(), String> {
  let lock_path = native_skill_lock_path()?;
  if let Some(parent) = lock_path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("Failed to create lock file directory: {}", e))?;
  }
  let content = serde_json::to_string_pretty(lock)
    .map_err(|e| format!("Failed to serialize lock file: {}", e))?;
  fs::write(&lock_path, content).map_err(|e| format!("Failed to write lock file: {}", e))?;
  Ok(())
}

pub fn add_skill_to_native_lock(skill_name: String) -> Result<(), String> {
  let mut lock = read_native_skill_lock_internal()?;
  lock.version = 1;

  let now = chrono::Utc::now().to_rfc3339();
  let existing_installed_at = lock.skills.get(&skill_name).map(|e| e.installed_at.clone());

  lock.skills.insert(
    skill_name,
    NativeSkillLockEntry {
      installed_at: existing_installed_at.unwrap_or_else(|| now.clone()),
      updated_at: now,
    },
  );

  write_native_skill_lock_internal(&lock)
}

pub fn remove_skill_from_native_lock(skill_name: String) -> Result<bool, String> {
  let mut lock = read_native_skill_lock_internal()?;
  if !lock.skills.contains_key(&skill_name) {
    return Ok(false);
  }
  lock.skills.remove(&skill_name);
  write_native_skill_lock_internal(&lock)?;
  Ok(true)
}
