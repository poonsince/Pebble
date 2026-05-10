use crate::state::AppState;
use pebble_core::{now_timestamp, PebbleError, UntrustedSender};
use tauri::State;

#[tauri::command]
pub async fn add_untrusted_sender(
    state: State<'_, AppState>,
    account_id: String,
    email: String,
) -> std::result::Result<(), PebbleError> {
    let sender = UntrustedSender {
        account_id,
        email,
        created_at: now_timestamp(),
    };
    state.store.add_untrusted_sender(&sender)
}

#[tauri::command]
pub async fn list_untrusted_senders(
    state: State<'_, AppState>,
    account_id: String,
) -> std::result::Result<Vec<UntrustedSender>, PebbleError> {
    state.store.list_untrusted_senders(&account_id)
}

#[tauri::command]
pub async fn remove_untrusted_sender(
    state: State<'_, AppState>,
    account_id: String,
    email: String,
) -> std::result::Result<(), PebbleError> {
    state.store.remove_untrusted_sender(&account_id, &email)
}
