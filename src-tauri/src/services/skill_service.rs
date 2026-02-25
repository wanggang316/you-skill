use crate::models::{
  DetectedSkill, InstallGithubRequest, InstallMethod, InstallNativeRequest, InstallResult,
  InstallScope, InstallTarget, InstallUnknownRequest, InstalledAgentApp, LocalSkill,
  ManageSkillAgentAppsRequest, SelectedAgentPath,
  SourceCheckResult, SourceType,
};
use crate::services::agent_apps_service::{
  local_agent_apps, resolve_all_available_apps_paths, resolve_selected_apps_paths,
};
use crate::services::native_skill_lock_service::{
  add_skill_to_native_lock, read_native_skill_lock_internal, remove_skill_from_native_lock,
};
use crate::services::project_skill_lock_service::{
  add_skill_to_project_lock, project_lock_source, project_lock_source_type,
  remove_skill_from_project_lock,
};
use crate::services::skill_lock_service::{
  add_skill_to_lock, lock_source, lock_source_type, read_skill_lock_internal,
  remove_skill_from_global_lock,
  SkillLockEntry,
};
use crate::utils::file::FileHelper;
use crate::utils::folder::{copy_dir_all_sync, FolderHelper};
use crate::utils::github::GithubHelper;
use crate::utils::path::{
  canonical_skill_folder_by_name, canonical_skills_root, expand_home, remove_path_any,
};
use crate::utils::time::now_millis;
use crate::utils::zip::ZipHelper;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const SKILL_MD_FILE_NAME: &str = "SKILL.md";

pub fn detect_folder(folder_path: String) -> Result<Vec<DetectedSkill>, String> {
  let folder = Path::new(&folder_path);
  if !folder.exists() || !folder.is_dir() {
    return Err(format!("Folder does not exist: {}", folder_path));
  }

  if folder.join(SKILL_MD_FILE_NAME).exists() {
    return Ok(vec![detect_folder_exact(folder)?]);
  }

  let skill_dirs = FolderHelper::find_dirs_containing_file(folder, SKILL_MD_FILE_NAME)?;
  if skill_dirs.is_empty() {
    return Err(format!("SKILL.md not found in folder: {}", folder_path));
  }

  let mut result = Vec::new();
  for dir in skill_dirs {
    if let Ok(mut detected) = detect_folder_exact(&dir) {
      let relative_dir = dir
        .strip_prefix(folder)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
      detected.skill_path = skill_path_from_relative_dir(&relative_dir);
      result.push(detected);
    }
  }

  if result.is_empty() {
    Err(format!(
      "No valid SKILL.md found after parsing in folder: {}",
      folder_path
    ))
  } else {
    Ok(result)
  }
}

pub fn detect_zip(zip_path: String) -> Result<Vec<DetectedSkill>, String> {
  let temp_extract_dir = create_temp_dir("detect-zip")?;
  ZipHelper::extract_to_dir(&zip_path, &temp_extract_dir)?;
  detect_folder(temp_extract_dir.to_string_lossy().to_string())
}

