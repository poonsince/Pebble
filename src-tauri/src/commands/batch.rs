use crate::state::AppState;
use super::messages::{connect_gmail, connect_imap, connect_outlook, refresh_search_document, find_folder_by_role, find_message_folder, get_imap_config};
use pebble_core::traits::{FolderProvider, LabelProvider};
use pebble_core::{FolderRole, Message, PebbleError, ProviderType};
use pebble_mail::ImapProvider;
use std::collections::HashMap;
use tauri::State;
use tracing::{info, warn};

/// Group messages by account_id and resolve their provider type.
fn group_by_account(
    state: &AppState,
    message_ids: &[String],
) -> HashMap<String, (ProviderType, Vec<Message>)> {
    let mut groups: HashMap<String, (ProviderType, Vec<Message>)> = HashMap::new();

    for message_id in message_ids {
        let msg = match state.store.get_message(message_id) {
            Ok(Some(m)) => m,
            _ => continue,
        };
        let account_id = msg.account_id.clone();
        groups
            .entry(account_id.clone())
            .or_insert_with(|| {
                let provider = state
                    .store
                    .get_account(&account_id)
                    .ok()
                    .flatten()
                    .map(|a| a.provider)
                    .unwrap_or(ProviderType::Imap);
                (provider, Vec::new())
            })
            .1
            .push(msg);
    }
    groups
}

#[tauri::command]
pub async fn batch_archive(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
) -> std::result::Result<u32, PebbleError> {
    if message_ids.is_empty() {
        return Ok(0);
    }

    let groups = group_by_account(&state, &message_ids);
    let mut success_count: u32 = 0;
    let mut archived_ids = Vec::new();

    for (account_id, (provider_type, messages)) in &groups {
        let archive_folder = find_folder_by_role(&state, account_id, FolderRole::Archive).ok();

        // Remote sync per provider
        match provider_type {
            ProviderType::Gmail => {
                if let Ok(provider) = connect_gmail(&state, account_id).await {
                    for msg in messages {
                        if let Err(e) = provider.modify_labels(&msg.remote_id, &[], &["INBOX".to_string()]).await {
                            warn!("Gmail batch archive failed for {}: {e}", msg.id);
                        }
                    }
                }
            }
            ProviderType::Outlook => {
                if let Some(ref af) = archive_folder {
                    if !af.remote_id.starts_with("__local_") {
                        if let Ok(provider) = connect_outlook(&state, account_id).await {
                            for msg in messages {
                                if let Err(e) = provider.move_message(&msg.remote_id, &af.remote_id).await {
                                    warn!("Outlook batch archive failed for {}: {e}", msg.id);
                                }
                            }
                        }
                    }
                }
            }
            ProviderType::Imap => {
                if let Some(ref af) = archive_folder {
                    if !af.remote_id.starts_with("__local_") {
                        if let Ok(imap) = connect_imap(&state, account_id).await {
                            for msg in messages {
                                if let Ok(uid) = msg.remote_id.parse::<u32>() {
                                    if let Ok(src) = find_message_folder(&state, &msg.id, account_id) {
                                        if let Err(e) = imap.move_message(&src.remote_id, uid, &af.remote_id).await {
                                            warn!("IMAP batch archive failed for {}: {e}", msg.id);
                                        }
                                    }
                                }
                            }
                            let _ = imap.disconnect().await;
                        }
                    }
                }
            }
        }

        // Local store update
        for msg in messages {
            let result = match &archive_folder {
                Some(af) => state.store.move_message_to_folder(&msg.id, &af.id),
                None => state.store.soft_delete_message(&msg.id),
            };
            match result {
                Ok(()) => {
                    success_count += 1;
                    archived_ids.push(msg.id.clone());
                }
                Err(e) => warn!("Failed to archive message {}: {e}", msg.id),
            }
        }
    }

    // Update search index for archived messages (refresh, not remove)
    for id in &archived_ids {
        if let Err(e) = refresh_search_document(&state, id) {
            warn!("Failed to refresh search document for archived message {id}: {e}");
        }
    }

    info!(
        "Batch archive: {}/{} messages archived",
        success_count,
        message_ids.len()
    );
    Ok(success_count)
}

