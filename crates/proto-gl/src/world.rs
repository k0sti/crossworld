use crate::config::WorldConfig;
use cube::{Cube, parse_csm};
use std::rc::Rc;

/// Generate world cube from configuration
pub fn generate_world(config: &WorldConfig) -> (Cube<u8>, u32) {
    // Parse CSM
    let parse_result = parse_csm(&config.root_cube);
    let mut cube = match parse_result {
        Ok(result) => result,
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
        cube = Cube::expand(&cube, config.border_materials, config.border_depth as i32);
    }

    (cube, total_depth)
}
