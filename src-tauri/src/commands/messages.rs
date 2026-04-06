use crate::state::AppState;
use crate::commands::oauth::ensure_account_oauth_tokens;
use pebble_core::traits::LabelProvider;
use pebble_core::{FolderRole, Message, MessageSummary, PebbleError, PrivacyMode, ProviderType, RenderedHtml, TrustType};
use pebble_mail::{GmailProvider, ImapConfig, ImapProvider};
use pebble_privacy::PrivacyGuard;
use tauri::State;
use tracing::{info, warn};

async fn connect_gmail(
    state: &AppState,
    account_id: &str,
) -> std::result::Result<GmailProvider, PebbleError> {
    let tokens = ensure_account_oauth_tokens(state, account_id, "gmail").await?;
    Ok(GmailProvider::new(tokens.access_token))
}

fn refresh_search_document(
    state: &AppState,
    message_id: &str,
) -> std::result::Result<(), PebbleError> {
    match state.store.get_message(message_id)? {
        Some(message) if !message.is_deleted => {
            let folder_ids = state.store.get_message_folder_ids(message_id)?;
            if folder_ids.is_empty() {
                state.search.remove_message(message_id)?;
            } else {
                state.search.index_message(&message, &folder_ids)?;
            }
        }
        Some(_) | None => {
            state.search.remove_message(message_id)?;
        }
    }

    state.search.commit()?;
    Ok(())
}

