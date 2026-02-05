use crate::models::{InstallRequest, InstallResult};
use std::process::Command;

#[tauri::command]
pub fn install_skill(request: InstallRequest) -> Result<InstallResult, String> {
  let npx_cmd = if cfg!(target_os = "windows") {
    "npx.cmd"
  } else {
    "npx"
  };

  let mut command = Command::new(npx_cmd);
  command.arg("skills").arg("add").arg(&request.source);
  command.arg("--skill").arg(&request.skill_id);
  command.arg("--agent").arg(&request.agent);
  command.arg("--yes");

  if request.global {
    command.arg("--global");
  }

  if let Some(project_dir) = request.project_dir {
    if !project_dir.trim().is_empty() {
      command.current_dir(project_dir);
    }
  }

  let output = command.output().map_err(|e| e.to_string())?;

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  let success = output.status.success();

  Ok(InstallResult {
    success,
    stdout,
    stderr,
    message: if success {
      "安装成功".to_string()
    } else {
      "安装失败".to_string()
    },
  })
}
