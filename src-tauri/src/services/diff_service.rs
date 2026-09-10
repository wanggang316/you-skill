//! Content diff between the hub copy of a skill and one of its install targets or its
//! source. Pure filesystem work; the GitHub download for a source diff happens in the
//! command layer before calling in here.

use crate::models::{DiffHunk, DiffLine, DiffLineKind, DiffStatus, FileDiff, SkillDiff};
use crate::services::env::Env;
use crate::utils::hash::is_excluded_component;
use crate::utils::path::path_to_string;
use similar::{ChangeTag, TextDiff};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Files larger than this are compared by content but never rendered as hunks.
const MAX_TEXT_BYTES: u64 = 1024 * 1024;
/// Rendered hunk lines per file; the rest is dropped and the file flagged `truncated`.
const MAX_LINES_PER_FILE: usize = 4000;
const CONTEXT_RADIUS: usize = 3;

enum Entry {
  File(PathBuf),
  Symlink(String),
}

/// Diff the hub copy of `name` (left) against `right` (an install target or a source
/// directory). Added / removed are relative to the hub: `added` exists only on the right.
pub fn diff_hub_against(
  env: &Env,
  name: &str,
  right: &Path,
  right_label: &str,
) -> Result<SkillDiff, String> {
  let hub_dir = env.hub_dir(name);
  let (files, unchanged) = diff_dirs(&hub_dir, right)?;
  Ok(SkillDiff {
    name: name.to_string(),
    left_label: path_to_string(&hub_dir),
    right_label: right_label.to_string(),
    files,
    unchanged,
  })
}

/// Compare two single files. `None` when their bytes are identical; a missing side shows
/// as added / removed relative to `left`.
pub fn diff_files(rel: &str, left: &Path, right: &Path) -> Result<Option<FileDiff>, String> {
  let entry = |path: &Path| -> Option<Entry> {
    if fs::symlink_metadata(path).is_err() {
      return None;
    }
    if path.is_file() {
      Some(Entry::File(path.to_path_buf()))
    } else {
      fs::read_link(path)
        .ok()
        .map(|target| Entry::Symlink(target.to_string_lossy().to_string()))
    }
  };
  match (entry(left), entry(right)) {
    (Some(old), Some(new)) => diff_entry(rel, &old, &new),
    (Some(old), None) => single_side(rel, &old, DiffStatus::Removed).map(Some),
    (None, Some(new)) => single_side(rel, &new, DiffStatus::Added).map(Some),
    (None, None) => Ok(None),
  }
}

/// Compare two directories file by file. Returns the changed files and the number of
/// identical ones.
pub fn diff_dirs(left: &Path, right: &Path) -> Result<(Vec<FileDiff>, usize), String> {
  let left_entries = collect_entries(left)?;
  let right_entries = collect_entries(right)?;
  let mut paths: Vec<&String> = left_entries.keys().chain(right_entries.keys()).collect();
  paths.sort();
  paths.dedup();

  let mut files = Vec::new();
  let mut unchanged = 0;
  for rel in paths {
    match (left_entries.get(rel), right_entries.get(rel)) {
      (Some(old), None) => files.push(single_side(rel, old, DiffStatus::Removed)?),
      (None, Some(new)) => files.push(single_side(rel, new, DiffStatus::Added)?),
      (Some(old), Some(new)) => match diff_entry(rel, old, new)? {
        Some(file) => files.push(file),
        None => unchanged += 1,
      },
      (None, None) => unreachable!(),
    }
  }
  Ok((files, unchanged))
}

fn collect_entries(root: &Path) -> Result<BTreeMap<String, Entry>, String> {
  let metadata = fs::symlink_metadata(root)
    .map_err(|_| format!("Directory does not exist: {}", root.to_string_lossy()))?;
  if !root.is_dir() {
    return Err(format!("Not a directory: {}", root.to_string_lossy()));
  }
  let resolved;
  let root = if metadata.file_type().is_symlink() {
    resolved = root.canonicalize().map_err(|e| e.to_string())?;
    resolved.as_path()
  } else {
    root
  };

  let mut entries = BTreeMap::new();
  let walker = WalkDir::new(root)
    .follow_links(false)
    .min_depth(1)
    .into_iter()
    .filter_entry(|entry| !is_excluded_component(&entry.file_name().to_string_lossy()));
  for entry in walker {
    let entry = entry.map_err(|e| e.to_string())?;
    let file_type = entry.file_type();
    let relative = entry
      .path()
      .strip_prefix(root)
      .map_err(|e| e.to_string())?
      .to_string_lossy()
      .replace('\\', "/");
    if file_type.is_symlink() {
      let target = fs::read_link(entry.path()).map_err(|e| e.to_string())?;
      entries.insert(
        relative,
        Entry::Symlink(target.to_string_lossy().to_string()),
      );
    } else if file_type.is_file() {
      entries.insert(relative, Entry::File(entry.path().to_path_buf()));
    }
  }
  Ok(entries)
}

struct Content {
  text: Option<String>,
  bytes: Vec<u8>,
  too_large: bool,
}

fn read_entry(entry: &Entry) -> Result<Content, String> {
  match entry {
    Entry::Symlink(target) => {
      let text = format!("-> {}\n", target);
      Ok(Content {
        bytes: text.as_bytes().to_vec(),
        text: Some(text),
        too_large: false,
      })
    },
    Entry::File(path) => {
      let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
      let bytes = fs::read(path).map_err(|e| format!("{}: {}", path.to_string_lossy(), e))?;
      let too_large = size > MAX_TEXT_BYTES;
      let text = if too_large || !looks_like_text(&bytes) {
        None
      } else {
        String::from_utf8(bytes.clone()).ok()
      };
      Ok(Content {
        text,
        bytes,
        too_large,
      })
    },
  }
}

