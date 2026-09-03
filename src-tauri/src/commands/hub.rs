use crate::config::load_config;
use crate::models::{
  ActionResult, HubSkillView, ImportItem, ImportOutcome, InstallMode, InstallRequest,
  MigrationReport, ScanDecision, ScanItem, SkillSource, SourceUpdate, SyncAction, UninstallRequest,
};
use crate::services::env::Env;
use crate::services::scan_service::DEFAULT_SCAN_DEPTH;
use crate::services::{
  hub_service, install_service, migration_service, scan_service, source_service,
};
use crate::utils::folder::sweep_temp_dirs;
use crate::utils::github::GithubHelper;
use std::time::Duration;

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
pub async fn list_hub_skills() -> Result<Vec<HubSkillView>, String> {
  let env = Env::current()?;
  blocking("list_hub_skills", move || {
    migration_service::ensure_migrated(&env)?;
    hub_service::sweep_trash(&env);
    sweep_temp_dirs(Duration::from_secs(24 * 60 * 60));
    hub_service::list_hub_skills(&env)
  })
  .await
}

#[tauri::command]
pub async fn get_hub_skill(name: String) -> Result<Option<HubSkillView>, String> {
  let env = Env::current()?;
  blocking("get_hub_skill", move || {
    hub_service::get_hub_skill(&env, &name)
  })
  .await
}

#[tauri::command]
pub async fn import_skills(
  items: Vec<ImportItem>,
  overwrite: Option<bool>,
) -> Result<Vec<ImportOutcome>, String> {
  let env = Env::current()?;
  let items = fill_remote_shas(items).await;
  blocking("import_skills", move || {
    hub_service::import_skills(&env, items, overwrite.unwrap_or(false))
  })
  .await
}

/// GitHub sources imported without a tree sha (manual URL import) get one now so that
/// later update checks have a baseline. Best effort: network failures leave it empty.
async fn fill_remote_shas(items: Vec<ImportItem>) -> Vec<ImportItem> {
  let mut filled = Vec::with_capacity(items.len());
  for mut item in items {
    if let SkillSource::Github {
      url,
      skill_path,
      branch,
      remote_sha,
      ..
    } = &mut item.source
    {
      if remote_sha.as_deref().unwrap_or_default().is_empty() && !skill_path.is_empty() {
        if let Ok(sha) =
          GithubHelper::get_skill_folder_hash(url, skill_path, branch.as_deref()).await
        {
          *remote_sha = Some(sha);
        }
      }
    }
    filled.push(item);
  }
  filled
}

#[tauri::command]
pub async fn install_skill(request: InstallRequest) -> Result<ActionResult, String> {
  let env = Env::current()?;
  let mut request = request;
  if request.mode.is_none() {
    let config = load_config()?;
    request.mode = Some(InstallMode::from_setting(&config.sync_mode));
  }
  blocking("install_skill", move || {
    install_service::install_skill(&env, request)
  })
  .await
}

#[tauri::command]
pub async fn uninstall_skill(request: UninstallRequest) -> Result<ActionResult, String> {
  let env = Env::current()?;
  blocking("uninstall_skill", move || {
    install_service::uninstall_skill(&env, request)
  })
  .await
}

#[tauri::command]
pub async fn remove_hub_skill(name: String, remove_installs: bool) -> Result<(), String> {
  let env = Env::current()?;
  blocking("remove_hub_skill", move || {
    hub_service::remove_hub_skill(&env, &name, remove_installs)
  })
  .await
}

#[tauri::command]
pub async fn sync_skill(name: String, action: SyncAction) -> Result<ActionResult, String> {
  let env = Env::current()?;
  if let SyncAction::PullSource { force } = &action {
    let is_github = env
      .lock_path
      .is_file()
      .then(|| crate::services::lock_service::store(&env).get(&name))
      .transpose()?
      .flatten()
      .map(|record| record.source.github_repo().is_some())
      .unwrap_or(false);
    if is_github {
      return source_service::pull_github_source(env, name, *force).await;
    }
  }
  blocking("sync_skill", move || {
    hub_service::sync_skill(&env, &name, action)
  })
  .await
}

#[tauri::command]
pub async fn check_source_updates(names: Option<Vec<String>>) -> Result<Vec<SourceUpdate>, String> {
  let env = Env::current()?;
  source_service::check_source_updates(env, names).await
}

#[tauri::command]
pub async fn scan_folder(path: String, max_depth: Option<usize>) -> Result<Vec<ScanItem>, String> {
  let env = Env::current()?;
  blocking("scan_folder", move || {
    scan_service::scan_folder(&env, &path, max_depth.unwrap_or(DEFAULT_SCAN_DEPTH))
  })
  .await
}

#[tauri::command]
pub async fn import_scanned(decisions: Vec<ScanDecision>) -> Result<Vec<ImportOutcome>, String> {
  let env = Env::current()?;
  blocking("import_scanned", move || {
    scan_service::import_scanned(&env, decisions)
  })
  .await
}

#[tauri::command]
pub async fn migrate_legacy() -> Result<MigrationReport, String> {
  let env = Env::current()?;
  blocking("migrate_legacy", move || {
    migration_service::migrate_legacy(&env)
  })
  .await
}

#[tauri::command]
pub async fn migration_status() -> Result<Option<MigrationReport>, String> {
  let env = Env::current()?;
  blocking("migration_status", move || {
    Ok(crate::services::lock_service::store(&env).read()?.migration)
  })
  .await
}
