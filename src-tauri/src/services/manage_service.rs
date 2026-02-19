use crate::config::load_config;
use crate::services::agent_apps_service::local_agent_apps;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COPY_MARKER_FILE: &str = ".you-skills-link";

fn is_symlink(path: &Path) -> bool {
  fs::symlink_metadata(path)
    .map(|m| m.file_type().is_symlink())
    .unwrap_or(false)
}

pub fn set_agent_link(
  name: String,
  agent: String,
  scope: String,
  linked: bool,
) -> Result<(), String> {
  let canonical_path = canonical_skill_path(&name, &scope)?;
  let agent_dir = agent_skills_dir(&agent, &scope)?;
  let link_path = agent_dir.join(sanitize_name(&name));
  let sync_mode = current_sync_mode();

  if linked {
    link_to_canonical(&canonical_path, &link_path, &sync_mode)?;
    return Ok(());
  }

  if !link_path.exists() {
    return Ok(());
  }
  let meta = fs::symlink_metadata(&link_path).map_err(|e| e.to_string())?;
  if meta.file_type().is_symlink() {
    return remove_symlink_if_points_to(&link_path, &canonical_path);
  }
  if sync_mode == "copy" || copy_marker_points_to(&link_path, &canonical_path) {
    return remove_path(&link_path);
  }
  Err("当前路径不是软链接，已取消操作".to_string())
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

fn current_sync_mode() -> String {
  load_config()
    .map(|config| config.sync_mode)
    .unwrap_or_else(|_| "symlink".to_string())
}

fn link_to_canonical(target: &Path, link_path: &Path, sync_mode: &str) -> Result<(), String> {
  if sync_mode == "copy" {
    if link_path.exists() {
      remove_path(link_path)?;
    }
    copy_dir_or_file(target, link_path)?;
    write_copy_marker(link_path, target)?;
    return Ok(());
  }
  ensure_symlink_dir(target, link_path)
}

fn write_copy_marker(link_path: &Path, canonical_path: &Path) -> Result<(), String> {
  let marker = link_path.join(COPY_MARKER_FILE);
  if let Some(parent) = marker.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  fs::write(marker, canonical_path.to_string_lossy().to_string()).map_err(|e| e.to_string())
}

fn copy_marker_points_to(link_path: &Path, canonical_path: &Path) -> bool {
  let marker = link_path.join(COPY_MARKER_FILE);
  let content = fs::read_to_string(marker).ok();
  match content {
    Some(text) => PathBuf::from(text.trim()) == canonical_path,
    None => false,
  }
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

fn remove_symlink_if_points_to(link_path: &Path, target: &Path) -> Result<(), String> {
  if !link_path.exists() && !is_symlink(link_path) {
    return Ok(());
  }
  let meta = fs::symlink_metadata(link_path).map_err(|e| e.to_string())?;
  if !meta.file_type().is_symlink() {
    return Err("当前路径不是软链接，已取消操作".to_string());
  }
  let current_target = fs::read_link(link_path).map_err(|e| e.to_string())?;
  let resolved = if current_target.is_absolute() {
    current_target
  } else {
    link_path
      .parent()
      .unwrap_or_else(|| Path::new("/"))
      .join(current_target)
  };
  // Canonicalize both paths to handle symlink differences (e.g. /Users vs /private/Users on macOS)
  let resolved_canonical = fs::canonicalize(&resolved).unwrap_or(resolved);
  let target_canonical = fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
  if resolved_canonical != target_canonical {
    return Err(format!(
      "软链接指向不同路径，已取消操作 (link -> {}, expected {})",
      resolved_canonical.display(),
      target_canonical.display()
    ));
  }
  fs::remove_file(link_path).map_err(|e| e.to_string())
}

fn agent_skills_dir(agent_id: &str, scope: &str) -> Result<PathBuf, String> {
  let app = local_agent_apps()
    .into_iter()
    .find(|a| a.id == agent_id)
    .ok_or("未知的应用类型")?;

  let (global_path, project_path) = (app.global_path, app.project_path);

  let path = if scope == "global" {
    global_path.ok_or("该应用不支持全局安装")?
  } else {
    project_path.ok_or("该应用不支持项目安装")?
  };

  let resolved = if path.starts_with("~/") {
    let home = dirs_next::home_dir().ok_or("无法获取用户目录")?;
    home.join(path.trim_start_matches("~/"))
  } else {
    env::current_dir().map_err(|e| e.to_string())?.join(path)
  };

  fs::create_dir_all(&resolved).map_err(|e| e.to_string())?;
  Ok(resolved)
}

#[cfg(target_os = "windows")]
fn create_symlink_dir(target: &Path, link_path: &Path) -> Result<(), String> {
  std::os::windows::fs::symlink_dir(target, link_path).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "windows"))]
fn create_symlink_dir(target: &Path, link_path: &Path) -> Result<(), String> {
  std::os::unix::fs::symlink(target, link_path).map_err(|e| e.to_string())
}
