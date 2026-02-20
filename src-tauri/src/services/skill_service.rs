use crate::models::{
  DetectedSkill, InstallGithubRequest, InstallKnownRequest, InstallMethod, InstallNativeRequest,
  InstallResult, InstalledAgentApp, LocalSkill, ManageSkillAgentAppsRequest, SourceCheckResult,
  SourceType,
};
use crate::services::agent_apps_service::local_agent_apps;
use crate::services::native_skill_lock_service::{
  add_skill_to_native_lock, read_native_skill_lock_internal, remove_skill_from_native_lock,
};
use crate::services::skill_lock_service::{
  add_skill_to_lock, read_skill_lock_internal, SkillLockEntry, SkillLockFile,
};
use crate::utils::file::FileHelper;
use crate::utils::folder::FolderHelper;
use crate::utils::github::GithubHelper;
use crate::utils::path::{expand_home, remove_path_any};
use crate::utils::zip::ZipHelper;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn detect_folder(folder_path: String) -> Result<DetectedSkill, String> {
  let folder = Path::new(&folder_path);
  if !folder.exists() || !folder.is_dir() {
    return Err(format!("Folder does not exist: {}", folder_path));
  }

  let skill_md = folder.join("SKILL.md");
  if !skill_md.exists() {
    return Err(format!("SKILL.md not found in folder: {}", folder_path));
  }

  let name = FileHelper::read_skill_frontmatter(&skill_md)?
    .name
    .ok_or("SKILL.md frontmatter missing valid 'name'".to_string())?;
  let tmp_dir = create_temp_dir("detect-folder")?;
  let tmp_path = tmp_dir.join(&name);
  copy_dir_all_sync(folder, &tmp_path)?;

  Ok(DetectedSkill {
    name,
    tmp_path: tmp_path.to_string_lossy().to_string(),
    skill_path: "SKILL.md".to_string(),
  })
}

pub fn detect_zip(zip_path: String) -> Result<DetectedSkill, String> {
  let temp_extract_dir = create_temp_dir("detect-zip")?;
  ZipHelper::extract_to_dir(&zip_path, &temp_extract_dir)?;
  detect_folder(temp_extract_dir.to_string_lossy().to_string())
}

pub fn detect_github_manual(github_path: String) -> Result<Vec<DetectedSkill>, String> {
  let (owner, repo) = GithubHelper::parse_github_url(&github_path)?;
  let clone_dir = create_temp_dir(&format!("detect-github-manual-{}-{}", owner, repo))?;
  GithubHelper::clone_repo_to(&owner, &repo, &clone_dir)?;

  let skill_dirs = FolderHelper::find_dirs_containing_file(&clone_dir, "SKILL.md")?;
  if skill_dirs.is_empty() {
    return Err("No SKILL.md found in repository".to_string());
  }

  let mut result = Vec::new();
  for dir in skill_dirs {
    match detect_folder(dir.to_string_lossy().to_string()) {
      Ok(mut detected) => {
        let relative_dir = dir
          .strip_prefix(&clone_dir)
          .map(|p| p.to_string_lossy().to_string())
          .unwrap_or_default();
        detected.skill_path = if relative_dir.is_empty() {
          "SKILL.md".to_string()
        } else {
          format!("{}/SKILL.md", relative_dir)
        };
        result.push(detected);
      },
      Err(_) => continue,
    }
  }

  if result.is_empty() {
    return Err("No valid skills found after parsing SKILL.md".to_string());
  }

  Ok(result)
}

pub fn detect_github_auto(
  github_path: String,
  skill_name: String,
) -> Result<DetectedSkill, String> {
  let (owner, repo) = GithubHelper::parse_github_url(&github_path)?;
  let clone_dir = create_temp_dir(&format!("detect-github-auto-{}-{}", owner, repo))?;
  GithubHelper::clone_repo_to(&owner, &repo, &clone_dir)?;

  let skill_dirs = FolderHelper::find_dirs_containing_file(&clone_dir, "SKILL.md")?;
  if skill_dirs.is_empty() {
    return Err("No SKILL.md found in repository".to_string());
  }

  for dir in skill_dirs {
    let skill_md = dir.join("SKILL.md");
    let Ok(frontmatter) = FileHelper::read_skill_frontmatter(&skill_md) else {
      continue;
    };
    let Some(name) = frontmatter.name else {
      continue;
    };
    if name != skill_name {
      continue;
    }

    let mut detected = detect_folder(dir.to_string_lossy().to_string())?;
    let relative_dir = dir
      .strip_prefix(&clone_dir)
      .map(|p| p.to_string_lossy().to_string())
      .unwrap_or_default();
    detected.skill_path = if relative_dir.is_empty() {
      "SKILL.md".to_string()
    } else {
      format!("{}/SKILL.md", relative_dir)
    };
    return Ok(detected);
  }

  Err(format!("No skill matched '{}'", skill_name))
}

