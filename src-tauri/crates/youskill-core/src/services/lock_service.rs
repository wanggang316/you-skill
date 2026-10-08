use crate::models::{LockFile, SkillRecord, LOCK_VERSION};
use crate::services::env::Env;
use crate::utils::path::OPS_LOCK_FILE;
use std::cell::Cell;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// Serializes every filesystem-mutating hub operation (import, install, sync, migrate)
/// inside this process. Across processes (the app and the CLI) the same is done with an
/// advisory lock on `~/.youskill/.ops.lock`, see [`ops_guard`].
static OPS: Mutex<()> = Mutex::new(());

thread_local! {
  /// Whether the current thread is inside an operation, so a lock-file write made from it
  /// must not take the operation lock again.
  static IN_OPS: Cell<bool> = const { Cell::new(false) };
}

static STORES: OnceLock<Mutex<HashMap<PathBuf, Arc<LockStore>>>> = OnceLock::new();

/// Holds the process mutex and the file lock of one operation. Dropping it releases both.
pub struct OpsGuard {
  file: File,
  _process: MutexGuard<'static, ()>,
}

impl Drop for OpsGuard {
  fn drop(&mut self) {
    let _ = self.file.unlock();
    IN_OPS.with(|flag| flag.set(false));
  }
}

/// Take the operation lock of the hub `env` points at. Blocks until every other operation,
/// in this process or another, has finished.
pub fn ops_guard(env: &Env) -> Result<OpsGuard, String> {
  ops_guard_at(&env.youskill_root)
}

/// Lock the operations of the hub rooted at `root` (`~/.youskill`). The lock file is
/// created on demand and never holds content.
pub fn ops_guard_at(root: &Path) -> Result<OpsGuard, String> {
  let process = OPS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  fs::create_dir_all(root).map_err(|e| format!("Failed to create {}: {}", root.display(), e))?;
  let path = root.join(OPS_LOCK_FILE);
  let file = OpenOptions::new()
    .create(true)
    .truncate(false)
    .write(true)
    .open(&path)
    .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
  file
    .lock()
    .map_err(|e| format!("Failed to lock {}: {}", path.display(), e))?;
  IN_OPS.with(|flag| flag.set(true));
  Ok(OpsGuard {
    file,
    _process: process,
  })
}

/// The operation lock for a lock-file write that happens outside an operation, such as
/// storing a source check. Returns `None` when the calling thread already holds it.
pub fn ops_guard_unless_held(root: &Path) -> Result<Option<OpsGuard>, String> {
  if IN_OPS.with(|flag| flag.get()) {
    Ok(None)
  } else {
    ops_guard_at(root).map(Some)
  }
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

  /// Read-modify-write under the operation lock and the store mutex. The file is only
  /// rewritten when the closure succeeds, through a temp file + rename so a crash never
  /// leaves a truncated lock.
  pub fn update<T>(&self, f: impl FnOnce(&mut LockFile) -> Result<T, String>) -> Result<T, String> {
    let root = self.path.parent().unwrap_or(Path::new("."));
    let _ops = ops_guard_unless_held(root)?;
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
  fn ops_guard_holds_the_file_lock_until_dropped() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(OPS_LOCK_FILE);
    let guard = ops_guard_at(tmp.path()).unwrap();
    assert!(path.is_file());

    // A second handle stands in for another process: advisory locks are per open file.
    let other = File::open(&path).unwrap();
    assert!(matches!(
      other.try_lock(),
      Err(std::fs::TryLockError::WouldBlock)
    ));

    drop(guard);
    assert!(other.try_lock().is_ok());
  }

  #[test]
  fn update_inside_an_operation_does_not_take_the_lock_again() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LockStore::new(tmp.path().join(".skill-lock.json"));
    let _ops = ops_guard_at(tmp.path()).unwrap();
    store
      .update(|lock| {
        lock.skills.insert("a".to_string(), record("1"));
        Ok(())
      })
      .unwrap();
    assert!(store.get("a").unwrap().is_some());
  }

  #[test]
  fn corrupt_lock_is_an_error_not_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".skill-lock.json");
    fs::write(&path, "{ not json").unwrap();
    assert!(read_lock_file(&path).is_err());
  }
}