fn remove_search_documents(
    state: &AppState,
    message_ids: &[String],
) -> std::result::Result<(), PebbleError> {
    for message_id in message_ids {
        state.search.remove_message(message_id)?;
    }
    if !message_ids.is_empty() {
        state.search.commit()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_messages(
    state: State<'_, AppState>,
    folder_id: String,
    folder_ids: Option<Vec<String>>,
    limit: u32,
    offset: u32,
) -> std::result::Result<Vec<MessageSummary>, PebbleError> {
    match folder_ids {
        Some(ids) if !ids.is_empty() => state.store.list_messages_by_folders(&ids, limit, offset),
        _ => state.store.list_messages_by_folder(&folder_id, limit, offset),
    }
}

#[tauri::command]
pub async fn list_starred_messages(
    state: State<'_, AppState>,
    account_id: String,
    limit: u32,
    offset: u32,
) -> std::result::Result<Vec<MessageSummary>, PebbleError> {
    state.store.list_starred_messages(&account_id, limit, offset)
}

#[tauri::command]
pub async fn get_message(
    state: State<'_, AppState>,
    message_id: String,
) -> std::result::Result<Option<Message>, PebbleError> {
    state.store.get_message(&message_id)
}

#[tauri::command]
pub async fn get_messages_batch(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
) -> std::result::Result<Vec<Message>, PebbleError> {
    state.store.get_messages_batch(&message_ids)
}

#[tauri::command]
pub async fn get_rendered_html(
    state: State<'_, AppState>,
    message_id: String,
    privacy_mode: PrivacyMode,
) -> std::result::Result<RenderedHtml, PebbleError> {
    let message = state
        .store
        .get_message(&message_id)?
        .ok_or_else(|| PebbleError::Internal(format!("Message not found: {message_id}")))?;

    // If the caller requested Strict mode, check if the sender is trusted
    // and automatically relax privacy based on trust level.
    let effective_mode = if matches!(privacy_mode, PrivacyMode::Strict) {
        match state
            .store
            .is_trusted_sender(&message.account_id, &message.from_address)?
        {
            Some(TrustType::All) => PrivacyMode::TrustSender(message.from_address.clone()),
            Some(TrustType::Images) => PrivacyMode::LoadOnce,
            None => privacy_mode,
        }
    } else {
        privacy_mode
    };

    let guard = PrivacyGuard::new();
    let rendered = guard.render_safe_html(&message.body_html_raw, &effective_mode);
    Ok(rendered)
}

#[tauri::command]
pub async fn get_message_with_html(
    state: State<'_, AppState>,
    message_id: String,
    privacy_mode: PrivacyMode,
) -> std::result::Result<Option<(Message, RenderedHtml)>, PebbleError> {
    let message = match state.store.get_message(&message_id)? {
        Some(m) => m,
        None => return Ok(None),
    };

    let effective_mode = if matches!(privacy_mode, PrivacyMode::Strict) {
        match state
            .store
            .is_trusted_sender(&message.account_id, &message.from_address)?
        {
            Some(TrustType::All) => PrivacyMode::TrustSender(message.from_address.clone()),
            Some(TrustType::Images) => PrivacyMode::LoadOnce,
            None => privacy_mode,
        }
    } else {
        privacy_mode
    };

    let guard = PrivacyGuard::new();
    let rendered = guard.render_safe_html(&message.body_html_raw, &effective_mode);
    Ok(Some((message, rendered)))
}

#[tauri::command]
pub async fn is_trusted_sender(
    state: State<'_, AppState>,
    account_id: String,
    email: String,
) -> std::result::Result<bool, PebbleError> {
    Ok(state.store.is_trusted_sender(&account_id, &email)?.is_some())
}

#[tauri::command]
pub async fn update_message_flags(
    state: State<'_, AppState>,
    message_id: String,
    is_read: Option<bool>,
    is_starred: Option<bool>,
) -> std::result::Result<(), PebbleError> {
    // 1. Local update
    state
        .store
        .update_message_flags(&message_id, is_read, is_starred)?;

    // 2. Provider-specific remote writeback (fire-and-forget)
    if let Ok(Some(msg)) = state.store.get_message(&message_id) {
        let provider_type = state
            .store
            .get_account(&msg.account_id)?
            .map(|account| account.provider);

        match provider_type {
            Some(ProviderType::Gmail) => {
                let mut add = Vec::new();
                let mut remove = Vec::new();
                if let Some(read) = is_read {
                    if read {
                        remove.push("UNREAD".to_string());
                    } else {
                        add.push("UNREAD".to_string());
                    }
                }
                if let Some(starred) = is_starred {
                    if starred {
                        add.push("STARRED".to_string());
                    } else {
                        remove.push("STARRED".to_string());
                    }
                }

                if !add.is_empty() || !remove.is_empty() {
                    let remote_id = msg.remote_id.clone();
                    if let Ok(provider) = connect_gmail(&state, &msg.account_id).await {
                        tokio::task::spawn(async move {
                            if let Err(e) = provider.modify_labels(&remote_id, &add, &remove).await {
                                warn!("Gmail flag writeback failed: {e}");
                            }
                        });
                    }
                }
            }
            Some(ProviderType::Imap) | Some(ProviderType::Outlook) | None => {
                if let Ok(folder) = find_message_folder(&state, &message_id, &msg.account_id) {
                    if let Ok(uid) = msg.remote_id.parse::<u32>() {
                        if let Ok(imap_config) = get_imap_config(&state, &msg.account_id) {
                            let remote_id = folder.remote_id.clone();
                            tokio::task::spawn(async move {
                                let provider = ImapProvider::new(imap_config);
                                if let Err(e) = provider.connect().await {
                                    warn!("IMAP flag writeback connect failed: {e}");
                                    return;
                                }
                                if let Err(e) = provider.set_flags(&remote_id, uid, is_read, is_starred).await {
                                    warn!("IMAP flag writeback failed: {e}");
                                }
                                let _ = provider.disconnect().await;
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Extract the IMAP config for an account (without connecting).
fn get_imap_config(state: &AppState, account_id: &str) -> std::result::Result<ImapConfig, PebbleError> {
    if let Some(encrypted) = state.store.get_auth_data(account_id)? {
        let decrypted = state.crypto.decrypt(&encrypted)?;
        let value: serde_json::Value = serde_json::from_slice(&decrypted)
            .map_err(|e| PebbleError::Internal(format!("Failed to parse config: {e}")))?;
        serde_json::from_value(value.get("imap").cloned().unwrap_or(value.clone()))
            .map_err(|e| PebbleError::Internal(format!("Failed to deserialize IMAP config: {e}")))
    } else {
        let sync_json = state.store.get_account_sync_state(account_id)?
            .ok_or_else(|| PebbleError::Internal(format!("No config for account {account_id}")))?;
        let value: serde_json::Value = serde_json::from_str(&sync_json)
            .map_err(|e| PebbleError::Internal(format!("Failed to parse sync state: {e}")))?;
        serde_json::from_value(value.get("imap").cloned().unwrap_or(value))
            .map_err(|e| PebbleError::Internal(format!("Failed to deserialize IMAP config: {e}")))
    }
}

/// Resolve an IMAP connection from the account's auth data.
async fn connect_imap(state: &AppState, account_id: &str) -> std::result::Result<ImapProvider, PebbleError> {
    let imap_config = get_imap_config(state, account_id)?;
    let provider = ImapProvider::new(imap_config);
    provider.connect().await?;
    Ok(provider)
}

/// Find the folder with a given role for an account.
fn find_folder_by_role(state: &AppState, account_id: &str, role: FolderRole) -> std::result::Result<pebble_core::Folder, PebbleError> {
    let folders = state.store.list_folders(account_id)?;
    folders.into_iter()
        .find(|f| f.role == Some(role.clone()))
        .ok_or_else(|| PebbleError::Internal(format!("No {:?} folder found", role)))
}

/// Find the folder containing a given message (via the message_folders junction table).
fn find_message_folder(state: &AppState, message_id: &str, account_id: &str) -> std::result::Result<pebble_core::Folder, PebbleError> {
    let folder_ids = state.store.get_message_folder_ids(message_id)?;
    if folder_ids.is_empty() {
        return Err(PebbleError::Internal("Message not found in any folder".to_string()));
    }
    let folders = state.store.list_folders(account_id)?;
    // Return the first matching folder (prefer inbox-like folders)
    for fid in &folder_ids {
        if let Some(folder) = folders.iter().find(|f| &f.id == fid) {
            return Ok(folder.clone());
        }
    }
    Err(PebbleError::Internal("Message folder not found".to_string()))
}

/// Returns "archived" or "unarchived" so the frontend can show the correct toast.
#[tauri::command]
pub async fn archive_message(
    state: State<'_, AppState>,
    message_id: String,
) -> std::result::Result<String, PebbleError> {
    let msg = state.store.get_message(&message_id)?
        .ok_or_else(|| PebbleError::Internal(format!("Message not found: {message_id}")))?;
    let provider_type = state
        .store
        .get_account(&msg.account_id)?
        .map(|account| account.provider)
        .ok_or_else(|| PebbleError::Internal(format!("Account not found: {}", msg.account_id)))?;

    let source_folder = find_message_folder(&state, &message_id, &msg.account_id)?;
    // If the message is already in an archive folder, unarchive it (move to inbox)
    if source_folder.role == Some(FolderRole::Archive) {
        info!("Message {} already in archive, restoring to inbox", message_id);
        let inbox = find_folder_by_role(&state, &msg.account_id, FolderRole::Inbox)?;
        if matches!(provider_type, ProviderType::Gmail) {
            if let Ok(provider) = connect_gmail(&state, &msg.account_id).await {
                if let Err(e) = provider.modify_labels(&msg.remote_id, &["INBOX".to_string()], &[]).await {
                    warn!("Gmail unarchive failed: {e}");
                }
            }
        }
        state.store.move_message_to_folder(&message_id, &inbox.id)?;
        refresh_search_document(&state, &message_id)?;
        return Ok("unarchived".to_string());
    }

    // Try to find Archive folder; if not available, just soft-delete locally
    match find_folder_by_role(&state, &msg.account_id, FolderRole::Archive) {
        Ok(archive_folder) => {
            let is_local = archive_folder.remote_id.starts_with("__local_");
            match provider_type {
                ProviderType::Gmail => {
                    if let Ok(provider) = connect_gmail(&state, &msg.account_id).await {
                        if let Err(e) = provider.modify_labels(&msg.remote_id, &[], &["INBOX".to_string()]).await {
                            warn!("Gmail archive failed: {e}");
                        }
                    }
                }
                ProviderType::Imap | ProviderType::Outlook => {
                    // Move on IMAP server (only if archive folder exists on server)
                    if !is_local {
                        let uid: u32 = msg.remote_id.parse()
                            .map_err(|_| PebbleError::Internal("Invalid remote_id (not a UID)".to_string()))?;
                        match connect_imap(&state, &msg.account_id).await {
                            Ok(imap) => {
                                imap.move_message(&source_folder.remote_id, uid, &archive_folder.remote_id).await?;
                                imap.disconnect().await?;
                                info!("Archived message {} (UID {}) from {} to {}", message_id, uid, source_folder.name, archive_folder.name);
                            }
                            Err(e) => {
                                tracing::warn!("IMAP connect failed for archive: {e}");
                            }
                        }
                    }
                }
            }
            // Move locally to archive folder so user can see it there
            state.store.move_message_to_folder(&message_id, &archive_folder.id)?;
            refresh_search_document(&state, &message_id)?;
            Ok("archived".to_string())
        }
        Err(_) => {
            // No archive folder — soft-delete as fallback
            info!("No archive folder found, soft-deleting message {} locally", message_id);
            state.store.soft_delete_message(&message_id)?;
            refresh_search_document(&state, &message_id)?;
            Ok("archived".to_string())
        }
    }
}

#[tauri::command]
pub async fn delete_message(
    state: State<'_, AppState>,
    message_id: String,
) -> std::result::Result<(), PebbleError> {
    let msg = state.store.get_message(&message_id)?
        .ok_or_else(|| PebbleError::Internal(format!("Message not found: {message_id}")))?;
    let provider_type = state
        .store
        .get_account(&msg.account_id)?
        .map(|account| account.provider)
        .ok_or_else(|| PebbleError::Internal(format!("Account not found: {}", msg.account_id)))?;

    let source_folder = find_message_folder(&state, &message_id, &msg.account_id)?;

    match provider_type {
        ProviderType::Gmail => {
            if source_folder.role == Some(FolderRole::Trash) {
                if let Ok(provider) = connect_gmail(&state, &msg.account_id).await {
                    if let Err(e) = provider.delete_message_permanently(&msg.remote_id).await {
                        warn!("Gmail permanent delete failed: {e}");
                    }
                }
            } else if let Ok(provider) = connect_gmail(&state, &msg.account_id).await {
                if let Err(e) = provider.trash_message(&msg.remote_id).await {
                    warn!("Gmail trash move failed: {e}");
                } else {
                    info!("Moved Gmail message {} to Trash on server", message_id);
                }
            }
        }
        ProviderType::Imap | ProviderType::Outlook => {
            // Try IMAP operations but don't block local deletion on failure
            if let Ok(uid) = msg.remote_id.parse::<u32>() {
                match connect_imap(&state, &msg.account_id).await {
                    Ok(imap) => {
                        match find_folder_by_role(&state, &msg.account_id, FolderRole::Trash) {
                            Ok(ref trash_folder) if trash_folder.id != source_folder.id => {
                                if let Err(e) = imap.move_message(&source_folder.remote_id, uid, &trash_folder.remote_id).await {
                                    tracing::warn!("IMAP move to trash failed: {e}");
                                } else {
                                    info!("Moved message {} to Trash on server", message_id);
                                }
                            }
                            _ => {
                                if let Err(e) = imap.delete_message(&source_folder.remote_id, uid).await {
                                    tracing::warn!("IMAP delete failed: {e}");
                                } else {
                                    info!("Permanently deleted message {} (UID {})", message_id, uid);
                                }
                            }
                        }
                        let _ = imap.disconnect().await;
                    }
                    Err(e) => {
                        tracing::warn!("IMAP connect failed for delete: {e}");
                    }
                }
            }
        }
    }

    // Move locally to trash folder if it exists, otherwise soft-delete
    match find_folder_by_role(&state, &msg.account_id, FolderRole::Trash) {
        Ok(trash_folder) if trash_folder.id != source_folder.id => {
            state.store.move_message_to_folder(&message_id, &trash_folder.id)?;
        }
        _ => {
            state.store.soft_delete_message(&message_id)?;
        }
    }

    refresh_search_document(&state, &message_id)?;
    Ok(())
}

#[tauri::command]
pub async fn restore_message(
    state: State<'_, AppState>,
    message_id: String,
) -> std::result::Result<(), PebbleError> {
    let msg = state.store.get_message(&message_id)?
        .ok_or_else(|| PebbleError::Internal(format!("Message not found: {message_id}")))?;
    let provider_type = state
        .store
        .get_account(&msg.account_id)?
        .map(|account| account.provider)
        .ok_or_else(|| PebbleError::Internal(format!("Account not found: {}", msg.account_id)))?;

    let inbox = find_folder_by_role(&state, &msg.account_id, FolderRole::Inbox)?;

    // Get source folder before moving locally
    let source_folder = find_message_folder(&state, &message_id, &msg.account_id).ok();

    // Move locally to inbox
    state.store.move_message_to_folder(&message_id, &inbox.id)?;

    match provider_type {
        ProviderType::Gmail => {
            if let Some(ref src) = source_folder {
                if let Ok(provider) = connect_gmail(&state, &msg.account_id).await {
                    let result = if src.role == Some(FolderRole::Trash) {
                        provider.untrash_message(&msg.remote_id).await
                    } else {
                        provider.modify_labels(&msg.remote_id, &["INBOX".to_string()], &[]).await
                    };
                    if let Err(e) = result {
                        warn!("Gmail restore failed: {e}");
                    }
                }
            }
        }
        ProviderType::Imap | ProviderType::Outlook => {
            // Try to move on IMAP server too (skip for local-only folders)
            if let Ok(uid) = msg.remote_id.parse::<u32>() {
                if let Some(ref src) = source_folder {
                    let is_local = src.remote_id.starts_with("__local_");
                    if !is_local && src.id != inbox.id {
                        if let Ok(imap) = connect_imap(&state, &msg.account_id).await {
                            let _ = imap.move_message(&src.remote_id, uid, &inbox.remote_id).await;
                            let _ = imap.disconnect().await;
                        }
                    }
                }
            }
        }
    }

    refresh_search_document(&state, &message_id)?;
    info!("Restored message {} to inbox", message_id);
    Ok(())
}

#[tauri::command]
pub async fn empty_trash(
    state: State<'_, AppState>,
    account_id: String,
) -> std::result::Result<u32, PebbleError> {
    let trash = find_folder_by_role(&state, &account_id, FolderRole::Trash)?;
    let messages = state.store.list_messages_by_folder(&trash.id, 10000, 0)?;
    let provider_type = state
        .store
        .get_account(&account_id)?
        .map(|account| account.provider)
        .ok_or_else(|| PebbleError::Internal(format!("Account not found: {account_id}")))?;

    if messages.is_empty() {
        return Ok(0);
    }

    let count = messages.len() as u32;

    match provider_type {
        ProviderType::Gmail => {
            if let Ok(provider) = connect_gmail(&state, &account_id).await {
                for msg in &messages {
                    let _ = provider.delete_message_permanently(&msg.remote_id).await;
                }
            }
        }
        ProviderType::Imap | ProviderType::Outlook => {
            // Try to permanently delete on IMAP server
            if let Ok(imap) = connect_imap(&state, &account_id).await {
                for msg in &messages {
                    if let Ok(uid) = msg.remote_id.parse::<u32>() {
                        let _ = imap.delete_message(&trash.remote_id, uid).await;
                    }
                }
                let _ = imap.disconnect().await;
            }
        }
    }

    // Permanently delete locally (hard delete, not soft delete + purge)
    let ids: Vec<String> = messages.iter().map(|m| m.id.clone()).collect();
    state.store.hard_delete_messages(&ids)?;
    remove_search_documents(&state, &ids)?;

    info!("Emptied trash: {} messages permanently deleted", count);
    Ok(count)
}
