use crossworld_cube::{Cube, IVec3Ext, glam::IVec3};
use noise::{Fbm, NoiseFn, Perlin};
use std::rc::Rc;

// Material indices from materials.json
const HARD_GROUND: i32 = 16;  // Bedrock
const STONE: i32 = 20;        // Primary underground
const DIRT: i32 = 18;         // Underground and surface
const GRASS: i32 = 19;        // Surface vegetation
const WATER: i32 = 17;        // Water bodies
const SAND: i32 = 22;         // Beaches and deserts
const SANDSTONE: i32 = 23;    // Desert underground
const GRAVEL: i32 = 24;       // River beds
const CLAY: i32 = 25;         // Underground pockets
const SNOW: i32 = 26;         // Mountain peaks
const ICE_SOLID: i32 = 27;    // Frozen biomes
const GRANITE: i32 = 30;      // Mountain core
const ANDESITE: i32 = 32;     // Mountain variation
const LIMESTONE: i32 = 34;    // Underground caves
const BASALT: i32 = 35;       // Volcanic areas
const COAL: i32 = 48;         // Ore veins
const IRON: i32 = 49;         // Ore veins
const NETHERRACK: i32 = 29;   // Deep underground
const COBBLESTONE: i32 = 21;  // Stone variation

/// Biome type determined by temperature and moisture
#[derive(Debug, Clone, Copy, PartialEq)]
enum Biome {
    Ocean,
    Beach,
    Desert,
    Plains,
    Forest,
    Mountains,
    Tundra,
    IceCap,
}

/// Build octree for ground with advanced terrain generation
pub fn build_ground_octree(noise: &Perlin, fbm: &Fbm<Perlin>, depth: u32) -> Cube<i32> {
    // Start recursive build at (0,0,0) with specified depth
    // Simplify after building to optimize octree structure
    build_octree_recursive(0, 0, 0, depth, depth, noise, fbm).simplified()
}

/// Recursively build octree from given position and depth
///
/// - base_x, base_y, base_z: Position in voxel grid coordinates [0, 2^max_depth)
/// - depth: Current depth level (max_depth = root, 0 = leaf voxel)
/// - max_depth: Maximum octree depth (for centering calculations)
fn build_octree_recursive(
    base_x: i32,
    base_y: i32,
    base_z: i32,
    depth: u32,
    max_depth: u32,
    noise: &Perlin,
    fbm: &Fbm<Perlin>,
) -> Cube<i32> {
    if depth == 0 {
        // Base case: create leaf voxel
        let voxel_x = base_x;
        let voxel_y = base_y;
        let voxel_z = base_z;

        // Convert to centered coordinates
        let half_grid = 1 << max_depth; // 2^max_depth / 2
        let world_y = voxel_y - half_grid;

        let value = get_voxel_value(voxel_x, world_y, voxel_z, max_depth, noise, fbm);

        return Cube::Solid(value);
    }

    // Recursive case: create 8 children at next depth level
    let step = 1 << depth; // 2^depth

    let children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|octant_idx| {
        let offset = IVec3::from_octant_index(octant_idx);
        let child_x = base_x + offset.x * step;
        let child_y = base_y + offset.y * step;
        let child_z = base_z + offset.z * step;

        Rc::new(build_octree_recursive(
            child_x,
            child_y,
            child_z,
            depth - 1,
            max_depth,
            noise,
            fbm,
        ))
    });

    // Simplify after creating each node to collapse uniform regions
    Cube::cubes(children).simplified()
}

/// Generate multi-octave terrain height using Fractional Brownian Motion
fn get_terrain_height(x: i32, z: i32, _max_depth: u32, noise: &Perlin, fbm: &Fbm<Perlin>) -> f64 {
    let scale = 0.02; // Base scale for large terrain features
    let wx = x as f64 * scale;
    let wz = z as f64 * scale;

    // Multi-octave terrain height with reduced amplitudes
    let octave1 = fbm.get([wx, wz]) * 8.0;            // Large features
    let octave2 = noise.get([wx * 2.0, wz * 2.0]) * 3.0;   // Medium features
    let octave3 = noise.get([wx * 4.0, wz * 4.0]) * 1.0;   // Small features
    let octave4 = noise.get([wx * 8.0, wz * 8.0]) * 0.5;   // Fine detail

    // Apply gentler power function for smoother terrain
    let raw_height = octave1 + octave2 + octave3 + octave4;
    let normalized = (raw_height / 12.5 + 0.5).clamp(0.0, 1.0); // Normalize to 0-1
    let redistributed = normalized.powf(1.8); // Gentler redistribution

    // Map to height range: 0 to +25 (25 block range, shifted up so min is at y=0)
    redistributed * 25.0 - 5.0
}

/// Get temperature value for biome classification
fn get_temperature(x: i32, z: i32, height: f64, noise: &Perlin) -> f64 {
    let scale = 0.015;
    let wx = x as f64 * scale;
    let wz = z as f64 * scale;

    // Temperature decreases with height
    let base_temp = noise.get([wx + 1000.0, wz + 1000.0]); // Offset for independence
    let height_factor = -(height / 50.0).max(0.0); // Colder at high elevations

    (base_temp + height_factor).clamp(-1.0, 1.0)
}

