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
      let outpath = dest_dir.join(file.name());

      if file.name().ends_with('/') {
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
