use crate::state::AppState;
use base64::Engine;
use pebble_core::{Message, PebbleError, PrivacyMode, RenderedHtml};
use pebble_privacy::PrivacyGuard;
use pebble_store::Store;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn get_rendered_html(
    state: State<'_, AppState>,
    message_id: String,
    privacy_mode: PrivacyMode,
) -> std::result::Result<RenderedHtml, PebbleError> {
    let store = state.store.clone();
    let attachments_dir = state.attachments_dir.clone();
    tokio::task::spawn_blocking(move || {
        let message = store
            .get_message(&message_id)?
            .ok_or_else(|| PebbleError::Internal(format!("Message not found: {message_id}")))?;

        let effective_mode = resolve_privacy_mode(&store, &message, privacy_mode)?;
        let guard = PrivacyGuard::new();
        let mut rendered =
            guard.render_message_html(&message.body_html_raw, &message.body_text, &effective_mode);

        rendered.html = load_inline_images(&store, &message_id, &attachments_dir, &rendered.html);

        Ok(rendered)
    })
    .await
    .map_err(|e| PebbleError::Internal(format!("Task join error: {e}")))?
}

#[tauri::command]
pub async fn get_message_with_html(
    state: State<'_, AppState>,
    message_id: String,
    privacy_mode: PrivacyMode,
) -> std::result::Result<Option<(Message, RenderedHtml)>, PebbleError> {
    let store = state.store.clone();
    let attachments_dir = state.attachments_dir.clone();
    tokio::task::spawn_blocking(move || {
        let message = match store.get_message(&message_id)? {
            Some(m) => m,
            None => return Ok(None),
        };

        let effective_mode = resolve_privacy_mode(&store, &message, privacy_mode)?;
        let guard = PrivacyGuard::new();
        let mut rendered =
            guard.render_message_html(&message.body_html_raw, &message.body_text, &effective_mode);

        rendered.html =
            load_inline_images(&store, &message_id, &attachments_dir, &rendered.html);

        Ok(Some((message, rendered)))
    })
    .await
    .map_err(|e| PebbleError::Internal(format!("Task join error: {e}")))?
}

#[tauri::command]
pub async fn is_untrusted_sender(
    state: State<'_, AppState>,
    account_id: String,
    email: String,
) -> std::result::Result<bool, PebbleError> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || Ok(store.is_untrusted_sender(&account_id, &email)?))
        .await
        .map_err(|e| PebbleError::Internal(format!("Task join error: {e}")))?
}

/// Replace `data-cid` attributes in HTML with data URIs from inline attachments.
fn load_inline_images(
    store: &Store,
    message_id: &str,
    attachments_dir: &std::path::Path,
    html: &str,
) -> String {
    let attachments = match store.list_attachments_by_message(message_id) {
        Ok(atts) => atts,
        Err(_) => return html.to_string(),
    };

    let mut cid_map: HashMap<String, String> = HashMap::new();
    for att in &attachments {
        if !att.is_inline {
            continue;
        }
        if let Some(ref cid) = att.content_id {
            if let Some(ref local_path) = att.local_path {
                let clean_cid = cid.trim_matches(|c: char| c == '<' || c == '>');
                let data_uri = match build_data_uri(attachments_dir, local_path, &att.mime_type) {
                    Some(uri) => uri,
                    None => continue,
                };
                cid_map.insert(clean_cid.to_string(), data_uri);
            }
        }
    }

    if cid_map.is_empty() {
        return html.to_string();
    }

    let mut result = html.to_string();
    for (cid, data_uri) in &cid_map {
        let pattern = format!("data-cid=\"{}\"", html_escape_attr(cid));
        let replacement = format!("src=\"{}\"", data_uri);
        result = result.replace(&pattern, &replacement);
    }

    result
}

fn build_data_uri(
    attachments_dir: &std::path::Path,
    local_path: &str,
    mime_type: &str,
) -> Option<String> {
    let full_path = attachments_dir.join(local_path);
    let data = std::fs::read(&full_path).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
    Some(format!("data:{};base64,{}", mime_type, encoded))
}

