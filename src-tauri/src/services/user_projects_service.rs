//! Registered projects. A project is identified by its path; its name is only a label, so
//! two folders with the same name in different places can both be registered.

use crate::models::UserProject;
use std::fs;
use std::path::{Path, PathBuf};

pub fn list_user_projects() -> Result<Vec<UserProject>, String> {
  load_user_projects()
}

pub fn remove_user_project(path: &str) -> Result<(), String> {
  let mut projects = load_user_projects()?;
  let previous_len = projects.len();
  projects.retain(|project| !project.path.eq_ignore_ascii_case(path));

  if projects.len() == previous_len {
    return Err(format!("Project '{}' not found", path));
  }

  save_user_projects(&projects)
}

/// Add several projects at once, skipping paths that are already registered. Returns the
/// projects that were added.
pub fn add_user_projects(items: Vec<UserProject>) -> Result<Vec<UserProject>, String> {
  let mut projects = load_user_projects()?;
  let mut added = Vec::new();

  for item in items {
    let path = item.path.trim().to_string();
    if path.is_empty() {
      continue;
    }
    if projects
      .iter()
      .any(|project| project.path.eq_ignore_ascii_case(&path))
    {
      continue;
    }
    let name = item.name.trim();
    let project = UserProject {
      name: if name.is_empty() {
        folder_name(&path)
      } else {
        name.to_string()
      },
      path,
      workspace_path: item.workspace_path,
    };
    projects.push(project.clone());
    added.push(project);
  }

  if !added.is_empty() {
    save_user_projects(&projects)?;
  }
  Ok(added)
}

/// Drop every project that was discovered in `workspace_path`.
pub fn remove_projects_of_workspace(workspace_path: &str) -> Result<usize, String> {
  let mut projects = load_user_projects()?;
  let before = projects.len();
  projects.retain(|project| {
    project
      .workspace_path
      .as_deref()
      .map(|path| !path.eq_ignore_ascii_case(workspace_path))
      .unwrap_or(true)
  });
  let removed = before - projects.len();
  if removed > 0 {
    save_user_projects(&projects)?;
  }
  Ok(removed)
}

/// Repair records written by earlier versions and save them when anything changed.
pub fn tidy_user_projects(workspace_paths: &[String]) -> Result<(), String> {
  let mut projects = load_user_projects()?;
  if tidy(&mut projects, workspace_paths) {
    save_user_projects(&projects)?;
  }
  Ok(())
}

/// Drop projects of workspaces that were removed (earlier versions kept them), and name
/// workspace projects after their folder (earlier versions added a " 2" suffix to make
/// names unique). Returns whether anything changed.
fn tidy(projects: &mut Vec<UserProject>, workspace_paths: &[String]) -> bool {
  let before = projects.len();
  projects.retain(|project| match project.workspace_path.as_deref() {
    Some(workspace) => workspace_paths
      .iter()
      .any(|path| path.eq_ignore_ascii_case(workspace)),
    None => true,
  });
  let mut changed = projects.len() != before;

  for project in projects.iter_mut() {
    if project.workspace_path.is_none() {
      continue;
    }
    let name = folder_name(&project.path);
    if !name.is_empty() && project.name != name {
      project.name = name;
      changed = true;
    }
  }
  changed
}

fn folder_name(path: &str) -> String {
  Path::new(path)
    .file_name()
    .map(|value| value.to_string_lossy().to_string())
    .unwrap_or_default()
}

fn user_projects_path() -> Result<PathBuf, String> {
  let config_dir = dirs_next::config_dir().ok_or("Unable to get config directory")?;
  Ok(config_dir.join("youskill").join("user_projects.json"))
}

fn load_user_projects() -> Result<Vec<UserProject>, String> {
  let path = user_projects_path()?;
  if !path.exists() {
    return Ok(Vec::new());
  }

  let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_user_projects(projects: &[UserProject]) -> Result<(), String> {
  let path = user_projects_path()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }

  let content = serde_json::to_string_pretty(projects).map_err(|e| e.to_string())?;
  fs::write(path, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::{tidy, UserProject};

  fn project(name: &str, path: &str, workspace: Option<&str>) -> UserProject {
    UserProject {
      name: name.to_string(),
      path: path.to_string(),
      workspace_path: workspace.map(|value| value.to_string()),
    }
  }

  #[test]
  fn tidy_drops_projects_of_removed_workspaces() {
    let mut projects = vec![
      project("alpha", "/dev/old/alpha", Some("/dev/old")),
      project("beta", "/dev/new/beta", Some("/dev/new")),
      project("manual", "/tmp/manual", None),
    ];

    assert!(tidy(&mut projects, &["/dev/new".to_string()]));
    let paths: Vec<&str> = projects.iter().map(|item| item.path.as_str()).collect();
    assert_eq!(paths, vec!["/dev/new/beta", "/tmp/manual"]);
  }

  #[test]
  fn tidy_names_workspace_projects_after_their_folder() {
    let mut projects = vec![
      project("alpha", "/dev/old/alpha", Some("/dev/old")),
      project("alpha 2", "/dev/new/alpha", Some("/dev/new")),
      project("Custom", "/tmp/manual", None),
    ];

    let workspaces = ["/dev/old".to_string(), "/dev/new".to_string()];
    assert!(tidy(&mut projects, &workspaces));
    let names: Vec<&str> = projects.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "alpha", "Custom"]);

    assert!(!tidy(&mut projects, &workspaces));
  }
}
