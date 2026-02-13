use crate::models::InstallResult;
use crate::services::install_service;
use std::path::Path;
use std::process::Command;

pub fn install_package(
  zip_path: String,
  skill_path: String,
  agents: Vec<String>,
) -> Result<InstallResult, String> {
  install_service::install_zip_skill(zip_path, skill_path, agents)
}

pub fn install_folder(
  folder_path: String,
  skill_path: String,
  agents: Vec<String>,
) -> Result<InstallResult, String> {
  install_service::install_folder_skill(folder_path, skill_path, agents)
}

pub fn install_github_manual(
  url: String,
  skill_path: String,
  agents: Vec<String>,
) -> Result<InstallResult, String> {
  install_service::install_github_skill(url, skill_path, agents, None)
}

pub fn install_github_auto(
  url: String,
  skill_path: String,
  agents: Vec<String>,
  skill_folder_hash: Option<String>,
) -> Result<InstallResult, String> {
  install_service::install_github_skill(url, skill_path, agents, skill_folder_hash)
}

pub fn open_in_file_manager(file_path: String) -> Result<(), String> {
  let path = Path::new(&file_path);

  if !path.exists() {
    return Err(format!("Path does not exist: {}", file_path));
  }

  #[cfg(target_os = "macos")]
  {
    Command::new("open")
      .arg("-R")
      .arg(&file_path)
      .spawn()
      .map_err(|e| format!("Failed to open file manager: {}", e))?;
    Ok(())
  }

  #[cfg(target_os = "linux")]
  {
    let managers = ["nautilus", "dolphin", "thunar", "pcmanfm"];
    let mut opened = false;

    for manager in managers {
      if Command::new(manager)
        .arg("--select")
        .arg(&file_path)
        .spawn()
        .is_ok()
      {
        opened = true;
        break;
      }
    }

    if !opened {
      Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn()
        .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
  }

  #[cfg(target_os = "windows")]
  {
    Command::new("explorer")
      .arg("/select,")
      .arg(&file_path)
      .spawn()
      .map_err(|e| format!("Failed to open file manager: {}", e))?;
    Ok(())
  }
}

pub async fn read_skill_readme(skill_path: String) -> Result<String, String> {
  let path = Path::new(&skill_path);

  if !path.exists() {
    return Err(format!("Skill directory does not exist: {}", skill_path));
  }

  let skill_md = path.join("SKILL.md");
  if skill_md.exists() {
    return tokio::fs::read_to_string(&skill_md)
      .await
      .map_err(|e| format!("Failed to read SKILL.md: {}", e));
  }

  let readme_md = path.join("README.md");
  if readme_md.exists() {
    return tokio::fs::read_to_string(&readme_md)
      .await
      .map_err(|e| format!("Failed to read README.md: {}", e));
  }

  Err(format!("No SKILL.md or README.md found in: {}", skill_path))
}

pub fn check_skill_update(skill_name: String, remote_sha: String) -> Result<bool, String> {
  let entry = crate::services::skill_lock_service::get_skill_from_lock(skill_name)?;

  let Some(entry) = entry else {
    return Ok(false);
  };

  let Some(local_sha) = entry.skill_folder_hash else {
    return Ok(false);
  };

  Ok(local_sha != remote_sha)
}
