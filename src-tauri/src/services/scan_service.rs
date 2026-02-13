use crate::models::LocalSkill;
use crate::services::agent_apps_service::local_agent_apps;
use crate::utils::file::FileHelper;
use crate::utils::folder::FolderHelper;
use crate::utils::path::{expand_home, is_under_canonical};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub fn scan_local_skills() -> Result<Vec<LocalSkill>, String> {
  let mut managed_map: HashMap<String, LocalSkill> = HashMap::new();
  let mut unmanaged_list: Vec<LocalSkill> = Vec::new();
  let mut managed_names: HashSet<String> = HashSet::new();

  let global_canonical = expand_home("~/.agents/skills");

  collect_canonical_skills(&global_canonical, &mut managed_map, &mut managed_names);

  // Scan all locally installed agent apps (built-in + custom)
  for app in local_agent_apps() {
    if let Some(ref global_path) = app.global_path {
      let dir = expand_home(global_path);
      collect_agent_skills(
        &dir,
        &app.id,
        &global_canonical,
        &mut managed_map,
        &mut managed_names,
        &mut unmanaged_list,
      );
    }
  }

  let mut all_skills: Vec<LocalSkill> = managed_map.into_values().collect();
  apply_name_conflicts(&mut unmanaged_list);
  all_skills.extend(unmanaged_list);
  Ok(all_skills)
}

fn collect_canonical_skills(
  base_dir: &Path,
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
      if let Some(mut skill) = parse_canonical_skill_dir(&path) {
        skill.managed_status = "managed".to_string();
        let name = skill.name.clone();
        let key = name.clone();
        map.entry(key).or_insert(skill);
        names.insert(name);
      }
    }
  }
}

fn parse_canonical_skill_dir(skill_dir: &Path) -> Option<LocalSkill> {
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
    scope: "global".to_string(),
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

      let (mut skill, is_managed_link) = match parse_skill_dir(&path, global_canonical) {
        Some(skill) => skill,
        None => continue,
      };

      if is_managed_link {
        let key = skill.name.clone();
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

fn parse_skill_dir(skill_dir: &Path, global_canonical: &Path) -> Option<(LocalSkill, bool)> {
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
      scope: "global".to_string(),
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
