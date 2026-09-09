use crate::config::load_config;
use crate::models::{
  InstallMode, InstallRequest, InstructionActionResult, InstructionImportItem,
  InstructionImportOutcome, InstructionScanDecision, InstructionScanItem, InstructionView,
  SkillDiff, SyncAction, UninstallRequest,
};
use crate::services::env::Env;
use crate::services::instruction_service;

async fn blocking<T, F>(label: &'static str, f: F) -> Result<T, String>
where
  T: Send + 'static,
  F: FnOnce() -> Result<T, String> + Send + 'static,
{
  tauri::async_runtime::spawn_blocking(f)
    .await
    .map_err(|e| format!("{} join error: {}", label, e))?
}

#[tauri::command]
pub async fn list_instructions() -> Result<Vec<InstructionView>, String> {
  let env = Env::current()?;
  blocking("list_instructions", move || {
    instruction_service::list_instructions(&env)
  })
  .await
}

#[tauri::command]
pub async fn get_instruction(name: String) -> Result<Option<InstructionView>, String> {
  let env = Env::current()?;
  blocking("get_instruction", move || {
    instruction_service::get_instruction(&env, &name)
  })
  .await
}

#[tauri::command]
pub async fn read_instruction(name: String) -> Result<String, String> {
  let env = Env::current()?;
  blocking("read_instruction", move || {
    instruction_service::read_instruction(&env, &name)
  })
  .await
}

#[tauri::command]
pub async fn import_instructions(
  items: Vec<InstructionImportItem>,
  overwrite: Option<bool>,
) -> Result<Vec<InstructionImportOutcome>, String> {
  let env = Env::current()?;
  blocking("import_instructions", move || {
    instruction_service::import_instructions(&env, items, overwrite.unwrap_or(false))
  })
  .await
}

#[tauri::command]
pub async fn create_instruction(
  name: String,
  content: Option<String>,
) -> Result<InstructionImportOutcome, String> {
  let env = Env::current()?;
  blocking("create_instruction", move || {
    instruction_service::create_instruction(&env, &name, content.as_deref().unwrap_or(""))
  })
  .await
}

#[tauri::command]
pub async fn install_instruction(
  request: InstallRequest,
) -> Result<InstructionActionResult, String> {
  let env = Env::current()?;
  let mut request = request;
  if request.mode.is_none() {
    let config = load_config()?;
    request.mode = Some(InstallMode::from_setting(&config.sync_mode));
  }
  blocking("install_instruction", move || {
    instruction_service::install_instruction(&env, request)
  })
  .await
}

#[tauri::command]
pub async fn uninstall_instruction(
  request: UninstallRequest,
) -> Result<InstructionActionResult, String> {
  let env = Env::current()?;
  blocking("uninstall_instruction", move || {
    instruction_service::uninstall_instruction(&env, request)
  })
  .await
}

#[tauri::command]
pub async fn remove_instruction(name: String, remove_installs: bool) -> Result<(), String> {
  let env = Env::current()?;
  blocking("remove_instruction", move || {
    instruction_service::remove_instruction(&env, &name, remove_installs)
  })
  .await
}

#[tauri::command]
pub async fn sync_instruction(
  name: String,
  action: SyncAction,
) -> Result<InstructionActionResult, String> {
  let env = Env::current()?;
  blocking("sync_instruction", move || {
    instruction_service::sync_instruction(&env, &name, action)
  })
  .await
}

#[tauri::command]
pub async fn diff_instruction(name: String, path: String) -> Result<SkillDiff, String> {
  let env = Env::current()?;
  blocking("diff_instruction", move || {
    instruction_service::diff_instruction(&env, &name, &path)
  })
  .await
}

#[tauri::command]
pub async fn scan_instruction_files() -> Result<Vec<InstructionScanItem>, String> {
  let env = Env::current()?;
  blocking("scan_instruction_files", move || {
    instruction_service::scan_instruction_files(&env)
  })
  .await
}

#[tauri::command]
pub async fn import_scanned_instructions(
  decisions: Vec<InstructionScanDecision>,
) -> Result<Vec<InstructionImportOutcome>, String> {
  let env = Env::current()?;
  blocking("import_scanned_instructions", move || {
    instruction_service::import_scanned_instructions(&env, decisions)
  })
  .await
}