pub fn list_skills() -> Result<Vec<LocalSkill>, String> {
  let github_lock = read_skill_lock_internal().unwrap_or_default();
  let native_lock = read_native_skill_lock_internal().unwrap_or_default();

  let mut skills: Vec<LocalSkill> = Vec::new();
  let mut skill_index: HashMap<String, usize> = HashMap::new();

  let global_root = expand_home("~/.agents/skills");
  if global_root.exists() && global_root.is_dir() {
    for entry in fs::read_dir(&global_root).map_err(|e| e.to_string())? {
      let entry = entry.map_err(|e| e.to_string())?;
      let skill_dir = entry.path();
      if !is_skill_folder(&skill_dir) {
        continue;
      }
      let Some(skill_name) = read_valid_skill_name(&skill_dir) else {
        continue;
      };
      if skill_index.contains_key(&skill_name) {
        continue;
      }

      let source_type = detect_source_type(&skill_name, &github_lock, &native_lock);
      let index = skills.len();
      skills.push(LocalSkill {
        name: skill_name.clone(),
        global_folder: Some(skill_dir.to_string_lossy().to_string()),
        installed_agent_apps: Vec::new(),
        source_type,
      });
      skill_index.insert(skill_name, index);
    }
  }

  for app in local_agent_apps() {
    let Some(global_path) = &app.global_path else {
      continue;
    };
    let app_root = expand_home(global_path);
    if !app_root.exists() || !app_root.is_dir() {
      continue;
    }

    tracing::info!("-------------App root: {:?}", app_root);

    let entries = match fs::read_dir(&app_root) {
      Ok(entries) => entries,
      Err(_) => continue,
    };
    for entry in entries {
      let Ok(entry) = entry else {
        continue;
      };
      let skill_dir = entry.path();
      if !is_skill_folder(&skill_dir) {
        continue;
      }
      let Some(skill_name) = read_valid_skill_name(&skill_dir) else {
        continue;
      };

      let method = detect_install_method(&skill_dir);
      let installed_app = InstalledAgentApp {
        id: app.id.clone(),
        skill_folder: skill_dir.to_string_lossy().to_string(),
        method,
      };

      tracing::info!("path: {:?}, Installed app: {:?}", skill_dir, installed_app);
      if let Some(index) = skill_index.get(&skill_name).copied() {
        if !skills[index].installed_agent_apps.iter().any(|item| {
          item.id == installed_app.id && item.skill_folder == installed_app.skill_folder
        }) {
          skills[index].installed_agent_apps.push(installed_app);
        }
        continue;
      }

      let source_type = detect_source_type(&skill_name, &github_lock, &native_lock);
      let index = skills.len();
      skills.push(LocalSkill {
        name: skill_name.clone(),
        global_folder: None,
        installed_agent_apps: vec![installed_app],
        source_type,
      });
      skill_index.insert(skill_name, index);
    }
  }

  tracing::info!("Local skills: {:?}", skills);
  Ok(skills)
}

pub fn install_from_native(request: InstallNativeRequest) -> Result<InstallResult, String> {
  validate_native_install_request(&request)?;
  let selected_apps = resolve_selected_apps_global_paths(&request.agent_apps)?;
  remove_existing_associations_from_all_apps(&request.name)?;
  install_skill_to_apps(
    Path::new(&request.tmp_path),
    &request.name,
    &request.method,
    &selected_apps,
  )?;
  add_skill_to_native_lock(request.name.clone())?;

  Ok(InstallResult {
    success: true,
    stdout: format!("Skill '{}' installed from native source", request.name),
    stderr: String::new(),
    message: "安装成功".to_string(),
  })
}

