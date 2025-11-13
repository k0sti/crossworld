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
