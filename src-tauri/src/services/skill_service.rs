use crate::models::{DetectedSkill, InstallMethod, InstallRequest, InstallResult};
use crate::services::agent_apps_service::{expand_tilde, local_agent_apps};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn detect_zip(zip_path: String) -> Result<DetectedSkill, String> {
  let temp_root = create_temp_dir("zip-detect")?;
  extract_zip(&zip_path, &temp_root)?;

  let mut skills = Vec::new();
  find_skill_md_dirs(&temp_root, &temp_root, &mut skills)?;
  let mut selected = select_single_skill(skills, "ZIP")?;
  selected.temp_skill_path = temp_root.join(&selected.path).to_string_lossy().to_string();
  Ok(selected)
}

pub fn detect_folder(folder_path: String) -> Result<DetectedSkill, String> {
  let base = Path::new(&folder_path);
  if !base.exists() || !base.is_dir() {
    return Err("Folder does not exist or is not a directory".to_string());
  }

  let temp_root = create_temp_dir("folder-detect")?;
  let source = if base.join("SKILL.md").exists() {
    base.to_path_buf()
  } else {
    let mut skills = Vec::new();
    find_skill_md_dirs(base, base, &mut skills)?;
    let selected = select_single_skill(skills, "folder")?;
    base.join(selected.path)
  };

  let skill_name = source
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| "skill".to_string());
  let target = temp_root.join(&skill_name);
  copy_dir_all_sync(&source, &target)?;

  Ok(DetectedSkill {
    name: skill_name,
    path: String::new(),
    temp_skill_path: target.to_string_lossy().to_string(),
  })
}

pub fn detect_github_manual(github_path: String) -> Result<Vec<DetectedSkill>, String> {
  let (owner, repo) = parse_github_url(&github_path)?;
  let temp_root = create_temp_dir(&format!("github-{}-{}", owner, repo))?;
  clone_repo(&owner, &repo, &temp_root)?;

  let mut skills = Vec::new();
  find_skill_md_dirs(&temp_root, &temp_root, &mut skills)?;
  if skills.is_empty() {
    return Err("No skills found in GitHub repository".to_string());
  }

  let detected = skills
    .into_iter()
    .map(|skill| DetectedSkill {
      temp_skill_path: temp_root.join(&skill.path).to_string_lossy().to_string(),
      ..skill
    })
    .collect();
  Ok(detected)
}

pub fn detect_github_auto(github_path: String) -> Result<DetectedSkill, String> {
  let skills = detect_github_manual(github_path)?;
  select_single_skill(skills, "GitHub repository")
}

pub fn install(request: InstallRequest) -> Result<InstallResult, String> {
  let source = Path::new(&request.detected_skill.temp_skill_path);
  if !source.exists() || !source.is_dir() {
    return Err(format!(
      "Detected skill path does not exist: {}",
      request.detected_skill.temp_skill_path
    ));
  }

  let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
  let canonical_root = home_dir.join(".agents/skills");
  fs::create_dir_all(&canonical_root).map_err(|e| e.to_string())?;

  let skill_name = request.detected_skill.name.clone();
  let canonical_skill_dir = canonical_root.join(&skill_name);
  if canonical_skill_dir.exists() {
    fs::remove_dir_all(&canonical_skill_dir).map_err(|e| e.to_string())?;
  }
  copy_dir_all_sync(source, &canonical_skill_dir)?;

  let mut errors = Vec::new();
  let apps = local_agent_apps();
  for app_id in &request.agent_apps {
    let Some(app) = apps.iter().find(|a| a.id == *app_id) else {
      errors.push(format!("Unknown app id: {}", app_id));
      continue;
    };
    let Some(global_path) = &app.global_path else {
      errors.push(format!("{} has no global path", app.display_name));
      continue;
    };

    let app_dir = expand_tilde(global_path);
    if let Err(e) = fs::create_dir_all(&app_dir) {
      errors.push(format!("{}: {}", app.display_name, e));
      continue;
    }

    let target = app_dir.join(&skill_name);
    if (target.exists() || target.is_symlink()) && remove_path_any(&target).is_err() {
      errors.push(format!("{}: failed to clean existing target", app.display_name));
      continue;
    }

    if let Err(e) = install_to_app(&canonical_skill_dir, &target, &request.method) {
      errors.push(format!("{}: {}", app.display_name, e));
    }
  }

  if errors.is_empty() {
    Ok(InstallResult {
      success: true,
      stdout: format!("Skill '{}' installed successfully", skill_name),
      stderr: String::new(),
      message: "安装成功".to_string(),
    })
  } else {
    Ok(InstallResult {
      success: true,
      stdout: format!("Skill '{}' installed with warnings", skill_name),
      stderr: errors.join("\n"),
      message: "安装完成，但部分 Agent 安装失败".to_string(),
    })
  }
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

fn create_temp_dir(prefix: &str) -> Result<PathBuf, String> {
  let dir = std::env::temp_dir().join(format!(
    "skill-kit-{}-{}-{}",
    prefix,
    std::process::id(),
    chrono_like_now_millis()
  ));
  fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  Ok(dir)
}

fn chrono_like_now_millis() -> u128 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0)
}

