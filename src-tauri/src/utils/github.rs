use std::path::Path;
use std::process::Command;
use reqwest::blocking::Client;
use serde::Deserialize;

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

  pub fn clone_repo_to(owner: &str, repo: &str, dest: &Path) -> Result<(), String> {
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

  pub fn get_skill_folder_hash(source_url: &str, skill_path: &str) -> Result<String, String> {
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
      .map_err(|e| format!("Failed to request GitHub tree: {}", e))?;

    if !resp.status().is_success() {
      return Err(format!("GitHub API returned status {}", resp.status()));
    }

    let tree: GitTreeResponse = resp
      .json()
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
