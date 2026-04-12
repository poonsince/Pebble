use crate::state::AppState;
use pebble_core::PebbleError;
use pebble_store::cloud_sync::{preview_backup, BackupPreview, WebDavClient};
use tauri::State;

const BACKUP_FILENAME: &str = "pebble-settings-backup.json";

#[tauri::command]
pub async fn test_webdav_connection(
    url: String,
    username: String,
    password: String,
) -> std::result::Result<String, PebbleError> {
    let client = WebDavClient::new(url, username, password)?;
    client.test_connection().await?;
    Ok("Connection successful".to_string())
}

#[tauri::command]
pub async fn backup_to_webdav(
    state: State<'_, AppState>,
    url: String,
    username: String,
    password: String,
) -> std::result::Result<String, PebbleError> {
    let data = state.store.export_settings()?;
    let client = WebDavClient::new(url, username, password)?;
    client.upload(BACKUP_FILENAME, &data).await?;
    Ok("Settings backup completed successfully".to_string())
}

/// Download the backup and return a summary so the user can review the
/// contents before committing to a restore. Enforces size limits and schema
/// version validation in `download` and `preview_backup`.
#[tauri::command]
pub async fn preview_webdav_backup(
    url: String,
    username: String,
    password: String,
) -> std::result::Result<BackupPreview, PebbleError> {
    let client = WebDavClient::new(url, username, password)?;
    let data = client.download(BACKUP_FILENAME).await?;
    preview_backup(&data)
}

#[tauri::command]
pub async fn restore_from_webdav(
    state: State<'_, AppState>,
    url: String,
    username: String,
    password: String,
) -> std::result::Result<String, PebbleError> {
    let client = WebDavClient::new(url, username, password)?;
    let data = client.download(BACKUP_FILENAME).await?;
    // Re-validate before import; `import_settings` enforces size + version too.
    let _ = preview_backup(&data)?;
    state.store.import_settings(&data)?;
    Ok("Settings backup restored. Reconnect accounts to continue syncing.".to_string())
}
