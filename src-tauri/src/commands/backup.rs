use chrono::Local;
use std::fs;
use std::path::Path;

use crate::config::{load_config, save_config};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct BackupResult {
  pub success: bool,
  pub message: String,
  pub backup_path: Option<String>,
  pub backup_time: Option<String>,
}

#[tauri::command]
pub fn get_backup_folder() -> Result<Option<String>, String> {
  let config = load_config()?;
  Ok(config.backup_folder)
}

#[tauri::command]
pub fn get_last_backup_time() -> Result<Option<String>, String> {
  let config = load_config()?;
  Ok(config.last_backup_time)
}

#[tauri::command]
pub fn set_backup_folder(path: String) -> Result<Option<String>, String> {
  let mut config = load_config()?;
  config.backup_folder = Some(path);
  save_config(&config)?;
  Ok(config.backup_folder)
}

#[tauri::command]
pub fn open_backup_folder(path: String) -> Result<(), String> {
  let backup_path = Path::new(&path);
  if !backup_path.exists() {
    fs::create_dir_all(backup_path).map_err(|e| format!("创建备份目录失败: {}", e))?;
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

#[tauri::command]
pub async fn backup_skills(backup_folder: String) -> Result<BackupResult, String> {
  // 使用 spawn_blocking 将耗时操作放在单独线程执行
  let result = tokio::task::spawn_blocking(move || {
    let home = dirs_next::home_dir().ok_or("无法获取用户目录")?;
    let skills_path = home.join(".agents").join("skills");

    if !skills_path.exists() {
      return Ok(BackupResult {
        success: false,
        message: "Skills 目录不存在".to_string(),
        backup_path: None,
        backup_time: None,
      });
    }

    // 创建备份目录
    let backup_path = Path::new(&backup_folder);
    if !backup_path.exists() {
      fs::create_dir_all(backup_path).map_err(|e| format!("创建备份目录失败: {}", e))?;
    }

    // 生成带时间戳的文件名 (yyyyMMddHHmmss)
    let now = Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let filename = format!("skills_backup_{}.zip", timestamp);
    let backup_file_path = backup_path.join(&filename);

    // 创建 ZIP 文件
    let file =
      fs::File::create(&backup_file_path).map_err(|e| format!("创建备份文件失败: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> =
      zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 遍历 skills 目录并添加到 ZIP
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

    add_dir_to_zip(&mut zip, &skills_path, &skills_path, options)?;

    zip
      .finish()
      .map_err(|e| format!("完成 ZIP 文件失败: {}", e))?;

    // 格式化备份时间显示
    let backup_time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

    Ok(BackupResult {
      success: true,
      message: "备份成功".to_string(),
      backup_path: Some(backup_file_path.to_string_lossy().to_string()),
      backup_time: Some(backup_time_str.clone()),
    })
  })
  .await
  .map_err(|e| format!("备份任务执行失败: {}", e))?;

  // 如果备份成功，保存备份时间到配置
  if let Ok(ref backup_result) = result {
    if backup_result.success {
      if let Ok(mut config) = load_config() {
        config.last_backup_time = backup_result.backup_time.clone();
        let _ = save_config(&config);
      }
    }
  }

  result
}
