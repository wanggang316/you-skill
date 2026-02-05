use crate::models::{CanonicalCheckResult, UnifyRequest, UnifyResult};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn delete_skill(path: String) -> Result<(), String> {
  let target = PathBuf::from(path);
  if target.is_dir() {
    fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
  } else if target.is_file() {
    fs::remove_file(&target).map_err(|e| e.to_string())?;
  }
  Ok(())
}

#[tauri::command]
pub fn move_skill(from: String, to: String) -> Result<(), String> {
  let from_path = PathBuf::from(from);
  let to_path = PathBuf::from(to);

  match fs::rename(&from_path, &to_path) {
    Ok(_) => Ok(()),
    Err(_) => {
      copy_dir_or_file(&from_path, &to_path)?;
      delete_skill(from_path.to_string_lossy().to_string())
    }
  }
}

#[tauri::command]
pub fn copy_skill(from: String, to: String) -> Result<(), String> {
  let from_path = PathBuf::from(from);
  let to_path = PathBuf::from(to);
  copy_dir_or_file(&from_path, &to_path)
}

#[tauri::command]
pub fn check_canonical_skill(name: String, scope: String) -> Result<CanonicalCheckResult, String> {
  let canonical_path = canonical_skill_path(&name, &scope)?;
  Ok(CanonicalCheckResult {
    exists: canonical_path.exists(),
    canonical_path: canonical_path.to_string_lossy().to_string(),
  })
}

#[tauri::command]
pub fn unify_skill(request: UnifyRequest) -> Result<UnifyResult, String> {
  let canonical_path = canonical_skill_path(&request.name, &request.scope)?;
  let current_path = PathBuf::from(&request.current_path);

  if !current_path.exists() {
    return Ok(UnifyResult {
      success: false,
      message: "当前 skill 路径不存在".to_string(),
    });
  }

  if request.prefer == "canonical" {
    if !canonical_path.exists() {
      copy_dir_or_file(&current_path, &canonical_path)?;
    }
    remove_path(&current_path)?;
    ensure_symlink_dir(&canonical_path, &current_path)?;
    return Ok(UnifyResult {
      success: true,
      message: "已保留 .agents 版本并建立软链接".to_string(),
    });
  }

  if canonical_path.exists() {
    remove_path(&canonical_path)?;
  }
  copy_dir_or_file(&current_path, &canonical_path)?;
  remove_path(&current_path)?;
  ensure_symlink_dir(&canonical_path, &current_path)?;

  Ok(UnifyResult {
    success: true,
    message: "已用当前版本覆盖 .agents 并建立软链接".to_string(),
  })
}

fn copy_dir_or_file(from: &Path, to: &Path) -> Result<(), String> {
  if from.is_dir() {
    copy_dir_recursive(from, to)
  } else {
    if let Some(parent) = to.parent() {
      fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(from, to).map_err(|e| e.to_string())?;
    Ok(())
  }
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
  if !to.exists() {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
  }
  for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let entry_path = entry.path();
    let target_path = to.join(entry.file_name());
    if entry_path.is_dir() {
      copy_dir_recursive(&entry_path, &target_path)?;
    } else {
      fs::copy(&entry_path, &target_path).map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn canonical_skill_path(name: &str, scope: &str) -> Result<PathBuf, String> {
  let sanitized = sanitize_name(name);
  let base = if scope == "global" {
    let home = dirs_next::home_dir().ok_or("无法获取用户目录")?;
    home.join(".agents").join("skills")
  } else {
    env::current_dir()
      .map_err(|e| e.to_string())?
      .join(".agents")
      .join("skills")
  };
  fs::create_dir_all(&base).map_err(|e| e.to_string())?;
  Ok(base.join(sanitized))
}

fn sanitize_name(name: &str) -> String {
  let mut out = String::new();
  for ch in name.chars() {
    if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
      out.push(ch.to_ascii_lowercase());
    } else {
      out.push('-');
    }
  }
  let trimmed = out.trim_matches(|c| c == '-' || c == '.');
  if trimmed.is_empty() {
    "unnamed-skill".to_string()
  } else {
    trimmed.chars().take(255).collect()
  }
}

fn remove_path(path: &Path) -> Result<(), String> {
  if !path.exists() {
    return Ok(());
  }
  let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
  if meta.file_type().is_symlink() || path.is_file() {
    fs::remove_file(path).map_err(|e| e.to_string())?;
  } else {
    fs::remove_dir_all(path).map_err(|e| e.to_string())?;
  }
  Ok(())
}

fn ensure_symlink_dir(target: &Path, link_path: &Path) -> Result<(), String> {
  if link_path.exists() {
    if let Ok(current_target) = fs::read_link(link_path) {
      let resolved = if current_target.is_absolute() {
        current_target
      } else {
        link_path
          .parent()
          .unwrap_or_else(|| Path::new("/"))
          .join(current_target)
      };
      if resolved == target {
        return Ok(());
      }
    }
    remove_path(link_path)?;
  } else if let Some(parent) = link_path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }

  create_symlink_dir(target, link_path)
}

#[cfg(target_os = "windows")]
fn create_symlink_dir(target: &Path, link_path: &Path) -> Result<(), String> {
  std::os::windows::fs::symlink_dir(target, link_path).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "windows"))]
fn create_symlink_dir(target: &Path, link_path: &Path) -> Result<(), String> {
  std::os::unix::fs::symlink(target, link_path).map_err(|e| e.to_string())
}
