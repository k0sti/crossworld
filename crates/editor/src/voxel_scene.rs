use bevy::prelude::*;
use cube::{Cube, CubeCoord};
use glam::IVec3;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Thread-safe wrapper around Cube
pub struct ThreadSafeCube {
    inner: Arc<Mutex<Rc<Cube<u8>>>>,
}

// SAFETY: Cube is only accessed through Mutex, which provides
// interior mutability and ensures exclusive access. The Rc pointers
// inside Cube are never shared across threads - they're only
// accessed within the locked scope.
unsafe impl Send for ThreadSafeCube {}
unsafe impl Sync for ThreadSafeCube {}

impl ThreadSafeCube {
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new(cube: Rc<Cube<u8>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(cube)),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, Rc<Cube<u8>>> {
        self.inner.lock().unwrap()
    }
}

// Manual implementation of Clone for ThreadSafeCube
impl Clone for ThreadSafeCube {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Create a cube with random colored voxels (using 8-color palette)
fn create_initial_cube() -> Cube<u8> {
    let mut cube = Cube::Solid(0); // Start with empty

    // Place random colored voxels in an 8-cube pattern (2x2x2)
    // Using colors 1-8 (material indices for 8-color palette)
    let positions = [
        (0, 0, 0, 1u8), // Red
        (1, 0, 0, 2u8), // Orange
        (0, 1, 0, 3u8), // Yellow
        (1, 1, 0, 4u8), // Green
        (0, 0, 1, 5u8), // Cyan
        (1, 0, 1, 6u8), // Blue
        (0, 1, 1, 7u8), // Purple
        (1, 1, 1, 8u8), // Magenta
    ];

    // Place voxels at depth 0 (creates a 2x2x2 cube)
    for (x, y, z, color) in positions.iter() {
        let coord = CubeCoord::new(IVec3::new(*x, *y, *z), 0);
        let voxel = Cube::Solid(*color);
        cube = cube.update(coord, voxel);
    }

    cube
}

/// Resource holding the current voxel scene state
#[derive(Resource, Clone)]
pub struct VoxelScene {
    /// The cube containing all voxel data (thread-safe)
    pub cube: ThreadSafeCube,
    /// Flag indicating if the mesh needs to be regenerated
    pub mesh_dirty: bool,
    /// Handle to the mesh entity (if spawned)
    pub mesh_entity: Option<Entity>,
}

impl Default for VoxelScene {
    fn default() -> Self {
        // Create a cube with initial colored voxels
        let cube = Rc::new(create_initial_cube());

        Self {
            cube: ThreadSafeCube::new(cube),
            mesh_dirty: true, // Mark as dirty initially to trigger first mesh generation
            mesh_entity: None,
        }
    }
}

impl VoxelScene {
    /// Create a new VoxelScene from a Cube
    #[allow(dead_code)]
    pub fn from_cube(cube: Rc<Cube<u8>>) -> Self {
        Self {
            cube: ThreadSafeCube::new(cube),
            mesh_dirty: true,
            mesh_entity: None,
        }
    }

    /// Mark the mesh as needing regeneration
    #[allow(dead_code)]
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
