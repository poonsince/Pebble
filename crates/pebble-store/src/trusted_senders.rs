use pebble_core::{Result, UntrustedSender};
use rusqlite::{params, OptionalExtension};

use crate::Store;

fn row_to_untrusted_sender(row: &rusqlite::Row) -> rusqlite::Result<UntrustedSender> {
    Ok(UntrustedSender {
        account_id: row.get(0)?,
        email: row.get(1)?,
        created_at: row.get(2)?,
    })
}

impl Store {
    pub fn add_untrusted_sender(&self, sender: &UntrustedSender) -> Result<()> {
        self.with_write(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO trusted_senders (account_id, email, trust_type, created_at)
                 VALUES (?1, ?2, 'all', ?3)",
                params![sender.account_id, sender.email, sender.created_at],
            )?;
            Ok(())
        })
    }

    pub fn is_untrusted_sender(&self, account_id: &str, email: &str) -> Result<bool> {
        self.with_read(|conn| {
            let result = conn
                .query_row(
                    "SELECT 1 FROM trusted_senders WHERE account_id = ?1 AND email = ?2",
                    params![account_id, email],
                    |_| Ok(()),
                )
                .optional()?;
            Ok(result.is_some())
        })
    }

    pub fn list_untrusted_senders(&self, account_id: &str) -> Result<Vec<UntrustedSender>> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT account_id, email, created_at
                     FROM trusted_senders WHERE account_id = ?1",
            )?;
            let rows = stmt.query_map(params![account_id], row_to_untrusted_sender)?;
            let mut senders = Vec::new();
            for row in rows {
                senders.push(row?);
            }
            Ok(senders)
        })
    }

    pub fn remove_untrusted_sender(&self, account_id: &str, email: &str) -> Result<()> {
        self.with_write(|conn| {
            conn.execute(
                "DELETE FROM trusted_senders WHERE account_id = ?1 AND email = ?2",
                params![account_id, email],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    #[test]
    fn test_untrusted_sender_crud() {
        let store = Store::open_in_memory().unwrap();
        let now = pebble_core::now_timestamp();
        let account = pebble_core::Account {
            id: pebble_core::new_id(),
            email: "me@example.com".to_string(),
            display_name: "Me".to_string(),
            color: None,
            provider: pebble_core::ProviderType::Imap,
            created_at: now,
            updated_at: now,
        };
        store.insert_account(&account).unwrap();

        let sender = UntrustedSender {
            account_id: account.id.clone(),
            email: "spammy@example.com".to_string(),
            created_at: now,
        };
        store.add_untrusted_sender(&sender).unwrap();

        // Should be untrusted
        assert!(store.is_untrusted_sender(&account.id, "spammy@example.com").unwrap());

        // Unknown sender should NOT be untrusted
        assert!(!store.is_untrusted_sender(&account.id, "unknown@example.com").unwrap());

        // List
        let senders = store.list_untrusted_senders(&account.id).unwrap();
        assert_eq!(senders.len(), 1);
        assert_eq!(senders[0].email, "spammy@example.com");

        // Remove
        store.remove_untrusted_sender(&account.id, "spammy@example.com").unwrap();
        let senders = store.list_untrusted_senders(&account.id).unwrap();
        assert_eq!(senders.len(), 0);
    }
}
