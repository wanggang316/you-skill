//! Library auto sync on a schedule (`docs/decisions/0005-library-auto-sync.md`): one cycle at
//! app start and one per interval, also while the window is hidden in the tray. The timer
//! restarts when the setting changes. A cycle is skipped while a manual library sync
//! command runs or a library sync dialog is open. The cycle logic is in
//! `library_sync_service::auto_sync`.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use tauri::{App, AppHandle, Emitter, Manager};
use youskill_core::config::{load_config, AutoSyncSettings};
use youskill_core::services::agent_apps_service::refresh_local_agent_apps;
use youskill_core::services::env::Env;
use youskill_core::services::library_sync_service::{
  self, AutoSyncOptions, AutoSyncOutcome, AutoSyncResult,
};
use youskill_core::utils::time::now_rfc3339;

/// Sent after each cycle and when the last result is cleared, with the `AutoSyncStatus`
/// (or `null`).
pub const CHANGED_EVENT: &str = "library-sync://auto-synced";

/// Library sync commands that run now, see [`ManualSync`].
static MANUAL: AtomicUsize = AtomicUsize::new(0);

/// Marks a manual library sync command as running while it lives.
pub struct ManualSync(());

impl ManualSync {
  pub fn start() -> Self {
    MANUAL.fetch_add(1, Ordering::SeqCst);
    Self(())
  }
}

impl Drop for ManualSync {
  fn drop(&mut self) {
    MANUAL.fetch_sub(1, Ordering::SeqCst);
  }
}

#[derive(Default)]
pub struct AutoSync {
  /// The running timer; a timer with an earlier value stops.
  timer: AtomicU64,
  running: AtomicBool,
  dialog_open: AtomicBool,
  /// The last cycle, kept in memory only.
  last: Mutex<Option<AutoSyncStatus>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoSyncStatus {
  pub at: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub outcome: Option<AutoSyncOutcome>,
  /// The cycle failed with this error.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

pub fn setup(app: &App) {
  app.manage(AutoSync::default());
  restart(app.handle(), true);
}

/// The last cycle, for a page that starts listening after it.
pub fn last(app: &AppHandle) -> Option<AutoSyncStatus> {
  app.state::<AutoSync>().last.lock().ok()?.clone()
}

pub fn set_dialog_open(app: &AppHandle, open: bool) {
  app
    .state::<AutoSync>()
    .dialog_open
    .store(open, Ordering::SeqCst);
}

/// Forget the last result after the user synced manually, so the badge goes away; the next
/// cycle reports again.
pub fn clear(app: &AppHandle) {
  let state = app.state::<AutoSync>();
  let Ok(mut last) = state.last.lock() else {
    return;
  };
  if last.take().is_some() {
    drop(last);
    emit(app, None);
  }
}

/// Settings changed from `before` to `after`: restart the timer when they differ. Turning
/// auto sync on runs a cycle now; another change waits one interval.
pub fn settings_changed(app: &AppHandle, before: &AutoSyncSettings, after: &AutoSyncSettings) {
  if before == after {
    return;
  }
  if !after.enabled {
    clear(app);
  }
  restart(app, !before.enabled && after.enabled);
}

fn settings() -> AutoSyncSettings {
  load_config()
    .map(|config| config.auto_sync)
    .unwrap_or_default()
}

/// Stop the running timer and start one for the current settings.
fn restart(app: &AppHandle, run_now: bool) {
  let state = app.state::<AutoSync>();
  let timer = state.timer.fetch_add(1, Ordering::SeqCst) + 1;
  let settings = settings();
  if !settings.enabled {
    return;
  }
  let app = app.clone();
  tauri::async_runtime::spawn(async move {
    let current = || app.state::<AutoSync>().timer.load(Ordering::SeqCst) == timer;
    if !run_now {
      tokio::time::sleep(settings.interval.duration()).await;
    }
    while current() {
      run(&app).await;
      tokio::time::sleep(settings.interval.duration()).await;
    }
  });
}

async fn run(app: &AppHandle) {
  let state = app.state::<AutoSync>();
  if MANUAL.load(Ordering::SeqCst) > 0 || state.dialog_open.load(Ordering::SeqCst) {
    tracing::info!("Auto sync skipped: a manual library sync is in progress");
    return;
  }
  if state.running.swap(true, Ordering::SeqCst) {
    return;
  }
  let settings = settings();
  let options = AutoSyncOptions {
    restore: settings.restore,
    backup: settings.backup,
    policy: settings.policy,
  };
  let result = tokio::task::spawn_blocking(move || {
    let outcome = library_sync_service::auto_sync(&Env::current()?, &options)?;
    if outcome.agents_changed {
      refresh_local_agent_apps();
    }
    Ok::<_, String>(outcome)
  })
  .await
  .map_err(|e| e.to_string())
  .and_then(|result| result);
  state.running.store(false, Ordering::SeqCst);

  let status = match result {
    // A skipped cycle says nothing new; the last result stays.
    Ok(outcome) if outcome.result == AutoSyncResult::Skipped => return,
    // Nothing to show: the library is not connected.
    Ok(outcome) if outcome.result == AutoSyncResult::NotConnected => None,
    Ok(outcome) => Some(AutoSyncStatus {
      at: now_rfc3339(),
      outcome: Some(outcome),
      error: None,
    }),
    Err(error) => {
      tracing::warn!("Auto sync failed: {error}");
      Some(AutoSyncStatus {
        at: now_rfc3339(),
        outcome: None,
        error: Some(error),
      })
    },
  };
  if let Ok(mut last) = state.last.lock() {
    *last = status.clone();
  }
  emit(app, status);
}

fn emit(app: &AppHandle, status: Option<AutoSyncStatus>) {
  if let Err(error) = app.emit(CHANGED_EVENT, status) {
    tracing::warn!("Failed to send the auto sync result: {error}");
  }
}
