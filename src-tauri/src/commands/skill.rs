use std::path::Path;
use std::process::Command;

/// Open a file in the system's file manager (selecting the file)
#[tauri::command]
pub fn open_in_file_manager(file_path: String) -> Result<(), String> {
  let path = Path::new(&file_path);

  if !path.exists() {
    return Err(format!("Path does not exist: {}", file_path));
  }

  #[cfg(target_os = "macos")]
  {
    // Use `open -R` to reveal the file in Finder
    Command::new("open")
      .arg("-R")
      .arg(&file_path)
      .spawn()
      .map_err(|e| format!("Failed to open file manager: {}", e))?;
    Ok(())
  }

  #[cfg(target_os = "linux")]
  {
    // Try common file managers for Linux
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

    // Fallback to xdg-open if no file manager worked
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
    // Use explorer /select to open in Explorer
    Command::new("explorer")
      .arg("/select,")
      .arg(&file_path)
      .spawn()
      .map_err(|e| format!("Failed to open file manager: {}", e))?;
    Ok(())
  }
}

/// Read SKILL.md or README.md from a skill directory
#[tauri::command]
pub async fn read_skill_readme(skill_path: String) -> Result<String, String> {
  let path = Path::new(&skill_path);

  if !path.exists() {
    return Err(format!("Skill directory does not exist: {}", skill_path));
  }

  // Try SKILL.md first
  let skill_md = path.join("SKILL.md");
  if skill_md.exists() {
    match tokio::fs::read_to_string(&skill_md).await {
      Ok(content) => return Ok(content),
      Err(e) => return Err(format!("Failed to read SKILL.md: {}", e)),
    }
  }

  // Try README.md as fallback
  let readme_md = path.join("README.md");
  if readme_md.exists() {
    match tokio::fs::read_to_string(&readme_md).await {
      Ok(content) => return Ok(content),
      Err(e) => return Err(format!("Failed to read README.md: {}", e)),
    }
  }

  Err(format!(
    "No SKILL.md or README.md found in: {}",
    skill_path
  ))
}
