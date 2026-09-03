//! Source detection (GitHub / zip / folder -> staged temp directories) and skill file access.

use crate::models::{DetectedSkill, SkillDirectoryEntry};
use crate::utils::folder::{
  copy_dir, create_temp_dir, find_skill_dirs, read_skill_name, CopyOpts, SKILL_MD,
};
use crate::utils::github::GithubHelper;
use crate::utils::zip::ZipHelper;
use std::path::Path;

const DETECT_MAX_DEPTH: usize = 8;

pub fn detect_folder(folder_path: String) -> Result<Vec<DetectedSkill>, String> {
  let folder = Path::new(&folder_path);
  if !folder.is_dir() {
    return Err(format!("Folder does not exist: {}", folder_path));
  }
  let skill_dirs = find_skill_dirs(folder, DETECT_MAX_DEPTH)?;
  if skill_dirs.is_empty() {
    return Err(format!("SKILL.md not found in folder: {}", folder_path));
  }
  let result = stage_skill_dirs(folder, &skill_dirs, None);
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
  let reference = GithubHelper::parse_github_ref(&github_path)?;
  let clone_dir = create_temp_dir(&format!(
    "detect-github-{}-{}",
    reference.owner, reference.repo
  ))?;
  let branch = GithubHelper::clone_repo_to(
    &reference.owner,
    &reference.repo,
    reference.branch.as_deref(),
    &clone_dir,
  )
  .await?;

  let search_root = match &reference.subpath {
    Some(subpath) if clone_dir.join(subpath).is_dir() => clone_dir.join(subpath),
    _ => clone_dir.clone(),
  };
  let skill_dirs = find_skill_dirs(&search_root, DETECT_MAX_DEPTH)?;
  if skill_dirs.is_empty() {
    return Err("No SKILL.md found in repository".to_string());
  }
  let result = stage_skill_dirs(&clone_dir, &skill_dirs, Some(&branch));
  if result.is_empty() {
    return Err("No valid skills found after parsing SKILL.md".to_string());
  }
  Ok(result)
}

pub async fn detect_github_auto(
  github_path: String,
  skill_name: String,
) -> Result<DetectedSkill, String> {
  let reference = GithubHelper::parse_github_ref(&github_path)?;
  let clone_dir = create_temp_dir(&format!(
    "detect-github-{}-{}",
    reference.owner, reference.repo
  ))?;
  let branch = GithubHelper::clone_repo_to(
    &reference.owner,
    &reference.repo,
    reference.branch.as_deref(),
    &clone_dir,
  )
  .await?;

  let skill_dirs = find_skill_dirs(&clone_dir, DETECT_MAX_DEPTH)?;
  if skill_dirs.is_empty() {
    return Err("No SKILL.md found in repository".to_string());
  }

  for dir in skill_dirs {
    if read_skill_name(&dir).as_deref() != Some(skill_name.as_str()) {
      continue;
    }
    return stage_skill_dir(&clone_dir, &dir, Some(&branch));
  }

  Err(format!("No skill matched '{}'", skill_name))
}

fn stage_skill_dirs(
  root: &Path,
  dirs: &[std::path::PathBuf],
  branch: Option<&str>,
) -> Vec<DetectedSkill> {
  dirs
    .iter()
    .filter_map(|dir| stage_skill_dir(root, dir, branch).ok())
    .collect()
}

/// Copy one skill directory into a fresh temp directory named after the skill.
fn stage_skill_dir(root: &Path, dir: &Path, branch: Option<&str>) -> Result<DetectedSkill, String> {
  let name = read_skill_name(dir).ok_or("SKILL.md frontmatter missing valid 'name'")?;
  let tmp_dir = create_temp_dir("staged")?;
  let tmp_path = tmp_dir.join(&name);
  copy_dir(dir, &tmp_path, &CopyOpts::skill())?;

  let relative_dir = dir
    .strip_prefix(root)
    .map(|p| p.to_string_lossy().replace('\\', "/"))
    .unwrap_or_default();
  let skill_path = if relative_dir.is_empty() {
    SKILL_MD.to_string()
  } else {
    format!("{}/{}", relative_dir, SKILL_MD)
  };

  Ok(DetectedSkill {
    name,
    tmp_path: tmp_path.to_string_lossy().to_string(),
    skill_path,
    branch: branch.map(str::to_string),
  })
}

pub fn open_in_file_manager(file_path: String) -> Result<(), String> {
  crate::utils::file::open_in_file_manager(file_path)
}

pub async fn read_skill_file(skill_path: String) -> Result<String, String> {
  crate::utils::file::read_skill_file(skill_path).await
}

pub fn list_skill_directory(skill_path: String) -> Result<Vec<SkillDirectoryEntry>, String> {
  GithubHelper::list_skill_directory(&skill_path)
}

pub async fn read_skill_relative_file(
  skill_path: String,
  relative_path: String,
) -> Result<String, String> {
  GithubHelper::read_skill_relative_file(&skill_path, &relative_path).await
}

pub async fn read_skill_relative_file_bytes(
  skill_path: String,
  relative_path: String,
) -> Result<Vec<u8>, String> {
  GithubHelper::read_skill_relative_file_bytes(&skill_path, &relative_path).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn detect_folder_stages_each_skill_with_relative_skill_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("skills/a")).unwrap();
    fs::write(root.join("skills/a/SKILL.md"), "---\nname: alpha\n---\n").unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    fs::write(root.join("b/SKILL.md"), "---\nname: beta\n---\n").unwrap();

    let mut detected = detect_folder(root.to_string_lossy().to_string()).unwrap();
    detected.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(detected.len(), 2);
    assert_eq!(detected[0].name, "alpha");
    assert_eq!(detected[0].skill_path, "skills/a/SKILL.md");
    assert!(Path::new(&detected[0].tmp_path).join("SKILL.md").is_file());
    assert!(detected[0].tmp_path.ends_with("alpha"));
    assert_eq!(detected[1].skill_path, "b/SKILL.md");

    let single = detect_folder(root.join("b").to_string_lossy().to_string()).unwrap();
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].skill_path, "SKILL.md");
  }
}
