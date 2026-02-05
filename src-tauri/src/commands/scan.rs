use crate::config::{load_config, normalize_path, save_config};
use crate::models::LocalSkill;
use crate::paths::agent_paths;
use serde_yaml::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[tauri::command]
pub fn get_scan_roots() -> Result<Vec<String>, String> {
  let config = load_config()?;
  Ok(config.scan_roots)
}

#[tauri::command]
pub fn add_scan_root(path: String) -> Result<Vec<String>, String> {
  let mut config = load_config()?;
  let normalized = normalize_path(&path);
  if !config.scan_roots.contains(&normalized) {
    config.scan_roots.push(normalized);
  }
  save_config(&config)?;
  Ok(config.scan_roots)
}

#[tauri::command]
pub fn remove_scan_root(path: String) -> Result<Vec<String>, String> {
  let mut config = load_config()?;
  let normalized = normalize_path(&path);
  config.scan_roots.retain(|root| root != &normalized);
  save_config(&config)?;
  Ok(config.scan_roots)
}

#[tauri::command]
pub fn scan_local_skills() -> Result<Vec<LocalSkill>, String> {
  let mut skills: Vec<LocalSkill> = Vec::new();
  let mut seen: HashSet<String> = HashSet::new();

  let config = load_config().unwrap_or_default();
  let mut roots: Vec<PathBuf> = Vec::new();

  for agent in agent_paths() {
    if let Some(global) = agent.global_path {
      if let Some(path) = expand_home(global) {
        roots.push(path);
      }
    }
  }

  for root in config.scan_roots.iter() {
    roots.push(PathBuf::from(root));
  }

  for root in roots {
    if !root.exists() {
      continue;
    }
    scan_root(&root, &mut skills, &mut seen);
  }

  Ok(skills)
}

fn scan_root(root: &Path, skills: &mut Vec<LocalSkill>, seen: &mut HashSet<String>) {
  let walker = WalkDir::new(root)
    .follow_links(true)
    .max_depth(6)
    .into_iter()
    .filter_entry(|entry| !is_ignored(entry.path()));

  for entry in walker.filter_map(Result::ok) {
    if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
      if let Some(skill) = parse_skill(entry.path()) {
        if seen.insert(skill.path.clone()) {
          skills.push(skill);
        }
      }
    }
  }
}

fn parse_skill(skill_file: &Path) -> Option<LocalSkill> {
  let content = fs::read_to_string(skill_file).ok()?;
  let (name, description) = parse_frontmatter(&content);
  let skill_dir = skill_file.parent()?;

  let folder_name = skill_dir
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  let install_mode = match fs::symlink_metadata(skill_dir) {
    Ok(meta) if meta.file_type().is_symlink() => "symlink",
    Ok(_) => "copy",
    Err(_) => "unknown",
  };

  let real_path = fs::canonicalize(skill_dir).ok().map(|p| p.to_string_lossy().to_string());
  let agent = detect_agent(skill_dir).unwrap_or_else(|| "unknown".to_string());

  Some(LocalSkill {
    name: name.unwrap_or(folder_name),
    description,
    path: skill_dir.to_string_lossy().to_string(),
    real_path,
    agent,
    install_mode: install_mode.to_string(),
  })
}

fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>) {
  let mut lines = content.lines();
  if lines.next().map(|s| s.trim()) != Some("---") {
    return (None, None);
  }

  let mut frontmatter = String::new();
  for line in lines.by_ref() {
    if line.trim() == "---" {
      break;
    }
    frontmatter.push_str(line);
    frontmatter.push('\n');
  }

  let yaml: Value = match serde_yaml::from_str(&frontmatter) {
    Ok(val) => val,
    Err(_) => return (None, None),
  };

  let name = yaml
    .get("name")
    .and_then(|v| v.as_str())
    .map(|v| v.to_string());
  let description = yaml
    .get("description")
    .and_then(|v| v.as_str())
    .map(|v| v.to_string());

  (name, description)
}

fn is_ignored(path: &Path) -> bool {
  let path_str = path.to_string_lossy();
  path_str.contains("/node_modules/")
    || path_str.contains("/.git/")
    || path_str.contains("/target/")
    || path_str.contains("/dist/")
}

fn expand_home(path: &str) -> Option<PathBuf> {
  if path.starts_with("~/") {
    let home = dirs_next::home_dir()?;
    return Some(home.join(path.trim_start_matches("~/")));
  }
  Some(PathBuf::from(path))
}

fn detect_agent(skill_dir: &Path) -> Option<String> {
  let normalized = skill_dir.to_string_lossy().replace('\\', "/").to_lowercase();
  for agent in agent_paths() {
    if let Some(global) = agent.global_path {
      let needle = global.replace('\\', "/").replace('~', "").to_lowercase();
      if normalized.contains(&needle) {
        return Some(agent.id.to_string());
      }
    }
    if let Some(project) = agent.project_path {
      let needle = format!("/{}", project.replace('\\', "/").to_lowercase());
      if normalized.contains(&needle) {
        return Some(agent.id.to_string());
      }
    }
  }
  None
}