fn html_escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Resolve the effective privacy mode for a message.
///
/// New design:
/// - Default: all senders are trusted → no blocking.
/// - Only senders in the **untrusted sender list** get tracker protection applied.
/// - Images are NEVER blocked by any privacy mode.
fn resolve_privacy_mode(
    store: &Store,
    message: &Message,
    privacy_mode: PrivacyMode,
) -> std::result::Result<PrivacyMode, PebbleError> {
    match privacy_mode {
        PrivacyMode::Normal | PrivacyMode::Strict => {
            // If sender is NOT in the untrusted list → no blocking (Off)
            if store.is_untrusted_sender(&message.account_id, &message.from_address)? {
                Ok(privacy_mode)
            } else {
                Ok(PrivacyMode::Off)
            }
        }
        PrivacyMode::Off => Ok(privacy_mode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pebble_core::{
        new_id, now_timestamp, Account, EmailAddress, Folder, FolderRole, FolderType, Message,
        ProviderType, UntrustedSender,
    };

    fn make_account(id: &str) -> Account {
        let now = now_timestamp();
        Account {
            id: id.to_string(),
            email: "me@example.com".to_string(),
            display_name: "Me".to_string(),
            color: None,
            provider: ProviderType::Imap,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_message(account_id: &str, from_address: &str) -> Message {
        let now = now_timestamp();
        Message {
            id: new_id(),
            account_id: account_id.to_string(),
            remote_id: "remote-1".to_string(),
            message_id_header: None,
            in_reply_to: None,
            references_header: None,
            thread_id: Some("thread-1".to_string()),
            subject: "Subject".to_string(),
            snippet: "Snippet".to_string(),
            from_address: from_address.to_string(),
            from_name: "Sender".to_string(),
            to_list: vec![EmailAddress {
                name: None,
                address: "me@example.com".to_string(),
            }],
            cc_list: vec![],
            bcc_list: vec![],
            body_text: "Body".to_string(),
            body_html_raw: "<p>Body</p>".to_string(),
            has_attachments: false,
            is_read: false,
            is_starred: false,
            is_draft: false,
            date: now,
            remote_version: None,
            is_deleted: false,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_folder(account_id: &str) -> Folder {
        Folder {
            id: new_id(),
            account_id: account_id.to_string(),
            remote_id: "INBOX".to_string(),
            name: "Inbox".to_string(),
            role: Some(FolderRole::Inbox),
            folder_type: FolderType::Folder,
            parent_id: None,
            color: None,
            is_system: true,
            sort_order: 0,
        }
    }

    #[test]
    fn untrusted_sender_gets_tracker_blocking() {
        let store = Store::open_in_memory().unwrap();
        let account = make_account("account-1");
        store.insert_account(&account).unwrap();
        let folder = make_folder(&account.id);
        store.insert_folder(&folder).unwrap();
        let message = make_message(&account.id, "spammy@spam.com");
        store.insert_message(&message, &[folder.id.clone()]).unwrap();
        store
            .add_untrusted_sender(&UntrustedSender {
                account_id: account.id.clone(),
                email: "spammy@spam.com".to_string(),
                created_at: now_timestamp(),
            })
            .unwrap();

        // Untrusted sender + Strict mode → keeps Strict
        let mode = resolve_privacy_mode(&store, &message, PrivacyMode::Strict).unwrap();
        assert_eq!(mode, PrivacyMode::Strict);

        // Untrusted sender + Normal mode → keeps Normal
        let mode = resolve_privacy_mode(&store, &message, PrivacyMode::Normal).unwrap();
        assert_eq!(mode, PrivacyMode::Normal);

        // Untrusted sender + Off mode → Off (blocking disabled)
        let mode = resolve_privacy_mode(&store, &message, PrivacyMode::Off).unwrap();
        assert_eq!(mode, PrivacyMode::Off);
    }

    #[test]
    fn trusted_sender_by_default_no_blocking() {
        let store = Store::open_in_memory().unwrap();
        let account = make_account("account-1");
        store.insert_account(&account).unwrap();
        let folder = make_folder(&account.id);
        store.insert_folder(&folder).unwrap();
        // NOT in untrusted list = trusted by default
        let message = make_message(&account.id, "friend@example.com");
        store.insert_message(&message, &[folder.id.clone()]).unwrap();

        // Even Strict mode → Off (no untrusted senders)
        let mode = resolve_privacy_mode(&store, &message, PrivacyMode::Strict).unwrap();
        assert_eq!(mode, PrivacyMode::Off);
    }
}
