//! Workspaces: folders that hold projects. A folder counts as a project when it has an
//! agent's project skills directory (`.agents/skills`, `.claude/skills`, ...) or an agent
//! instruction file (`AGENTS.md`, `CLAUDE.md`).

use crate::models::{
  InstallScope, MemoryFile, ProjectCandidate, ProjectRegistration, UserProject, UserWorkspace,
};
use crate::services::env::Env;
use crate::services::user_projects_service::{
  add_user_projects, list_user_projects, remove_projects_of_workspace,
};
use crate::utils::folder::SKILL_MD;
use crate::utils::hash::is_excluded_component;
use crate::utils::path::{expand_home_with, normalize_dir_path, path_to_string, same_path};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// How deep below a workspace a project may sit (`<workspace>/a/b/c/d`).
pub const DEFAULT_WORKSPACE_DEPTH: usize = 4;

pub fn list_workspaces() -> Result<Vec<UserWorkspace>, String> {
  load_workspaces()
}

pub fn add_workspace(env: &Env, name: String, path: String) -> Result<UserWorkspace, String> {
  let name = name.trim().to_string();
  if name.is_empty() {
    return Err("Workspace name is required".to_string());
  }
  let root = normalize_dir_path(&path, &env.home)?;
  let path = path_to_string(&root);

  let mut workspaces = load_workspaces()?;
  if workspaces
    .iter()
    .any(|workspace| same_path(Path::new(&workspace.path), &root))
  {
    return Err(format!("Workspace '{}' already exists", path));
  }
  if workspaces
    .iter()
    .any(|workspace| workspace.name.eq_ignore_ascii_case(&name))
  {
    return Err(format!("Workspace name '{}' already exists", name));
  }

  let workspace = UserWorkspace { name, path };
  workspaces.push(workspace.clone());
  save_workspaces(&workspaces)?;
  Ok(workspace)
}

/// Remove a workspace, optionally dropping the projects that were discovered in it.
pub fn remove_workspace(path: &str, remove_projects: bool) -> Result<(), String> {
  let mut workspaces = load_workspaces()?;
  let before = workspaces.len();
  workspaces.retain(|workspace| !workspace.path.eq_ignore_ascii_case(path));
  if workspaces.len() == before {
    return Err(format!("Workspace '{}' not found", path));
  }
  save_workspaces(&workspaces)?;
  if remove_projects {
    remove_projects_of_workspace(path)?;
  }
  Ok(())
}

/// Every project folder inside `path`, deepest-first pruning: once a folder is recognised
/// as a project, its subfolders are not searched.
pub fn scan_workspace(
  env: &Env,
  path: &str,
  max_depth: usize,
) -> Result<Vec<ProjectCandidate>, String> {
  let root = normalize_dir_path(path, &env.home)?;
  let known = list_user_projects()?;
  let mut out = Vec::new();

  let mut walker = WalkDir::new(&root)
    .follow_links(false)
    .max_depth(max_depth.max(1))
    .sort_by_file_name()
    .into_iter();

  while let Some(entry) = walker.next() {
    let Ok(entry) = entry else {
      continue;
    };
    if !entry.file_type().is_dir() {
      continue;
    }
    let name = entry.file_name().to_string_lossy().to_string();
    if entry.depth() > 0 && (is_excluded_component(&name) || name.starts_with('.')) {
      walker.skip_current_dir();
      continue;
    }
    if entry.path() == env.youskill_root {
      walker.skip_current_dir();
      continue;
    }
    let Some(markers) = project_markers(env, entry.path()) else {
      continue;
    };
    let dir = entry.path().to_path_buf();
    out.push(ProjectCandidate {
      name: dir
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path_to_string(&dir)),
      path: path_to_string(&dir),
      agent_ids: markers.agent_ids,
      profiles: markers.profiles,
      skill_count: markers.skill_count,
      registered: known
        .iter()
        .any(|project| same_path(Path::new(&project.path), &dir)),
    });
    walker.skip_current_dir();
  }

  Ok(out)
}

pub fn register_projects(items: Vec<ProjectRegistration>) -> Result<Vec<UserProject>, String> {
  let projects = items
    .into_iter()
    .map(|item| UserProject {
      name: item.name,
      path: item.path,
      workspace_path: item.workspace_path,
    })
    .collect();
  add_user_projects(projects)
}

/// The instruction files of one scope: user-level files come from `global_profile_path`,
/// project-level ones from `profile_path` inside the project. Agents reading the same file
/// share one entry.
pub fn list_memory_files(
  env: &Env,
  scope: InstallScope,
  project_path: Option<&str>,
) -> Result<Vec<MemoryFile>, String> {
  let project_root = match scope {
    InstallScope::User => None,
    InstallScope::Project => Some(normalize_dir_path(
      project_path.unwrap_or_default(),
      &env.home,
    )?),
  };

  let mut files: Vec<MemoryFile> = Vec::new();
  for app in &env.agent_apps {
    let path = match scope {
      InstallScope::User => app
        .global_profile_path
        .as_deref()
        .map(|value| expand_home_with(value, &env.home)),
      InstallScope::Project => app
        .profile_path
        .as_deref()
        .zip(project_root.as_ref())
        .map(|(value, root)| root.join(value)),
    };
    let Some(path) = path else {
      continue;
    };
    match files
      .iter_mut()
      .find(|file| same_path(Path::new(&file.path), &path))
    {
      Some(file) => file.agent_ids.push(app.id.clone()),
      None => files.push(MemoryFile {
        name: path
          .file_name()
          .map(|value| value.to_string_lossy().to_string())
          .unwrap_or_default(),
        exists: path.is_file(),
        path: path_to_string(&path),
        agent_ids: vec![app.id.clone()],
      }),
    }
  }

  files.sort_by(|a, b| b.exists.cmp(&a.exists).then_with(|| a.path.cmp(&b.path)));
  Ok(files)
}

