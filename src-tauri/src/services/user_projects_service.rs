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

  let project = UserProject { name, path };
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

  let project = UserProject { name, path };
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
      },
      UserProject {
        name: "Beta".to_string(),
        path: "/tmp/beta".to_string(),
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
    }];

    let result = validate_user_project("Gamma", "/tmp/alpha", &existing, None);
    assert!(result.is_err());
  }

  #[test]
  fn validate_allows_current_project_on_update() {
    let existing = vec![UserProject {
      name: "Alpha".to_string(),
      path: "/tmp/alpha".to_string(),
    }];

    let result = validate_user_project("Alpha", "/tmp/alpha", &existing, Some("Alpha"));
    assert!(result.is_ok());
  }
}
