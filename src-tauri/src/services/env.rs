use crate::models::{AgentApp, InstallScope, UserProject};
use crate::services::agent_apps_service::local_agent_apps;
use crate::services::user_projects_service::list_user_projects;
use crate::utils::path::{expand_home_with, youskill_root, HUB_DIR, LOCK_FILE, TRASH_DIR};
use std::path::{Path, PathBuf};

/// Everything a hub operation needs to know about the machine it runs on. Built once per
/// command from the real environment, or from a temp directory in tests.
#[derive(Debug, Clone)]
pub struct Env {
  pub home: PathBuf,
  /// `<config_dir>/youskill`, where app config and legacy lock files live.
  pub config_dir: PathBuf,
  pub youskill_root: PathBuf,
  pub hub_root: PathBuf,
  pub lock_path: PathBuf,
  pub trash_root: PathBuf,
  pub agent_apps: Vec<AgentApp>,
  pub projects: Vec<UserProject>,
}

impl Env {
  pub fn current() -> Result<Env, String> {
    let home = dirs_next::home_dir().ok_or("Could not find home directory")?;
    let config_dir = dirs_next::config_dir()
      .ok_or("Could not find config directory")?
      .join("youskill");
    let projects = list_user_projects().unwrap_or_default();
    Ok(Self::with_home(
      home,
      config_dir,
      local_agent_apps(),
      projects,
    ))
  }

  pub fn with_home(
    home: PathBuf,
    config_dir: PathBuf,
    agent_apps: Vec<AgentApp>,
    projects: Vec<UserProject>,
  ) -> Env {
    let youskill_root = youskill_root(&home);
    Env {
      hub_root: youskill_root.join(HUB_DIR),
      lock_path: youskill_root.join(LOCK_FILE),
      trash_root: youskill_root.join(TRASH_DIR),
      youskill_root,
      home,
      config_dir,
      agent_apps,
      projects,
    }
  }

  #[cfg(test)]
  pub fn for_test(root: &Path, agent_apps: Vec<AgentApp>, projects: Vec<UserProject>) -> Env {
    let home = root.join("home");
    let config_dir = root.join("config").join("youskill");
    std::fs::create_dir_all(&home).expect("create test home");
    std::fs::create_dir_all(&config_dir).expect("create test config dir");
    Self::with_home(home, config_dir, agent_apps, projects)
  }

  pub fn hub_dir(&self, name: &str) -> PathBuf {
    self.hub_root.join(name)
  }

  pub fn agent(&self, id: &str) -> Option<&AgentApp> {
    self.agent_apps.iter().find(|app| app.id == id)
  }

  /// Skills root of an agent app for the given scope.
  pub fn agent_root(
    &self,
    app: &AgentApp,
    scope: InstallScope,
    project_root: Option<&Path>,
  ) -> Result<PathBuf, String> {
    match scope {
      InstallScope::User => {
        let global_path = app
          .global_path
          .as_deref()
          .ok_or_else(|| format!("{} has no user-level skills path", app.display_name))?;
        Ok(expand_home_with(global_path, &self.home))
      },
      InstallScope::Project => {
        let project_root =
          project_root.ok_or_else(|| "project path is required for project scope".to_string())?;
        let project_path = app
          .project_path
          .as_deref()
          .ok_or_else(|| format!("{} has no project-level skills path", app.display_name))?;
        Ok(project_root.join(project_path))
      },
    }
  }
}
