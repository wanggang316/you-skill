use crate::utils::file::FileHelper;
use crate::utils::hash::{is_excluded_component, HASH_EXCLUDES};
use crate::utils::path::{is_symlink, remove_path_any};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use uuid::Uuid;
use walkdir::WalkDir;

pub const SKILL_MD: &str = "SKILL.md";
const TEMP_PREFIX: &str = "youskill-";
const LEGACY_TEMP_PREFIX: &str = "skill-kit-";

#[derive(Debug, Clone, Copy)]
pub struct CopyOpts {
  pub exclude: &'static [&'static str],
  pub preserve_symlinks: bool,
}

impl CopyOpts {
  /// Copy a skill directory: skip VCS/junk entries, keep symlinks as symlinks.
  pub const fn skill() -> Self {
    Self {
      exclude: HASH_EXCLUDES,
      preserve_symlinks: true,
    }
  }

  /// Copy everything verbatim.
  pub const fn all() -> Self {
    Self {
      exclude: &[],
      preserve_symlinks: true,
    }
  }
}

/// Recursively copy `src` into `dst` (created if missing). Existing files in `dst` are
/// overwritten; extra files in `dst` are left alone (use `replace_dir_atomic` for a mirror).
pub fn copy_dir(src: &Path, dst: &Path, opts: &CopyOpts) -> Result<(), String> {
  if !src.is_dir() {
    return Err(format!(
      "Source directory does not exist: {}",
      src.to_string_lossy()
    ));
  }
  fs::create_dir_all(dst).map_err(|e| e.to_string())?;
  for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let name = entry.file_name();
    if opts.exclude.contains(&name.to_string_lossy().as_ref()) {
      continue;
    }
    let src_path = entry.path();
    let dst_path = dst.join(&name);
    let file_type = entry.file_type().map_err(|e| e.to_string())?;

    if file_type.is_symlink() {
      copy_symlink(&src_path, &dst_path, opts)?;
    } else if file_type.is_dir() {
      copy_dir(&src_path, &dst_path, opts)?;
    } else {
      if dst_path.exists() || is_symlink(&dst_path) {
        remove_path_any(&dst_path)?;
      }
      fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn copy_symlink(src: &Path, dst: &Path, opts: &CopyOpts) -> Result<(), String> {
  if !opts.preserve_symlinks {
    return copy_following_link(src, dst, opts);
  }
  let target = fs::read_link(src).map_err(|e| e.to_string())?;
  if dst.exists() || is_symlink(dst) {
    remove_path_any(dst)?;
  }
  #[cfg(unix)]
  {
    std::os::unix::fs::symlink(&target, dst).map_err(|e| e.to_string())
  }
  #[cfg(windows)]
  {
    let resolved = if target.is_absolute() {
      target.clone()
    } else {
      src
        .parent()
        .map(|p| p.join(&target))
        .unwrap_or(target.clone())
    };
    let result = if resolved.is_dir() {
      std::os::windows::fs::symlink_dir(&target, dst)
    } else {
      std::os::windows::fs::symlink_file(&target, dst)
    };
    match result {
      Ok(()) => Ok(()),
      Err(_) => copy_following_link(src, dst, opts),
    }
  }
}

fn copy_following_link(src: &Path, dst: &Path, opts: &CopyOpts) -> Result<(), String> {
  if src.is_dir() {
    copy_dir(src, dst, opts)
  } else if src.is_file() {
    fs::copy(src, dst).map(|_| ()).map_err(|e| e.to_string())
  } else {
    tracing::warn!("skipping dangling symlink {}", src.to_string_lossy());
    Ok(())
  }
}

/// Replace `dst` with an exact copy of `src` (files only present in `dst` are removed).
/// The swap is done through sibling temp/old directories so a failure never leaves `dst`
/// half-written. Entries named in `keep` are carried over from the old `dst` (e.g. `.git`).
pub fn replace_dir_atomic(
  src: &Path,
  dst: &Path,
  opts: &CopyOpts,
  keep: &[&str],
) -> Result<(), String> {
  if is_symlink(dst) {
    return Err(format!(
      "Refusing to replace a symlink: {}",
      dst.to_string_lossy()
    ));
  }
  let parent = dst
    .parent()
    .ok_or_else(|| format!("Invalid destination: {}", dst.to_string_lossy()))?;
  let name = dst
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .ok_or_else(|| format!("Invalid destination: {}", dst.to_string_lossy()))?;
  fs::create_dir_all(parent).map_err(|e| e.to_string())?;

  let tmp = parent.join(format!(".{}.youskill-tmp", name));
  let old = parent.join(format!(".{}.youskill-old", name));
  for stale in [&tmp, &old] {
    if stale.exists() || is_symlink(stale) {
      remove_path_any(stale)?;
    }
  }

  if let Err(err) = copy_dir(src, &tmp, opts) {
    let _ = remove_path_any(&tmp);
    return Err(err);
  }

  let had_old = dst.exists();
  if had_old {
    fs::rename(dst, &old).map_err(|e| e.to_string())?;
  }
  if let Err(err) = fs::rename(&tmp, dst) {
    if had_old {
      let _ = fs::rename(&old, dst);
    }
    let _ = remove_path_any(&tmp);
    return Err(err.to_string());
  }

  if had_old {
    for entry in keep {
      let kept = old.join(entry);
      if kept.exists() || is_symlink(&kept) {
        let target = dst.join(entry);
        if target.exists() || is_symlink(&target) {
          let _ = remove_path_any(&target);
        }
        let _ = fs::rename(&kept, &target);
      }
    }
    let _ = remove_path_any(&old);
  }
  Ok(())
}

/// Move a directory, falling back to copy + delete when `rename` fails (cross-device).
pub fn move_dir(src: &Path, dst: &Path) -> Result<(), String> {
  if !src.is_dir() {
    return Err(format!(
      "Source directory does not exist: {}",
      src.to_string_lossy()
    ));
  }
  if let Some(parent) = dst.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  if fs::rename(src, dst).is_ok() {
    return Ok(());
  }
  copy_dir(src, dst, &CopyOpts::all())?;
  remove_path_any(src)
}

/// Find every directory under `root` (inclusive) that contains a `SKILL.md`. Search stops
/// descending once a skill directory is found, never follows symlinks, and skips excluded
/// components such as `.git` and `node_modules`.
pub fn find_skill_dirs(root: &Path, max_depth: usize) -> Result<Vec<PathBuf>, String> {
  if !root.is_dir() {
    return Err(format!(
      "Directory does not exist: {}",
      root.to_string_lossy()
    ));
  }
  let mut out = Vec::new();
  let mut walker = WalkDir::new(root)
    .follow_links(false)
    .max_depth(max_depth)
    .sort_by_file_name()
    .into_iter();

  while let Some(entry) = walker.next() {
    let Ok(entry) = entry else {
      continue;
    };
    if !entry.file_type().is_dir() {
      continue;
    }
    if entry.depth() > 0 && is_excluded_component(&entry.file_name().to_string_lossy()) {
      walker.skip_current_dir();
      continue;
    }
    if entry.path().join(SKILL_MD).is_file() {
      out.push(entry.path().to_path_buf());
      walker.skip_current_dir();
    }
  }
  Ok(out)
}

/// Skill name from `SKILL.md` frontmatter, falling back to the directory name.
pub fn read_skill_name(dir: &Path) -> Option<String> {
  let skill_md = dir.join(SKILL_MD);
  if !skill_md.is_file() {
    return None;
  }
  if let Ok(frontmatter) = FileHelper::read_skill_frontmatter(&skill_md) {
    if let Some(name) = frontmatter.name {
      return Some(name);
    }
  }
  dir.file_name().map(|n| n.to_string_lossy().to_string())
}

/// True for paths inside a temp directory created by `create_temp_dir` (safe to delete
/// after import). Deliberately narrower than "inside the OS temp dir".
pub fn is_staged_temp_path(path: &Path) -> bool {
  let temp_root = std::env::temp_dir();
  let Ok(relative) = path.strip_prefix(&temp_root) else {
    return false;
  };
  relative
    .components()
    .next()
    .map(|first| first.as_os_str().to_string_lossy().starts_with(TEMP_PREFIX))
    .unwrap_or(false)
}

pub fn create_temp_dir(prefix: &str) -> Result<PathBuf, String> {
  let dir = std::env::temp_dir().join(format!("{}{}-{}", TEMP_PREFIX, prefix, Uuid::new_v4()));
  fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  Ok(dir)
}

/// Delete temp directories created by detect/import flows that are older than `max_age`.
pub fn sweep_temp_dirs(max_age: Duration) {
  let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
    return;
  };
  let now = SystemTime::now();
  for entry in entries.flatten() {
    let name = entry.file_name().to_string_lossy().to_string();
    if !name.starts_with(TEMP_PREFIX) && !name.starts_with(LEGACY_TEMP_PREFIX) {
      continue;
    }
    let Ok(meta) = entry.metadata() else {
      continue;
    };
    let Ok(modified) = meta.modified() else {
      continue;
    };
    if now.duration_since(modified).unwrap_or_default() > max_age {
      let _ = remove_path_any(&entry.path());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
  }

  #[test]
  fn replace_dir_atomic_mirrors_and_removes_extras() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    write(&src, "SKILL.md", "new");
    write(&dst, "SKILL.md", "old");
    write(&dst, "extra.md", "gone");
    write(&dst, ".git/HEAD", "keep");

    replace_dir_atomic(&src, &dst, &CopyOpts::skill(), &[".git"]).unwrap();

    assert_eq!(fs::read_to_string(dst.join("SKILL.md")).unwrap(), "new");
    assert!(!dst.join("extra.md").exists());
    assert_eq!(fs::read_to_string(dst.join(".git/HEAD")).unwrap(), "keep");
    assert!(!tmp.path().join(".dst.youskill-tmp").exists());
    assert!(!tmp.path().join(".dst.youskill-old").exists());
  }

  #[test]
  fn replace_dir_atomic_keeps_dst_when_src_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("dst");
    write(&dst, "SKILL.md", "old");
    let missing = tmp.path().join("missing");
    assert!(replace_dir_atomic(&missing, &dst, &CopyOpts::skill(), &[]).is_err());
    assert_eq!(fs::read_to_string(dst.join("SKILL.md")).unwrap(), "old");
  }

  #[test]
  fn move_dir_moves_content() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("nested").join("dst");
    write(&src, "a/b.md", "x");
    move_dir(&src, &dst).unwrap();
    assert!(!src.exists());
    assert_eq!(fs::read_to_string(dst.join("a/b.md")).unwrap(), "x");
  }

  #[test]
  fn find_skill_dirs_prunes_nested_and_skips_excluded() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "one/SKILL.md", "");
    write(root, "one/examples/inner/SKILL.md", "");
    write(root, "group/two/SKILL.md", "");
    write(root, "node_modules/three/SKILL.md", "");
    write(root, ".git/four/SKILL.md", "");
    write(root, "plain/readme.md", "");

    let found = find_skill_dirs(root, 8).unwrap();
    assert_eq!(found, vec![root.join("group/two"), root.join("one")]);
  }

  #[test]
  fn find_skill_dirs_returns_root_when_root_is_skill() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "SKILL.md", "");
    write(tmp.path(), "sub/SKILL.md", "");
    let found = find_skill_dirs(tmp.path(), 8).unwrap();
    assert_eq!(found, vec![tmp.path().to_path_buf()]);
  }

  #[test]
  fn read_skill_name_prefers_frontmatter() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("dir-name");
    write(&dir, "SKILL.md", "---\nname: fm-name\n---\n");
    assert_eq!(read_skill_name(&dir).as_deref(), Some("fm-name"));
    write(&dir, "SKILL.md", "no frontmatter");
    assert_eq!(read_skill_name(&dir).as_deref(), Some("dir-name"));
  }

  #[cfg(unix)]
  #[test]
  fn copy_dir_preserves_symlinks() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    write(&src, "a.md", "A");
    std::os::unix::fs::symlink("a.md", src.join("link")).unwrap();
    copy_dir(&src, &dst, &CopyOpts::skill()).unwrap();
    assert!(is_symlink(&dst.join("link")));
    assert_eq!(
      fs::read_link(dst.join("link")).unwrap(),
      PathBuf::from("a.md")
    );
  }
}
