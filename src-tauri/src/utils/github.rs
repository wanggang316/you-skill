use crate::models::SkillDirectoryEntry;
use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use reqwest::Client;
use serde::Deserialize;
use zip::ZipArchive;

pub struct GithubHelper;

impl GithubHelper {
  pub fn parse_github_url(url: &str) -> Result<(String, String), String> {
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

  /// Download and extract a GitHub repository as a ZIP archive.
  /// This approach doesn't require git to be installed on the system.
  pub async fn clone_repo_to(owner: &str, repo: &str, dest: &Path) -> Result<(), String> {
    // Try branches in order: main -> master
    let branches = ["main", "master"];
    let mut last_error = String::new();

    for branch in branches {
      match Self::download_and_extract(owner, repo, branch, dest).await {
        Ok(()) => return Ok(()),
        Err(e) => {
          last_error = e;
          // Clean up destination if it was partially created
          let _ = std::fs::remove_dir_all(dest);
        }
      }
    }

    Err(format!(
      "Failed to download repository {}/{}: {}",
      owner, repo, last_error
    ))
  }

  async fn download_and_extract(
    owner: &str,
    repo: &str,
    branch: &str,
    dest: &Path,
  ) -> Result<(), String> {
    let url = format!(
      "https://github.com/{}/{}/archive/refs/heads/{}.zip",
      owner, repo, branch
    );

    let client = Client::builder()
      .timeout(std::time::Duration::from_secs(60))
      .build()
      .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
      .get(&url)
      .header("User-Agent", "you-skill")
      .send()
      .await
      .map_err(|e| format!("Failed to download repository: {}", e))?;

    if !response.status().is_success() {
      return Err(format!("HTTP error: {}", response.status()));
    }

    let bytes = response
      .bytes()
      .await
      .map_err(|e| format!("Failed to read response: {}", e))?;

    // Extract ZIP synchronously (this is CPU-bound, not I/O bound)
    Self::extract_zip(&bytes, dest)?;

    Ok(())
  }

  fn extract_zip(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)
      .map_err(|e| format!("Failed to parse ZIP archive: {}", e))?;

    // Create destination directory
    std::fs::create_dir_all(dest)
      .map_err(|e| format!("Failed to create directory: {}", e))?;

    // Extract files, stripping the root folder (e.g., repo-main/)
    for i in 0..archive.len() {
      let mut file = archive
        .by_index(i)
        .map_err(|e| format!("Failed to read ZIP entry: {}", e))?;

      let path = file
        .enclosed_name()
        .ok_or_else(|| "Invalid ZIP entry path".to_string())?;

      // Strip the root folder (first component)
      let stripped_path = path
        .components()
        .skip(1)
        .collect::<std::path::PathBuf>();

      if stripped_path.components().count() == 0 {
        continue;
      }

      let out_path = dest.join(&stripped_path);

      if file.is_dir() {
        std::fs::create_dir_all(&out_path)
          .map_err(|e| format!("Failed to create directory: {}", e))?;
      } else {
        if let Some(parent) = out_path.parent() {
          std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }

        let mut out_file = File::create(&out_path)
          .map_err(|e| format!("Failed to create file: {}", e))?;

        io::copy(&mut file, &mut out_file)
          .map_err(|e| format!("Failed to write file: {}", e))?;
      }
    }

    Ok(())
  }

  pub async fn get_skill_folder_hash(source_url: &str, skill_path: &str) -> Result<String, String> {
    let (owner, repo) = Self::parse_github_url(source_url)?;
    let url = format!(
      "https://api.github.com/repos/{}/{}/git/trees/main?recursive=1",
      owner, repo
    );

    let client = Client::new();
    let resp = client
      .get(url)
      .header("User-Agent", "skill-kit")
      .send()
      .await
      .map_err(|e| format!("Failed to request GitHub tree: {}", e))?;

    if !resp.status().is_success() {
      return Err(format!("GitHub API returned status {}", resp.status()));
    }

    let tree: GitTreeResponse = resp
      .json()
      .await
      .map_err(|e| format!("Failed to parse GitHub tree response: {}", e))?;

    let normalized = skill_path.trim_start_matches("./");
    let skill_folder = if normalized == "SKILL.md" {
      String::new()
    } else if normalized.ends_with("/SKILL.md") {
      normalized.trim_end_matches("/SKILL.md").to_string()
    } else {
      return Err(format!("Invalid skill_path: {}", skill_path));
    };

    if skill_folder.is_empty() {
      return Ok(tree.sha);
    }

    let entry = tree
      .tree
      .into_iter()
      .find(|item| item.kind == "tree" && item.path == skill_folder)
      .ok_or(format!("Skill folder not found in GitHub tree: {}", skill_folder))?;

    Ok(entry.sha)
  }

