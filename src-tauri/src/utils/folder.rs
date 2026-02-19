use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FolderHelper;

impl FolderHelper {
  pub fn find_dirs_containing_file(root: &Path, file_name: &str) -> Result<Vec<PathBuf>, String> {
    if !root.exists() || !root.is_dir() {
      return Err(format!("Directory does not exist: {}", root.to_string_lossy()));
    }

    let mut dirs = Vec::new();
    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
      if entry.file_type().is_file() && entry.file_name() == file_name {
        let Some(parent) = entry.path().parent() else {
          continue;
        };
        dirs.push(parent.to_path_buf());
      }
    }

    Ok(dirs)
  }

  pub fn are_folders_identical(path_a: &Path, path_b: &Path) -> Result<bool, String> {
    if !path_a.exists() || !path_a.is_dir() {
      return Err(format!("Directory does not exist: {}", path_a.to_string_lossy()));
    }
    if !path_b.exists() || !path_b.is_dir() {
      return Err(format!("Directory does not exist: {}", path_b.to_string_lossy()));
    }
    Ok(Self::compute_folder_digest(path_a)? == Self::compute_folder_digest(path_b)?)
  }

  fn relative_file_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).follow_links(false).into_iter() {
      let entry = entry.map_err(|e| e.to_string())?;
      if !entry.file_type().is_file() {
        continue;
      }
      let rel = entry
        .path()
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .to_path_buf();
      files.push(rel);
    }
    files.sort();
    Ok(files)
  }

  fn compute_folder_digest(root: &Path) -> Result<String, String> {
    let files = Self::relative_file_paths(root)?;
    let mut hasher = Sha256::new();

    for rel in files {
      let rel_text = rel.to_string_lossy();
      hasher.update(rel_text.as_bytes());
      hasher.update([0]);

      let file_path = root.join(&rel);
      let file = File::open(&file_path).map_err(|e| e.to_string())?;
      let mut reader = BufReader::new(file);
      let mut buf = [0u8; 8192];
      loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
          break;
        }
        hasher.update(&buf[..n]);
      }
      hasher.update([0xff]);
    }

    let digest = hasher.finalize();
    Ok(format!("{:x}", digest))
  }
}