struct Markers {
  agent_ids: Vec<String>,
  profiles: Vec<String>,
  skill_count: usize,
}

/// What makes `dir` a project, or `None` when nothing does.
fn project_markers(env: &Env, dir: &Path) -> Option<Markers> {
  let mut agent_ids: Vec<String> = Vec::new();
  let mut profiles: Vec<String> = Vec::new();
  let mut counted: Vec<PathBuf> = Vec::new();
  let mut skill_count = 0;

  for app in &env.agent_apps {
    if let Some(relative) = app.project_path.as_deref() {
      let skills_dir = dir.join(relative);
      if skills_dir.is_dir() {
        agent_ids.push(app.id.clone());
        if !counted.iter().any(|seen| same_path(seen, &skills_dir)) {
          skill_count += count_skills(&skills_dir);
          counted.push(skills_dir);
        }
      }
    }
    if let Some(profile) = app.profile_path.as_deref() {
      if !profiles.iter().any(|seen| seen == profile) && dir.join(profile).is_file() {
        profiles.push(profile.to_string());
      }
    }
  }

  if agent_ids.is_empty() && profiles.is_empty() {
    return None;
  }
  profiles.sort();
  Some(Markers {
    agent_ids,
    profiles,
    skill_count,
  })
}

fn count_skills(dir: &Path) -> usize {
  let Ok(entries) = fs::read_dir(dir) else {
    return 0;
  };
  entries
    .flatten()
    .filter(|entry| entry.path().join(SKILL_MD).is_file())
    .count()
}

fn workspaces_path() -> Result<PathBuf, String> {
  let config_dir = dirs_next::config_dir().ok_or("Unable to get config directory")?;
  Ok(config_dir.join("youskill").join("user_workspaces.json"))
}

fn load_workspaces() -> Result<Vec<UserWorkspace>, String> {
  let path = workspaces_path()?;
  if !path.exists() {
    return Ok(Vec::new());
  }
  let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_workspaces(workspaces: &[UserWorkspace]) -> Result<(), String> {
  let path = workspaces_path()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  let content = serde_json::to_string_pretty(workspaces).map_err(|e| e.to_string())?;
  fs::write(path, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::AgentApp;
  use std::fs;

  fn app(id: &str, project: Option<&str>, profile: Option<&str>) -> AgentApp {
    AgentApp {
      id: id.to_string(),
      display_name: id.to_string(),
      project_path: project.map(|value| value.to_string()),
      global_path: Some(format!("~/.{}/skills", id)),
      detect_path: None,
      profile_path: profile.map(|value| value.to_string()),
      global_profile_path: None,
      is_user_custom: false,
    }
  }

  fn test_env(root: &Path) -> Env {
    Env::for_test(
      root,
      vec![
        app("agents", Some(".agents/skills"), Some("AGENTS.md")),
        app("claude-code", Some(".claude/skills"), Some("CLAUDE.md")),
      ],
      Vec::new(),
    )
  }

  #[test]
  fn finds_projects_by_skills_dir_and_profile_file() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("dev");
    // A project with a skills directory holding one skill.
    fs::create_dir_all(workspace.join("alpha/.agents/skills/foo")).unwrap();
    fs::write(workspace.join("alpha/.agents/skills/foo/SKILL.md"), "x").unwrap();
    // A project recognised by its instruction file only.
    fs::create_dir_all(workspace.join("group/beta")).unwrap();
    fs::write(workspace.join("group/beta/CLAUDE.md"), "x").unwrap();
    // Neither: not a project.
    fs::create_dir_all(workspace.join("group/gamma/src")).unwrap();
    // Nested below a project: not reported separately.
    fs::create_dir_all(workspace.join("alpha/nested")).unwrap();
    fs::write(workspace.join("alpha/nested/AGENTS.md"), "x").unwrap();

    let env = test_env(tmp.path());
    let found = scan_workspace(&env, workspace.to_str().unwrap(), 4).unwrap();
    let names: Vec<&str> = found.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "beta"]);

    let alpha = &found[0];
    assert_eq!(alpha.agent_ids, vec!["agents".to_string()]);
    assert!(alpha.profiles.is_empty());
    assert_eq!(alpha.skill_count, 1);

    let beta = &found[1];
    assert!(beta.agent_ids.is_empty());
    assert_eq!(beta.profiles, vec!["CLAUDE.md".to_string()]);
    assert_eq!(beta.skill_count, 0);
  }

  #[test]
  fn skips_hidden_and_excluded_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("dev");
    fs::create_dir_all(workspace.join("node_modules/pkg")).unwrap();
    fs::write(workspace.join("node_modules/pkg/AGENTS.md"), "x").unwrap();
    fs::create_dir_all(workspace.join(".cache/thing")).unwrap();
    fs::write(workspace.join(".cache/thing/AGENTS.md"), "x").unwrap();

    let env = test_env(tmp.path());
    let found = scan_workspace(&env, workspace.to_str().unwrap(), 4).unwrap();
    assert!(found.is_empty());
  }

  #[test]
  fn the_workspace_itself_can_be_a_project() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("repo");
    fs::create_dir_all(workspace.join("sub")).unwrap();
    fs::write(workspace.join("AGENTS.md"), "x").unwrap();
    fs::write(workspace.join("sub/AGENTS.md"), "x").unwrap();

    let env = test_env(tmp.path());
    let found = scan_workspace(&env, workspace.to_str().unwrap(), 4).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, path_to_string(&workspace));
  }
}
