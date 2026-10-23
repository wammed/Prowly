//! Soft-discard safety backup service.
//! Saves backups of files or whole worktrees before destructive discards.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use std::sync::atomic::{AtomicU64, Ordering};

static BACKUP_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Metadata regarding a safety backup saved during a discard operation.
#[derive(Debug, Clone)]
pub struct BackupRecord {
    pub id: String,
    pub timestamp_secs: u64,
    pub original_path: String,
    pub backup_path: PathBuf,
}

/// Manages temporary and hidden backups so user modifications are never permanently lost.
pub struct BackupService {
    backup_root: PathBuf,
}

impl BackupService {
    /// Initialize backup manager in local data directory.
    pub fn new() -> Self {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(".prowly_data"))
            .join("prowly")
            .join("backups");
        Self { backup_root: base }
    }

    /// Initialize backup manager with a custom root directory (useful for unit testing).
    pub fn with_root(root: PathBuf) -> Self {
        Self { backup_root: root }
    }

    /// Save a safety copy of a file before discarding modifications.
    pub fn backup_file(&self, file_path: &Path) -> Result<BackupRecord, std::io::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp = now.as_secs();
        let nanos = now.as_nanos();
        let counter = BACKUP_COUNTER.fetch_add(1, Ordering::Relaxed);

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let safe_name = file_name.replace(['/', '\\'], "_");

        let id = format!("discard_{}_{}_{}", nanos, counter, safe_name);
        let dest_dir = self.backup_root.join(&id);
        fs::create_dir_all(&dest_dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dest_dir, fs::Permissions::from_mode(0o700));
        }

        let dest_file = dest_dir.join(&file_name);
        let tmp_file = dest_dir.join(format!(".{}.tmp", file_name));

        if file_path.exists() {
            fs::copy(file_path, &tmp_file)?;
            fs::rename(&tmp_file, &dest_file)?;
        } else {
            // Empty placeholder if the file was newly created or untracked
            fs::write(&tmp_file, "")?;
            fs::rename(&tmp_file, &dest_file)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dest_file, fs::Permissions::from_mode(0o600));
        }

        Ok(BackupRecord {
            id,
            timestamp_secs: timestamp,
            original_path: file_path.to_string_lossy().to_string(),
            backup_path: dest_file,
        })
    }

    /// Restore a previously saved backup file safely, rejecting path traversals.
    pub fn restore_backup(&self, record: &BackupRecord) -> Result<(), std::io::Error> {
        let dest = Path::new(&record.original_path);
        // Security check: ensure path does not contain parent directory traversal ('..')
        if dest
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Restoration path contains invalid parent directory traversals",
            ));
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&record.backup_path, dest)?;
        Ok(())
    }
}

impl Default for BackupService {
    fn default() -> Self {
        Self::new()
    }
}
