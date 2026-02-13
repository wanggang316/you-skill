use std::fs;
use std::path::Path;

pub struct FileHelper;

impl FileHelper {
  pub fn read_to_string(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
  }

  pub fn metadata(path: &Path) -> Result<fs::Metadata, String> {
    fs::metadata(path).map_err(|e| e.to_string())
  }
}
