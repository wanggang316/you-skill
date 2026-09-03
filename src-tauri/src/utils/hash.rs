use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

/// Path components ignored by the content hash and by copy/mirror operations.
pub const HASH_EXCLUDES: &[&str] = &[".git", "node_modules", ".DS_Store", "Thumbs.db"];

/// Cheap stat-only fingerprint of a directory tree, used as the cache key for `hash_dir_cached`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirSignature(Vec<(String, u64, u128)>);

static HASH_CACHE: RwLock<Option<HashMap<PathBuf, (DirSignature, String)>>> = RwLock::new(None);

pub fn is_excluded_component(name: &str) -> bool {
  HASH_EXCLUDES.contains(&name)
}

enum EntryKind {
  File,
  Symlink,
}

struct HashEntry {
  relative: String,
  path: PathBuf,
  kind: EntryKind,
  size: u64,
  mtime_nanos: u128,
}

fn collect_entries(root: &Path) -> Result<Vec<HashEntry>, String> {
  let metadata = fs::symlink_metadata(root)
    .map_err(|_| format!("Directory does not exist: {}", root.to_string_lossy()))?;
  if !root.is_dir() {
    return Err(format!("Not a directory: {}", root.to_string_lossy()));
  }
  // Hash through a symlinked root (an install target linked elsewhere, a linked source).
  let resolved_root;
  let root = if metadata.file_type().is_symlink() {
    resolved_root = root.canonicalize().map_err(|e| e.to_string())?;
    resolved_root.as_path()
  } else {
    root
  };

  let mut entries = Vec::new();
  let walker = WalkDir::new(root)
    .follow_links(false)
    .min_depth(1)
    .into_iter()
    .filter_entry(|entry| !is_excluded_component(&entry.file_name().to_string_lossy()));

  for entry in walker {
    let entry = entry.map_err(|e| e.to_string())?;
    let file_type = entry.file_type();
    let kind = if file_type.is_symlink() {
      EntryKind::Symlink
    } else if file_type.is_file() {
      EntryKind::File
    } else {
      continue;
    };

    let relative = entry
      .path()
      .strip_prefix(root)
      .map_err(|e| e.to_string())?
      .to_string_lossy()
      .replace('\\', "/");
    let meta = entry.metadata().map_err(|e| e.to_string())?;
    let mtime_nanos = meta
      .modified()
      .ok()
      .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
      .map(|duration| duration.as_nanos())
      .unwrap_or(0);

    entries.push(HashEntry {
      relative,
      path: entry.path().to_path_buf(),
      kind,
      size: meta.len(),
      mtime_nanos,
    });
  }

  entries.sort_by(|a, b| a.relative.cmp(&b.relative));
  Ok(entries)
}

/// Stat-only signature of a directory (relative path, size, mtime for every file/symlink).
pub fn dir_signature(root: &Path) -> Result<DirSignature, String> {
  let entries = collect_entries(root)?;
  Ok(DirSignature(
    entries
      .into_iter()
      .map(|entry| (entry.relative, entry.size, entry.mtime_nanos))
      .collect(),
  ))
}

/// Content hash of a directory. Deterministic across machines: entries are sorted by
/// relative path, each file contributes `rel 0x00 content 0xFF`, each symlink contributes
/// `rel 0x01 target 0xFF`. Excluded components (see `HASH_EXCLUDES`) are skipped.
pub fn hash_dir(root: &Path) -> Result<String, String> {
  let entries = collect_entries(root)?;
  let mut hasher = Sha256::new();
  let mut buf = [0u8; 16 * 1024];

  for entry in entries {
    hasher.update(entry.relative.as_bytes());
    match entry.kind {
      EntryKind::File => {
        hasher.update([0x00]);
        let file = File::open(&entry.path).map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(file);
        loop {
          let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
          if n == 0 {
            break;
          }
          hasher.update(&buf[..n]);
        }
      },
      EntryKind::Symlink => {
        hasher.update([0x01]);
        let target = fs::read_link(&entry.path).map_err(|e| e.to_string())?;
        hasher.update(target.to_string_lossy().replace('\\', "/").as_bytes());
      },
    }
    hasher.update([0xff]);
  }

  Ok(format!("{:x}", hasher.finalize()))
}

/// `hash_dir` with an in-memory cache keyed by directory signature.
pub fn hash_dir_cached(root: &Path) -> Result<String, String> {
  let signature = dir_signature(root)?;
  let key = root.to_path_buf();

  if let Ok(cache) = HASH_CACHE.read() {
    if let Some(map) = cache.as_ref() {
      if let Some((cached_signature, hash)) = map.get(&key) {
        if *cached_signature == signature {
          return Ok(hash.clone());
        }
      }
    }
  }

  let hash = hash_dir(root)?;
  if let Ok(mut cache) = HASH_CACHE.write() {
    cache
      .get_or_insert_with(HashMap::new)
      .insert(key, (signature, hash.clone()));
  }
  Ok(hash)
}

pub fn invalidate_hash_cache(root: &Path) {
  if let Ok(mut cache) = HASH_CACHE.write() {
    if let Some(map) = cache.as_mut() {
      map.retain(|key, _| !key.starts_with(root));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
  }

  #[test]
  fn identical_trees_hash_equal() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(a.path(), "SKILL.md", "---\nname: x\n---\n");
    write(a.path(), "ref/a.md", "A");
    write(b.path(), "SKILL.md", "---\nname: x\n---\n");
    write(b.path(), "ref/a.md", "A");
    assert_eq!(hash_dir(a.path()).unwrap(), hash_dir(b.path()).unwrap());
  }

  #[test]
  fn rename_changes_hash() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(a.path(), "a.md", "A");
    write(b.path(), "b.md", "A");
    assert_ne!(hash_dir(a.path()).unwrap(), hash_dir(b.path()).unwrap());
  }

  #[test]
  fn separator_prevents_collision() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(a.path(), "a", "bc");
    write(b.path(), "ab", "c");
    assert_ne!(hash_dir(a.path()).unwrap(), hash_dir(b.path()).unwrap());
  }

  #[test]
  fn excluded_components_are_ignored() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(a.path(), "SKILL.md", "x");
    write(b.path(), "SKILL.md", "x");
    write(b.path(), ".git/HEAD", "ref");
    write(b.path(), "node_modules/x/index.js", "1");
    write(b.path(), ".DS_Store", "junk");
    assert_eq!(hash_dir(a.path()).unwrap(), hash_dir(b.path()).unwrap());
  }

  #[test]
  fn empty_dir_hashes_to_constant() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    assert_eq!(hash_dir(a.path()).unwrap(), hash_dir(b.path()).unwrap());
  }

  #[test]
  fn cache_invalidates_on_content_change() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.md", "one");
    let first = hash_dir_cached(dir.path()).unwrap();
    // Force a different size so the signature changes even with coarse mtime resolution.
    write(dir.path(), "a.md", "one-two");
    let second = hash_dir_cached(dir.path()).unwrap();
    assert_ne!(first, second);
    assert_eq!(second, hash_dir(dir.path()).unwrap());
  }

  #[cfg(unix)]
  #[test]
  fn symlink_entries_hash_their_target() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(a.path(), "a.md", "A");
    write(b.path(), "a.md", "A");
    std::os::unix::fs::symlink("a.md", a.path().join("link")).unwrap();
    std::os::unix::fs::symlink("other.md", b.path().join("link")).unwrap();
    assert_ne!(hash_dir(a.path()).unwrap(), hash_dir(b.path()).unwrap());
  }
}