fn looks_like_text(bytes: &[u8]) -> bool {
  let probe = &bytes[..bytes.len().min(8192)];
  !probe.contains(&0)
}

fn single_side(rel: &str, entry: &Entry, status: DiffStatus) -> Result<FileDiff, String> {
  let content = read_entry(entry)?;
  let (hunks, truncated) = match &content.text {
    Some(text) => match status {
      DiffStatus::Added => hunks_for("", text),
      _ => hunks_for(text, ""),
    },
    None => (Vec::new(), false),
  };
  Ok(FileDiff {
    path: rel.to_string(),
    status,
    binary: content.text.is_none() && !content.too_large,
    truncated: truncated || content.too_large,
    hunks,
  })
}

fn diff_entry(rel: &str, old: &Entry, new: &Entry) -> Result<Option<FileDiff>, String> {
  let old_content = read_entry(old)?;
  let new_content = read_entry(new)?;
  if old_content.bytes == new_content.bytes {
    return Ok(None);
  }
  let (hunks, truncated) = match (&old_content.text, &new_content.text) {
    (Some(a), Some(b)) => hunks_for(a, b),
    _ => (Vec::new(), false),
  };
  let too_large = old_content.too_large || new_content.too_large;
  Ok(Some(FileDiff {
    path: rel.to_string(),
    status: DiffStatus::Modified,
    binary: hunks.is_empty() && !too_large,
    truncated: truncated || too_large,
    hunks,
  }))
}

fn hunks_for(old: &str, new: &str) -> (Vec<DiffHunk>, bool) {
  let diff = TextDiff::from_lines(old, new);
  let mut unified = diff.unified_diff();
  unified.context_radius(CONTEXT_RADIUS);
  let mut hunks = Vec::new();
  let mut emitted = 0;
  let mut truncated = false;
  'outer: for hunk in unified.iter_hunks() {
    let mut lines = Vec::new();
    for change in hunk.iter_changes() {
      if emitted >= MAX_LINES_PER_FILE {
        truncated = true;
        if !lines.is_empty() {
          hunks.push(DiffHunk {
            header: hunk.header().to_string(),
            lines,
          });
        }
        break 'outer;
      }
      let kind = match change.tag() {
        ChangeTag::Equal => DiffLineKind::Context,
        ChangeTag::Delete => DiffLineKind::Delete,
        ChangeTag::Insert => DiffLineKind::Insert,
      };
      lines.push(DiffLine {
        kind,
        old_line: change.old_index().map(|index| index + 1),
        new_line: change.new_index().map(|index| index + 1),
        text: change.value().trim_end_matches(['\n', '\r']).to_string(),
      });
      emitted += 1;
    }
    hunks.push(DiffHunk {
      header: hunk.header().to_string(),
      lines,
    });
  }
  (hunks, truncated)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn write(root: &Path, rel: &str, content: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
  }

  #[test]
  fn reports_added_removed_modified_and_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let left = tmp.path().join("left");
    let right = tmp.path().join("right");
    write(
      &left,
      "SKILL.md",
      b"---\nname: demo\n---\nline one\nline two\n",
    );
    write(
      &right,
      "SKILL.md",
      b"---\nname: demo\n---\nline one\nline changed\n",
    );
    write(&left, "same.txt", b"same\n");
    write(&right, "same.txt", b"same\n");
    write(&left, "gone.txt", b"bye\n");
    write(&right, "nested/new.txt", b"hello\n");
    write(&left, ".git/config", b"ignored\n");

    let (files, unchanged) = diff_dirs(&left, &right).unwrap();
    assert_eq!(unchanged, 1);
    let summary: Vec<(String, DiffStatus)> = files
      .iter()
      .map(|file| (file.path.clone(), file.status.clone()))
      .collect();
    assert_eq!(
      summary,
      vec![
        ("SKILL.md".to_string(), DiffStatus::Modified),
        ("gone.txt".to_string(), DiffStatus::Removed),
        ("nested/new.txt".to_string(), DiffStatus::Added),
      ]
    );

    let modified = &files[0];
    assert!(!modified.binary && !modified.truncated);
    let kinds: Vec<(DiffLineKind, &str)> = modified.hunks[0]
      .lines
      .iter()
      .map(|line| (line.kind.clone(), line.text.as_str()))
      .collect();
    assert!(kinds.contains(&(DiffLineKind::Delete, "line two")));
    assert!(kinds.contains(&(DiffLineKind::Insert, "line changed")));

    let added = &files[2];
    assert!(added.hunks[0]
      .lines
      .iter()
      .all(|line| line.kind == DiffLineKind::Insert && line.old_line.is_none()));
  }

  #[test]
  fn binary_files_have_no_hunks() {
    let tmp = tempfile::tempdir().unwrap();
    let left = tmp.path().join("left");
    let right = tmp.path().join("right");
    write(&left, "img.bin", &[0, 1, 2, 3]);
    write(&right, "img.bin", &[0, 1, 2, 4]);

    let (files, unchanged) = diff_dirs(&left, &right).unwrap();
    assert_eq!(unchanged, 0);
    assert_eq!(files.len(), 1);
    assert!(files[0].binary);
    assert!(files[0].hunks.is_empty());
  }

  #[test]
  fn identical_directories_have_no_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let left = tmp.path().join("left");
    let right = tmp.path().join("right");
    write(&left, "SKILL.md", b"a\n");
    write(&right, "SKILL.md", b"a\n");
    let (files, unchanged) = diff_dirs(&left, &right).unwrap();
    assert!(files.is_empty());
    assert_eq!(unchanged, 1);
  }
}
