use crate::protocol::EditOperation;
use cube::Cube;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};
use thiserror::Error;

/// Errors emitted by storage implementations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("world file missing")]
    NotFound,
}

/// Backend trait for persisting world data.
pub trait StorageBackend: Send + Sync + 'static {
    fn save_world(&self, root: &Cube<u8>) -> Result<(), StorageError>;
    fn load_world(&self) -> Result<Cube<u8>, StorageError>;
    fn save_edit(
        &self,
        edit: &EditOperation,
        timestamp: u64,
        author: &str,
    ) -> Result<(), StorageError>;
    fn compact(&self) -> Result<(), StorageError>;
}

/// File-based storage that mirrors the layout described in docs/server.md.
#[derive(Debug)]
pub struct FileStorage {
    world_path: PathBuf,
    edit_log_path: PathBuf,
}

impl FileStorage {
    pub fn new(world_path: impl Into<PathBuf>, edit_log_path: impl Into<PathBuf>) -> Self {
        Self {
            world_path: world_path.into(),
            edit_log_path: edit_log_path.into(),
        }
    }
}

impl StorageBackend for FileStorage {
    fn save_world(&self, root: &Cube<u8>) -> Result<(), StorageError> {
        let data = bincode::serialize(root)?;
        let tmp = self.world_path.with_extension("tmp");
        fs::write(&tmp, data)?;
        fs::rename(tmp, &self.world_path)?;
        Ok(())
    }

    fn load_world(&self) -> Result<Cube<u8>, StorageError> {
        match fs::read(&self.world_path) {
            Ok(bytes) => {
                let cube = bincode::deserialize(&bytes)?;
                Ok(cube)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Err(StorageError::NotFound),
            Err(err) => Err(StorageError::Io(err)),
        }
    }

    fn save_edit(
        &self,
        edit: &EditOperation,
        timestamp: u64,
        author: &str,
    ) -> Result<(), StorageError> {
        let entry = EditLogEntry {
            timestamp,
            author: author.to_string(),
            operation: edit.clone(),
        };

        let data = bincode::serialize(&entry)?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.edit_log_path)?;

        file.write_all(&(data.len() as u32).to_be_bytes())?;
        file.write_all(&data)?;

        Ok(())
    }

    fn compact(&self) -> Result<(), StorageError> {
        fs::write(&self.edit_log_path, &[])?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct EditLogEntry {
    timestamp: u64,
    author: String,
    operation: EditOperation,
}
