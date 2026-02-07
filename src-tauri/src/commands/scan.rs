use crate::config::{load_config, normalize_path, save_config};
use crate::models::LocalSkill;
use crate::paths::agent_paths;
use serde_yaml::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const COPY_MARKER_FILE: &str = ".you-skills-link";

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
    let mut managed_map: HashMap<String, LocalSkill> = HashMap::new();
    let mut unmanaged_list: Vec<LocalSkill> = Vec::new();
    let mut managed_names: HashSet<String> = HashSet::new();
    let config = load_config().unwrap_or_default();

    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    let project_canonical = cwd.join(".agents").join("skills");
    let global_canonical = expand_home("~/.agents/skills").ok_or("无法获取用户目录")?;

    collect_canonical_skills(
      &project_canonical,
      "project",
      &mut managed_map,
      &mut managed_names,
    );
    collect_canonical_skills(
      &global_canonical,
      "global",
      &mut managed_map,
      &mut managed_names,
    );

    for agent in agent_paths() {
      if let Some(project) = agent.project_path {
        let dir = cwd.join(project);
        collect_agent_skills(
          &dir,
          "project",
          agent.id,
          &project_canonical,
          &global_canonical,
          &mut managed_map,
          &mut managed_names,
          &mut unmanaged_list,
        );
      }
      if let Some(global) = agent.global_path {
        if let Some(dir) = expand_home(global) {
          collect_agent_skills(
            &dir,
            "global",
            agent.id,
            &project_canonical,
            &global_canonical,
            &mut managed_map,
            &mut managed_names,
            &mut unmanaged_list,
          );
        }
      }
    }

    for root in config.scan_roots.iter() {
      let path = PathBuf::from(root);
      collect_custom_skills(
        &path,
        &project_canonical,
        &global_canonical,
        &mut managed_names,
        &mut unmanaged_list,
      );
    }

    let mut all_skills: Vec<LocalSkill> = managed_map.into_values().collect();
    apply_name_conflicts(&mut unmanaged_list);
    all_skills.extend(unmanaged_list);
    Ok(all_skills)
  })
  .await
  .map_err(|e| e.to_string())?
}

fn collect_canonical_skills(
  base_dir: &Path,
  scope: &str,
  map: &mut HashMap<String, LocalSkill>,
  names: &mut HashSet<String>,
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
      if let Some(mut skill) = parse_canonical_skill_dir(&path, scope) {
        skill.managed_status = "managed".to_string();
        let name = skill.name.clone();
        let key = format!("{}::{}", scope, name);
        map.entry(key).or_insert(skill);
        names.insert(name);
      }
    }
  }
}

fn parse_canonical_skill_dir(skill_dir: &Path, scope: &str) -> Option<LocalSkill> {
  let skill_file = skill_dir.join("SKILL.md");
  let content = fs::read_to_string(&skill_file).ok()?;
  let (name, description) = parse_frontmatter(&content);

  let folder_name = skill_dir
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  let canonical_path = fs::canonicalize(skill_dir)
    .ok()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| skill_dir.to_string_lossy().to_string());

  Some(LocalSkill {
    name: name.unwrap_or(folder_name),
    description,
    scope: scope.to_string(),
    canonical_path,
    agents: Vec::new(),
    managed_status: "managed".to_string(),
    name_conflict: false,
    created_at: get_created_at(skill_dir),
    conflict_with_managed: false,
  })
}

fn collect_agent_skills(
  base_dir: &Path,
  scope: &str,
  agent_id: &str,
  project_canonical: &Path,
  global_canonical: &Path,
  managed_map: &mut HashMap<String, LocalSkill>,
  managed_names: &mut HashSet<String>,
  unmanaged_list: &mut Vec<LocalSkill>,
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

      let (mut skill, is_managed_link) =
        match parse_skill_dir(&path, scope, project_canonical, global_canonical) {
          Some(skill) => skill,
          None => continue,
        };

      if is_managed_link {
        let key = format!("{}::{}", scope, skill.name);
        let entry = managed_map.entry(key).or_insert_with(|| {
          skill.managed_status = "managed".to_string();
          skill
        });
        if !entry.agents.contains(&agent_id.to_string()) {
          entry.agents.push(agent_id.to_string());
        }
        managed_names.insert(entry.name.clone());
      } else {
        let has_managed = managed_names.contains(&skill.name);
        skill.managed_status = if has_managed {
          "mixed".to_string()
        } else {
          "unmanaged".to_string()
        };
        skill.conflict_with_managed = has_managed;
        skill.agents.push(agent_id.to_string());
        unmanaged_list.push(skill);
      }
    }
  }
}

