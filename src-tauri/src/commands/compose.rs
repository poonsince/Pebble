use crate::state::AppState;
use crate::commands::oauth::ensure_account_oauth_tokens;
use pebble_core::traits::{MailTransport, OutgoingMessage};
use pebble_core::{EmailAddress, PebbleError, ProviderType};
use pebble_mail::GmailProvider;
use pebble_mail::smtp::SmtpSender;
use tauri::State;

fn parse_recipients(addresses: Vec<String>) -> Vec<EmailAddress> {
    addresses
        .into_iter()
        .map(|address| EmailAddress {
            name: None,
            address: address.trim().to_string(),
        })
        .filter(|address| !address.address.is_empty())
        .collect()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn send_email(
    state: State<'_, AppState>,
    account_id: String,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: String,
    body_text: String,
    body_html: Option<String>,
    in_reply_to: Option<String>,
) -> std::result::Result<(), PebbleError> {
    let account = state
        .store
        .get_account(&account_id)?
        .ok_or_else(|| PebbleError::Internal(format!("Account not found: {account_id}")))?;

    if matches!(account.provider, ProviderType::Gmail) {
        let tokens = ensure_account_oauth_tokens(&state, &account_id, "gmail").await?;
        let provider = GmailProvider::new(tokens.access_token);
        let message = OutgoingMessage {
            to: parse_recipients(to),
            cc: parse_recipients(cc),
            bcc: parse_recipients(bcc),
            subject,
            body_text,
            body_html,
            in_reply_to,
        };
        return provider.send_message(&message).await;
    }

    // Read SMTP config from encrypted auth_data (where add_account stores it)
    let encrypted = state
        .store
        .get_auth_data(&account_id)?
        .ok_or_else(|| {
            PebbleError::Internal(format!("No auth data found for account {account_id}"))
        })?;
    let decrypted = state.crypto.decrypt(&encrypted)?;
    let config: serde_json::Value = serde_json::from_slice(&decrypted)
        .map_err(|e| PebbleError::Internal(format!("Failed to parse decrypted config: {e}")))?;

    let smtp_config: pebble_mail::SmtpConfig = serde_json::from_value(
        config
            .get("smtp")
            .cloned()
            .ok_or_else(|| PebbleError::Internal("No SMTP config in auth data".to_string()))?,
    )
    .map_err(|e| PebbleError::Internal(format!("Failed to deserialize SMTP config: {e}")))?;

    let sender = SmtpSender::new(
        smtp_config.host,
        smtp_config.port,
        smtp_config.username,
        smtp_config.password,
        smtp_config.security,
    );

    let from_email = account.email.clone();
    let result = tokio::task::spawn_blocking(move || {
        sender.send(
            &from_email,
            &to,
            &cc,
            &bcc,
            &subject,
            &body_text,
            body_html.as_deref(),
            in_reply_to.as_deref(),
        )
    })
    .await
    .map_err(|e| PebbleError::Internal(format!("Send task failed: {e}")))?;
    result
}
