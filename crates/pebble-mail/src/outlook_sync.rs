use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use pebble_core::traits::{FetchResult, FolderProvider};
use pebble_core::{now_timestamp, Message, PebbleError, Result};
use pebble_store::Store;
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

use crate::backoff::SyncBackoff;
use crate::provider::outlook::OutlookProvider;
use crate::gmail_sync::TokenRefresher;
use crate::sync::{StoredMessage, SyncConfig, SyncError, SyncWorkerBase, persist_message_attachments_async};
use crate::thread::compute_thread_id;

async fn collect_outlook_folder_pages<F, Fut>(
    folder_id: &str,
    limit: u32,
    mut fetch_page: F,
) -> Result<Vec<Message>>
where
    F: FnMut(String, u32, Option<String>) -> Fut,
    Fut: Future<Output = Result<FetchResult>>,
{
    let mut messages = Vec::new();
    let mut cursor = None;

    loop {
        let mut result = fetch_page(folder_id.to_string(), limit, cursor.take()).await?;
        messages.append(&mut result.messages);

        match result.cursor.value {
            value if !value.is_empty() => cursor = Some(value),
            _ => break,
        }
    }

    Ok(messages)
}

/// A sync worker for Outlook accounts using the Microsoft Graph API.
pub struct OutlookSyncWorker {
    pub(crate) base: SyncWorkerBase,
    provider: Arc<OutlookProvider>,
    token_refresher: Option<Arc<TokenRefresher>>,
    /// Last known token expiry (unix timestamp).
    token_expires_at: StdMutex<Option<i64>>,
}

