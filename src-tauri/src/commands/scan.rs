use crate::config::{load_config, normalize_path, save_config};
use crate::models::LocalSkill;
use crate::paths::agent_paths;
use serde_yaml::Value;
use std::collections::HashMap;
use std::env;
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
pub async fn scan_local_skills() -> Result<Vec<LocalSkill>, String> {
  tauri::async_runtime::spawn_blocking(|| {
    let mut skills: HashMap<String, LocalSkill> = HashMap::new();
    let config = load_config().unwrap_or_default();

    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    let project_canonical = cwd.join(".agents").join("skills");
    let global_canonical = expand_home("~/.agents/skills").ok_or("无法获取用户目录")?;

    collect_canonical_skills(&project_canonical, "project", &mut skills);
    collect_canonical_skills(&global_canonical, "global", &mut skills);

    for agent in agent_paths() {
      if let Some(project) = agent.project_path {
        let dir = cwd.join(project);
        collect_agent_skills(&dir, "project", agent.id, &mut skills);
      }
      if let Some(global) = agent.global_path {
        if let Some(dir) = expand_home(global) {
          collect_agent_skills(&dir, "global", agent.id, &mut skills);
        }
      }
    }

    for root in config.scan_roots.iter() {
      let path = PathBuf::from(root);
      collect_custom_skills(&path, &mut skills);
    }

    Ok(skills.into_values().collect())
  })
  .await
  .map_err(|e| e.to_string())?
}

fn collect_canonical_skills(base_dir: &Path, scope: &str, map: &mut HashMap<String, LocalSkill>) {
  if !base_dir.exists() {
    return;
  }
  for entry in fs::read_dir(base_dir).into_iter().flatten() {
    if let Ok(entry) = entry {
      let path = entry.path();
      if !is_skill_dir(&path) {
        continue;
      }
      if let Some(skill) = parse_skill_dir(&path, scope) {
        let key = format!("{}::{}", scope, skill.name);
        map.entry(key).or_insert(skill);
      }
    }
  }
}

fn collect_agent_skills(
  base_dir: &Path,
  scope: &str,
  agent_id: &str,
  map: &mut HashMap<String, LocalSkill>,
) {
  if !base_dir.exists() {
    return;
  }
  for entry in fs::read_dir(base_dir).into_iter().flatten() {
    if let Ok(entry) = entry {
      let path = entry.path();
      if !is_skill_dir(&path) {
        continue;
      }

      let mut skill = match parse_skill_dir(&path, scope) {
        Some(skill) => skill,
        None => continue,
      };

      if let Ok(real) = fs::canonicalize(&path) {
        skill.canonical_path = real.to_string_lossy().to_string();
      }

      let key = format!("{}::{}", scope, skill.name);
      let entry = map.entry(key).or_insert(skill);
      if !entry.agents.contains(&agent_id.to_string()) {
        entry.agents.push(agent_id.to_string());
      }
    }
  }
}

fn collect_custom_skills(base_dir: &Path, map: &mut HashMap<String, LocalSkill>) {
  if !base_dir.exists() {
    return;
  }

  let walker = WalkDir::new(base_dir)
    .follow_links(false)
    .max_depth(5)
    .into_iter()
    .filter_entry(|entry| !is_ignored(entry.path()));

  for entry in walker.filter_map(Result::ok) {
    if entry.file_type().is_dir() && is_symlink_dir(entry.path()) {
      let candidate = entry.path().join("SKILL.md");
      if candidate.exists() {
        if let Some(skill) = parse_skill_dir(entry.path(), "custom") {
          let key = format!("custom::{}", skill.name);
          map.entry(key).or_insert(skill);
        }
      }
      continue;
    }

    if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
      if let Some(dir) = entry.path().parent() {
        if let Some(skill) = parse_skill_dir(dir, "custom") {
          let key = format!("custom::{}", skill.name);
          map.entry(key).or_insert(skill);
        }
      }
    }
  }
}

fn parse_skill_dir(skill_dir: &Path, scope: &str) -> Option<LocalSkill> {
  let skill_file = skill_dir.join("SKILL.md");
  let content = fs::read_to_string(&skill_file).ok()?;
  let (name, description) = parse_frontmatter(&content);

  let folder_name = skill_dir
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  let canonical = fs::canonicalize(skill_dir)
    .ok()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| skill_dir.to_string_lossy().to_string());

  Some(LocalSkill {
    name: name.unwrap_or(folder_name),
    description,
    scope: scope.to_string(),
    canonical_path: canonical,
    agents: Vec::new(),
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

fn is_skill_dir(path: &Path) -> bool {
  if is_symlink_dir(path) {
    return path.join("SKILL.md").exists();
  }
  path.is_dir() && path.join("SKILL.md").exists()
}

fn is_symlink_dir(path: &Path) -> bool {
  match fs::symlink_metadata(path) {
    Ok(meta) => meta.file_type().is_symlink() && path.is_dir(),
    Err(_) => false,
  }
}

fn expand_home(path: &str) -> Option<PathBuf> {
  if path.starts_with("~/") {
    let home = dirs_next::home_dir()?;
    return Some(home.join(path.trim_start_matches("~/")));
  }
  Some(PathBuf::from(path))
}
