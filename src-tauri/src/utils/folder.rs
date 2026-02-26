use crate::utils::file::FileHelper;
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
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

  pub fn compute_folder_digest(root: &Path) -> Result<String, String> {
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

  pub fn quick_folder_signature(path: &Path) -> Result<(usize, u64, u64), String> {
    let mut entry_count: usize = 0;
    let mut total_size: u64 = 0;
    let mut latest_mtime_secs: u64 = 0;

    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
      let entry = entry.map_err(|e| e.to_string())?;
      entry_count += 1;
      let metadata = entry.metadata().map_err(|e| e.to_string())?;
      total_size = total_size.saturating_add(metadata.len());
      if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
          latest_mtime_secs = latest_mtime_secs.max(duration.as_secs());
        }
      }
    }

    Ok((entry_count, total_size, latest_mtime_secs))
  }

  pub fn compute_skill_folder_hash(skill_dir: &Path) -> Result<String, String> {
    if !skill_dir.exists() || !skill_dir.is_dir() {
      return Err(format!(
        "Skill directory does not exist: {}",
        skill_dir.to_string_lossy()
      ));
    }

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    for entry in WalkDir::new(skill_dir).into_iter().filter_map(Result::ok) {
      let path = entry.path();
      if path == skill_dir {
        continue;
      }

      let relative = path
        .strip_prefix(skill_dir)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .replace('\\', "/");

      if relative.split('/').any(|part| part == ".git" || part == "node_modules") {
        continue;
      }

      if entry.file_type().is_file() {
        let content = FileHelper::read_bytes(path)?;
        files.push((relative, content));
      }
    }

    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Sha256::new();
    for (relative_path, content) in files {
      hasher.update(relative_path.as_bytes());
      hasher.update(&content);
    }

    Ok(format!("{:x}", hasher.finalize()))
  }
}

pub fn copy_dir_all_sync(src: &Path, dst: &Path) -> Result<(), String> {
  fs::create_dir_all(dst).map_err(|e| e.to_string())?;
  for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());
    if src_path.is_dir() {
      copy_dir_all_sync(&src_path, &dst_path)?;
    } else {
      fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}
