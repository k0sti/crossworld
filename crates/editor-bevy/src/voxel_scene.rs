use bevy::prelude::*;

/// Resource holding the voxel scene state
/// Note: Full Cube integration requires NonSend resources due to Cube's Rc internals
#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct VoxelScene {
    /// Flag indicating mesh needs regeneration
    pub mesh_dirty: bool,
    /// Scene depth for voxel operations
    pub depth: u32,
}

#[allow(dead_code)]
impl VoxelScene {
    /// Create a new empty scene
    pub fn new() -> Self {
        Self {
            mesh_dirty: true,
            depth: 6,
        }
    }

    /// Mark mesh as needing update
    pub fn mark_dirty(&mut self) {
        self.mesh_dirty = true;
    }
}

/// System that initializes the voxel scene
fn init_voxel_scene(mut scene: ResMut<VoxelScene>) {
    scene.mesh_dirty = true;
    info!("VoxelScene initialized");
}

/// Plugin for voxel scene management
pub struct VoxelScenePlugin;

impl Plugin for VoxelScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelScene>()
            .add_systems(Startup, init_voxel_scene);
    }
}
