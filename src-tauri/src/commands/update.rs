use crate::config::{load_config, save_config, AppConfig, UpdateInfo};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const UPDATE_CHECK_INTERVAL_SECS: u64 = 3600; // 1 hour
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateCheckResult {
    pub should_check: bool,
    pub update_info: Option<UpdateInfo>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Check if we should check for updates based on the last check time
fn should_check_update(config: &AppConfig) -> bool {
    let last_check = config.last_update_check.unwrap_or(0);
    let elapsed = now_secs().saturating_sub(last_check);
    elapsed >= UPDATE_CHECK_INTERVAL_SECS
}

/// Fetch latest version info from update server
/// This is a placeholder - replace with actual update server URL
async fn fetch_latest_version() -> Result<UpdateInfo, String> {
    // TODO: Replace with your actual update server endpoint
    // For now, we'll simulate by returning no update
    // Example implementation:
    // let client = reqwest::Client::new();
    // let response = client
    //     .get("https://your-update-server.com/api/latest")
    //     .header("X-API-Key", api_key.unwrap_or_default())
    //     .send()
    //     .await
    //     .map_err(|e| format!("Failed to fetch update: {}", e))?;
    //
    // let data: LatestVersionResponse = response
    //     .json()
    //     .await
    //     .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Placeholder: no update available
    Ok(UpdateInfo {
        has_update: false,
        current_version: CURRENT_VERSION.to_string(),
        latest_version: CURRENT_VERSION.to_string(),
        download_url: String::new(),
        release_notes: String::new(),
    })
}

/// Check for updates (respects the 1-hour cooldown)
#[tauri::command]
pub async fn check_for_update() -> Result<UpdateCheckResult, String> {
    let mut config = load_config()?;

    // Check if we should perform a check
    if !should_check_update(&config) {
        // Return cached update info if available
        let update_info = config.pending_update.clone();
        return Ok(UpdateCheckResult {
            should_check: false,
            update_info,
        });
    }

    // Perform the actual check
    let update_info = fetch_latest_version().await?;

    // Update last check time
    config.last_update_check = Some(now_secs());

    // Store update info if there's an update pending
    if update_info.has_update {
        config.pending_update = Some(update_info.clone());
    } else {
        config.pending_update = None;
    }

    save_config(&config)?;

    Ok(UpdateCheckResult {
        should_check: true,
        update_info: if update_info.has_update {
            Some(update_info)
        } else {
            None
        },
    })
}

/// Force check for updates (ignores cooldown - used in settings page)
#[tauri::command]
pub async fn force_check_for_update() -> Result<Option<UpdateInfo>, String> {
    let mut config = load_config()?;

    let update_info = fetch_latest_version().await?;

    // Update last check time
    config.last_update_check = Some(now_secs());

    // Store update info if there's an update pending
    if update_info.has_update {
        config.pending_update = Some(update_info.clone());
        save_config(&config)?;
        Ok(Some(update_info))
    } else {
        config.pending_update = None;
        save_config(&config)?;
        Ok(None)
    }
}

/// Get current app version
#[tauri::command]
pub fn get_app_version() -> String {
    CURRENT_VERSION.to_string()
}

/// Get pending update info (if any)
#[tauri::command]
pub fn get_pending_update() -> Result<Option<UpdateInfo>, String> {
    let config = load_config()?;
    Ok(config.pending_update)
}

/// Clear pending update (called after successful update)
#[tauri::command]
pub fn clear_pending_update() -> Result<(), String> {
    let mut config = load_config()?;
    config.pending_update = None;
    save_config(&config)
}

/// Download and install update
#[tauri::command]
pub async fn download_and_install_update(download_url: String) -> Result<(), String> {
    // TODO: Implement actual download and install logic
    // This would typically:
    // 1. Download the update package
    // 2. Verify the signature/checksum
    // 3. Apply the update (platform-specific)
    // 4. Restart the app

    // Placeholder implementation
    let _ = download_url;
    Err("Auto-update not yet implemented".to_string())
}
