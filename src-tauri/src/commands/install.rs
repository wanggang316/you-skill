use crate::models::{DetectedSkill, InstallGithubRequest, InstallRequest, InstallResult, InstallZipRequest};
use std::fs;
use std::path::Path;
use std::process::Command;

#[tauri::command]
pub fn install_skill(request: InstallRequest) -> Result<InstallResult, String> {
  let npx_cmd = if cfg!(target_os = "windows") {
    "npx.cmd"
  } else {
    "npx"
  };

  let mut command = Command::new(npx_cmd);
  command.arg("skills").arg("add").arg(&request.source);
  command.arg("--skill").arg(&request.skill_id);
  command.arg("--agent").arg(&request.agent);
  command.arg("--yes");

  if request.global {
    command.arg("--global");
  }

  if let Some(project_dir) = request.project_dir {
    if !project_dir.trim().is_empty() {
      command.current_dir(project_dir);
    }
  }

  let output = command.output().map_err(|e| e.to_string())?;

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  let success = output.status.success();

  Ok(InstallResult {
    success,
    stdout,
    stderr,
    message: if success {
      "安装成功".to_string()
    } else {
      "安装失败".to_string()
    },
  })
}

/// Detect skills in a GitHub repository by cloning it to a temp directory
/// and finding all directories containing SKILL.md files
#[tauri::command]
pub async fn detect_github_skills(url: String) -> Result<Vec<DetectedSkill>, String> {
  // Parse owner/repo from URL
  let (owner, repo) = parse_github_url(&url)?;

  // Create temp directory
  let temp_dir = std::env::temp_dir().join(format!("skill-kit-clone-{}-{}", owner, repo));

  // Clean up temp directory if it exists
  let _ = fs::remove_dir_all(&temp_dir);

  // Clone repository
  let git_url = format!("https://github.com/{}/{}.git", owner, repo);
  let clone_result = Command::new("git")
    .args(&["clone", "--depth", "1", &git_url, temp_dir.to_str().unwrap()])
    .output();

  match clone_result {
    Ok(output) => {
      if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to clone repository: {}", stderr));
      }
    }
    Err(e) => return Err(format!("Failed to execute git clone: {}", e)),
  }

  // Find all SKILL.md files
  let mut skills = Vec::new();
  find_skill_md_files(&temp_dir, &temp_dir, &mut skills)?;

  // Clean up temp directory
  let _ = fs::remove_dir_all(&temp_dir);

  Ok(skills)
}

fn parse_github_url(url: &str) -> Result<(String, String), String> {
  // Handle various GitHub URL formats:
  // - https://github.com/owner/repo
  // - https://github.com/owner/repo.git
  // - github.com/owner/repo
  // - owner/repo

  let url = url.trim();

  if url.contains("github.com") {
    let parts: Vec<&str> = url.split("github.com/").collect();
    if parts.len() < 2 {
      return Err("Invalid GitHub URL".to_string());
    }
    let path = parts[1];
    let path = path.trim_end_matches(".git");
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2 {
      return Err("Invalid GitHub URL format".to_string());
    }
    return Ok((segments[0].to_string(), segments[1].to_string()));
  }

  // Handle owner/repo format
  let parts: Vec<&str> = url.split('/').collect();
  if parts.len() == 2 {
    return Ok((parts[0].to_string(), parts[1].to_string()));
  }

  Err("Unsupported URL format. Use https://github.com/owner/repo or owner/repo".to_string())
}

fn find_skill_md_files(
  dir: &Path,
  base_dir: &Path,
  skills: &mut Vec<DetectedSkill>,
) -> Result<(), String> {
  let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;

  for entry in entries {
    let entry = entry.map_err(|e| e.to_string())?;
    let path = entry.path();

    if path.is_dir() {
      // Check if this directory contains SKILL.md
      let skill_md = path.join("SKILL.md");
      if skill_md.exists() {
        // Get relative path from base
        let relative_path = path
          .strip_prefix(base_dir)
          .map_err(|e| e.to_string())?
          .to_string_lossy()
          .to_string();

        // Extract skill name from directory name
        let skill_name = path
          .file_name()
          .map(|n| n.to_string_lossy().to_string())
          .unwrap_or_else(|| "Unknown".to_string());

        skills.push(DetectedSkill {
          name: skill_name,
          path: relative_path,
        });
      }

      // Recursively search subdirectories (but not node_modules, .git, etc.)
      let dir_name = path.file_name().map(|n| n.to_string_lossy().to_string());
      if let Some(name) = dir_name {
        if name != "node_modules" && name != ".git" && name != "target" {
          find_skill_md_files(&path, base_dir, skills)?;
        }
      }
    }
  }

  Ok(())
}

