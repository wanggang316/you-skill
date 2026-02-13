use crate::config::load_config;
use crate::models::LocalSkill;
use crate::services::agent_apps_service::{expand_tilde, local_agent_apps};
use crate::utils::file::FileHelper;
use crate::utils::folder::FolderHelper;
use crate::utils::path::{expand_home, is_under_canonical};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan_local_skills() -> Result<Vec<LocalSkill>, String> {
  let mut managed_map: HashMap<String, LocalSkill> = HashMap::new();
  let mut unmanaged_list: Vec<LocalSkill> = Vec::new();
  let mut managed_names: HashSet<String> = HashSet::new();
  let config = load_config().unwrap_or_default();

  let global_canonical = expand_home("~/.agents/skills").ok_or("无法获取用户目录")?;

  collect_canonical_skills(&global_canonical, "global", &mut managed_map, &mut managed_names);

  // Scan all locally installed agent apps (built-in + custom)
  for app in local_agent_apps() {
    if let Some(ref global_path) = app.global_path {
      let dir = expand_tilde(global_path);
      collect_agent_skills(
        &dir,
        "global",
        &app.id,
        &global_canonical,
        &mut managed_map,
        &mut managed_names,
        &mut unmanaged_list,
      );
    }
  }

  for root in config.scan_roots.iter() {
    let path = PathBuf::from(root);
    collect_custom_skills(&path, &global_canonical, &mut managed_names, &mut unmanaged_list);
  }

  let mut all_skills: Vec<LocalSkill> = managed_map.into_values().collect();
  apply_name_conflicts(&mut unmanaged_list);
  all_skills.extend(unmanaged_list);
  Ok(all_skills)
}

fn collect_canonical_skills(
  base_dir: &Path,
  scope: &str,
  map: &mut HashMap<String, LocalSkill>,
  names: &mut HashSet<String>,
) {
  if !FolderHelper::exists(base_dir) {
    return;
  }
  for entry in FolderHelper::read_dir(base_dir).into_iter().flatten() {
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
  let frontmatter = FileHelper::read_skill_frontmatter(&skill_file).ok();

  let folder_name = skill_dir
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  let canonical_path = FolderHelper::canonicalize(skill_dir)
    .ok()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| skill_dir.to_string_lossy().to_string());

  Some(LocalSkill {
    name: frontmatter
      .as_ref()
      .and_then(|fm| fm.name.clone())
      .unwrap_or(folder_name),
    description: frontmatter.and_then(|fm| fm.description),
    scope: scope.to_string(),
    canonical_path,
    agents: Vec::new(),
    managed_status: "managed".to_string(),
    name_conflict: false,
    created_at: FileHelper::get_created_at(skill_dir),
    conflict_with_managed: false,
  })
}

fn collect_agent_skills(
  base_dir: &Path,
  scope: &str,
  agent_id: &str,
  global_canonical: &Path,
  managed_map: &mut HashMap<String, LocalSkill>,
  managed_names: &mut HashSet<String>,
  unmanaged_list: &mut Vec<LocalSkill>,
) {
  if !FolderHelper::exists(base_dir) {
    return;
  }
  for entry in FolderHelper::read_dir(base_dir).into_iter().flatten() {
    if let Ok(entry) = entry {
      let path = entry.path();
      if !is_skill_dir(&path) {
        continue;
      }

      let (mut skill, is_managed_link) = match parse_skill_dir(&path, scope, global_canonical) {
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
  global_canonical: &Path,
  managed_names: &mut HashSet<String>,
  unmanaged_list: &mut Vec<LocalSkill>,
) {
  if !FolderHelper::exists(base_dir) {
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
        if let Some((mut skill, _)) = parse_skill_dir(entry.path(), "custom", global_canonical) {
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
        if let Some((mut skill, _)) = parse_skill_dir(dir, "custom", global_canonical) {
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
  global_canonical: &Path,
) -> Option<(LocalSkill, bool)> {
  let skill_file = skill_dir.join("SKILL.md");
  let frontmatter = FileHelper::read_skill_frontmatter(&skill_file).ok();

  let folder_name = skill_dir
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  let canonical = FolderHelper::canonicalize(skill_dir).ok();
  let canonical_path = canonical
    .as_ref()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| skill_dir.to_string_lossy().to_string());

  let is_link = is_symlink_dir(skill_dir);
  let is_link_target = canonical
    .as_ref()
    .map(|p| is_under_canonical(p, global_canonical))
    .unwrap_or(false);
  let is_managed_link = is_link && is_link_target;

  Some((
    LocalSkill {
      name: frontmatter
        .as_ref()
        .and_then(|fm| fm.name.clone())
        .unwrap_or(folder_name),
      description: frontmatter.and_then(|fm| fm.description),
      scope: scope.to_string(),
      canonical_path,
      agents: Vec::new(),
      managed_status: "unknown".to_string(),
      name_conflict: false,
      created_at: FileHelper::get_created_at(skill_dir),
      conflict_with_managed: false,
    },
    is_managed_link,
  ))
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
    return FolderHelper::exists(&path.join("SKILL.md"));
  }
  FolderHelper::is_dir(path) && FolderHelper::exists(&path.join("SKILL.md"))
}

fn is_symlink_dir(path: &Path) -> bool {
  match FolderHelper::symlink_metadata(path) {
    Ok(meta) => meta.file_type().is_symlink() && FolderHelper::is_dir(path),
    Err(_) => false,
  }
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