fn collect_custom_skills(
  base_dir: &Path,
  project_canonical: &Path,
  global_canonical: &Path,
  managed_names: &mut HashSet<String>,
  unmanaged_list: &mut Vec<LocalSkill>,
) {
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
        if let Some((mut skill, _)) =
          parse_skill_dir(entry.path(), "custom", project_canonical, global_canonical)
        {
          let has_managed = managed_names.contains(&skill.name);
          skill.managed_status = if has_managed {
            "mixed".to_string()
          } else {
            "unmanaged".to_string()
          };
          skill.conflict_with_managed = has_managed;
          unmanaged_list.push(skill);
        }
      }
      continue;
    }

    if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
      if let Some(dir) = entry.path().parent() {
        if let Some((mut skill, _)) =
          parse_skill_dir(dir, "custom", project_canonical, global_canonical)
        {
          let has_managed = managed_names.contains(&skill.name);
          skill.managed_status = if has_managed {
            "mixed".to_string()
          } else {
            "unmanaged".to_string()
          };
          skill.conflict_with_managed = has_managed;
          unmanaged_list.push(skill);
        }
      }
    }
  }
}

fn parse_skill_dir(
  skill_dir: &Path,
  scope: &str,
  project_canonical: &Path,
  global_canonical: &Path,
) -> Option<(LocalSkill, bool)> {
  let skill_file = skill_dir.join("SKILL.md");
  let content = fs::read_to_string(&skill_file).ok()?;
  let (name, description) = parse_frontmatter(&content);

  let folder_name = skill_dir
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  let copy_target = read_copy_marker(skill_dir);
  let canonical = copy_target
    .as_ref()
    .map(|p| p.to_path_buf())
    .or_else(|| fs::canonicalize(skill_dir).ok());
  let canonical_path = canonical
    .as_ref()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| skill_dir.to_string_lossy().to_string());

  let is_link = is_symlink_dir(skill_dir);
  let is_link_target = canonical
    .as_ref()
    .map(|p| is_under_canonical(p, project_canonical, global_canonical))
    .unwrap_or(false);
  let is_copy_link = copy_target
    .as_ref()
    .map(|p| is_under_canonical(p, project_canonical, global_canonical))
    .unwrap_or(false);
  let is_managed_link = (is_link && is_link_target) || is_copy_link;

  Some((
    LocalSkill {
      name: name.unwrap_or(folder_name),
      description,
      scope: scope.to_string(),
      canonical_path,
      agents: Vec::new(),
      managed_status: "unknown".to_string(),
      name_conflict: false,
      created_at: get_created_at(skill_dir),
      conflict_with_managed: false,
    },
    is_managed_link,
  ))
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

fn is_under_canonical(path: &Path, project_canonical: &Path, global_canonical: &Path) -> bool {
  let path_str = path.to_string_lossy();
  let project_str = project_canonical.to_string_lossy();
  let global_str = global_canonical.to_string_lossy();
  path_str.starts_with(project_str.as_ref()) || path_str.starts_with(global_str.as_ref())
}

fn read_copy_marker(skill_dir: &Path) -> Option<PathBuf> {
  let marker_path = skill_dir.join(COPY_MARKER_FILE);
  let content = fs::read_to_string(marker_path).ok()?;
  let trimmed = content.trim();
  if trimmed.is_empty() {
    return None;
  }
  Some(PathBuf::from(trimmed))
}

fn apply_name_conflicts(list: &mut Vec<LocalSkill>) {
  let mut counts: HashMap<String, usize> = HashMap::new();
  for skill in list.iter() {
    *counts.entry(skill.name.clone()).or_insert(0) += 1;
  }
  for skill in list.iter_mut() {
    if let Some(count) = counts.get(&skill.name) {
      if *count > 1 {
        skill.name_conflict = true;
      }
    }
  }
}

fn get_created_at(path: &Path) -> Option<i64> {
  let metadata = fs::metadata(path).ok()?;
  let time = metadata.created().or_else(|_| metadata.modified()).ok()?;
  to_millis(time)
}

fn to_millis(time: SystemTime) -> Option<i64> {
  time.duration_since(UNIX_EPOCH)
    .ok()
    .map(|duration| duration.as_millis() as i64)
}

fn expand_home(path: &str) -> Option<PathBuf> {
  if path.starts_with("~/") {
    let home = dirs_next::home_dir()?;
    return Some(home.join(path.trim_start_matches("~/")));
  }
  Some(PathBuf::from(path))
}
