use crate::models::UserProject;
use std::fs;
use std::path::PathBuf;

pub fn list_user_projects() -> Result<Vec<UserProject>, String> {
  load_user_projects()
}

pub fn add_user_project(name: String, path: String) -> Result<UserProject, String> {
  let name = name.trim().to_string();
  let path = path.trim().to_string();

  let mut projects = load_user_projects()?;
  validate_user_project(&name, &path, &projects, None)?;

  let project = UserProject {
    name,
    path,
    workspace_path: None,
  };
  projects.push(project.clone());
  save_user_projects(&projects)?;

  Ok(project)
}

pub fn update_user_project(
  original_name: String,
  name: String,
  path: String,
) -> Result<UserProject, String> {
  let original_name = original_name.trim().to_string();
  let name = name.trim().to_string();
  let path = path.trim().to_string();

  let mut projects = load_user_projects()?;
  let index = projects
    .iter()
    .position(|project| project.name == original_name)
    .ok_or(format!("Project '{}' not found", original_name))?;

  validate_user_project(&name, &path, &projects, Some(original_name.as_str()))?;

  let project = UserProject {
    name,
    path,
    workspace_path: projects[index].workspace_path.clone(),
  };
  projects[index] = project.clone();
  save_user_projects(&projects)?;

  Ok(project)
}

pub fn remove_user_project(name: String) -> Result<(), String> {
  let name = name.trim().to_string();
  let mut projects = load_user_projects()?;
  let previous_len = projects.len();
  projects.retain(|project| project.name != name);

  if projects.len() == previous_len {
    return Err(format!("Project '{}' not found", name));
  }

  save_user_projects(&projects)?;
  Ok(())
}

/// Add several projects at once, skipping paths that are already registered and making
/// every name unique. Returns the projects that were added.
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
    let name = unique_name(item.name.trim(), &projects);
    let project = UserProject {
      name,
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

fn unique_name(name: &str, existing: &[UserProject]) -> String {
  let base = if name.is_empty() { "project" } else { name };
  let taken = |candidate: &str| {
    existing
      .iter()
      .any(|project| project.name.eq_ignore_ascii_case(candidate))
  };
  if !taken(base) {
    return base.to_string();
  }
  let mut index = 2;
  loop {
    let candidate = format!("{} {}", base, index);
    if !taken(&candidate) {
      return candidate;
    }
    index += 1;
  }
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

fn validate_user_project(
  name: &str,
  path: &str,
  existing: &[UserProject],
  current_name: Option<&str>,
) -> Result<(), String> {
  if name.is_empty() {
    return Err("Project name is required".to_string());
  }

  if path.is_empty() {
    return Err("Project path is required".to_string());
  }

  let same_project = |project: &UserProject| Some(project.name.as_str()) == current_name;

  if existing
    .iter()
    .filter(|project| !same_project(project))
    .any(|project| project.name.eq_ignore_ascii_case(name))
  {
    return Err(format!("Project name '{}' already exists", name));
  }

  if existing
    .iter()
    .filter(|project| !same_project(project))
    .any(|project| project.path.eq_ignore_ascii_case(path))
  {
    return Err(format!("Project path '{}' already exists", path));
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::{validate_user_project, UserProject};

  #[test]
  fn validate_rejects_duplicate_name() {
    let existing = vec![
      UserProject {
        name: "Alpha".to_string(),
        path: "/tmp/alpha".to_string(),
        workspace_path: None,
      },
      UserProject {
        name: "Beta".to_string(),
        path: "/tmp/beta".to_string(),
        workspace_path: None,
      },
    ];

    let result = validate_user_project("alpha", "/tmp/new", &existing, None);
    assert!(result.is_err());
  }

  #[test]
  fn validate_rejects_duplicate_path() {
    let existing = vec![UserProject {
      name: "Alpha".to_string(),
      path: "/tmp/alpha".to_string(),
      workspace_path: None,
    }];

    let result = validate_user_project("Gamma", "/tmp/alpha", &existing, None);
    assert!(result.is_err());
  }

  #[test]
  fn validate_allows_current_project_on_update() {
    let existing = vec![UserProject {
      name: "Alpha".to_string(),
      path: "/tmp/alpha".to_string(),
      workspace_path: None,
    }];

    let result = validate_user_project("Alpha", "/tmp/alpha", &existing, Some("Alpha"));
    assert!(result.is_ok());
  }
}
