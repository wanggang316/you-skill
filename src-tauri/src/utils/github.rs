use crate::models::SkillDirectoryEntry;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const USER_AGENT: &str = "you-skill";
const DEFAULT_BRANCHES: [&str; 2] = ["main", "master"];

/// Parsed GitHub reference: `owner/repo`, optionally with `/tree/<branch>[/<path>]` or
/// `/blob/<branch>/<file>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubRef {
  pub owner: String,
  pub repo: String,
  pub branch: Option<String>,
  pub subpath: Option<String>,
}

pub struct GithubHelper;

impl GithubHelper {
  pub fn parse_github_ref(url: &str) -> Result<GithubRef, String> {
    let url = url.trim();
    let path = if let Some(index) = url.find("github.com") {
      let rest = &url[index + "github.com".len()..];
      rest.trim_start_matches(':').trim_start_matches('/')
    } else {
      url
    };
    let path = path.trim_end_matches('/');
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
      return Err(
        "Unsupported URL format. Use https://github.com/owner/repo or owner/repo".to_string(),
      );
    }
    let owner = segments[0].to_string();
    let repo = segments[1].trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
      return Err("Invalid GitHub URL format".to_string());
    }
    let mut branch = None;
    let mut subpath = None;
    if segments.len() >= 4 && (segments[2] == "tree" || segments[2] == "blob") {
      branch = Some(segments[3].to_string());
      if segments.len() > 4 {
        subpath = Some(segments[4..].join("/"));
      }
    }
    Ok(GithubRef {
      owner,
      repo,
      branch,
      subpath,
    })
  }

  pub fn parse_github_url(url: &str) -> Result<(String, String), String> {
    let parsed = Self::parse_github_ref(url)?;
    Ok((parsed.owner, parsed.repo))
  }

  /// Download and extract a repository archive (no git binary needed). Returns the branch
  /// that was actually downloaded.
  pub async fn clone_repo_to(
    owner: &str,
    repo: &str,
    preferred_branch: Option<&str>,
    dest: &Path,
  ) -> Result<String, String> {
    let mut branches: Vec<String> = Vec::new();
    if let Some(branch) = preferred_branch.map(str::trim).filter(|b| !b.is_empty()) {
      branches.push(branch.to_string());
    }
    for branch in DEFAULT_BRANCHES {
      if !branches.iter().any(|b| b == branch) {
        branches.push(branch.to_string());
      }
    }

    let mut last_error = String::new();
    for branch in &branches {
      match Self::download_and_extract(owner, repo, branch, dest).await {
        Ok(()) => return Ok(branch.clone()),
        Err(e) => {
          last_error = e;
          let _ = fs::remove_dir_all(dest);
        },
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
      .header("User-Agent", USER_AGENT)
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

    Self::extract_zip(&bytes, dest)?;
    Ok(())
  }

  fn extract_zip(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let reader = Cursor::new(bytes);
    let mut archive =
      ZipArchive::new(reader).map_err(|e| format!("Failed to parse ZIP archive: {}", e))?;

    fs::create_dir_all(dest).map_err(|e| format!("Failed to create directory: {}", e))?;

    // Extract files, stripping the root folder (e.g., repo-main/)
    for i in 0..archive.len() {
      let mut file = archive
        .by_index(i)
        .map_err(|e| format!("Failed to read ZIP entry: {}", e))?;

      let path = file
        .enclosed_name()
        .ok_or_else(|| "Invalid ZIP entry path".to_string())?;

      let stripped_path = path.components().skip(1).collect::<PathBuf>();
      if stripped_path.components().count() == 0 {
        continue;
      }

      let out_path = dest.join(&stripped_path);

      if file.is_dir() {
        fs::create_dir_all(&out_path).map_err(|e| format!("Failed to create directory: {}", e))?;
      } else {
        if let Some(parent) = out_path.parent() {
          fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
        let mut out_file =
          File::create(&out_path).map_err(|e| format!("Failed to create file: {}", e))?;
        io::copy(&mut file, &mut out_file).map_err(|e| format!("Failed to write file: {}", e))?;
      }
    }

    Ok(())
  }

  /// Folder part of a `skill_path` (`skills/foo/SKILL.md` -> `skills/foo`, `SKILL.md` -> ``).
  pub fn skill_folder_of(skill_path: &str) -> Result<String, String> {
    let normalized = skill_path.trim().trim_start_matches("./");
    if normalized == "SKILL.md" {
      return Ok(String::new());
    }
    if let Some(folder) = normalized.strip_suffix("/SKILL.md") {
      return Ok(folder.to_string());
    }
    Err(format!("Invalid skill_path: {}", skill_path))
  }

  /// Tree SHAs for several skill folders of one repository/branch in a single request.
  /// Keys of the returned map are the `skill_path` values that were found.
  pub async fn get_tree_shas(
    owner: &str,
    repo: &str,
    branch: &str,
    skill_paths: &[String],
  ) -> Result<HashMap<String, String>, String> {
    let url = format!(
      "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
      owner, repo, branch
    );
    let client = Client::builder()
      .timeout(std::time::Duration::from_secs(30))
      .build()
      .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let resp = client
      .get(url)
      .header("User-Agent", USER_AGENT)
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

    let mut result = HashMap::new();
    for skill_path in skill_paths {
      let Ok(folder) = Self::skill_folder_of(skill_path) else {
        continue;
      };
      if folder.is_empty() {
        result.insert(skill_path.clone(), tree.sha.clone());
        continue;
      }
      if let Some(entry) = tree
        .tree
        .iter()
        .find(|item| item.kind == "tree" && item.path == folder)
      {
        result.insert(skill_path.clone(), entry.sha.clone());
      }
    }
    Ok(result)
  }

  pub async fn get_skill_folder_hash(
    source_url: &str,
    skill_path: &str,
    branch: Option<&str>,
  ) -> Result<String, String> {
    let (owner, repo) = Self::parse_github_url(source_url)?;
    let branch = branch.unwrap_or("main");
    let shas = Self::get_tree_shas(&owner, &repo, branch, &[skill_path.to_string()]).await?;
    shas.get(skill_path).cloned().ok_or(format!(
      "Skill folder not found in GitHub tree: {}",
      skill_path
    ))
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_plain_and_full_refs() {
    let r = GithubHelper::parse_github_ref("owner/repo").unwrap();
    assert_eq!((r.owner.as_str(), r.repo.as_str()), ("owner", "repo"));
    assert!(r.branch.is_none());

    let r = GithubHelper::parse_github_ref("https://github.com/owner/repo.git").unwrap();
    assert_eq!(r.repo, "repo");

    let r =
      GithubHelper::parse_github_ref("https://github.com/owner/repo/tree/dev/skills/foo").unwrap();
    assert_eq!(r.branch.as_deref(), Some("dev"));
    assert_eq!(r.subpath.as_deref(), Some("skills/foo"));

    let r = GithubHelper::parse_github_ref("git@github.com:owner/repo.git").unwrap();
    assert_eq!((r.owner.as_str(), r.repo.as_str()), ("owner", "repo"));

    let r =
      GithubHelper::parse_github_ref("https://github.com/owner/repo/blob/main/AGENTS.md").unwrap();
    assert_eq!(r.branch.as_deref(), Some("main"));
    assert_eq!(r.subpath.as_deref(), Some("AGENTS.md"));

    assert!(GithubHelper::parse_github_ref("nonsense").is_err());
  }

  #[test]
  fn skill_folder_extraction() {
    assert_eq!(GithubHelper::skill_folder_of("SKILL.md").unwrap(), "");
    assert_eq!(
      GithubHelper::skill_folder_of("skills/foo/SKILL.md").unwrap(),
      "skills/foo"
    );
    assert!(GithubHelper::skill_folder_of("skills/foo").is_err());
  }
}
