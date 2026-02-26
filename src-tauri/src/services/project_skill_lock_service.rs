use crate::models::SourceType;
use crate::utils::folder::FolderHelper;
use std::fs;
use std::path::Path;

const PROJECT_LOCK_FILE: &str = "skills-lock.json";
const CURRENT_VERSION: i64 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSkillLockEntry {
  pub source: String,
  pub source_type: String,
  pub computed_hash: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectSkillLockFile {
  #[serde(default = "default_version")]
  pub version: i64,
  #[serde(default)]
  pub skills: std::collections::BTreeMap<String, ProjectSkillLockEntry>,
}

impl Default for ProjectSkillLockFile {
  fn default() -> Self {
    Self {
      version: CURRENT_VERSION,
      skills: std::collections::BTreeMap::new(),
    }
  }
}

fn default_version() -> i64 {
  CURRENT_VERSION
}

fn project_lock_path(project_root: &Path) -> std::path::PathBuf {
  project_root.join(PROJECT_LOCK_FILE)
}

pub fn read_project_skill_lock_internal(project_root: &Path) -> Result<ProjectSkillLockFile, String> {
  let lock_path = project_lock_path(project_root);
  if !lock_path.exists() {
    return Ok(ProjectSkillLockFile::default());
  }

  let content = fs::read_to_string(&lock_path).unwrap_or_default();
  let parsed = serde_json::from_str(&content).unwrap_or_default();
  Ok(parsed)
}

fn write_project_skill_lock_internal(
  project_root: &Path,
  lock: &ProjectSkillLockFile,
) -> Result<(), String> {
  let lock_path = project_lock_path(project_root);
  if let Some(parent) = lock_path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("Failed to create lock file directory: {}", e))?;
  }
  let content = serde_json::to_string_pretty(lock)
    .map_err(|e| format!("Failed to serialize lock file: {}", e))?;
  fs::write(lock_path, content).map_err(|e| format!("Failed to write lock file: {}", e))?;
  Ok(())
}

pub fn add_skill_to_project_lock(
  project_root: &Path,
  skill_name: String,
  source: String,
  source_type: SourceType,
  skill_dir: &Path,
) -> Result<(), String> {
  let mut lock = read_project_skill_lock_internal(project_root)?;
  lock.version = CURRENT_VERSION;
  let computed_hash = FolderHelper::compute_skill_folder_hash(skill_dir)?;

  lock.skills.insert(
    skill_name,
    ProjectSkillLockEntry {
      source,
      source_type: source_type_to_lock_value(&source_type).to_string(),
      computed_hash,
    },
  );
  write_project_skill_lock_internal(project_root, &lock)
}

pub fn remove_skill_from_project_lock(project_root: &Path, skill_name: String) -> Result<bool, String> {
  let mut lock = read_project_skill_lock_internal(project_root)?;
  if !lock.skills.contains_key(&skill_name) {
    return Ok(false);
  }
  lock.skills.remove(&skill_name);
  write_project_skill_lock_internal(project_root, &lock)?;
  Ok(true)
}

pub fn project_lock_source_type(
  skill_name: &str,
  project_lock: &ProjectSkillLockFile,
) -> SourceType {
  let Some(entry) = project_lock.skills.get(skill_name) else {
    return SourceType::Unknown;
  };
  match entry.source_type.as_str() {
    "github" => SourceType::Github,
    "native" => SourceType::Native,
    _ => SourceType::Unknown,
  }
}

pub fn project_lock_source(skill_name: &str, project_lock: &ProjectSkillLockFile) -> Option<String> {
  let Some(entry) = project_lock.skills.get(skill_name) else {
    return None;
  };
  if entry.source_type != "github" {
    return None;
  }
  let source = entry.source.trim();
  if source.is_empty() {
    return None;
  }
  Some(source.to_string())
}

fn source_type_to_lock_value(source_type: &SourceType) -> &'static str {
  match source_type {
    SourceType::Github => "github",
    SourceType::Native => "native",
    SourceType::Unknown => "unknown",
  }
}
