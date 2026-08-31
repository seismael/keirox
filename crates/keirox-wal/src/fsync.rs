use keirox_core::error::{KeiroxError, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Fsync storage for persistent state that requires strong durability (like Raft HardState).
/// Implements atomic file writes and hardware `fsync` via `sync_all`.
pub struct FsyncStorage {
    file_path: PathBuf,
}

impl FsyncStorage {
    /// Create a new FsyncStorage referencing a specific file path.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            file_path: path.as_ref().to_path_buf(),
        }
    }

    /// Read the persisted state from disk. Returns `None` if the file does not exist.
    pub fn read_state(&self) -> Result<Option<Vec<u8>>> {
        if !self.file_path.exists() {
            return Ok(None);
        }

        let mut file = File::open(&self.file_path).map_err(|e| {
            KeiroxError::StorageIo(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to open HardState file: {}", e),
            ))
        })?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| {
            KeiroxError::StorageIo(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read HardState file: {}", e),
            ))
        })?;

        Ok(Some(buffer))
    }

    /// Write state to disk atomically using a temporary file and `sync_all`.
    pub fn write_state_sync(&self, data: &[u8]) -> Result<()> {
        let dir = self.file_path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(dir).map_err(|e| KeiroxError::StorageIo(e))?;

        // 1. Write to temp file
        let mut temp_file = tempfile::NamedTempFile::new_in(dir).map_err(|e| {
            KeiroxError::StorageIo(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create temporary state file: {}", e),
            ))
        })?;

        temp_file.write_all(data).map_err(|e| {
            KeiroxError::StorageIo(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write state data: {}", e),
            ))
        })?;

        // 2. Hardware fsync to ensure data is physically on NVMe/disk
        temp_file.as_file_mut().sync_all().map_err(|e| {
            KeiroxError::StorageIo(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("fsync failed on state data: {}", e),
            ))
        })?;

        // 3. Atomic rename to the final path
        temp_file.persist(&self.file_path).map_err(|e| {
            KeiroxError::StorageIo(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to persist state file: {}", e.error),
            ))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fsync_storage_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("hard_state.bin");
        let storage = FsyncStorage::new(&path);

        assert!(storage.read_state().unwrap().is_none());

        let state_data = b"MOCK_RAFT_HARD_STATE_V1";
        storage.write_state_sync(state_data).unwrap();

        let read_back = storage.read_state().unwrap().unwrap();
        assert_eq!(read_back, state_data);
    }
}
