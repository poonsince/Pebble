use crate::state::AppState;
use pebble_core::PebbleError;
use pebble_store::cloud_sync::WebDavClient;
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

#[tauri::command]
pub async fn restore_from_webdav(
    state: State<'_, AppState>,
    url: String,
    username: String,
    password: String,
) -> std::result::Result<String, PebbleError> {
    let client = WebDavClient::new(url, username, password)?;
    let data = client.download(BACKUP_FILENAME).await?;
    state.store.import_settings(&data)?;
    Ok("Settings backup restored. Reconnect accounts to continue syncing.".to_string())
}
