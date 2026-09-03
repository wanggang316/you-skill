use crate::models::{LockFile, SkillRecord, LOCK_VERSION};
use crate::services::env::Env;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// Serializes every filesystem-mutating hub operation (import, install, sync, migrate).
static OPS: Mutex<()> = Mutex::new(());

static STORES: OnceLock<Mutex<HashMap<PathBuf, Arc<LockStore>>>> = OnceLock::new();

pub fn ops_guard() -> MutexGuard<'static, ()> {
  OPS.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One store per lock path so concurrent commands share the same write mutex.
pub fn store(env: &Env) -> Arc<LockStore> {
  let stores = STORES.get_or_init(|| Mutex::new(HashMap::new()));
  let mut map = stores
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner());
  map
    .entry(env.lock_path.clone())
    .or_insert_with(|| Arc::new(LockStore::new(env.lock_path.clone())))
    .clone()
}

pub struct LockStore {
  path: PathBuf,
  write_lock: Mutex<()>,
}

impl LockStore {
  pub fn new(path: PathBuf) -> Self {
    Self {
      path,
      write_lock: Mutex::new(()),
    }
  }

  pub fn exists(&self) -> bool {
    self.path.is_file()
  }

  pub fn read(&self) -> Result<LockFile, String> {
    read_lock_file(&self.path)
  }

  pub fn get(&self, name: &str) -> Result<Option<SkillRecord>, String> {
    Ok(self.read()?.skills.get(name).cloned())
  }

  /// Read-modify-write under the store mutex. The file is only rewritten when the closure
  /// succeeds, through a temp file + rename so a crash never leaves a truncated lock.
  pub fn update<T>(&self, f: impl FnOnce(&mut LockFile) -> Result<T, String>) -> Result<T, String> {
    let _guard = self
      .write_lock
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut lock = self.read()?;
    let result = f(&mut lock)?;
    lock.version = LOCK_VERSION;
    write_lock_file(&self.path, &lock)?;
    Ok(result)
  }
}

pub fn read_lock_file(path: &Path) -> Result<LockFile, String> {
  if !path.exists() {
    return Ok(LockFile::default());
  }
  let content = fs::read_to_string(path)
    .map_err(|e| format!("Failed to read {}: {}", path.to_string_lossy(), e))?;
  if content.trim().is_empty() {
    return Ok(LockFile::default());
  }
  serde_json::from_str(&content)
    .map_err(|e| format!("Failed to parse {}: {}", path.to_string_lossy(), e))
}

pub fn write_lock_file(path: &Path, lock: &LockFile) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  let content = serde_json::to_string_pretty(lock).map_err(|e| e.to_string())?;
  let tmp = path.with_extension("json.tmp");
  fs::write(&tmp, content).map_err(|e| e.to_string())?;
  fs::rename(&tmp, path).map_err(|e| {
    let _ = fs::remove_file(&tmp);
    e.to_string()
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{SkillRecord, SkillSource};
  use std::thread;

  fn record(hash: &str) -> SkillRecord {
    SkillRecord {
      source: SkillSource::None,
      hash: hash.to_string(),
      imported_at: "t".to_string(),
      updated_at: "t".to_string(),
      installs: Vec::new(),
    }
  }

  #[test]
  fn update_persists_and_failed_closure_leaves_file_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LockStore::new(tmp.path().join(".skill-lock.json"));
    assert!(!store.exists());

    store
      .update(|lock| {
        lock.skills.insert("a".to_string(), record("1"));
        Ok(())
      })
      .unwrap();
    assert!(store.exists());
    assert_eq!(store.get("a").unwrap().unwrap().hash, "1");

    let err = store.update(|lock| {
      lock.skills.insert("b".to_string(), record("2"));
      Err::<(), String>("boom".to_string())
    });
    assert!(err.is_err());
    assert!(store.get("b").unwrap().is_none());
    assert!(!tmp.path().join(".skill-lock.json.tmp").exists());
  }

  #[test]
  fn concurrent_updates_do_not_lose_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(LockStore::new(tmp.path().join(".skill-lock.json")));
    let handles: Vec<_> = (0..8)
      .map(|i| {
        let store = Arc::clone(&store);
        thread::spawn(move || {
          store
            .update(|lock| {
              lock.skills.insert(format!("s{i}"), record("h"));
              Ok(())
            })
            .unwrap();
        })
      })
      .collect();
    for handle in handles {
      handle.join().unwrap();
    }
    assert_eq!(store.read().unwrap().skills.len(), 8);
  }

  #[test]
  fn corrupt_lock_is_an_error_not_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".skill-lock.json");
    fs::write(&path, "{ not json").unwrap();
    assert!(read_lock_file(&path).is_err());
  }
}
