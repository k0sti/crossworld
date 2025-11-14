pub mod cache;
pub mod lod;
pub mod storage;

use self::{cache::WorldCache, storage::StorageBackend};
use crate::{
    config::WorldConfig,
    protocol::{EditOperation, WorldInfo},
};
use cube::{glam::IVec3, Cube, CubeCoord};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("storage error: {0}")]
    Storage(#[from] storage::StorageError),
}

/// Shared world state (backed by storage + cache).
#[derive(Debug)]
pub struct WorldState<B: StorageBackend> {
    inner: Arc<WorldStateInner<B>>,
}

#[derive(Debug)]
struct WorldStateInner<B: StorageBackend> {
    backend: B,
    info: WorldInfo,
    cache: WorldCache,
    root: RwLock<Cube<u8>>,
}

impl<B: StorageBackend> Clone for WorldState<B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<B: StorageBackend> WorldState<B> {
    pub fn load_or_default(config: WorldConfig, backend: B) -> Result<Self, WorldError> {
        let root = match backend.load_world() {
            Ok(cube) => cube,
            Err(storage::StorageError::NotFound) => Cube::Solid(0),
            Err(err) => return Err(WorldError::Storage(err)),
        };
        Ok(Self::new(config, backend, root))
    }

    pub fn new(config: WorldConfig, backend: B, root: Cube<u8>) -> Self {
        let info = WorldInfo {
            world_id: config.world_id.clone(),
            max_depth: config.max_depth(),
            macro_depth: config.macro_depth,
            border_depth: config.border_depth,
        };

        Self {
            inner: Arc::new(WorldStateInner {
                backend,
                info,
                cache: WorldCache::new(config.cache_capacity),
                root: RwLock::new(root),
            }),
        }
    }

    pub fn info(&self) -> WorldInfo {
        self.inner.info.clone()
    }

    pub async fn get_cube(&self, coord: CubeCoord) -> Result<Cube<u8>, WorldError> {
        if let Some(cached) = self.inner.cache.get(coord).await {
            return Ok(cached);
        }

        // TODO: traverse octree based on coord. For now, fall back to root clone.
        Ok(self.inner.root.read().await.clone())
    }

    pub async fn apply_edit(
        &self,
        operation: &EditOperation,
        author: &str,
        timestamp: u64,
    ) -> Result<(), WorldError> {
        match operation {
            EditOperation::SetCube { coord, cube } => {
                if is_root(coord) {
                    let mut root = self.inner.root.write().await;
                    *root = cube.clone();
                    self.inner.cache.clear().await;
                    self.inner.backend.save_world(&*root)?;
                } else {
                    self.inner.cache.insert(*coord, cube.clone()).await;
                    self.inner.backend.save_edit(operation, timestamp, author)?;
                }
            }
        }

        Ok(())
    }
}

fn is_root(coord: &CubeCoord) -> bool {
    coord.pos == IVec3::ZERO && coord.depth == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::EditOperation;
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[derive(Clone)]
    struct MemoryBackend {
        root: Arc<Mutex<Vec<u8>>>,
        edits: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl MemoryBackend {
        fn new(initial: Cube<u8>) -> Self {
            Self {
                root: Arc::new(Mutex::new(
                    bincode::serialize(&initial).expect("serialize initial"),
                )),
                edits: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl StorageBackend for MemoryBackend {
        fn save_world(&self, root: &Cube<u8>) -> Result<(), storage::StorageError> {
            *self.root.lock().unwrap() = bincode::serialize(root)?;
            Ok(())
        }

        fn load_world(&self) -> Result<Cube<u8>, storage::StorageError> {
            Ok(bincode::deserialize(&self.root.lock().unwrap()).unwrap())
        }

        fn save_edit(
            &self,
            edit: &EditOperation,
            _timestamp: u64,
            _author: &str,
        ) -> Result<(), storage::StorageError> {
            self.edits
                .lock()
                .unwrap()
                .push(bincode::serialize(edit).unwrap());
            Ok(())
        }

        fn compact(&self) -> Result<(), storage::StorageError> {
            Ok(())
        }
    }

    fn test_config() -> WorldConfig {
        WorldConfig {
            world_id: "test".into(),
            world_path: PathBuf::from("world"),
            edit_log_path: PathBuf::from("log"),
            macro_depth: 5,
            micro_depth: 3,
            border_depth: 1,
            cache_capacity: 4,
        }
    }

    #[tokio::test]
    async fn replaces_root_on_edit() {
        let backend = MemoryBackend::new(Cube::Solid(0));
        let state = WorldState::new(test_config(), backend.clone(), Cube::Solid(0));

        let operation = EditOperation::SetCube {
            coord: CubeCoord::new(IVec3::ZERO, 0),
            cube: Cube::Solid(7),
        };
        state
            .apply_edit(&operation, "tester", 1)
            .await
            .expect("apply");

        let stored: Cube<u8> =
            bincode::deserialize(&backend.root.lock().unwrap()).expect("decode root");
        assert_eq!(stored, Cube::Solid(7));
    }

    #[tokio::test]
    async fn non_root_edit_logged() {
        let backend = MemoryBackend::new(Cube::Solid(0));
        let state = WorldState::new(test_config(), backend.clone(), Cube::Solid(0));

        let coord = CubeCoord::new(IVec3::new(1, 0, 0), 1);
        let operation = EditOperation::SetCube {
            coord,
            cube: Cube::Solid(5),
        };

        state
            .apply_edit(&operation, "tester", 123)
            .await
            .expect("apply");

        let edits = backend.edits.lock().unwrap();
        assert_eq!(edits.len(), 1);
        let decoded: EditOperation = bincode::deserialize(&edits[0]).unwrap();
        match decoded {
            EditOperation::SetCube {
                coord: recorded, ..
            } => assert_eq!(recorded.pos, coord.pos),
        }
    }
}
