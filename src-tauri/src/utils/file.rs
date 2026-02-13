use crate::utils::time::to_millis;
use serde_yaml::Value;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Default)]
pub struct SkillFrontmatter {
  pub name: Option<String>,
  pub description: Option<String>,
}

pub struct FileHelper;

impl FileHelper {
  pub fn read_to_string(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
  }

  pub fn metadata(path: &Path) -> Result<fs::Metadata, String> {
    fs::metadata(path).map_err(|e| e.to_string())
  }

  pub fn get_created_at(path: &Path) -> Option<i64> {
    let metadata = Self::metadata(path).ok()?;
    let time: SystemTime = metadata.created().or_else(|_| metadata.modified()).ok()?;
    to_millis(time)
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
    let description = yaml
      .get("description")
      .and_then(|v| v.as_str())
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty());

    Ok(SkillFrontmatter { name, description })
  }
}
