use std::path::Path;

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