/// Get moisture value for biome classification
fn get_moisture(x: i32, z: i32, noise: &Perlin) -> f64 {
    let scale = 0.018;
    let wx = x as f64 * scale;
    let wz = z as f64 * scale;

    // Independent moisture map
    noise.get([wx + 2000.0, wz + 2000.0]) // Offset for independence from temperature
}

/// Determine biome from temperature and moisture
fn get_biome(height: f64, temperature: f64, moisture: f64) -> Biome {
    // Ocean and beach based on height (range now 0-25)
    if height < 4.0 {
        return Biome::Ocean;
    }
    if height < 7.0 {
        return Biome::Beach;
    }

    // Mountain ranges at high elevations
    if height > 18.0 {
        if temperature < -0.3 {
            return Biome::IceCap;
        }
        return Biome::Mountains;
    }

    // Temperature-moisture based classification
    if temperature < -0.5 {
        Biome::Tundra
    } else if temperature < 0.0 && moisture < -0.3 {
        Biome::Tundra
    } else if temperature > 0.3 && moisture < -0.2 {
        Biome::Desert
    } else if moisture > 0.2 {
        Biome::Forest
    } else {
        Biome::Plains
    }
}

/// Get voxel material at given coordinates
fn get_voxel_value(x: i32, y: i32, z: i32, max_depth: u32, noise: &Perlin, fbm: &Fbm<Perlin>) -> i32 {
    let height = get_terrain_height(x, z, max_depth, noise, fbm);
    let depth_below = height - y as f64;

    // Air above terrain
    if depth_below < 0.0 {
        // Fill all space below y=0 with water
        if y < 0 {
            return WATER;
        }
        return 0;
    }

    // Bedrock at very deep levels
    if y < -50 {
        return HARD_GROUND;
    }

    // Deep underground layer (hell)
    if y < -40 {
        return if noise.get([x as f64 * 0.1, y as f64 * 0.1, z as f64 * 0.1]) > 0.3 {
            NETHERRACK
        } else {
            BASALT
        };
    }

    // Get biome
    let temperature = get_temperature(x, z, height, noise);
    let moisture = get_moisture(x, z, noise);
    let biome = get_biome(height, temperature, moisture);

    // Ore generation (coal and iron veins)
    if depth_below > 4.0 && depth_below < 40.0 {
        let ore_noise = noise.get([x as f64 * 0.3, y as f64 * 0.3, z as f64 * 0.3]);
        if ore_noise > 0.75 {
            return COAL;
        } else if ore_noise < -0.78 {
            return IRON;
        }
    }

    // Cave generation using 3D noise
    if depth_below > 3.0 && depth_below < 50.0 {
        let cave_noise = fbm.get([x as f64 * 0.08, y as f64 * 0.08, z as f64 * 0.08]);
        if cave_noise > 0.55 {
            return 0; // Cave air
        }
    }

    // Surface layer material based on biome
    if depth_below < 1.0 {
        return match biome {
            Biome::Ocean => SAND,
            Biome::Beach => SAND,
            Biome::Desert => SAND,
            Biome::Plains => GRASS,
            Biome::Forest => GRASS,
            Biome::Mountains => if height > 21.0 { SNOW } else { STONE },
            Biome::Tundra => SNOW,
            Biome::IceCap => ICE_SOLID,
        };
    }

    // Sub-surface layers (1-4 blocks deep)
    if depth_below < 4.0 {
        return match biome {
            Biome::Ocean => SAND,
            Biome::Beach => SAND,
            Biome::Desert => SANDSTONE,
            Biome::Plains => DIRT,
            Biome::Forest => DIRT,
            Biome::Mountains => STONE,
            Biome::Tundra => DIRT,
            Biome::IceCap => ICE_SOLID,
        };
    }

    // Medium depth (4-10 blocks)
    if depth_below < 10.0 {
        // Clay pockets
        let clay_noise = noise.get([x as f64 * 0.2, z as f64 * 0.2]);
        if clay_noise > 0.6 {
            return CLAY;
        }

        return match biome {
            Biome::Desert => SANDSTONE,
            Biome::Beach => GRAVEL,
            Biome::Mountains => GRANITE,
            _ => STONE,
        };
    }

    // Deep underground (10+ blocks)
    let stone_variant = noise.get([x as f64 * 0.15, y as f64 * 0.15, z as f64 * 0.15]);

    if biome == Biome::Mountains && height > 16.0 {
        // Mountain core (adjusted threshold for new height range 0-25)
        if stone_variant > 0.3 {
            GRANITE
        } else if stone_variant < -0.3 {
            ANDESITE
        } else {
            STONE
        }
    } else {
        // Regular underground
        if stone_variant > 0.5 {
            COBBLESTONE
        } else if stone_variant > 0.2 {
            LIMESTONE
        } else if stone_variant < -0.5 {
            BASALT
        } else {
            STONE
        }
    }
}
