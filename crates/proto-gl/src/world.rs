use std::rc::Rc;
use cube::{Cube, parse_csm};
use crate::config::WorldConfig;

/// Generate world cube from configuration
pub fn generate_world(config: &WorldConfig) -> (Cube<u8>, u32) {
    // Parse CSM
    let parse_result = parse_csm(&config.root_cube);
    let mut cube = match parse_result {
        Ok(result) => result.root,
        Err(e) => {
            eprintln!("Warning: Failed to parse CSM: {}", e);
            eprintln!("Using simple octree default");
            Cube::cubes([
                Rc::new(Cube::solid(5)),
                Rc::new(Cube::solid(5)),
                Rc::new(Cube::solid(4)),
                Rc::new(Cube::solid(9)),
                Rc::new(Cube::solid(5)),
                Rc::new(Cube::solid(5)),
                Rc::new(Cube::solid(0)),
                Rc::new(Cube::solid(0)),
            ]);
            Cube::solid(8)
        }
    };

    // Calculate total depth
    let total_depth = config.macro_depth + config.micro_depth + config.border_depth;

    // Apply border layers if needed
    if config.border_depth > 0 {
        cube = add_border_layers(cube, config.border_depth, config.border_materials);
    }
    // cube = Cube::solid(32);

    (cube, total_depth)
}

/// Add border layers to cube (copied pattern from proto)
pub fn add_border_layers(cube: Cube<u8>, border_depth: u32, materials: [u8; 4]) -> Cube<u8> {
    let mut result = cube;
    for _ in 0..border_depth {
        let child = Rc::new(result.clone());
        result = Cube::cubes([
            Rc::new(Cube::solid(materials[0])), // Bottom layer
            child.clone(),
            child.clone(),
            child.clone(),
            child.clone(),
            child.clone(),
            child.clone(),
            Rc::new(Cube::solid(materials[1])), // Top layer
        ]);
    }
    result
}