pub fn install_from_github(request: InstallGithubRequest) -> Result<InstallResult, String> {
  validate_github_install_request(&request)?;
  let selected_apps = resolve_selected_apps_global_paths(&request.agent_apps)?;
  remove_existing_associations_from_all_apps(&request.name)?;
  install_skill_to_apps(
    Path::new(&request.tmp_path),
    &request.name,
    &request.method,
    &selected_apps,
  )?;

  let skill_folder_hash = match normalize_optional_string(request.skill_folder_hash.clone()) {
    Some(hash) => hash,
    None => GithubHelper::get_skill_folder_hash(&request.source_url, &request.skill_path)?,
  };

  let source = GithubHelper::parse_github_url(&request.source_url)
    .map(|(owner, repo)| format!("{}/{}", owner, repo))
    .unwrap_or_else(|_| request.source_url.clone());

  add_skill_to_lock(
    request.name.clone(),
    SkillLockEntry {
      source,
      source_type: "github".to_string(),
      source_url: request.source_url.clone(),
      skill_path: Some(request.skill_path.clone()),
      skill_folder_hash: Some(skill_folder_hash),
      installed_at: String::new(),
      updated_at: String::new(),
    },
  )?;

  Ok(InstallResult {
    success: true,
    stdout: format!("Skill '{}' installed from github source", request.name),
    stderr: String::new(),
    message: "安装成功".to_string(),
  })
}

pub fn check_skill_version(
  name: String,
  global_folder: Option<String>,
  skill_paths: Vec<String>,
) -> Result<SourceCheckResult, String> {
  if let Some(global_folder) = normalize_optional_string(global_folder) {
    let global_path = PathBuf::from(&global_folder);
    if global_path.exists() && global_path.is_dir() {
      return Ok(SourceCheckResult {
        source_path: Some(global_folder),
        version_groups: Vec::new(),
        requires_selection: false,
      });
    }
  }
  resolve_source_path_by_consistency(&name, &skill_paths)
}

pub fn install_from_known(request: InstallKnownRequest) -> Result<InstallResult, String> {
  validate_install_from_known_request(&request)?;

  let source = Path::new(&request.source_path);
  let canonical_path = prepare_canonical_skill_dir(source, &request.name)?;

  install_from_native(InstallNativeRequest {
    name: request.name,
    tmp_path: canonical_path.to_string_lossy().to_string(),
    skill_path: "SKILL.md".to_string(),
    agent_apps: request.agent_apps,
    method: request.method,
  })
}

