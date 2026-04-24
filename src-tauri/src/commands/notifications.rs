use crate::state::AppState;
use pebble_core::PebbleError;
use std::sync::atomic::Ordering;
use tauri::State;

#[tauri::command]
pub async fn set_notifications_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> std::result::Result<(), PebbleError> {
    state.notifications_enabled.store(enabled, Ordering::SeqCst);
    Ok(())
}
