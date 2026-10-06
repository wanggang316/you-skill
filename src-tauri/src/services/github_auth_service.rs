//! GitHub authentication through the local GitHub CLI (`gh`).
//!
//! The app does not store GitHub credentials. It reads the accounts that `gh` knows for
//! github.com and asks `gh` for the token of the account that the user selected (or the
//! active `gh` account when no selection exists).

use crate::config::{load_config, save_config};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

pub const GITHUB_HOST: &str = "github.com";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GhAccount {
  pub login: String,
  /// The account `gh` uses by default for github.com.
  pub active: bool,
  /// `gh` reports `success` when the stored token is valid.
  pub state: String,
  pub scopes: String,
  pub token_source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GithubAuthStatus {
  pub gh_installed: bool,
  pub gh_path: Option<String>,
  pub accounts: Vec<GhAccount>,
  /// Account the app uses for GitHub requests, or `None` when no usable account exists.
  pub selected_login: Option<String>,
  /// Set when `gh` is installed but its status could not be read.
  pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthStatusJson {
  #[serde(default)]
  hosts: HashMap<String, Vec<AuthStatusEntry>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthStatusEntry {
  #[serde(default)]
  login: String,
  #[serde(default)]
  active: bool,
  #[serde(default)]
  state: String,
  #[serde(default)]
  scopes: String,
  #[serde(default)]
  token_source: String,
}

/// Tokens are cached per login so that a batch of GitHub requests does not start one
/// `gh` process for each request.
static TOKEN_CACHE: Mutex<Option<(Option<String>, String)>> = Mutex::new(None);

/// Parse `gh auth status --json hosts` output into the github.com accounts.
pub fn parse_auth_status(json: &str) -> Result<Vec<GhAccount>, String> {
  let parsed: AuthStatusJson =
    serde_json::from_str(json).map_err(|e| format!("Failed to parse gh auth status: {}", e))?;
  Ok(
    parsed
      .hosts
      .get(GITHUB_HOST)
      .map(|entries| {
        entries
          .iter()
          .filter(|entry| !entry.login.is_empty())
          .map(|entry| GhAccount {
            login: entry.login.clone(),
            active: entry.active,
            state: entry.state.clone(),
            scopes: entry.scopes.clone(),
            token_source: entry.token_source.clone(),
          })
          .collect()
      })
      .unwrap_or_default(),
  )
}

/// The account to use: the preferred login when `gh` still knows it, else the active one.
pub fn resolve_selected_login(accounts: &[GhAccount], preferred: Option<&str>) -> Option<String> {
  let usable = |account: &&GhAccount| account.state == "success";
  if let Some(login) = preferred {
    if let Some(account) = accounts
      .iter()
      .filter(usable)
      .find(|account| account.login == login)
    {
      return Some(account.login.clone());
    }
  }
  accounts
    .iter()
    .filter(usable)
    .find(|account| account.active)
    .or_else(|| accounts.iter().find(usable))
    .map(|account| account.login.clone())
}

/// Locate the `gh` binary. GUI apps on macOS do not inherit the login shell PATH, so the
/// usual install locations are checked as well.
pub fn find_gh() -> Option<PathBuf> {
  let binary = if cfg!(windows) { "gh.exe" } else { "gh" };
  let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
    .map(|path| std::env::split_paths(&path).collect())
    .unwrap_or_default();
  if cfg!(windows) {
    for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
      if let Some(base) = std::env::var_os(var) {
        dirs.push(Path::new(&base).join("GitHub CLI"));
      }
    }
  } else {
    dirs.extend(
      [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/snap/bin",
      ]
      .iter()
      .map(PathBuf::from),
    );
    if let Some(home) = dirs_next::home_dir() {
      dirs.push(home.join(".local").join("bin"));
    }
  }
  dirs
    .into_iter()
    .map(|dir| dir.join(binary))
    .find(|candidate| candidate.is_file())
}

fn run_gh(gh: &Path, args: &[&str]) -> Result<String, String> {
  let output = Command::new(gh)
    .args(args)
    .env("GH_PROMPT_DISABLED", "1")
    .output()
    .map_err(|e| format!("Failed to run gh: {}", e))?;
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(if stderr.is_empty() {
      format!("gh exited with {}", output.status)
    } else {
      stderr
    });
  }
  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn list_accounts(gh: &Path) -> Result<Vec<GhAccount>, String> {
  let json = run_gh(
    gh,
    &[
      "auth",
      "status",
      "--hostname",
      GITHUB_HOST,
      "--json",
      "hosts",
    ],
  )?;
  parse_auth_status(&json)
}

fn preferred_login() -> Option<String> {
  load_config().ok().and_then(|config| config.github_account)
}

pub fn get_status() -> GithubAuthStatus {
  clear_token_cache();
  let Some(gh) = find_gh() else {
    return GithubAuthStatus {
      gh_installed: false,
      gh_path: None,
      accounts: Vec::new(),
      selected_login: None,
      error: None,
    };
  };
  let gh_path = Some(gh.to_string_lossy().to_string());
  match list_accounts(&gh) {
    Ok(accounts) => {
      let selected_login = resolve_selected_login(&accounts, preferred_login().as_deref());
      GithubAuthStatus {
        gh_installed: true,
        gh_path,
        accounts,
        selected_login,
        error: None,
      }
    },
    Err(error) => GithubAuthStatus {
      gh_installed: true,
      gh_path,
      accounts: Vec::new(),
      selected_login: None,
      error: Some(error),
    },
  }
}

/// Persist the account to use. `None` follows the active `gh` account.
pub fn set_account(login: Option<String>) -> Result<GithubAuthStatus, String> {
  let mut config = load_config()?;
  config.github_account = login
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty());
  save_config(&config)?;
  Ok(get_status())
}

fn clear_token_cache() {
  if let Ok(mut cache) = TOKEN_CACHE.lock() {
    *cache = None;
  }
}

/// Token for GitHub requests, or `None` when `gh` is missing or not signed in. Blocking:
/// call it from `spawn_blocking` in async code.
pub fn token() -> Option<String> {
  let preferred = preferred_login();
  if let Ok(cache) = TOKEN_CACHE.lock() {
    if let Some((login, token)) = cache.as_ref() {
      if *login == preferred {
        return Some(token.clone());
      }
    }
  }

  let gh = find_gh()?;
  let by_login = preferred.as_deref().and_then(|login| {
    run_gh(
      &gh,
      &["auth", "token", "--hostname", GITHUB_HOST, "--user", login],
    )
    .ok()
  });
  // A selected account that `gh` no longer knows falls back to the active account.
  let token = by_login
    .or_else(|| run_gh(&gh, &["auth", "token", "--hostname", GITHUB_HOST]).ok())
    .filter(|token| !token.is_empty())?;

  if let Ok(mut cache) = TOKEN_CACHE.lock() {
    *cache = Some((preferred, token.clone()));
  }
  Some(token)
}

pub async fn token_async() -> Option<String> {
  tokio::task::spawn_blocking(token).await.ok().flatten()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn account(login: &str, active: bool, state: &str) -> GhAccount {
    GhAccount {
      login: login.to_string(),
      active,
      state: state.to_string(),
      scopes: String::new(),
      token_source: String::new(),
    }
  }

  #[test]
  fn parse_auth_status_reads_github_com_accounts() {
    let json = r#"{"hosts":{
      "github.com":[
        {"state":"success","active":true,"host":"github.com","login":"alice","tokenSource":"keyring","scopes":"repo, read:org","gitProtocol":"ssh"},
        {"state":"success","active":false,"host":"github.com","login":"bob","tokenSource":"keyring","scopes":"repo","gitProtocol":"https"}
      ],
      "ghe.example.com":[
        {"state":"success","active":true,"host":"ghe.example.com","login":"corp"}
      ]
    }}"#;
    let accounts = parse_auth_status(json).unwrap();
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].login, "alice");
    assert!(accounts[0].active);
    assert_eq!(accounts[0].scopes, "repo, read:org");
    assert_eq!(accounts[0].token_source, "keyring");
    assert_eq!(accounts[1].login, "bob");
    assert!(!accounts[1].active);
  }

  #[test]
  fn parse_auth_status_without_accounts_is_empty() {
    assert!(parse_auth_status(r#"{"hosts":{}}"#).unwrap().is_empty());
    assert!(parse_auth_status("{}").unwrap().is_empty());
  }

  #[test]
  fn parse_auth_status_rejects_invalid_json() {
    assert!(parse_auth_status("not json").is_err());
  }

  #[test]
  fn resolve_selected_login_prefers_saved_account() {
    let accounts = vec![
      account("alice", true, "success"),
      account("bob", false, "success"),
    ];
    assert_eq!(
      resolve_selected_login(&accounts, Some("bob")),
      Some("bob".to_string())
    );
  }

  #[test]
  fn resolve_selected_login_falls_back_to_active_account() {
    let accounts = vec![
      account("alice", false, "success"),
      account("bob", true, "success"),
    ];
    assert_eq!(
      resolve_selected_login(&accounts, None),
      Some("bob".to_string())
    );
    assert_eq!(
      resolve_selected_login(&accounts, Some("gone")),
      Some("bob".to_string())
    );
  }

  #[test]
  fn resolve_selected_login_skips_accounts_with_invalid_tokens() {
    let accounts = vec![
      account("alice", true, "error"),
      account("bob", false, "success"),
    ];
    assert_eq!(
      resolve_selected_login(&accounts, Some("alice")),
      Some("bob".to_string())
    );
    assert_eq!(
      resolve_selected_login(&[account("alice", true, "error")], None),
      None
    );
  }
}