pub fn manage_skill_agent_apps(
  request: ManageSkillAgentAppsRequest,
) -> Result<InstallResult, String> {
  if request.name.trim().is_empty() {
    return Err("Skill name is required".to_string());
  }
  if request.agent_apps.is_empty() {
    return Err("agent_apps is required".to_string());
  }

  if request.source_type == SourceType::Known {
    return Err(
      "Known source should use install_from_known flow; manage_skill_agent_apps only handles managed skills"
        .to_string(),
    );
  }

  let selected_apps = resolve_selected_apps_global_paths(&request.agent_apps)?;
  let source = if matches!(request.method, InstallMethod::Copy) {
    let resolved = normalize_optional_string(request.source_path)
      .ok_or("source_path is required for copy management")?;
    let source_path = PathBuf::from(&resolved);
    if !source_path.exists() || !source_path.is_dir() {
      return Err("source_path does not exist".to_string());
    }
    source_path
  } else {
    let global_folder = request
      .global_folder
      .ok_or("global_folder is required for symlink management")?;
    let global_path = PathBuf::from(global_folder);
    if !global_path.exists() || !global_path.is_dir() {
      return Err("Global skill folder does not exist".to_string());
    }
    global_path
  };

  remove_existing_associations_from_all_apps(&request.name)?;
  install_skill_to_apps(&source, &request.name, &request.method, &selected_apps)?;

  Ok(InstallResult {
    success: true,
    stdout: format!("Skill '{}' agent apps updated", request.name),
    stderr: String::new(),
    message: "安装成功".to_string(),
  })
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

pub fn delete_skill(
  name: String,
  canonical_path: String,
  _scope: String,
  _agents: Vec<String>,
) -> Result<(), String> {
  let skill_path = PathBuf::from(&canonical_path);
  let folder_name = skill_path
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .ok_or("无法获取技能文件夹名")?;
  let cwd = env::current_dir().map_err(|e| e.to_string())?;

  for app in local_agent_apps() {
    let mut dirs_to_check: Vec<PathBuf> = Vec::new();
    if let Some(ref project_path) = app.project_path {
      dirs_to_check.push(cwd.join(project_path));
    }
    if let Some(ref global_path) = app.global_path {
      dirs_to_check.push(expand_home(global_path));
    }
    for dir in dirs_to_check {
      let link_path = dir.join(&folder_name);
      if link_path.exists() || link_path.is_symlink() {
        let _ = remove_path_any(&link_path);
      }
    }
  }

  if skill_path.exists() || skill_path.is_symlink() {
    remove_path_any(&skill_path)?;
  }

  let _ = crate::services::skill_lock_service::remove_skill_from_lock(name.clone());
  let _ = remove_skill_from_native_lock(name);

  Ok(())
}

fn validate_native_install_request(request: &InstallNativeRequest) -> Result<(), String> {
  validate_common_install_request(
    &request.name,
    &request.tmp_path,
    &request.skill_path,
    &request.agent_apps,
  )
}

fn validate_github_install_request(request: &InstallGithubRequest) -> Result<(), String> {
  validate_common_install_request(
    &request.name,
    &request.tmp_path,
    &request.skill_path,
    &request.agent_apps,
  )?;
  GithubHelper::parse_github_url(&request.source_url)?;
  Ok(())
}

fn validate_install_from_known_request(request: &InstallKnownRequest) -> Result<(), String> {
  if request.name.trim().is_empty() {
    return Err("Skill name is required".to_string());
  }
  if request.source_path.trim().is_empty() {
    return Err("source_path is required".to_string());
  }
  if request.agent_apps.is_empty() {
    return Err("agent_apps is required".to_string());
  }

  let source = Path::new(&request.source_path);
  if !source.exists() || !source.is_dir() {
    return Err(format!(
      "Source path does not exist: {}",
      source.to_string_lossy()
    ));
  }
  let skill_md = source.join("SKILL.md");
  let frontmatter = FileHelper::read_skill_frontmatter(&skill_md)?;
  let name = frontmatter
    .name
    .ok_or("SKILL.md frontmatter missing valid 'name'".to_string())?;
  if name != request.name {
    return Err(format!(
      "Source path skill name mismatch: expected '{}', got '{}'",
      request.name, name
    ));
  }
  Ok(())
}

fn validate_common_install_request(
  name: &str,
  tmp_path: &str,
  skill_path: &str,
  agent_apps: &[String],
) -> Result<(), String> {
  if name.trim().is_empty() {
    return Err("Skill name is required".to_string());
  }
  if tmp_path.trim().is_empty() {
    return Err("tmp_path is required".to_string());
  }
  if skill_path.trim().is_empty() {
    return Err("skill_path is required".to_string());
  }
  if agent_apps.is_empty() {
    return Err("agent_apps is required".to_string());
  }

  let source = Path::new(tmp_path);
  if !source.exists() || !source.is_dir() {
    return Err(format!(
      "Detected skill tmp path does not exist: {}",
      source.to_string_lossy()
    ));
  }
  Ok(())
}

#[derive(Clone)]
struct SelectedAgentPath {
  display_name: String,
  global_path: PathBuf,
}

fn resolve_selected_apps_global_paths(
  agent_apps: &[String],
) -> Result<Vec<SelectedAgentPath>, String> {
  let installed_apps = local_agent_apps();
  let mut selected = Vec::new();
  let mut errors = Vec::new();

  for app_id in agent_apps {
    let Some(app) = installed_apps.iter().find(|a| a.id == *app_id) else {
      errors.push(format!("Unknown app id: {}", app_id));
      continue;
    };
    let Some(global_path) = &app.global_path else {
      errors.push(format!("{} has no global path", app.display_name));
      continue;
    };
    selected.push(SelectedAgentPath {
      display_name: app.display_name.clone(),
      global_path: expand_home(global_path),
    });
  }

  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  if selected.is_empty() {
    return Err("No valid agent apps selected".to_string());
  }

  Ok(selected)
}

fn remove_existing_associations(
  skill_name: &str,
  selected_apps: &[SelectedAgentPath],
) -> Result<(), String> {
  let mut errors = Vec::new();
  for app in selected_apps {
    let target = app.global_path.join(skill_name);
    if target.exists() || target.is_symlink() {
      if let Err(err) = remove_path_any(&target) {
        errors.push(format!("{}: {}", app.display_name, err));
      }
    }
  }
  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  Ok(())
}

fn remove_existing_associations_from_all_apps(skill_name: &str) -> Result<(), String> {
  let all_apps: Vec<SelectedAgentPath> = local_agent_apps()
    .into_iter()
    .filter_map(|app| {
      app.global_path.map(|global_path| SelectedAgentPath {
        display_name: app.display_name,
        global_path: expand_home(&global_path),
      })
    })
    .collect();
  remove_existing_associations(skill_name, &all_apps)
}

fn resolve_source_path_by_consistency(
  skill_name: &str,
  skill_paths: &[String],
) -> Result<SourceCheckResult, String> {
  let candidates = normalized_existing_skill_paths(skill_name, skill_paths)?;
  if candidates.is_empty() {
    return Err(format!(
      "No valid skill source path found for '{}'",
      skill_name
    ));
  }
  if candidates.len() == 1 {
    return Ok(SourceCheckResult {
      source_path: Some(candidates[0].to_string_lossy().to_string()),
      version_groups: vec![crate::models::SourceVersionGroup {
        version: "版本 1".to_string(),
        source_path: candidates[0].to_string_lossy().to_string(),
        paths: vec![candidates[0].to_string_lossy().to_string()],
      }],
      requires_selection: false,
    });
  }

  let mut grouped: Vec<(String, Vec<PathBuf>)> = Vec::new();
  for path in &candidates {
    let digest = FolderHelper::compute_folder_digest(path)?;
    if let Some((_, paths)) = grouped.iter_mut().find(|(hash, _)| *hash == digest) {
      paths.push(path.clone());
    } else {
      grouped.push((digest, vec![path.clone()]));
    }
  }

  if grouped.len() == 1 {
    return Ok(SourceCheckResult {
      source_path: Some(candidates[0].to_string_lossy().to_string()),
      version_groups: vec![crate::models::SourceVersionGroup {
        version: "版本 1".to_string(),
        source_path: candidates[0].to_string_lossy().to_string(),
        paths: candidates.iter().map(|p| p.to_string_lossy().to_string()).collect(),
      }],
      requires_selection: false,
    });
  }

  let version_groups = grouped
    .into_iter()
    .enumerate()
    .map(|(idx, (_, paths))| {
      let source_path = paths[0].to_string_lossy().to_string();
      crate::models::SourceVersionGroup {
        version: format!("版本 {}", idx + 1),
        source_path,
        paths: paths.into_iter().map(|p| p.to_string_lossy().to_string()).collect(),
      }
    })
    .collect();

  Ok(SourceCheckResult {
    source_path: None,
    version_groups,
    requires_selection: true,
  })
}

fn normalized_existing_skill_paths(
  skill_name: &str,
  skill_paths: &[String],
) -> Result<Vec<PathBuf>, String> {
  let mut out: Vec<PathBuf> = Vec::new();
  for path in skill_paths {
    let path = PathBuf::from(path);
    if !path.exists() || !path.is_dir() {
      continue;
    }
    let skill_md = path.join("SKILL.md");
    if !skill_md.exists() {
      continue;
    }
    let frontmatter = match FileHelper::read_skill_frontmatter(&skill_md) {
      Ok(frontmatter) => frontmatter,
      Err(_) => continue,
    };
    if frontmatter.name.as_deref() != Some(skill_name) {
      continue;
    }
    if !out.iter().any(|existing| existing == &path) {
      out.push(path);
    }
  }
  Ok(out)
}

fn install_skill_to_apps(
  source: &Path,
  name: &str,
  method: &InstallMethod,
  selected_apps: &[SelectedAgentPath],
) -> Result<(), String> {
  let mut errors = Vec::new();

  match method {
    InstallMethod::Symlink => {
      let canonical_skill_dir = prepare_canonical_skill_dir(source, name)?;
      for app in selected_apps {
        if let Err(err) = fs::create_dir_all(&app.global_path) {
          errors.push(format!("{}: {}", app.display_name, err));
          continue;
        }
        let target = app.global_path.join(name);
        if let Err(err) = install_to_app(&canonical_skill_dir, &target, method) {
          errors.push(format!("{}: {}", app.display_name, err));
        }
      }
    },
    InstallMethod::Copy => {
      for app in selected_apps {
        if let Err(err) = fs::create_dir_all(&app.global_path) {
          errors.push(format!("{}: {}", app.display_name, err));
          continue;
        }
        let target = app.global_path.join(name);
        if let Err(err) = install_to_app(source, &target, method) {
          errors.push(format!("{}: {}", app.display_name, err));
        }
      }
    },
  }

  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  Ok(())
}

fn prepare_canonical_skill_dir(source: &Path, name: &str) -> Result<PathBuf, String> {
  let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
  let canonical_root = home_dir.join(".agents/skills");
  fs::create_dir_all(&canonical_root).map_err(|e| e.to_string())?;

  let canonical_skill_dir = canonical_root.join(name);
  if canonical_skill_dir == source {
    if !canonical_skill_dir.exists() {
      return Err(format!(
        "Canonical skill path does not exist: {}",
        canonical_skill_dir.to_string_lossy()
      ));
    }
    return Ok(canonical_skill_dir);
  }
  if canonical_skill_dir.exists() {
    remove_path_any(&canonical_skill_dir)?;
  }
  copy_dir_all_sync(source, &canonical_skill_dir)?;
  Ok(canonical_skill_dir)
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
  value.and_then(|v| {
    let trimmed = v.trim();
    if trimmed.is_empty() {
      None
    } else {
      Some(trimmed.to_string())
    }
  })
}

fn create_temp_dir(prefix: &str) -> Result<PathBuf, String> {
  let dir = std::env::temp_dir().join(format!(
    "skill-kit-{}-{}-{}",
    prefix,
    std::process::id(),
    now_millis()
  ));
  fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  Ok(dir)
}

fn now_millis() -> u128 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0)
}