impl OutlookSyncWorker {
    pub fn new(
        account_id: impl Into<String>,
        provider: Arc<OutlookProvider>,
        store: Arc<Store>,
        attachments_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            base: SyncWorkerBase {
                account_id: account_id.into(),
                store,
                attachments_dir: attachments_dir.into(),
                error_tx: None,
                message_tx: None,
            },
            provider,
            token_refresher: None,
            token_expires_at: StdMutex::new(None),
        }
    }

    pub fn with_error_tx(mut self, tx: mpsc::UnboundedSender<SyncError>) -> Self {
        self.base.error_tx = Some(tx);
        self
    }

    pub fn with_message_tx(mut self, tx: mpsc::UnboundedSender<StoredMessage>) -> Self {
        self.base.message_tx = Some(tx);
        self
    }

    pub fn with_token_refresher(mut self, refresher: TokenRefresher, expires_at: Option<i64>) -> Self {
        self.token_refresher = Some(Arc::new(refresher));
        *self.token_expires_at.lock().unwrap_or_else(|e| e.into_inner()) = expires_at;
        self
    }

    pub fn with_token_expires_at(self, expires_at: Option<i64>) -> Self {
        *self.token_expires_at.lock().unwrap_or_else(|e| e.into_inner()) = expires_at;
        self
    }

    /// Ensure the access token is still valid; refresh if needed.
    async fn ensure_valid_token(&self) -> Result<()> {
        let now = now_timestamp();
        let needs_refresh = {
            let expires = self.token_expires_at.lock().unwrap_or_else(|e| e.into_inner());
            match *expires {
                Some(exp) => now >= exp - 60,
                None => false,
            }
        };

        if needs_refresh {
            if let Some(ref refresher) = self.token_refresher {
                match refresher().await {
                    Ok((new_token, new_expires_at)) => {
                        self.provider.set_access_token(new_token);
                        let mut expires = self.token_expires_at.lock().unwrap_or_else(|e| e.into_inner());
                        *expires = new_expires_at.or(Some(now + 3600));
                        info!("Outlook OAuth token refreshed for account {}", self.base.account_id);
                    }
                    Err(e) => {
                        warn!("Failed to refresh Outlook OAuth token: {}", e);
                        self.base.emit_error("token_refresh", &format!("Outlook token refresh failed: {e}"));
                        return Err(PebbleError::Auth(format!("Outlook token refresh failed: {e}")));
                    }
                }
            }
        }
        Ok(())
    }

    /// Main sync loop.
    pub async fn run(&self, config: SyncConfig, mut stop_rx: watch::Receiver<bool>) {
        let poll_interval = tokio::time::Duration::from_secs(config.poll_interval_secs);
        let mut backoff = SyncBackoff::new();

        loop {
            if *stop_rx.borrow() {
                break;
            }

            // Check circuit breaker at start of each iteration
            if backoff.is_circuit_open() {
                let delay = backoff.current_delay();
                warn!(
                    "Circuit open for Outlook account {} ({} failures), waiting {:?}",
                    self.base.account_id, backoff.failure_count(), delay
                );
                match tokio::time::timeout(delay, stop_rx.changed()).await {
                    Ok(Ok(())) if *stop_rx.borrow() => break,
                    _ => {}
                }
                continue;
            }

            // Refresh token if needed
            if let Err(e) = self.ensure_valid_token().await {
                warn!("Outlook token validation failed: {}", e);
                self.base.emit_error("auth", &format!("Token validation failed: {e}"));
                let _ = backoff.record_failure();
                if backoff.is_circuit_open() {
                    let delay = backoff.current_delay();
                    let _ = tokio::time::timeout(delay, stop_rx.changed()).await;
                }
                continue;
            }

            // List folders and fetch messages per folder
            let folders = match self.provider.list_folders().await {
                Ok(f) => {
                    backoff.record_success();
                    f
                }
                Err(e) => {
                    warn!("Outlook folder list failed: {e}");
                    self.base.emit_error("sync", &format!("Outlook folder list failed: {e}"));
                    let delay = backoff.record_failure();
                    if backoff.is_circuit_open() {
                        warn!(
                            "Circuit open for Outlook account {} ({} failures), waiting {:?}",
                            self.base.account_id, backoff.failure_count(), delay
                        );
                    }
                    let wait = if backoff.is_circuit_open() { delay } else { poll_interval };
                    let _ = tokio::time::timeout(wait, stop_rx.changed()).await;
                    continue;
                }
            };

            for folder in &folders {
                // Persist folder
                let _ = self.base.store.insert_folder(folder);
            }

            // Re-read folders from DB so we use persisted IDs (upsert may keep old IDs)
            let db_folders = self.base.store.list_folders(&self.base.account_id).unwrap_or_default();

            for folder in &db_folders {
                match collect_outlook_folder_pages(&folder.remote_id, 50, |folder_id, limit, cursor| {
                    let provider = Arc::clone(&self.provider);
                    async move {
                        provider
                            .fetch_messages_page(&folder_id, limit, cursor.as_deref())
                            .await
                    }
                })
                .await
                {
                    Ok(messages) => {
                        let remote_ids: Vec<String> =
                            messages.iter().map(|m| m.remote_id.clone()).collect();
                        let existing = self
                            .base.store
                            .get_existing_remote_ids(&self.base.account_id, &remote_ids)
                            .unwrap_or_default();

                        // Collect all referenced message-ID headers from this batch so we can
                        // load thread mappings in a single query.
                        let ref_ids: Vec<String> = {
                            let mut refs = std::collections::HashSet::new();
                            for msg in &messages {
                                if let Some(irt) = &msg.in_reply_to {
                                    for id in irt.split_whitespace() {
                                        refs.insert(id.trim().to_string());
                                    }
                                }
                                if let Some(r) = &msg.references_header {
                                    for id in r.split_whitespace() {
                                        refs.insert(id.trim().to_string());
                                    }
                                }
                            }
                            refs.into_iter().collect()
                        };

                        // Load thread mappings for this batch. Kept mutable so intra-batch
                        // replies can find their parent within the same fetch.
                        let mut thread_mappings = self
                            .base.store
                            .get_thread_mappings_for_refs(&self.base.account_id, &ref_ids)
                            .unwrap_or_default();

                        for msg in &messages {
                            if existing.contains(&msg.remote_id) {
                                continue;
                            }

                            // Compute thread_id before inserting.
                            let mut msg = msg.clone();
                            let thread_id = compute_thread_id(&msg, &thread_mappings);
                            msg.thread_id = Some(thread_id);

                            let folder_ids = vec![folder.id.clone()];
                            if let Err(e) = self.base.store.insert_message(&msg, &folder_ids) {
                                warn!("Failed to store Outlook message: {e}");
                                continue;
                            }

                            // Update in-memory thread mappings so later messages in this batch
                            // can find this message as a thread parent.
                            if let (Some(mid), Some(tid)) = (&msg.message_id_header, &msg.thread_id) {
                                thread_mappings.insert(mid.clone(), tid.clone());
                            }

                            // Fetch + persist attachments for messages that advertise them.
                            if msg.has_attachments {
                                match self.provider.list_message_attachments(&msg.remote_id).await {
                                    Ok(attachments) if !attachments.is_empty() => {
                                        persist_message_attachments_async(
                                            Arc::clone(&self.base.store),
                                            self.base.attachments_dir.clone(),
                                            msg.id.clone(),
                                            attachments,
                                        )
                                        .await;
                                    }
                                    Ok(_) => {}
                                    Err(e) => {
                                        warn!(
                                            "Failed to fetch Outlook attachments for {}: {e}",
                                            msg.remote_id
                                        );
                                    }
                                }
                            }

                            self.base.emit_message(StoredMessage {
                                message: msg.clone(),
                                folder_ids,
                            });
                        }
                    }
                    Err(e) => {
                        warn!("Outlook sync fetch failed for folder {}: {e}", folder.name);
                    }
                }

                if *stop_rx.borrow() {
                    break;
                }
            }

            // Wait for next poll or stop signal
            let _ = tokio::time::timeout(poll_interval, stop_rx.changed()).await;
        }

        info!("Outlook sync task completed for account {}", self.base.account_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pebble_core::traits::SyncCursor;
    use std::collections::VecDeque;

    fn make_message(remote_id: &str) -> Message {
        let now = now_timestamp();
        Message {
            id: format!("local-{remote_id}"),
            account_id: "account-1".to_string(),
            remote_id: remote_id.to_string(),
            message_id_header: None,
            in_reply_to: None,
            references_header: None,
            thread_id: None,
            subject: remote_id.to_string(),
            snippet: String::new(),
            from_address: String::new(),
            from_name: String::new(),
            to_list: Vec::new(),
            cc_list: Vec::new(),
            bcc_list: Vec::new(),
            body_text: String::new(),
            body_html_raw: String::new(),
            has_attachments: false,
            is_read: true,
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

    #[tokio::test]
    async fn collect_outlook_folder_pages_follows_next_link_until_empty() {
        let mut pages = VecDeque::from([
            FetchResult {
                messages: vec![make_message("outlook-1")],
                cursor: SyncCursor {
                    value: "https://graph.example/next".to_string(),
                },
            },
            FetchResult {
                messages: vec![make_message("outlook-2")],
                cursor: SyncCursor {
                    value: String::new(),
                },
            },
        ]);
        let mut requested_cursors = Vec::new();

        let messages = collect_outlook_folder_pages("folder-1", 50, |folder_id, limit, cursor| {
            requested_cursors.push((folder_id, limit, cursor));
            let page = pages.pop_front().expect("expected a page request");
            async move { Ok(page) }
        })
        .await
        .unwrap();

        let remote_ids: Vec<_> = messages.into_iter().map(|m| m.remote_id).collect();
        assert_eq!(remote_ids, vec!["outlook-1".to_string(), "outlook-2".to_string()]);
        assert_eq!(
            requested_cursors,
            vec![
                ("folder-1".to_string(), 50, None),
                (
                    "folder-1".to_string(),
                    50,
                    Some("https://graph.example/next".to_string()),
                ),
            ]
        );
    }
}
