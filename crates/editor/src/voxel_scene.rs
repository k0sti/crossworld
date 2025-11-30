use bevy::prelude::*;
use crossworld_world::WorldCube;
use std::sync::{Arc, Mutex};

/// Thread-safe wrapper around WorldCube
pub struct ThreadSafeWorldCube {
    inner: Arc<Mutex<WorldCube>>,
}

// SAFETY: WorldCube is only accessed through Mutex, which provides
// interior mutability and ensures exclusive access. The Rc pointers
// inside WorldCube are never shared across threads - they're only
// accessed within the locked scope.
unsafe impl Send for ThreadSafeWorldCube {}
unsafe impl Sync for ThreadSafeWorldCube {}

impl ThreadSafeWorldCube {
    pub fn new(world: WorldCube) -> Self {
        Self {
            inner: Arc::new(Mutex::new(world)),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<WorldCube> {
        self.inner.lock().unwrap()
    }
}

// Manual implementation of Clone for ThreadSafeWorldCube
impl Clone for ThreadSafeWorldCube {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Resource holding the current voxel scene state
#[derive(Resource, Clone)]
pub struct VoxelScene {
    /// The world cube containing all voxel data (thread-safe)
    pub world: ThreadSafeWorldCube,
    /// Flag indicating if the mesh needs to be regenerated
    pub mesh_dirty: bool,
    /// Handle to the mesh entity (if spawned)
    pub mesh_entity: Option<Entity>,
}

impl Default for VoxelScene {
    fn default() -> Self {
        // Create a WorldCube with reasonable defaults
        // macro_depth: 3, micro_depth: 5, border_depth: 1, seed: 12345
        let world = WorldCube::new(3, 5, 1, 12345);

        Self {
            world: ThreadSafeWorldCube::new(world),
            mesh_dirty: true, // Mark as dirty initially to trigger first mesh generation
            mesh_entity: None,
        }
    }
}

impl VoxelScene {
    /// Create a new VoxelScene with specified parameters
    pub fn new(macro_depth: u32, micro_depth: u32, border_depth: u32, seed: u32) -> Self {
        let world = WorldCube::new(macro_depth, micro_depth, border_depth, seed);

        Self {
            world: ThreadSafeWorldCube::new(world),
            mesh_dirty: true,
            mesh_entity: None,
        }
    }

    /// Mark the mesh as needing regeneration
    pub fn mark_dirty(&mut self) {
        self.mesh_dirty = true;
    }

    /// Clear the dirty flag (called after mesh regeneration)
    pub fn clear_dirty(&mut self) {
        self.mesh_dirty = false;
    }
}

/// Plugin for voxel scene management
pub struct VoxelScenePlugin;

impl Plugin for VoxelScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelScene>();
    }
}
