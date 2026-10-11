use crate::config::config_path;
use reqwest::Client;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Avatars change rarely; refetch at most once a week.
const AVATAR_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const AVATAR_SIZE: u32 = 40;

fn avatar_cache_dir() -> Result<PathBuf, String> {
  let base = config_path()?;
  let parent = base
    .parent()
    .ok_or_else(|| "Failed to resolve the avatar cache folder".to_string())?;
  Ok(parent.join("avatar_cache"))
}

/// GitHub logins hold only ASCII letters, digits and hyphens. The check also keeps the
/// owner safe to use as a file name.
fn is_valid_owner(owner: &str) -> bool {
  !owner.is_empty()
    && owner.len() <= 39
    && owner
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn cache_file_path(cache_dir: &Path, owner: &str) -> PathBuf {
  cache_dir.join(format!("{}.img", owner.to_ascii_lowercase()))
}

/// Returns the cached bytes and whether they are still fresh at `now`.
fn read_cache(path: &Path, now: SystemTime) -> Option<(Vec<u8>, bool)> {
  let modified = fs::metadata(path).and_then(|meta| meta.modified()).ok()?;
  let bytes = fs::read(path).ok().filter(|bytes| !bytes.is_empty())?;
  let fresh = now
    .duration_since(modified)
    .map(|age| age < AVATAR_TTL)
    .unwrap_or(true);
  Some((bytes, fresh))
}

fn write_cache(path: &Path, bytes: &[u8]) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| format!("Failed to create avatar cache: {}", e))?;
  }
  fs::write(path, bytes).map_err(|e| format!("Failed to write avatar cache: {}", e))
}

async fn download_avatar(owner: &str) -> Result<Vec<u8>, String> {
  let url = format!("https://github.com/{}.png?size={}", owner, AVATAR_SIZE);
  let client = Client::builder()
    .timeout(Duration::from_secs(15))
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
  let response = client
    .get(&url)
    .send()
    .await
    .map_err(|e| format!("Failed to download avatar: {}", e))?;
  if !response.status().is_success() {
    return Err(format!(
      "Failed to download avatar: HTTP {}",
      response.status()
    ));
  }
  let bytes = response
    .bytes()
    .await
    .map_err(|e| format!("Failed to read avatar: {}", e))?;
  if bytes.is_empty() {
    return Err("Failed to download avatar: empty response".to_string());
  }
  Ok(bytes.to_vec())
}

async fn load_avatar<F, Fut>(cache_dir: &Path, owner: &str, download: F) -> Result<Vec<u8>, String>
where
  F: FnOnce() -> Fut,
  Fut: std::future::Future<Output = Result<Vec<u8>, String>>,
{
  let path = cache_file_path(cache_dir, owner);
  let cached = read_cache(&path, SystemTime::now());
  if let Some((bytes, true)) = cached {
    return Ok(bytes);
  }
  match download().await {
    Ok(bytes) => {
      if let Err(error) = write_cache(&path, &bytes) {
        tracing::warn!("{}", error);
      }
      Ok(bytes)
    },
    // A stale avatar is better than none when GitHub is not reachable.
    Err(error) => cached.map(|(bytes, _)| bytes).ok_or(error),
  }
}

/// Returns the avatar image of a GitHub user or organisation, served from the disk cache
/// when possible.
pub async fn github_avatar(owner: String) -> Result<Vec<u8>, String> {
  if !is_valid_owner(&owner) {
    return Err(format!("Invalid GitHub owner: {}", owner));
  }
  let cache_dir = avatar_cache_dir()?;
  load_avatar(&cache_dir, &owner, || download_avatar(&owner)).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::Cell;

  fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
      .build()
      .unwrap()
      .block_on(future)
  }

  fn set_age(path: &Path, age: Duration) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - age).unwrap();
  }

  #[test]
  fn owner_validation() {
    assert!(is_valid_owner("anthropics"));
    assert!(is_valid_owner("Factory-AI"));
    assert!(!is_valid_owner(""));
    assert!(!is_valid_owner("../etc"));
    assert!(!is_valid_owner("a/b"));
    assert!(!is_valid_owner(&"a".repeat(40)));
  }

  #[test]
  fn fresh_cache_skips_download() {
    let dir = tempfile::tempdir().unwrap();
    write_cache(&cache_file_path(dir.path(), "Owner"), b"cached").unwrap();
    let called = Cell::new(false);
    let bytes = block_on(load_avatar(dir.path(), "owner", || async {
      called.set(true);
      Ok(b"new".to_vec())
    }))
    .unwrap();
    assert_eq!(bytes, b"cached");
    assert!(!called.get());
  }

  #[test]
  fn missing_cache_downloads_and_stores() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = block_on(load_avatar(dir.path(), "owner", || async {
      Ok(b"new".to_vec())
    }))
    .unwrap();
    assert_eq!(bytes, b"new");
    assert_eq!(
      fs::read(cache_file_path(dir.path(), "owner")).unwrap(),
      b"new"
    );
  }

  #[test]
  fn stale_cache_is_refreshed() {
    let dir = tempfile::tempdir().unwrap();
    let path = cache_file_path(dir.path(), "owner");
    write_cache(&path, b"old").unwrap();
    set_age(&path, AVATAR_TTL + Duration::from_secs(60));
    let bytes = block_on(load_avatar(dir.path(), "owner", || async {
      Ok(b"new".to_vec())
    }))
    .unwrap();
    assert_eq!(bytes, b"new");
    assert_eq!(fs::read(&path).unwrap(), b"new");
  }

  #[test]
  fn stale_cache_is_used_when_download_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = cache_file_path(dir.path(), "owner");
    write_cache(&path, b"old").unwrap();
    set_age(&path, AVATAR_TTL + Duration::from_secs(60));
    let bytes = block_on(load_avatar(dir.path(), "owner", || async {
      Err("offline".to_string())
    }))
    .unwrap();
    assert_eq!(bytes, b"old");
  }

  #[test]
  fn download_error_without_cache_is_returned() {
    let dir = tempfile::tempdir().unwrap();
    let result = block_on(load_avatar(dir.path(), "owner", || async {
      Err("offline".to_string())
    }));
    assert_eq!(result, Err("offline".to_string()));
  }
}
