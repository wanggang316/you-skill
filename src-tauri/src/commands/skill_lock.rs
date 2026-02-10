use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CURRENT_VERSION: u32 = 3;
const AGENTS_DIR: &str = ".agents";
const LOCK_FILE: &str = ".skill-lock.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillLockEntry {
  /// 标准化的源标识符（例如 "owner/repo"）
  pub source: String,
  /// 提供者/源类型（例如 "github", "mintlify", "huggingface", "local"）
  pub source_type: String,
  /// 用于安装技能的原始 URL
  pub source_url: String,
  /// 源仓库内的子路径（如果适用）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub skill_path: Option<String>,
  /// GitHub tree SHA for the entire skill folder
  #[serde(skip_serializing_if = "Option::is_none")]
  pub skill_folder_hash: Option<String>,
  /// 首次安装技能时的 ISO 时间戳
  pub installed_at: String,
  /// 最后更新技能时的 ISO 时间戳
  pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillLockFile {
  /// 架构版本
  pub version: u32,
  /// 技能名称到其锁条目的映射
  #[serde(default)]
  pub skills: HashMap<String, SkillLockEntry>,
}

/// 获取技能锁文件路径
fn skill_lock_path() -> Result<PathBuf, String> {
  let home_dir = dirs_next::home_dir().ok_or("无法获取用户主目录")?;
  Ok(home_dir.join(AGENTS_DIR).join(LOCK_FILE))
}

/// 创建空的锁文件结构
fn create_empty_lock_file() -> SkillLockFile {
  SkillLockFile {
    version: CURRENT_VERSION,
    skills: HashMap::new(),
  }
}

/// 读取技能锁文件
fn read_skill_lock_internal() -> Result<SkillLockFile, String> {
  let lock_path = skill_lock_path()?;

  if !lock_path.exists() {
    return Ok(create_empty_lock_file());
  }

  let content = fs::read_to_string(&lock_path).map_err(|e| format!("读取锁文件失败: {}", e))?;
  let parsed: SkillLockFile =
    serde_json::from_str(&content).map_err(|e| format!("解析锁文件失败: {}", e))?;

  // 验证版本 - 如果是旧版本则返回空锁文件
  if parsed.version < CURRENT_VERSION {
    return Ok(create_empty_lock_file());
  }

  Ok(parsed)
}

/// 写入技能锁文件
fn write_skill_lock_internal(lock: &SkillLockFile) -> Result<(), String> {
  let lock_path = skill_lock_path()?;

  // 确保目录存在
  if let Some(parent) = lock_path.parent() {
    fs::create_dir_all(parent).map_err(|e| format!("创建锁文件目录失败: {}", e))?;
  }

  // 写入格式化的 JSON
  let content =
    serde_json::to_string_pretty(lock).map_err(|e| format!("序列化锁文件失败: {}", e))?;
  fs::write(&lock_path, content).map_err(|e| format!("写入锁文件失败: {}", e))?;

  Ok(())
}

#[tauri::command]
pub fn read_skill_lock() -> Result<SkillLockFile, String> {
  read_skill_lock_internal()
}

#[tauri::command]
pub fn write_skill_lock(lock: SkillLockFile) -> Result<(), String> {
  write_skill_lock_internal(&lock)
}

#[tauri::command]
pub fn add_skill_to_lock(
  skill_name: String,
  entry: SkillLockEntryWithoutTimestamps,
) -> Result<(), String> {
  let mut lock = read_skill_lock_internal()?;
  let now = chrono::Utc::now().to_rfc3339();

  let existing_entry = lock.skills.get(&skill_name);

  lock.skills.insert(
    skill_name,
    SkillLockEntry {
      source: entry.source,
      source_type: entry.source_type,
      source_url: entry.source_url,
      skill_path: entry.skill_path,
      skill_folder_hash: entry.skill_folder_hash,
      installed_at: existing_entry
        .map(|e| e.installed_at.clone())
        .unwrap_or_else(|| now.clone()),
      updated_at: now,
    },
  );

  write_skill_lock_internal(&lock)
}

#[tauri::command]
pub fn remove_skill_from_lock(skill_name: String) -> Result<bool, String> {
  let mut lock = read_skill_lock_internal()?;

  if !lock.skills.contains_key(&skill_name) {
    return Ok(false);
  }

  lock.skills.remove(&skill_name);
  write_skill_lock_internal(&lock)?;
  Ok(true)
}

#[tauri::command]
pub fn get_skill_from_lock(skill_name: String) -> Result<Option<SkillLockEntry>, String> {
  let lock = read_skill_lock_internal()?;
  Ok(lock.skills.get(&skill_name).cloned())
}

#[tauri::command]
pub fn get_all_locked_skills() -> Result<HashMap<String, SkillLockEntry>, String> {
  let lock = read_skill_lock_internal()?;
  Ok(lock.skills)
}

/// 用于接收没有时间戳的条目
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillLockEntryWithoutTimestamps {
  pub source: String,
  pub source_type: String,
  pub source_url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub skill_path: Option<String>,
  /// GitHub tree SHA for the entire skill folder
  #[serde(skip_serializing_if = "Option::is_none")]
  pub skill_folder_hash: Option<String>,
}
