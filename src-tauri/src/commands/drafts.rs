use crate::state::AppState;
use pebble_core::{traits::DraftProvider, DraftMessage, EmailAddress, PebbleError};
use tauri::State;
use tracing::warn;

use super::messages::provider_dispatch::ConnectedProvider;

#[tauri::command]
pub async fn save_draft(
    state: State<'_, AppState>,
    account_id: String,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: String,
    body_text: String,
    body_html: Option<String>,
    in_reply_to: Option<String>,
    existing_draft_id: Option<String>,
) -> std::result::Result<String, PebbleError> {
    let draft = DraftMessage {
        id: existing_draft_id.clone(),
        to: to.into_iter().map(|a| EmailAddress { name: None, address: a }).collect(),
        cc: cc.into_iter().map(|a| EmailAddress { name: None, address: a }).collect(),
        bcc: bcc.into_iter().map(|a| EmailAddress { name: None, address: a }).collect(),
        subject,
        body_text,
        body_html,
        in_reply_to,
    };

    let provider_type = state.store.get_account(&account_id)?
        .map(|a| a.provider);

    match provider_type {
        Some(pt) => {
            if let Ok(conn) = ConnectedProvider::connect(&state, &account_id, &pt).await {
                let result = match (&conn, &existing_draft_id) {
                    (ConnectedProvider::Gmail(p), Some(did)) => {
                        p.update_draft(did, &draft).await.map(|_| did.clone())
                    }
                    (ConnectedProvider::Gmail(p), None) => p.save_draft(&draft).await,
                    (ConnectedProvider::Outlook(p), Some(did)) => {
                        p.update_draft(did, &draft).await.map(|_| did.clone())
                    }
                    (ConnectedProvider::Outlook(p), None) => p.save_draft(&draft).await,
                    _ => {
                        // IMAP — fall back to local-only
                        save_draft_locally(&state, &account_id, &draft)
                    }
                };
                conn.disconnect().await;
                result
            } else {
                save_draft_locally(&state, &account_id, &draft)
            }
        }
        None => save_draft_locally(&state, &account_id, &draft),
    }
}

fn save_draft_locally(
    state: &AppState,
    account_id: &str,
    draft: &DraftMessage,
) -> std::result::Result<String, PebbleError> {
    let id = draft.id.clone().unwrap_or_else(pebble_core::new_id);
    let msg = pebble_core::Message {
        id: id.clone(),
        account_id: account_id.to_string(),
        remote_id: String::new(),
        message_id_header: None,
        in_reply_to: draft.in_reply_to.clone(),
        references_header: None,
        thread_id: None,
        subject: draft.subject.clone(),
        snippet: draft.body_text.chars().take(200).collect(),
        from_address: String::new(),
        from_name: String::new(),
        to_list: draft.to.clone(),
        cc_list: draft.cc.clone(),
        bcc_list: draft.bcc.clone(),
        body_text: draft.body_text.clone(),
        body_html_raw: draft.body_html.clone().unwrap_or_default(),
        has_attachments: false,
        is_read: true,
        is_starred: false,
        is_draft: true,
        date: pebble_core::now_timestamp(),
        remote_version: None,
        is_deleted: false,
        deleted_at: None,
        created_at: pebble_core::now_timestamp(),
        updated_at: pebble_core::now_timestamp(),
    };
    state.store.insert_message(&msg, &[])?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_draft(
    state: State<'_, AppState>,
    account_id: String,
    draft_id: String,
) -> std::result::Result<(), PebbleError> {
    let provider_type = state.store.get_account(&account_id)?
        .map(|a| a.provider);
    if let Some(pt) = provider_type {
        if let Ok(conn) = ConnectedProvider::connect(&state, &account_id, &pt).await {
            let result = match &conn {
                ConnectedProvider::Gmail(p) => p.delete_draft(&draft_id).await,
                ConnectedProvider::Outlook(p) => p.delete_draft(&draft_id).await,
                _ => Ok(()),
            };
            conn.disconnect().await;
            if let Err(e) = result {
                warn!("Failed to delete remote draft: {e}");
            }
        }
    }
    let _ = state.store.hard_delete_messages(&[draft_id]);
    Ok(())
}
