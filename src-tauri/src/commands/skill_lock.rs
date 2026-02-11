use crate::services::skill_lock_service::{SkillLockEntry, SkillLockFile};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SkillLockEntryWithoutTimestamps {
  pub source: String,
  pub source_type: String,
  pub source_url: String,
  pub skill_path: Option<String>,
  pub skill_folder_hash: Option<String>,
}

impl From<SkillLockEntryWithoutTimestamps> for SkillLockEntry {
  fn from(value: SkillLockEntryWithoutTimestamps) -> Self {
    SkillLockEntry {
      source: value.source,
      source_type: value.source_type,
      source_url: value.source_url,
      skill_path: value.skill_path,
      skill_folder_hash: value.skill_folder_hash,
      installed_at: chrono::Utc::now().to_rfc3339(),
      updated_at: chrono::Utc::now().to_rfc3339(),
    }
  }
}

#[tauri::command]
pub fn read_skill_lock() -> Result<SkillLockFile, String> {
  crate::services::skill_lock_service::read_skill_lock_internal()
}

#[tauri::command]
pub fn write_skill_lock(lock: SkillLockFile) -> Result<(), String> {
  crate::services::skill_lock_service::write_skill_lock_internal(&lock)
}

#[tauri::command]
pub fn add_skill_to_lock(
  skill_name: String,
  entry: SkillLockEntryWithoutTimestamps,
) -> Result<(), String> {
  crate::services::skill_lock_service::add_skill_to_lock(skill_name, entry.into())
}

#[tauri::command]
pub fn remove_skill_from_lock(skill_name: String) -> Result<bool, String> {
  crate::services::skill_lock_service::remove_skill_from_lock(skill_name)
}

#[tauri::command]
pub fn get_skill_from_lock(skill_name: String) -> Result<Option<SkillLockEntry>, String> {
  crate::services::skill_lock_service::get_skill_from_lock(skill_name)
}

#[tauri::command]
pub fn get_all_locked_skills() -> Result<std::collections::HashMap<String, SkillLockEntry>, String>
{
  crate::services::skill_lock_service::get_all_locked_skills()
}
