use crate::services::github_auth_service::{self, GithubAuthStatus};

#[tauri::command]
pub async fn get_github_auth_status() -> Result<GithubAuthStatus, String> {
  tokio::task::spawn_blocking(github_auth_service::get_status)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_github_account(login: Option<String>) -> Result<GithubAuthStatus, String> {
  tokio::task::spawn_blocking(move || github_auth_service::set_account(login))
    .await
    .map_err(|e| e.to_string())?
}
