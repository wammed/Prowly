//! Soft-discard safety backup service.
//! Saves backups of files or whole worktrees before destructive discards.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        let id = format!("discard_{}_{}", timestamp, file_name);
        let dest_dir = self.backup_root.join(&id);
        fs::create_dir_all(&dest_dir)?;

        let dest_file = dest_dir.join(&file_name);
        if file_path.exists() {
            fs::copy(file_path, &dest_file)?;
        } else {
            // Empty placeholder if the file was newly created or untracked
            fs::write(&dest_file, "")?;
        }

        Ok(BackupRecord {
            id,
            timestamp_secs: timestamp,
            original_path: file_path.to_string_lossy().to_string(),
            backup_path: dest_file,
        })
    }

    /// Restore a previously saved backup file.
    pub fn restore_backup(&self, record: &BackupRecord) -> Result<(), std::io::Error> {
        let dest = Path::new(&record.original_path);
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
