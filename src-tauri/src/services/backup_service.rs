use crate::config::{load_config, save_config};
use crate::utils::path::canonical_skills_root;
use chrono::Local;
use std::fs;
use std::path::Path;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct BackupResult {
  pub success: bool,
  pub message: String,
  pub backup_path: Option<String>,
  pub backup_time: Option<String>,
}

/// Backup skills directory to a ZIP file
pub fn backup_skills_sync(backup_folder: String) -> Result<BackupResult, String> {
  let skills_path = canonical_skills_root()?;

  if !skills_path.exists() {
    return Ok(BackupResult {
      success: false,
      message: "Skills 目录不存在".to_string(),
      backup_path: None,
      backup_time: None,
    });
  }

  // Create backup directory
  let backup_path = Path::new(&backup_folder);
  if !backup_path.exists() {
    fs::create_dir_all(backup_path).map_err(|e| format!("创建备份目录失败: {}", e))?;
  }

  // Generate timestamped filename (yyyyMMddHHmmss)
  let now = Local::now();
  let timestamp = now.format("%Y%m%d%H%M%S").to_string();
  let filename = format!("skills_backup_{}.zip", timestamp);
  let backup_file_path = backup_path.join(&filename);

  // Create ZIP file
  let file = fs::File::create(&backup_file_path).map_err(|e| format!("创建备份文件失败: {}", e))?;
  let mut zip = zip::ZipWriter::new(file);
  let options: zip::write::FileOptions<()> =
    zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

  // Add skills directory to ZIP
  add_dir_to_zip(&mut zip, &skills_path, &skills_path, options)?;

  zip
    .finish()
    .map_err(|e| format!("完成 ZIP 文件失败: {}", e))?;

  // Format backup time for display
  let backup_time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

  Ok(BackupResult {
    success: true,
    message: "备份成功".to_string(),
    backup_path: Some(backup_file_path.to_string_lossy().to_string()),
    backup_time: Some(backup_time_str.clone()),
  })
}

pub fn open_backup_folder(path: String) -> Result<(), String> {
  let backup_path = Path::new(&path);
  if !backup_path.exists() {
    std::fs::create_dir_all(backup_path).map_err(|e| format!("创建备份目录失败: {}", e))?;
  }

  #[cfg(target_os = "macos")]
  {
    std::process::Command::new("open")
      .arg(&path)
      .spawn()
      .map_err(|e| format!("打开目录失败: {}", e))?;
  }

  #[cfg(target_os = "windows")]
  {
    std::process::Command::new("explorer")
      .arg(&path)
      .spawn()
      .map_err(|e| format!("打开目录失败: {}", e))?;
  }

  #[cfg(target_os = "linux")]
  {
    std::process::Command::new("xdg-open")
      .arg(&path)
      .spawn()
      .map_err(|e| format!("打开目录失败: {}", e))?;
  }

  Ok(())
}

pub async fn backup_skills(backup_folder: String) -> Result<BackupResult, String> {
  let result = tokio::task::spawn_blocking(move || backup_skills_sync(backup_folder))
    .await
    .map_err(|e| format!("备份任务执行失败: {}", e))??;

  if result.success {
    if let Ok(mut config) = load_config() {
      config.last_backup_time = result.backup_time.clone();
      let _ = save_config(&config);
    }
  }

  Ok(result)
}

fn add_dir_to_zip<P: AsRef<Path>>(
  zip: &mut zip::ZipWriter<fs::File>,
  base_path: P,
  current_path: P,
  options: zip::write::FileOptions<()>,
) -> Result<(), String> {
  let base_path = base_path.as_ref();
  let current_path = current_path.as_ref();

  for entry in fs::read_dir(current_path).map_err(|e| format!("读取目录失败: {}", e))? {
    let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
    let path = entry.path();
    let name = path
      .strip_prefix(base_path)
      .map_err(|_| format!("路径处理失败"))?
      .to_string_lossy()
      .to_string();

    if path.is_file() {
      let mut file = fs::File::open(&path).map_err(|e| format!("打开文件失败: {}", e))?;
      zip
        .start_file(name, options)
        .map_err(|e| format!("添加文件到 ZIP 失败: {}", e))?;
      std::io::copy(&mut file, zip).map_err(|e| format!("写入文件到 ZIP 失败: {}", e))?;
    } else if path.is_dir() {
      zip
        .add_directory(name.clone(), options)
        .map_err(|e| format!("添加目录到 ZIP 失败: {}", e))?;
      add_dir_to_zip(zip, base_path, &path, options)?;
    }
  }
  Ok(())
}