fn install_to_app(source: &Path, target: &Path, method: &InstallMethod) -> Result<(), String> {
  match method {
    InstallMethod::Symlink => {
      #[cfg(unix)]
      {
        std::os::unix::fs::symlink(source, target).map_err(|e| e.to_string())
      }
      #[cfg(windows)]
      {
        match std::os::windows::fs::symlink_dir(source, target) {
          Ok(_) => Ok(()),
          Err(_) => copy_dir_all_sync(source, target),
        }
      }
    },
    InstallMethod::Copy => copy_dir_all_sync(source, target),
  }
}

fn copy_dir_all_sync(src: &Path, dst: &Path) -> Result<(), String> {
  fs::create_dir_all(dst).map_err(|e| e.to_string())?;
  for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());
    if src_path.is_dir() {
      copy_dir_all_sync(&src_path, &dst_path)?;
    } else {
      fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn is_skill_folder(path: &Path) -> bool {
  (path.is_dir() || path.is_symlink()) && path.join("SKILL.md").exists()
}

fn read_valid_skill_name(skill_dir: &Path) -> Option<String> {
  let skill_md = skill_dir.join("SKILL.md");
  if !skill_md.exists() {
    return None;
  }
  let frontmatter = FileHelper::read_skill_frontmatter(&skill_md).ok()?;
  frontmatter.name
}

fn detect_install_method(skill_dir: &Path) -> String {
  if fs::symlink_metadata(skill_dir)
    .map(|m| m.file_type().is_symlink())
    .unwrap_or(false)
  {
    "symlink".to_string()
  } else {
    "copy".to_string()
  }
}

fn detect_source_type(
  skill_name: &str,
  github_lock: &SkillLockFile,
  native_lock: &crate::services::native_skill_lock_service::NativeSkillLockFile,
) -> SourceType {
  if github_lock.skills.contains_key(skill_name) {
    SourceType::Github
  } else if native_lock.skills.contains_key(skill_name) {
    SourceType::Native
  } else {
    SourceType::Known
  }
}