  pub fn list_skill_directory(skill_path: &str) -> Result<Vec<SkillDirectoryEntry>, String> {
    let root = PathBuf::from(skill_path);
    if !root.exists() || !root.is_dir() {
      return Err(format!("Skill directory does not exist: {}", skill_path));
    }

    let mut out = Vec::new();
    collect_skill_directory_entries(&root, &root, &mut out)?;
    Ok(out)
  }

  pub async fn read_skill_relative_file(
    skill_path: &str,
    relative_path: &str,
  ) -> Result<String, String> {
    let root = PathBuf::from(skill_path);
    if !root.exists() || !root.is_dir() {
      return Err(format!("Skill directory does not exist: {}", skill_path));
    }

    let relative = sanitize_relative_path(relative_path)?;
    let file_path = root.join(&relative);
    if !file_path.exists() || !file_path.is_file() {
      return Err(format!("Skill file does not exist: {}", relative_path));
    }

    tokio::fs::read_to_string(&file_path)
      .await
      .map_err(|e| format!("Failed to read file '{}': {}", relative, e))
  }

  pub async fn read_skill_relative_file_bytes(
    skill_path: &str,
    relative_path: &str,
  ) -> Result<Vec<u8>, String> {
    let root = PathBuf::from(skill_path);
    if !root.exists() || !root.is_dir() {
      return Err(format!("Skill directory does not exist: {}", skill_path));
    }

    let relative = sanitize_relative_path(relative_path)?;
    let file_path = root.join(&relative);
    if !file_path.exists() || !file_path.is_file() {
      return Err(format!("Skill file does not exist: {}", relative_path));
    }

    tokio::fs::read(&file_path)
      .await
      .map_err(|e| format!("Failed to read file bytes '{}': {}", relative, e))
  }
}

fn sanitize_relative_path(path: &str) -> Result<String, String> {
  let trimmed = path.trim();
  if trimmed.is_empty() {
    return Err("relative_path is required".to_string());
  }

  let parsed = Path::new(trimmed);
  let mut components = Vec::new();
  for component in parsed.components() {
    match component {
      std::path::Component::Normal(part) => {
        components.push(part.to_string_lossy().to_string());
      },
      _ => return Err(format!("Invalid relative path: {}", path)),
    }
  }

  if components.is_empty() {
    return Err(format!("Invalid relative path: {}", path));
  }

  Ok(components.join("/"))
}

fn collect_skill_directory_entries(
  root: &Path,
  current: &Path,
  out: &mut Vec<SkillDirectoryEntry>,
) -> Result<(), String> {
  let mut entries = Vec::new();
  for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
    entries.push(entry.map_err(|e| e.to_string())?);
  }

  entries.sort_by(|a, b| {
    a.file_name()
      .to_string_lossy()
      .to_lowercase()
      .cmp(&b.file_name().to_string_lossy().to_lowercase())
  });

  for entry in entries {
    let path = entry.path();
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    let relative = path
      .strip_prefix(root)
      .map_err(|e| e.to_string())?
      .to_string_lossy()
      .replace('\\', "/");

    if metadata.is_dir() {
      out.push(SkillDirectoryEntry {
        path: relative.clone(),
        is_directory: true,
      });
      collect_skill_directory_entries(root, &path, out)?;
      continue;
    }

    if metadata.is_file() {
      out.push(SkillDirectoryEntry {
        path: relative,
        is_directory: false,
      });
    }
  }

  Ok(())
}

#[derive(Debug, Deserialize)]
struct GitTreeResponse {
  sha: String,
  tree: Vec<GitTreeItem>,
}

#[derive(Debug, Deserialize)]
struct GitTreeItem {
  path: String,
  #[serde(rename = "type")]
  kind: String,
  sha: String,
}
