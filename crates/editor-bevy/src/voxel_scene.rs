use bevy::prelude::*;

/// Resource holding the voxel scene state
/// Note: Full Cube integration requires NonSend resources due to Cube's Rc internals
#[derive(Resource, Default)]
pub struct VoxelScene {
    /// Flag indicating mesh needs regeneration
    pub mesh_dirty: bool,
    /// Scene depth for voxel operations
    #[allow(dead_code)]
    pub depth: u32,
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