#[tauri::command]
pub async fn batch_delete(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
) -> std::result::Result<u32, PebbleError> {
    if message_ids.is_empty() {
        return Ok(0);
    }

    let groups = group_by_account(&state, &message_ids);

    // Remote sync per provider
    for (account_id, (provider_type, messages)) in &groups {
        match provider_type {
            ProviderType::Gmail => {
                if let Ok(provider) = connect_gmail(&state, account_id).await {
                    for msg in messages {
                        if let Err(e) = provider.trash_message(&msg.remote_id).await {
                            warn!("Gmail batch delete failed for {}: {e}", msg.id);
                        }
                    }
                }
            }
            ProviderType::Outlook => {
                if let Ok(provider) = connect_outlook(&state, account_id).await {
                    for msg in messages {
                        if let Err(e) = provider.trash_message(&msg.remote_id).await {
                            warn!("Outlook batch delete failed for {}: {e}", msg.id);
                        }
                    }
                }
            }
            ProviderType::Imap => {
                if let Ok(trash_folder) = find_folder_by_role(&state, account_id, FolderRole::Trash) {
                    if let Ok(imap) = connect_imap(&state, account_id).await {
                        for msg in messages {
                            if let Ok(uid) = msg.remote_id.parse::<u32>() {
                                if let Ok(src) = find_message_folder(&state, &msg.id, account_id) {
                                    if src.id != trash_folder.id {
                                        if let Err(e) = imap.move_message(&src.remote_id, uid, &trash_folder.remote_id).await {
                                            warn!("IMAP batch delete failed for {}: {e}", msg.id);
                                        }
                                    }
                                }
                            }
                        }
                        let _ = imap.disconnect().await;
                    }
                }
            }
        }
    }

    // Local bulk soft-delete
    state.store.bulk_soft_delete(&message_ids)?;
    let success_count = message_ids.len() as u32;

    // Update search index — remove deleted messages
    for id in &message_ids {
        if let Err(e) = state.search.remove_message(id) {
            warn!("Failed to remove deleted message {id} from search index: {e}");
        }
    }
    if let Err(e) = state.search.commit() {
        warn!("Failed to commit search index after batch delete: {e}");
    }

    info!(
        "Batch delete: {}/{} messages deleted",
        success_count,
        message_ids.len()
    );
    Ok(success_count)
}

#[tauri::command]
pub async fn batch_mark_read(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
    is_read: bool,
) -> std::result::Result<u32, PebbleError> {
    if message_ids.is_empty() {
        return Ok(0);
    }

    let groups = group_by_account(&state, &message_ids);

    // Remote sync per provider
    for (account_id, (provider_type, messages)) in &groups {
        match provider_type {
            ProviderType::Gmail => {
                if let Ok(provider) = connect_gmail(&state, account_id).await {
                    let (add, remove) = if is_read {
                        (vec![], vec!["UNREAD".to_string()])
                    } else {
                        (vec!["UNREAD".to_string()], vec![])
                    };
                    for msg in messages {
                        if let Err(e) = provider.modify_labels(&msg.remote_id, &add, &remove).await {
                            warn!("Gmail batch mark_read failed for {}: {e}", msg.id);
                        }
                    }
                }
            }
            ProviderType::Outlook => {
                if let Ok(provider) = connect_outlook(&state, account_id).await {
                    for msg in messages {
                        if let Err(e) = provider.update_read_status(&msg.remote_id, is_read).await {
                            warn!("Outlook batch mark_read failed for {}: {e}", msg.id);
                        }
                    }
                }
            }
            ProviderType::Imap => {
                if let Ok(imap_config) = get_imap_config(&state, account_id) {
                    let imap = ImapProvider::new(imap_config);
                    if let Ok(()) = imap.connect().await {
                        for msg in messages {
                            if let Ok(uid) = msg.remote_id.parse::<u32>() {
                                if let Ok(folder) = find_message_folder(&state, &msg.id, account_id) {
                                    if let Err(e) = imap.set_flags(&folder.remote_id, uid, Some(is_read), None).await {
                                        warn!("IMAP batch mark_read failed for {}: {e}", msg.id);
                                    }
                                }
                            }
                        }
                        let _ = imap.disconnect().await;
                    }
                }
            }
        }
    }

    // Local bulk flag update
    let changes: Vec<(String, Option<bool>, Option<bool>)> = message_ids
        .iter()
        .map(|id| (id.clone(), Some(is_read), None))
        .collect();
    state.store.bulk_update_flags(&changes)?;
    let success_count = message_ids.len() as u32;

    info!(
        "Batch mark_read({}): {}/{} messages updated",
        is_read,
        success_count,
        message_ids.len()
    );
    Ok(success_count)
}