/// Install a skill from a ZIP file to .agents/skills/ folder and create symlinks to selected agents
#[tauri::command]
pub fn install_zip_skill(request: InstallZipRequest) -> Result<InstallResult, String> {
  use crate::paths::agent_paths;

  // Get the skill name from the skill_path, or fall back to zip file name
  let skill_name = if request.skill_path.is_empty() {
    let zip_path = Path::new(&request.zip_path);
    zip_path
      .file_stem()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or_else(|| "unnamed-skill".to_string())
  } else {
    Path::new(&request.skill_path)
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_else(|| "unnamed-skill".to_string())
  };

  // Get global skills directory
  let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
  let agents_skills_dir = home_dir.join(".agents/skills");

  // Ensure directory exists
  fs::create_dir_all(&agents_skills_dir).map_err(|e| e.to_string())?;

  // Extract ZIP file
  let skill_dir = agents_skills_dir.join(&skill_name);

  // Remove existing directory if it exists
  if skill_dir.exists() {
    fs::remove_dir_all(&skill_dir).map_err(|e| e.to_string())?;
  }

  // Create skill directory
  fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;

  // Extract ZIP
  extract_zip(&request.zip_path, &skill_dir)?;

  // Create symlinks for selected agents
  let agent_paths = agent_paths();
  let mut errors = Vec::new();

  for agent_id in &request.agents {
    if let Some(agent) = agent_paths.iter().find(|a| a.id == *agent_id) {
      if let Some(global_path) = agent.global_path {
        let expanded_path = expand_tilde(global_path)?;
        let agent_skill_dir = Path::new(&expanded_path);

        // Ensure parent directory exists
        if let Some(parent) = agent_skill_dir.parent() {
          fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Create symlink
        let symlink_path = agent_skill_dir.join(&skill_name);

        // Remove existing symlink or directory
        if symlink_path.exists() || symlink_path.is_symlink() {
          if symlink_path.is_dir() && !symlink_path.is_symlink() {
            fs::remove_dir_all(&symlink_path).map_err(|e| e.to_string())?;
          } else {
            fs::remove_file(&symlink_path).map_err(|e| e.to_string())?;
          }
        }

        #[cfg(unix)]
        {
          use std::os::unix::fs::symlink;
          if let Err(e) = symlink(&skill_dir, &symlink_path) {
            errors.push(format!("{}: {}", agent.display_name, e));
          }
        }

        #[cfg(windows)]
        {
          use std::os::windows::fs::symlink_dir;
          if let Err(e) = symlink_dir(&skill_dir, &symlink_path) {
            // On Windows, if symlink fails, copy the directory instead
            if let Err(copy_err) = copy_dir_all(&skill_dir, &symlink_path) {
              errors.push(format!("{}: symlink failed ({}), copy failed ({})", agent.display_name, e, copy_err));
            }
          }
        }
      }
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
      stdout: format!("Skill '{}' installed with some warnings", skill_name),
      stderr: errors.join("\n"),
      message: "安装完成，但部分 Agent 链接失败".to_string(),
    })
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

fn expand_tilde(path: &str) -> Result<String, String> {
  if path.starts_with("~/") {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(path.replace("~", home.to_string_lossy().as_ref()))
  } else {
    Ok(path.to_string())
  }
}

#[cfg(windows)]
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
  fs::create_dir_all(dst).map_err(|e| e.to_string())?;

  for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());

    if src_path.is_dir() {
      copy_dir_all(&src_path, &dst_path)?;
    } else {
      fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
    }
  }

  Ok(())
}

