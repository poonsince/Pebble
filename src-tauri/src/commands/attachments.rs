use crate::state::AppState;
use pebble_core::{Attachment, PebbleError};
use std::path::Path;
use tauri::{Emitter, State};

fn is_windows_reserved_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

/// Validate that save_to path is within a safe directory (user's home).
///
/// This is the security gate for attachment writes: it *rejects* suspicious
/// paths rather than normalizing them. The FE helper at `src/lib/sanitizeFilename.ts`
/// covers the complementary UX role of cleaning up suggested defaults; it is
/// not a substitute for this check. Keep the character sets below in sync with
/// the FE when they change.
fn validate_save_path(save_to: &str) -> Result<(), PebbleError> {
    let path = Path::new(save_to);
    let canonical = path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .ok_or_else(|| PebbleError::Internal("Invalid save directory".to_string()))?;

    // Ensure no path traversal components in the filename
    let filename = path
        .file_name()
        .ok_or_else(|| PebbleError::Internal("No filename specified".to_string()))?;
    let filename_str = filename.to_string_lossy();
    if filename_str.contains("..") || filename_str.contains('/') || filename_str.contains('\\') {
        return Err(PebbleError::Internal(
            "Invalid filename in save path".to_string(),
        ));
    }
    if filename_str.ends_with(' ') || filename_str.ends_with('.') {
        return Err(PebbleError::Validation(
            "Filename cannot end with a dot or space".to_string(),
        ));
    }
    if filename_str
        .chars()
        .any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err(PebbleError::Validation(
            "Filename contains characters unsupported on Windows".to_string(),
        ));
    }
    if filename_str.chars().any(|c| (c as u32) < 0x20) {
        return Err(PebbleError::Validation(
            "Filename contains control characters".to_string(),
        ));
    }
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if is_windows_reserved_name(stem) {
        return Err(PebbleError::Validation(
            "Filename is reserved on Windows".to_string(),
        ));
    }

    // Ensure parent directory actually exists and is absolute
    if !canonical.is_absolute() {
        return Err(PebbleError::Internal(
            "Save path must be absolute".to_string(),
        ));
    }

    // Ensure target is within user home directory to prevent writes to system paths
    let home = home_dir()
        .ok_or_else(|| PebbleError::Internal("Cannot determine user home directory".to_string()))?;
    if !canonical.starts_with(&home) {
        return Err(PebbleError::Validation(
            "Save path must be within user home directory".to_string(),
        ));
    }

    Ok(())
}

/// Get the user's home directory.
fn home_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .ok()
            .map(std::path::PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(std::path::PathBuf::from)
    }
}

fn copy_attachment_file_safely<F>(
    source: &Path,
    save_path: &Path,
    mut on_progress: F,
) -> Result<(), PebbleError>
where
    F: FnMut(u64, u64),
{
    use std::io::{Read, Write};

    let mut src_file = std::fs::File::open(source)
        .map_err(|e| PebbleError::Internal(format!("Failed to open source: {e}")))?;
    let total_bytes = src_file
        .metadata()
        .map_err(|e| PebbleError::Internal(format!("Failed to read file metadata: {e}")))?
        .len();

    // create_new refuses to follow or replace an existing target, including a
    // symlink planted after path validation and before the file is opened.
    let mut dst_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(save_path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                PebbleError::Validation(
                    "Target file already exists; choose a new filename".to_string(),
                )
            } else {
                PebbleError::Internal(format!("Failed to create target file: {e}"))
            }
        })?;

    let mut buf = [0u8; 8192];
    let mut bytes_copied: u64 = 0;
    let copy_result: std::result::Result<(), PebbleError> = (|| {
        loop {
            let n = src_file
                .read(&mut buf)
                .map_err(|e| PebbleError::Internal(format!("Read error: {e}")))?;
            if n == 0 {
                break;
            }
            dst_file
                .write_all(&buf[..n])
                .map_err(|e| PebbleError::Internal(format!("Write error: {e}")))?;
            bytes_copied += n as u64;
            on_progress(bytes_copied, total_bytes);
        }
        dst_file
            .sync_all()
            .map_err(|e| PebbleError::Internal(format!("Failed to flush file: {e}")))?;
        Ok(())
    })();

    if let Err(e) = copy_result {
        drop(dst_file);
        let _ = std::fs::remove_file(save_path);
        return Err(e);
    }

    Ok(())
}

#[tauri::command]
pub async fn list_attachments(
    state: State<'_, AppState>,
    message_id: String,
) -> std::result::Result<Vec<Attachment>, PebbleError> {
    state.store.list_attachments_by_message(&message_id)
}

#[tauri::command]
pub async fn get_attachment_path(
    state: State<'_, AppState>,
    attachment_id: String,
) -> std::result::Result<Option<String>, PebbleError> {
    let att = state.store.get_attachment(&attachment_id)?;
    Ok(att.and_then(|a| a.local_path))
}

#[tauri::command]
pub async fn download_attachment(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    attachment_id: String,
    save_to: String,
) -> std::result::Result<(), PebbleError> {
    let att = state
        .store
        .get_attachment(&attachment_id)?
        .ok_or_else(|| PebbleError::Internal("Attachment not found".to_string()))?;
    // Validate save path to prevent path traversal
    validate_save_path(&save_to)?;

    let source = att
        .local_path
        .ok_or_else(|| PebbleError::Internal("Attachment file not available".to_string()))?;

    let att_id = attachment_id.clone();
    // Use spawn_blocking to avoid blocking the async executor on large files
    tokio::task::spawn_blocking(move || {
        let source_path = std::path::Path::new(&source);
        let save_path = std::path::Path::new(&save_to);
        copy_attachment_file_safely(source_path, save_path, |bytes_copied, total_bytes| {
            let _ = app.emit(
                "attachment:download-progress",
                serde_json::json!({
                    "attachment_id": att_id,
                    "bytes_copied": bytes_copied,
                    "total_bytes": total_bytes,
                }),
            );
        })
    })
    .await
    .map_err(|e| PebbleError::Internal(format!("Copy task failed: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_attachment_file_safely_rejects_existing_target() {
        let unique = pebble_core::new_id();
        let base = std::env::temp_dir().join(format!("pebble-attachment-copy-{unique}"));
        std::fs::create_dir_all(&base).expect("test dir");
        let source = base.join("source.txt");
        let target = base.join("target.txt");
        std::fs::write(&source, b"new content").expect("source write");
        std::fs::write(&target, b"existing content").expect("target write");

        let err = copy_attachment_file_safely(&source, &target, |_copied, _total| {})
            .expect_err("existing targets must not be overwritten");

        assert!(matches!(err, PebbleError::Validation(_)));
        assert_eq!(
            std::fs::read(&target).expect("target read"),
            b"existing content"
        );

        let _ = std::fs::remove_dir_all(base);
    }
}
