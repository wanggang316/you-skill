use serde_yaml::Value;
use std::fs;
use std::path::Path;

pub struct FileHelper;

impl FileHelper {
  pub fn read_to_string(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
  }

  pub fn metadata(path: &Path) -> Result<fs::Metadata, String> {
    fs::metadata(path).map_err(|e| e.to_string())
  }

  pub fn read_skill_name_from_frontmatter(skill_md_path: &Path) -> Result<String, String> {
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
      .filter(|s| !s.is_empty())
      .ok_or("SKILL.md frontmatter missing valid 'name'".to_string())?;

    Ok(name)
  }
}