pub async fn detect_github_manual(github_path: String) -> Result<Vec<DetectedSkill>, String> {
  let (owner, repo) = GithubHelper::parse_github_url(&github_path)?;
  let clone_dir = create_temp_dir(&format!("detect-github-manual-{}-{}", owner, repo))?;
  GithubHelper::clone_repo_to(&owner, &repo, &clone_dir).await?;

  let skill_dirs = FolderHelper::find_dirs_containing_file(&clone_dir, SKILL_MD_FILE_NAME)?;
  if skill_dirs.is_empty() {
    return Err("No SKILL.md found in repository".to_string());
  }

  let mut result = Vec::new();
  for dir in skill_dirs {
    match detect_folder_exact(&dir) {
      Ok(mut detected) => {
        let relative_dir = dir
          .strip_prefix(&clone_dir)
          .map(|p| p.to_string_lossy().to_string())
          .unwrap_or_default();
        detected.skill_path = skill_path_from_relative_dir(&relative_dir);
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

pub async fn detect_github_auto(
  github_path: String,
  skill_name: String,
) -> Result<DetectedSkill, String> {
  let (owner, repo) = GithubHelper::parse_github_url(&github_path)?;
  let clone_dir = create_temp_dir(&format!("detect-github-auto-{}-{}", owner, repo))?;
  GithubHelper::clone_repo_to(&owner, &repo, &clone_dir).await?;

  let skill_dirs = FolderHelper::find_dirs_containing_file(&clone_dir, SKILL_MD_FILE_NAME)?;
  if skill_dirs.is_empty() {
    return Err("No SKILL.md found in repository".to_string());
  }

  for dir in skill_dirs {
    let skill_md = dir.join(SKILL_MD_FILE_NAME);
    let Ok(frontmatter) = FileHelper::read_skill_frontmatter(&skill_md) else {
      continue;
    };
    let Some(name) = frontmatter.name else {
      continue;
    };
    if name != skill_name {
      continue;
    }

    let mut detected = detect_folder_exact(&dir)?;
    let relative_dir = dir
      .strip_prefix(&clone_dir)
      .map(|p| p.to_string_lossy().to_string())
      .unwrap_or_default();
    detected.skill_path = skill_path_from_relative_dir(&relative_dir);
    return Ok(detected);
  }

  Err(format!("No skill matched '{}'", skill_name))
}

fn detect_folder_exact(folder: &Path) -> Result<DetectedSkill, String> {
  let skill_md = folder.join(SKILL_MD_FILE_NAME);
  if !skill_md.exists() {
    return Err(format!(
      "SKILL.md not found in folder: {}",
      folder.to_string_lossy()
    ));
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
    skill_path: SKILL_MD_FILE_NAME.to_string(),
  })
}

pub fn list_skills(scope: InstallScope, project_path: Option<String>) -> Result<Vec<LocalSkill>, String> {
  let install_target = InstallTarget::from_scope(&scope, &project_path)?;
  match &install_target {
    InstallTarget::Global => list_skills_global(),
    InstallTarget::Project(project_root) => list_skills_project(project_root.as_path()),
  }
}

fn list_skills_global() -> Result<Vec<LocalSkill>, String> {
  let github_lock = read_skill_lock_internal().unwrap_or_default();
  let native_lock = read_native_skill_lock_internal().unwrap_or_default();

  let mut skills: Vec<LocalSkill> = Vec::new();
  let mut skill_index: HashMap<String, usize> = HashMap::new();

  let global_root = canonical_skills_root()?;
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

      let source_type = lock_source_type(&skill_name, &github_lock, &native_lock);
      let source = lock_source(&skill_name, &source_type, &github_lock);
      let index = skills.len();
      skills.push(LocalSkill {
        name: skill_name.clone(),
        source,
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
    // If the path doesn't exist, skip reading (no skills installed yet)
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

      let source_type = lock_source_type(&skill_name, &github_lock, &native_lock);
      let source = lock_source(&skill_name, &source_type, &github_lock);
      let index = skills.len();
      skills.push(LocalSkill {
        name: skill_name.clone(),
        source,
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

fn list_skills_project(project_root: &Path) -> Result<Vec<LocalSkill>, String> {
  let project_lock = crate::services::project_skill_lock_service::read_project_skill_lock_internal(project_root)
    .unwrap_or_default();

  let mut skills: Vec<LocalSkill> = Vec::new();
  let mut skill_index: HashMap<String, usize> = HashMap::new();

  let canonical_root = project_root.join(".agents").join("skills");
  if canonical_root.exists() && canonical_root.is_dir() {
    for entry in fs::read_dir(&canonical_root).map_err(|e| e.to_string())? {
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

      let source_type = project_lock_source_type(&skill_name, &project_lock);
      let source = project_lock_source(&skill_name, &project_lock);
      let index = skills.len();
      skills.push(LocalSkill {
        name: skill_name.clone(),
        source,
        global_folder: Some(skill_dir.to_string_lossy().to_string()),
        installed_agent_apps: Vec::new(),
        source_type,
      });
      skill_index.insert(skill_name, index);
    }
  }

  for app in local_agent_apps() {
    let Some(project_path) = &app.project_path else {
      continue;
    };
    let app_root = project_root.join(project_path);
    if !app_root.exists() || !app_root.is_dir() {
      continue;
    }

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

      if let Some(index) = skill_index.get(&skill_name).copied() {
        if !skills[index].installed_agent_apps.iter().any(|item| {
          item.id == installed_app.id && item.skill_folder == installed_app.skill_folder
        }) {
          skills[index].installed_agent_apps.push(installed_app);
        }
        continue;
      }

      let source_type = project_lock_source_type(&skill_name, &project_lock);
      let source = project_lock_source(&skill_name, &project_lock);
      let index = skills.len();
      skills.push(LocalSkill {
        name: skill_name.clone(),
        source,
        global_folder: None,
        installed_agent_apps: vec![installed_app],
        source_type,
      });
      skill_index.insert(skill_name, index);
    }
  }

  Ok(skills)
}

pub fn install_from_native(request: InstallNativeRequest) -> Result<InstallResult, String> {
  validate_native_install_request(&request)?;
  let install_target = InstallTarget::from_scope(&request.scope, &request.project_path)?;
  let selected_apps = resolve_selected_apps_paths(&request.agent_apps, &install_target)?;
  remove_existing_associations(&request.name, &install_target)?;
  install_skill_to_apps(
    Path::new(&request.tmp_path),
    &request.name,
    &request.method,
    &selected_apps,
    &install_target,
  )?;

  match &install_target {
    InstallTarget::Global => sync_lock_for_native_install(&request.name)?,
    InstallTarget::Project(_) => {
      sync_project_lock_for_install(&request.name, "native", SourceType::Native, &install_target)?
    },
  }

  Ok(InstallResult {
    success: true,
    stdout: format!("Skill '{}' installed from native source", request.name),
    stderr: String::new(),
    message: "安装成功".to_string(),
  })
}

pub async fn install_from_github(request: InstallGithubRequest) -> Result<InstallResult, String> {
  validate_github_install_request(&request)?;
  let install_target = InstallTarget::from_scope(&request.scope, &request.project_path)?;
  let selected_apps = resolve_selected_apps_paths(&request.agent_apps, &install_target)?;
  remove_existing_associations(&request.name, &install_target)?;
  install_skill_to_apps(
    Path::new(&request.tmp_path),
    &request.name,
    &request.method,
    &selected_apps,
    &install_target,
  )?;

  let skill_folder_hash = match normalize_optional_string(request.skill_folder_hash.clone()) {
    Some(hash) => hash,
    None => GithubHelper::get_skill_folder_hash(&request.source_url, &request.skill_path).await?,
  };

  let source = GithubHelper::parse_github_url(&request.source_url)
    .map(|(owner, repo)| format!("{}/{}", owner, repo))
    .unwrap_or_else(|_| request.source_url.clone());

  let lock_entry = SkillLockEntry {
    source: source.clone(),
    source_type: "github".to_string(),
    source_url: request.source_url.clone(),
    skill_path: Some(request.skill_path.clone()),
    skill_folder_hash: Some(skill_folder_hash),
    installed_at: String::new(),
    updated_at: String::new(),
  };

  match &install_target {
    InstallTarget::Global => sync_lock_for_github_install(request.name.clone(), lock_entry)?,
    InstallTarget::Project(_) => sync_project_lock_for_install(
      &request.name,
      &source,
      SourceType::Github,
      &install_target,
    )?,
  }

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

pub fn install_from_unknown(request: InstallUnknownRequest) -> Result<InstallResult, String> {
  validate_install_from_unknown_request(&request)?;
  let install_target = InstallTarget::from_scope(&request.scope, &request.project_path)?;

  let source = Path::new(&request.source_path);
  let source_path = if matches!(install_target, InstallTarget::Global) {
    prepare_canonical_skill_dir(source, &request.name, &install_target)?
  } else {
    source.to_path_buf()
  };

  install_from_native(InstallNativeRequest {
    name: request.name,
    tmp_path: source_path.to_string_lossy().to_string(),
    skill_path: SKILL_MD_FILE_NAME.to_string(),
    agent_apps: request.agent_apps,
    method: request.method,
    scope: request.scope,
    project_path: request.project_path,
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
  let install_target = InstallTarget::from_scope(&request.scope, &request.project_path)?;

  if request.source_type == SourceType::Unknown {
    return Err(
      "Unknown source should use install_from_unknown flow; manage_skill_agent_apps only handles managed skills"
        .to_string(),
    );
  }

  let source_path = PathBuf::from(&request.source_path);
  if request.source_path.trim().is_empty() {
    return Err("source_path is required for manage_skill_agent_apps".to_string());
  }
  if !source_path.exists() || !source_path.is_dir() {
    return Err("source_path does not exist".to_string());
  }

  let canonical_root = canonical_skills_root()?;
  let source = if source_path.starts_with(&canonical_root) {
    source_path
  } else {
    prepare_canonical_skill_dir(&source_path, &request.name, &InstallTarget::Global)?
  };

  let selected_apps = resolve_selected_apps_paths(&request.agent_apps, &install_target)?;
  remove_existing_associations(&request.name, &install_target)?;
  install_skill_to_apps(
    &source,
    &request.name,
    &request.method,
    &selected_apps,
    &install_target,
  )?;

  Ok(InstallResult {
    success: true,
    stdout: format!("Skill '{}' agent apps updated", request.name),
    stderr: String::new(),
    message: "安装成功".to_string(),
  })
}

pub fn open_in_file_manager(file_path: String) -> Result<(), String> {
  crate::utils::file::open_in_file_manager(file_path)
}

pub async fn read_skill_file(skill_path: String) -> Result<String, String> {
  crate::utils::file::read_skill_file(skill_path).await
}

pub fn check_skills_updates(
  checks: Vec<crate::models::SkillUpdateCheckItem>,
) -> Result<Vec<String>, String> {
  if checks.is_empty() {
    return Ok(Vec::new());
  }

  let lock = read_skill_lock_internal()?;
  let mut updated = Vec::new();
  for check in checks {
    let Some(entry) = lock.skills.get(&check.name) else {
      continue;
    };
    if entry.source.trim() != check.source.trim() {
      continue;
    }
    let Some(local_sha) = &entry.skill_folder_hash else {
      continue;
    };
    if local_sha != &check.remote_sha {
      updated.push(check.name);
    }
  }
  Ok(updated)
}

pub fn delete_skill(name: String, scope: InstallScope, project_path: Option<String>) -> Result<(), String> {
  let install_target = InstallTarget::from_scope(&scope, &project_path)?;
  let skill_path = canonical_skill_folder_by_name(&name, &install_target)?;
  if skill_path.exists() || skill_path.is_symlink() {
    remove_path_any(&skill_path)?;
  }

  remove_existing_associations(&name, &install_target)?;
  remove_skill_from_lock(&name, &install_target)?;

  Ok(())
}

fn validate_native_install_request(request: &InstallNativeRequest) -> Result<(), String> {
  validate_common_install_request(
    &request.name,
    &request.tmp_path,
    &request.skill_path,
    &request.agent_apps,
    &request.scope,
    &request.project_path,
  )
}

fn validate_github_install_request(request: &InstallGithubRequest) -> Result<(), String> {
  validate_common_install_request(
    &request.name,
    &request.tmp_path,
    &request.skill_path,
    &request.agent_apps,
    &request.scope,
    &request.project_path,
  )?;
  GithubHelper::parse_github_url(&request.source_url)?;
  Ok(())
}

fn validate_install_from_unknown_request(request: &InstallUnknownRequest) -> Result<(), String> {
  if request.name.trim().is_empty() {
    return Err("Skill name is required".to_string());
  }
  if request.source_path.trim().is_empty() {
    return Err("source_path is required".to_string());
  }
  if request.agent_apps.is_empty() {
    return Err("agent_apps is required".to_string());
  }
  InstallTarget::validate_install_scope(&request.scope, &request.project_path)?;

  let source = Path::new(&request.source_path);
  if !source.exists() || !source.is_dir() {
    return Err(format!(
      "Source path does not exist: {}",
      source.to_string_lossy()
    ));
  }
  let skill_md = source.join(SKILL_MD_FILE_NAME);
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
  scope: &InstallScope,
  project_path: &Option<String>,
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
  InstallTarget::validate_install_scope(scope, project_path)?;

  let source = Path::new(tmp_path);
  if !source.exists() || !source.is_dir() {
    return Err(format!(
      "Detected skill tmp path does not exist: {}",
      source.to_string_lossy()
    ));
  }
  Ok(())
}

fn remove_existing_associations(
  skill_name: &str,
  install_target: &InstallTarget,
) -> Result<(), String> {
  let all_apps = resolve_all_available_apps_paths(install_target)?;

  let mut errors = Vec::new();
  for app in &all_apps {
    let target = app.install_root.join(skill_name);
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
        paths: candidates
          .iter()
          .map(|p| p.to_string_lossy().to_string())
          .collect(),
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
        paths: paths
          .into_iter()
          .map(|p| p.to_string_lossy().to_string())
          .collect(),
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
    let skill_md = path.join(SKILL_MD_FILE_NAME);
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
  install_target: &InstallTarget,
) -> Result<(), String> {
  let mut errors = Vec::new();
  let canonical_skill_dir = prepare_canonical_skill_dir(source, name, install_target)?;

  for app in selected_apps {
    if let Err(err) = fs::create_dir_all(&app.install_root) {
      errors.push(format!("{}: {}", app.display_name, err));
      continue;
    }
    let target = app.install_root.join(name);
    if let Err(err) = install_to_app(&canonical_skill_dir, &target, method) {
      errors.push(format!("{}: {}", app.display_name, err));
    }
  }

  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  Ok(())
}

fn prepare_canonical_skill_dir(
  source: &Path,
  name: &str,
  target: &InstallTarget,
) -> Result<PathBuf, String> {
  let canonical_root = target.skill_folder_path()?;
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

fn skill_path_from_relative_dir(relative_dir: &str) -> String {
  if relative_dir.is_empty() {
    SKILL_MD_FILE_NAME.to_string()
  } else {
    format!("{}/{}", relative_dir, SKILL_MD_FILE_NAME)
  }
}

fn sync_lock_for_native_install(skill_name: &str) -> Result<(), String> {
  let _ = remove_skill_from_global_lock(skill_name.to_string())?;
  add_skill_to_native_lock(skill_name.to_string())
}

fn sync_lock_for_github_install(skill_name: String, entry: SkillLockEntry) -> Result<(), String> {
  let _ = remove_skill_from_native_lock(skill_name.clone())?;
  add_skill_to_lock(skill_name, entry)
}

fn remove_skill_from_lock(skill_name: &str, install_target: &InstallTarget) -> Result<(), String> {
  match install_target {
    InstallTarget::Global => {
      let _ = remove_skill_from_global_lock(skill_name.to_string())?;
      let _ = remove_skill_from_native_lock(skill_name.to_string())?;
    },
    InstallTarget::Project(project_root) => {
      let _ = remove_skill_from_project_lock(project_root, skill_name.to_string())?;
    },
  }
  Ok(())
}

fn sync_project_lock_for_install(
  skill_name: &str,
  source: &str,
  source_type: SourceType,
  install_target: &InstallTarget,
) -> Result<(), String> {
  let skill_name = skill_name.to_string();
  let source = source.to_string();
  let project_root = install_target.root_path()?;
  let skill_dir = install_target.skill_folder_path()?.join(&skill_name);
  add_skill_to_project_lock(
    &project_root,
    skill_name,
    source,
    source_type,
    &skill_dir,
  )?;
  Ok(())
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

fn install_to_app(source: &Path, target: &Path, method: &InstallMethod) -> Result<(), String> {
  if source == target {
    return Ok(());
  }
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

fn is_skill_folder(path: &Path) -> bool {
  (path.is_dir() || path.is_symlink()) && path.join(SKILL_MD_FILE_NAME).exists()
}

fn read_valid_skill_name(skill_dir: &Path) -> Option<String> {
  let skill_md = skill_dir.join(SKILL_MD_FILE_NAME);
  if !skill_md.exists() {
    return None;
  }
  let frontmatter = FileHelper::read_skill_frontmatter(&skill_md).ok()?;
  frontmatter.name
}

fn detect_install_method(skill_dir: &Path) -> InstallMethod {
  if fs::symlink_metadata(skill_dir)
    .map(|m| m.file_type().is_symlink())
    .unwrap_or(false)
  {
    InstallMethod::Symlink
  } else {
    InstallMethod::Copy
  }
}
