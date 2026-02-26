use serde_yaml::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct SkillFrontmatter {
  pub name: Option<String>,
}

pub struct FileHelper;

impl FileHelper {
  pub fn read_to_string(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
  }

  pub fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| e.to_string())
  }

  pub fn read_skill_frontmatter(skill_md_path: &Path) -> Result<SkillFrontmatter, String> {
    let content = Self::read_to_string(skill_md_path)?;
    let mut lines = content.lines();

    if lines.next().map(|s| s.trim()) != Some("---") {
      return Err("SKILL.md frontmatter not found".to_string());
    }

    let mut frontmatter = String::new();
    let mut closed = false;
    for line in lines.by_ref() {
      if line.trim() == "---" {
        closed = true;
        break;
      }
      frontmatter.push_str(line);
      frontmatter.push('\n');
    }

    if !closed {
      return Err("SKILL.md frontmatter not closed".to_string());
    }

    let yaml: Value = serde_yaml::from_str(&frontmatter)
      .map_err(|e| format!("Failed to parse SKILL.md frontmatter: {}", e))?;
    let name = yaml
      .get("name")
      .and_then(|v| v.as_str())
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty());
    Ok(SkillFrontmatter { name })
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

pub async fn read_skill_file(skill_path: String) -> Result<String, String> {
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

  Err(format!("SKILL.md not found in: {}", skill_path))
}