fn parse_github_url(url: &str) -> Result<(String, String), String> {
  let url = url.trim();
  if url.contains("github.com") {
    let parts: Vec<&str> = url.split("github.com/").collect();
    if parts.len() < 2 {
      return Err("Invalid GitHub URL".to_string());
    }
    let path = parts[1].trim_end_matches(".git");
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2 {
      return Err("Invalid GitHub URL format".to_string());
    }
    return Ok((segments[0].to_string(), segments[1].to_string()));
  }
  let parts: Vec<&str> = url.split('/').collect();
  if parts.len() == 2 {
    return Ok((parts[0].to_string(), parts[1].to_string()));
  }
  Err("Unsupported URL format. Use https://github.com/owner/repo or owner/repo".to_string())
}

fn clone_repo(owner: &str, repo: &str, dest: &Path) -> Result<(), String> {
  let git_url = format!("https://github.com/{}/{}.git", owner, repo);
  let out = Command::new("git")
    .args(["clone", "--depth", "1", &git_url, dest.to_str().unwrap_or_default()])
    .output()
    .map_err(|e| format!("Failed to execute git clone: {}", e))?;
  if !out.status.success() {
    return Err(format!(
      "Failed to clone repository: {}",
      String::from_utf8_lossy(&out.stderr)
    ));
  }
  Ok(())
}

fn find_skill_md_dirs(
  dir: &Path,
  base_dir: &Path,
  skills: &mut Vec<DetectedSkill>,
) -> Result<(), String> {
  for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let path = entry.path();
    if path.is_dir() {
      if path.join("SKILL.md").exists() {
        let rel = path
          .strip_prefix(base_dir)
          .map_err(|e| e.to_string())?
          .to_string_lossy()
          .to_string();
        let name = path
          .file_name()
          .map(|n| n.to_string_lossy().to_string())
          .unwrap_or_else(|| "Unknown".to_string());
        skills.push(DetectedSkill {
          name,
          path: rel.clone(),
          temp_skill_path: base_dir.join(rel).to_string_lossy().to_string(),
        });
      }
      let dir_name = path.file_name().map(|n| n.to_string_lossy().to_string());
      if let Some(name) = dir_name {
        if name != "node_modules" && name != ".git" && name != "target" {
          find_skill_md_dirs(&path, base_dir, skills)?;
        }
      }
    }
  }
  Ok(())
}

fn select_single_skill(mut skills: Vec<DetectedSkill>, source: &str) -> Result<DetectedSkill, String> {
  if skills.is_empty() {
    return Err(format!("No skills found in {}", source));
  }
  skills.sort_by(|a, b| a.path.cmp(&b.path));
  Ok(skills.remove(0))
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

fn remove_path_any(path: &Path) -> Result<(), String> {
  if path.is_dir() && !path.is_symlink() {
    fs::remove_dir_all(path).map_err(|e| e.to_string())
  } else {
    fs::remove_file(path).map_err(|e| e.to_string())
  }
}

fn extract_zip(zip_path: &str, dest_dir: &Path) -> Result<(), String> {
  let file = fs::File::open(zip_path).map_err(|e| format!("Failed to open ZIP: {}", e))?;
  let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP: {}", e))?;
  for i in 0..archive.len() {
    let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
    let outpath = dest_dir.join(file.name());
    if file.name().ends_with('/') {
      fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
    } else {
      if let Some(p) = outpath.parent() {
        if !p.exists() {
          fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
      }
      let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
      std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
    }
  }
  Ok(())
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
