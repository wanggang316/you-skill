use std::fs;
use std::path::Path;

pub struct ZipHelper;

impl ZipHelper {
  pub fn extract_to_dir(zip_path: &str, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Failed to open ZIP: {}", e))?;
    let mut archive =
      zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP: {}", e))?;

    for i in 0..archive.len() {
      let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
      // `enclosed_name` rejects absolute paths and `..` components (zip-slip).
      let relative = file
        .enclosed_name()
        .ok_or_else(|| format!("Invalid ZIP entry path: {}", file.name()))?;
      let outpath = dest_dir.join(relative);

      if file.is_dir() {
        fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
      } else {
        if let Some(p) = outpath.parent() {
          if !p.exists() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
          }
        }
        let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
        std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;

  fn build_zip(path: &Path, entries: &[(&str, &str)]) {
    let file = fs::File::create(path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
    for (name, content) in entries {
      writer.start_file(*name, options).unwrap();
      writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
  }

  #[test]
  fn extracts_regular_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("a.zip");
    build_zip(&zip_path, &[("skill/SKILL.md", "hello")]);
    let dest = tmp.path().join("out");
    ZipHelper::extract_to_dir(&zip_path.to_string_lossy(), &dest).unwrap();
    assert_eq!(
      fs::read_to_string(dest.join("skill/SKILL.md")).unwrap(),
      "hello"
    );
  }

  #[test]
  fn rejects_path_traversal() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("evil.zip");
    build_zip(&zip_path, &[("../evil.txt", "x")]);
    let dest = tmp.path().join("out");
    assert!(ZipHelper::extract_to_dir(&zip_path.to_string_lossy(), &dest).is_err());
    assert!(!tmp.path().join("evil.txt").exists());
  }
}