/// Install a skill from GitHub repository
#[tauri::command]
pub async fn install_github_skill(request: InstallGithubRequest) -> Result<InstallResult, String> {
  use crate::paths::agent_paths;

  // Parse owner/repo from URL
  let (owner, repo) = parse_github_url(&request.url)?;

  // Create temp directory
  let temp_dir = std::env::temp_dir().join(format!("skill-kit-clone-{}-{}", owner, repo));

  // Clean up temp directory if it exists
  let _ = fs::remove_dir_all(&temp_dir);

  // Clone repository
  let git_url = format!("https://github.com/{}/{}.git", owner, repo);
  let clone_result = Command::new("git")
    .args(&["clone", "--depth", "1", &git_url, temp_dir.to_str().unwrap()])
    .output();

  match clone_result {
    Ok(output) => {
      if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to clone repository: {}", stderr));
      }
    }
    Err(e) => return Err(format!("Failed to execute git clone: {}", e)),
  }

  // Get skill name from the skill path
  let skill_path = Path::new(&request.skill_path);
  let skill_name = skill_path
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| "unnamed-skill".to_string());

  // Get source directory
  let source_dir = temp_dir.join(&request.skill_path);

  if !source_dir.exists() {
    let _ = fs::remove_dir_all(&temp_dir);
    return Err(format!("Skill path not found: {}", request.skill_path));
  }

  // Get global skills directory
  let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
  let agents_skills_dir = home_dir.join(".agents/skills");

  // Ensure directory exists
  fs::create_dir_all(&agents_skills_dir).map_err(|e| e.to_string())?;

  // Copy skill to .agents/skills/
  let target_dir = agents_skills_dir.join(&skill_name);

  // Remove existing directory if it exists
  if target_dir.exists() {
    fs::remove_dir_all(&target_dir).map_err(|e| e.to_string())?;
  }

  // Copy directory
  copy_dir_all_sync(&source_dir, &target_dir)?;

  // Create symlinks for selected agents
  let agent_paths = agent_paths();
  let mut errors = Vec::new();

  for agent_id in &request.agents {
    if let Some(agent) = agent_paths.iter().find(|a| a.id == *agent_id) {
      if let Some(global_path) = agent.global_path {
        let expanded_path = expand_tilde(global_path)?;
        let agent_skill_dir = Path::new(&expanded_path);

        // Ensure parent directory exists
        if let Some(parent) = agent_skill_dir.parent() {
          fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Create symlink
        let symlink_path = agent_skill_dir.join(&skill_name);

        // Remove existing symlink or directory
        if symlink_path.exists() || symlink_path.is_symlink() {
          if symlink_path.is_dir() && !symlink_path.is_symlink() {
            fs::remove_dir_all(&symlink_path).map_err(|e| e.to_string())?;
          } else {
            fs::remove_file(&symlink_path).map_err(|e| e.to_string())?;
          }
        }

        #[cfg(unix)]
        {
          use std::os::unix::fs::symlink;
          if let Err(e) = symlink(&target_dir, &symlink_path) {
            errors.push(format!("{}: {}", agent.display_name, e));
          }
        }

        #[cfg(windows)]
        {
          use std::os::windows::fs::symlink_dir;
          if let Err(e) = symlink_dir(&target_dir, &symlink_path) {
            // On Windows, if symlink fails, copy the directory instead
            if let Err(copy_err) = copy_dir_all_sync(&target_dir, &symlink_path) {
              errors.push(format!("{}: symlink failed ({}), copy failed ({})", agent.display_name, e, copy_err));
            }
          }
        }
      }
    }
  }

  // Clean up temp directory
  let _ = fs::remove_dir_all(&temp_dir);

  if errors.is_empty() {
    Ok(InstallResult {
      success: true,
      stdout: format!("Skill '{}' installed successfully from GitHub", skill_name),
      stderr: String::new(),
      message: "安装成功".to_string(),
    })
  } else {
    Ok(InstallResult {
      success: true,
      stdout: format!("Skill '{}' installed with some warnings", skill_name),
      stderr: errors.join("\n"),
      message: "安装完成，但部分 Agent 链接失败".to_string(),
    })
  }
}

/// Detect skills in a ZIP file by extracting to temp and finding SKILL.md files
#[tauri::command]
pub fn detect_zip_skills(zip_path: String) -> Result<Vec<DetectedSkill>, String> {
  // Create temp directory for extraction
  let temp_dir = std::env::temp_dir().join(format!("skill-kit-zip-detect-{}", std::process::id()));

  // Clean up temp directory if it exists
  let _ = fs::remove_dir_all(&temp_dir);

  // Create temp directory
  fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

  // Extract ZIP file
  let extract_result = extract_zip(&zip_path, &temp_dir);

  if let Err(e) = extract_result {
    let _ = fs::remove_dir_all(&temp_dir);
    return Err(e);
  }

  // Find all SKILL.md files
  let mut skills = Vec::new();
  let find_result = find_skill_md_files(&temp_dir, &temp_dir, &mut skills);

  // Clean up temp directory
  let _ = fs::remove_dir_all(&temp_dir);

  find_result?;

  Ok(skills)
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
