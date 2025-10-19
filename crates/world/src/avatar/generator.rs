use super::voxel_model::{Voxel, VoxelModel};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Generate hash from string
fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Body types for humanoid/animal models
#[derive(Clone, Copy, Debug)]
pub enum BodyType {
    Slim,
    Normal,
    Bulky,
}

impl BodyType {
    fn scale_factor(&self) -> f32 {
        match self {
            BodyType::Slim => 0.8,
            BodyType::Normal => 1.0,
            BodyType::Bulky => 1.3,
        }
    }
}

/// Generate a sphere voxel model
pub fn generate_sphere(size: u8, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);

    // Palette
    let hash = hash_string(seed);
    let hue = ((hash % 360) as f32) / 360.0;
    model.palette.add_color(hue, 0.8, 0.6);

    let center = size as f32 / 2.0;
    let radius = center - 1.0;

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dz = z as f32 - center;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();

                if dist <= radius {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    model
}

/// Generate a cube voxel model
pub fn generate_cube(size: u8, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);

    // Palette
    let hash = hash_string(seed);
    let hue = ((hash % 360) as f32) / 360.0;
    model.palette.add_color(hue, 0.9, 0.5);

    let border = 1;
    for x in border..(size - border) {
        for y in border..(size - border) {
            for z in border..(size - border) {
                model.add_voxel(Voxel {
                    x,
                    y,
                    z,
                    color_index: 0,
                });
            }
        }
    }

    model
}

/// Generate a pyramid voxel model
pub fn generate_pyramid(size: u8, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);

    // Palette - golden color
    let hash = hash_string(seed);
    let hue = ((hash % 60) as f32) / 360.0 + 0.1; // Yellow-orange range
    model.palette.add_color(hue, 0.8, 0.6);

    let _center = size / 2;
    for y in 0..size {
        let layer_size = size - y * 2 / 3;
        if layer_size == 0 {
            break;
        }

        let offset = (size - layer_size) / 2;
        for x in offset..(offset + layer_size) {
            for z in offset..(offset + layer_size) {
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    model
}

/// Generate a torus (donut) voxel model
pub fn generate_torus(size: u8, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);

    // Palette - pink/brown donut color
    let hash = hash_string(seed);
    let hue = ((hash % 40) as f32) / 360.0 + 0.9; // Pink range
    model.palette.add_color(hue, 0.7, 0.6);

    let center = size as f32 / 2.0;
    let major_radius = center - 2.0;
    let minor_radius = (size as f32 / 6.0).max(1.0);

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dz = z as f32 - center;

                let dist_to_center_xz = (dx * dx + dz * dz).sqrt();
                let dist_to_ring = (dist_to_center_xz - major_radius).abs();
                let dist = (dist_to_ring * dist_to_ring + dy * dy).sqrt();

                if dist <= minor_radius {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    model
}

/// Generate a cylinder voxel model
pub fn generate_cylinder(size: u8, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);

    let hash = hash_string(seed);
    let hue = ((hash % 360) as f32) / 360.0;
    model.palette.add_color(hue, 0.7, 0.6);

    let center_x = size as f32 / 2.0;
    let center_z = size as f32 / 2.0;
    let radius = (size as f32 / 2.0) - 1.0;

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let dx = x as f32 - center_x;
                let dz = z as f32 - center_z;
                let dist = (dx * dx + dz * dz).sqrt();

                if dist <= radius {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    model
}

/// Generate a diamond/gem voxel model
pub fn generate_diamond(size: u8, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);

    // Crystal-like colors
    let hash = hash_string(seed);
    let hue = ((hash % 360) as f32) / 360.0;
    model.palette.add_color(hue, 0.9, 0.8);

    let center = size / 2;
    let mid_y = size / 2;

    // Top pyramid
    for y in mid_y..size {
        let layer_size = ((size - y) as f32 * 1.5) as u8;
        if layer_size == 0 {
            continue;
        }

        let offset = center.saturating_sub(layer_size / 2);
        for x in offset..(offset + layer_size).min(size) {
            for z in offset..(offset + layer_size).min(size) {
                model.add_voxel(Voxel {
                    x,
                    y,
                    z,
                    color_index: 0,
                });
            }
        }
    }

    // Bottom pyramid
    for y in 0..mid_y {
        let layer_size = ((y + 1) as f32 * 1.5) as u8;
        if layer_size == 0 {
            continue;
        }

        let offset = center.saturating_sub(layer_size / 2);
        for x in offset..(offset + layer_size).min(size) {
            for z in offset..(offset + layer_size).min(size) {
                model.add_voxel(Voxel {
                    x,
                    y,
                    z,
                    color_index: 0,
                });
            }
        }
    }

    model
}

/// Generate noise-based abstract model
pub fn generate_noise(size: u8, seed: &str, complexity: f32) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);

    // Multi-color palette for noise
    let hash = hash_string(seed);
    let base_hue = ((hash % 360) as f32) / 360.0;
    for i in 0..5 {
        let hue = (base_hue + i as f32 * 0.2) % 1.0;
        model.palette.add_color(hue, 0.7, 0.6);
    }

    let threshold = 0.5 - (complexity * 0.3);

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let fx = x as f32 / size as f32;
                let fy = y as f32 / size as f32;
                let fz = z as f32 / size as f32;

                let noise = simple_noise_3d(fx * 3.0, fy * 3.0, fz * 3.0, hash);

                if noise > threshold {
                    let color_idx = ((noise * 5.0) as u8).min(4);
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: color_idx,
                    });
                }
            }
        }
    }

    model
}

/// Simple 3D noise function
fn simple_noise_3d(x: f32, y: f32, z: f32, seed: u64) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;

    let _fx = x - ix as f32;
    let _fy = y - iy as f32;
    let _fz = z - iz as f32;

    let h = hash_3d(ix, iy, iz, seed);
    ((h % 1000) as f32) / 1000.0
}

fn hash_3d(x: i32, y: i32, z: i32, seed: u64) -> u64 {
    let mut h = seed;
    h ^= (x as u64).wrapping_mul(374761393);
    h ^= (y as u64).wrapping_mul(668265263);
    h ^= (z as u64).wrapping_mul(2147483647);
    h = h.wrapping_mul(1103515245).wrapping_add(12345);
    h
}

/// Generate a humanoid warrior
pub fn generate_warrior(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let _base_size = (size as f32 * 0.5).max(16.0) as u8;
    let scale = body_type.scale_factor();
    let mut model = VoxelModel::new(size, size, size);

    // Warrior palette
    let _hash = hash_string(seed);
    model.palette.add_color(0.8, 0.6, 0.4); // Skin
    model.palette.add_color(0.3, 0.3, 0.3); // Armor (dark gray)
    model.palette.add_color(0.6, 0.1, 0.1); // Cape (red)
    model.palette.add_color(0.4, 0.4, 0.4); // Metal

    let center_x = size / 2;
    let center_z = size / 2;
    let head_width = (4.0 * scale) as u8;
    let torso_width = (5.0 * scale) as u8;

    // Build armored humanoid
    // Head
    let head_y_start = size - head_width - 2;
    for y in head_y_start..(head_y_start + head_width) {
        for dx in 0..head_width {
            for dz in 0..head_width {
                let x = center_x - head_width / 2 + dx;
                let z = center_z - head_width / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 1,
                    }); // Helmet
                }
            }
        }
    }

    // Torso (armored)
    let torso_height = (8.0 * scale) as u8;
    let torso_y_start = head_y_start - torso_height;
    for y in torso_y_start..head_y_start {
        for dx in 0..torso_width {
            for dz in 0..(torso_width - 1) {
                let x = center_x - torso_width / 2 + dx;
                let z = center_z - (torso_width - 1) / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 1,
                    });
                }
            }
        }
    }

    model
}

/// Generate a simple peasant
pub fn generate_peasant(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let scale = body_type.scale_factor();
    let mut model = VoxelModel::new(size, size, size);

    // Peasant palette - simple cloth colors
    let _hash = hash_string(seed);
    model.palette.add_color(0.8, 0.5, 0.5); // Skin
    model.palette.add_color(0.15, 0.4, 0.3); // Brown cloth
    model.palette.add_color(0.05, 0.3, 0.2); // Dark brown

    let center_x = size / 2;
    let center_z = size / 2;
    let head_width = (4.0 * scale) as u8;
    let torso_width = (4.0 * scale) as u8;

    // Head (skin)
    let head_y_start = size - head_width - 2;
    for y in head_y_start..(head_y_start + head_width) {
        for dx in 0..head_width {
            for dz in 0..head_width {
                let x = center_x - head_width / 2 + dx;
                let z = center_z - head_width / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Torso (simple cloth)
    let torso_height = (8.0 * scale) as u8;
    let torso_y_start = head_y_start - torso_height;
    for y in torso_y_start..head_y_start {
        for dx in 0..torso_width {
            for dz in 0..(torso_width - 1) {
                let x = center_x - torso_width / 2 + dx;
                let z = center_z - (torso_width - 1) / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 1,
                    });
                }
            }
        }
    }

    model
}

/// Generate a mage
pub fn generate_mage(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let scale = body_type.scale_factor();
    let mut model = VoxelModel::new(size, size, size);

    // Mage palette - mystical colors
    let _hash = hash_string(seed);
    model.palette.add_color(0.8, 0.5, 0.4); // Skin
    model.palette.add_color(0.65, 0.7, 0.5); // Purple robe
    model.palette.add_color(0.55, 0.8, 0.7); // Bright purple
    model.palette.add_color(0.15, 0.6, 0.9); // Magical glow (cyan)

    let center_x = size / 2;
    let center_z = size / 2;
    let head_width = (4.0 * scale) as u8;
    let torso_width = (5.0 * scale) as u8;

    // Head
    let head_y_start = size - head_width - 4;
    for y in head_y_start..(head_y_start + head_width) {
        for dx in 0..head_width {
            for dz in 0..head_width {
                let x = center_x - head_width / 2 + dx;
                let z = center_z - head_width / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Wizard hat
    let hat_y = head_y_start + head_width;
    for dy in 0..4 {
        let hat_size = 4u8.saturating_sub(dy);
        for dx in 0..hat_size {
            for dz in 0..hat_size {
                let x = center_x - hat_size / 2 + dx;
                let z = center_z - hat_size / 2 + dz;
                let y = hat_y + dy;
                if x < size && z < size && y < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 2,
                    });
                }
            }
        }
    }

    // Robe
    let torso_height = (10.0 * scale) as u8;
    let torso_y_start = head_y_start - torso_height;
    for y in torso_y_start..head_y_start {
        for dx in 0..torso_width {
            for dz in 0..(torso_width - 1) {
                let x = center_x - torso_width / 2 + dx;
                let z = center_z - (torso_width - 1) / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 1,
                    });
                }
            }
        }
    }

    // Magic orb (front of mage)
    let orb_y = head_y_start - 2;
    let orb_z = center_z + (torso_width / 2) + 1;
    if orb_z < size {
        model.add_voxel(Voxel {
            x: center_x,
            y: orb_y,
            z: orb_z,
            color_index: 3,
        });
    }

    model
}

/// Generate a knight
pub fn generate_knight(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let scale = body_type.scale_factor();
    let mut model = VoxelModel::new(size, size, size);

    // Knight palette - metallic silver and red
    let _hash = hash_string(seed);
    model.palette.add_color(0.0, 0.0, 0.7); // Silver metal
    model.palette.add_color(0.0, 0.8, 0.4); // Red accent
    model.palette.add_color(0.05, 0.1, 0.2); // Dark metal

    let center_x = size / 2;
    let center_z = size / 2;
    let head_width = (5.0 * scale) as u8;
    let torso_width = (6.0 * scale) as u8;

    // Helmet (full coverage)
    let head_y_start = size - head_width - 2;
    for y in head_y_start..(head_y_start + head_width) {
        for dx in 0..head_width {
            for dz in 0..head_width {
                let x = center_x - head_width / 2 + dx;
                let z = center_z - head_width / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Plume on helmet
    let plume_y = head_y_start + head_width;
    for dy in 0..2 {
        if plume_y + dy < size {
            model.add_voxel(Voxel {
                x: center_x,
                y: plume_y + dy,
                z: center_z,
                color_index: 1,
            });
        }
    }

    // Heavy plate armor
    let torso_height = (9.0 * scale) as u8;
    let torso_y_start = head_y_start - torso_height;
    for y in torso_y_start..head_y_start {
        for dx in 0..torso_width {
            for dz in 0..(torso_width - 1) {
                let x = center_x - torso_width / 2 + dx;
                let z = center_z - (torso_width - 1) / 2 + dz;
                if x < size && z < size {
                    // Add some dark metal accents
                    let color = if (x + y + z).is_multiple_of(3) { 2 } else { 0 };
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: color,
                    });
                }
            }
        }
    }

    model
}

/// Generate an archer
pub fn generate_archer(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let scale = body_type.scale_factor();
    let mut model = VoxelModel::new(size, size, size);

    // Archer palette - green/brown camouflage
    let _hash = hash_string(seed);
    model.palette.add_color(0.8, 0.5, 0.4); // Skin
    model.palette.add_color(0.25, 0.6, 0.3); // Green tunic
    model.palette.add_color(0.1, 0.4, 0.2); // Brown leather
    model.palette.add_color(0.05, 0.2, 0.1); // Dark wood

    let center_x = size / 2;
    let center_z = size / 2;
    let head_width = (4.0 * scale) as u8;
    let torso_width = (4.0 * scale) as u8;

    // Head with hood
    let head_y_start = size - head_width - 2;
    for y in head_y_start..(head_y_start + head_width) {
        for dx in 0..head_width {
            for dz in 0..head_width {
                let x = center_x - head_width / 2 + dx;
                let z = center_z - head_width / 2 + dz;
                if x < size && z < size {
                    // Outer voxels are hood, inner are skin
                    let is_edge =
                        dx == 0 || dx == head_width - 1 || dz == 0 || dz == head_width - 1;
                    let color = if is_edge { 1 } else { 0 };
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: color,
                    });
                }
            }
        }
    }

    // Light armor/tunic
    let torso_height = (7.0 * scale) as u8;
    let torso_y_start = head_y_start - torso_height;
    for y in torso_y_start..head_y_start {
        for dx in 0..torso_width {
            for dz in 0..(torso_width - 1) {
                let x = center_x - torso_width / 2 + dx;
                let z = center_z - (torso_width - 1) / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 1,
                    });
                }
            }
        }
    }

    // Quiver on back
    let quiver_x = center_x + torso_width / 2;
    let quiver_z = center_z;
    for y in (head_y_start - 4)..head_y_start {
        if quiver_x < size {
            model.add_voxel(Voxel {
                x: quiver_x,
                y,
                z: quiver_z,
                color_index: 2,
            });
        }
    }

    model
}

/// Generate a robot
pub fn generate_robot(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let scale = body_type.scale_factor();
    let mut model = VoxelModel::new(size, size, size);

    // Robot palette - metallic and tech colors
    let _hash = hash_string(seed);
    model.palette.add_color(0.0, 0.0, 0.6); // Silver
    model.palette.add_color(0.55, 0.8, 0.5); // Cyan tech
    model.palette.add_color(0.6, 0.9, 0.6); // Bright cyan (eyes)
    model.palette.add_color(0.1, 0.1, 0.3); // Dark metal

    let center_x = size / 2;
    let center_z = size / 2;
    let head_width = (5.0 * scale) as u8;
    let torso_width = (5.0 * scale) as u8;

    // Cubic robot head
    let head_y_start = size - head_width - 2;
    for y in head_y_start..(head_y_start + head_width) {
        for dx in 0..head_width {
            for dz in 0..head_width {
                let x = center_x - head_width / 2 + dx;
                let z = center_z - head_width / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Robot eyes (glowing)
    let eye_y = head_y_start + head_width - 2;
    let eye_z = center_z + head_width / 2;
    if eye_z < size {
        model.add_voxel(Voxel {
            x: center_x - 1,
            y: eye_y,
            z: eye_z,
            color_index: 2,
        });
        model.add_voxel(Voxel {
            x: center_x + 1,
            y: eye_y,
            z: eye_z,
            color_index: 2,
        });
    }

    // Antenna
    let antenna_y = head_y_start + head_width;
    for dy in 0..2 {
        if antenna_y + dy < size {
            model.add_voxel(Voxel {
                x: center_x,
                y: antenna_y + dy,
                z: center_z,
                color_index: 1,
            });
        }
    }

    // Mechanical torso with tech panels
    let torso_height = (8.0 * scale) as u8;
    let torso_y_start = head_y_start - torso_height;
    for y in torso_y_start..head_y_start {
        for dx in 0..torso_width {
            for dz in 0..(torso_width - 1) {
                let x = center_x - torso_width / 2 + dx;
                let z = center_z - (torso_width - 1) / 2 + dz;
                if x < size && z < size {
                    // Tech panels pattern
                    let is_panel = (x + y).is_multiple_of(3) || (z + y).is_multiple_of(3);
                    let color = if is_panel { 1 } else { 0 };
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: color,
                    });
                }
            }
        }
    }

    // Core light (center of torso)
    let core_y = torso_y_start + torso_height / 2;
    let core_z = center_z;
    model.add_voxel(Voxel {
        x: center_x,
        y: core_y,
        z: core_z,
        color_index: 2,
    });

    model
}

/// Generate a cat
pub fn generate_cat(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);
    let scale = body_type.scale_factor();

    // Cat palette
    let hash = hash_string(seed);
    let color_variant = hash % 3;
    match color_variant {
        0 => model.palette.add_color(0.9, 0.5, 0.2), // Orange
        1 => model.palette.add_color(0.3, 0.3, 0.3), // Gray
        _ => model.palette.add_color(0.9, 0.9, 0.9), // White
    }
    model.palette.add_color(0.1, 0.6, 0.8); // Eyes (blue)

    let center_x = size / 2;
    let center_z = size / 2;

    // Cat head (cube with ears)
    let head_size = (4.0 * scale) as u8;
    let head_y = size / 2;
    for y in head_y..(head_y + head_size) {
        for dx in 0..head_size {
            for dz in 0..head_size {
                let x = center_x - head_size / 2 + dx;
                let z = center_z - head_size / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Ears
    let ear_y = head_y + head_size;
    for dy in 0..2 {
        model.add_voxel(Voxel {
            x: center_x - 1,
            y: ear_y + dy,
            z: center_z,
            color_index: 0,
        });
        model.add_voxel(Voxel {
            x: center_x + 1,
            y: ear_y + dy,
            z: center_z,
            color_index: 0,
        });
    }

    // Body
    let body_length = (6.0 * scale) as u8;
    let body_width = (3.0 * scale) as u8;
    let body_y = head_y / 2;
    for x in (center_x - body_width / 2)..(center_x + body_width / 2) {
        for y in body_y..(body_y + body_width) {
            for z in (center_z - body_length / 2)..(center_z + body_length / 2) {
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    model
}

/// Generate a dog
pub fn generate_dog(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);
    let scale = body_type.scale_factor();

    // Dog palette
    let hash = hash_string(seed);
    let color_variant = hash % 3;
    match color_variant {
        0 => model.palette.add_color(0.1, 0.5, 0.3),  // Brown
        1 => model.palette.add_color(0.9, 0.9, 0.9),  // White
        _ => model.palette.add_color(0.05, 0.2, 0.1), // Black
    }
    model.palette.add_color(0.05, 0.3, 0.2); // Nose (dark)

    let center_x = size / 2;
    let center_z = size / 2;

    // Dog head (snout)
    let head_size = (4.0 * scale) as u8;
    let head_y = size / 2 + 2;
    for y in head_y..(head_y + head_size) {
        for dx in 0..head_size {
            for dz in 0..head_size {
                let x = center_x - head_size / 2 + dx;
                let z = center_z - head_size / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Snout extension
    let snout_z = center_z + head_size / 2;
    for dy in 0..2 {
        for dz in 0..2 {
            if snout_z + dz < size {
                model.add_voxel(Voxel {
                    x: center_x,
                    y: head_y + dy,
                    z: snout_z + dz,
                    color_index: 0,
                });
            }
        }
    }

    // Nose
    if snout_z + 2 < size {
        model.add_voxel(Voxel {
            x: center_x,
            y: head_y,
            z: snout_z + 2,
            color_index: 1,
        });
    }

    // Ears (floppy)
    let ear_y = head_y + head_size - 1;
    for dy in 0..3 {
        model.add_voxel(Voxel {
            x: center_x - 2,
            y: ear_y - dy,
            z: center_z,
            color_index: 0,
        });
        model.add_voxel(Voxel {
            x: center_x + 2,
            y: ear_y - dy,
            z: center_z,
            color_index: 0,
        });
    }

    // Body (horizontal)
    let body_length = (7.0 * scale) as u8;
    let body_width = (3.0 * scale) as u8;
    let body_y = head_y - 1;
    for x in (center_x - body_width / 2)..(center_x + body_width / 2) {
        for y in (body_y - body_width)..(body_y + 1) {
            for z in (center_z - body_length / 2)..(center_z + body_length / 2) {
                if x < size && z < size && y < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Tail
    let tail_z = center_z.saturating_sub(body_length / 2);
    for dy in 0..3 {
        if tail_z > 0 {
            model.add_voxel(Voxel {
                x: center_x,
                y: body_y + dy,
                z: tail_z - 1,
                color_index: 0,
            });
        }
    }

    model
}

/// Generate a bird
pub fn generate_bird(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);
    let scale = body_type.scale_factor();

    // Bird palette - colorful
    let hash = hash_string(seed);
    let hue = ((hash % 360) as f32) / 360.0;
    model.palette.add_color(hue, 0.8, 0.6); // Body color
    model.palette.add_color((hue + 0.1) % 1.0, 0.9, 0.7); // Wing highlight
    model.palette.add_color(0.1, 0.8, 0.5); // Beak (orange)

    let center_x = size / 2;
    let center_z = size / 2;

    // Small round body
    let body_size = (3.0 * scale) as u8;
    let body_y = size / 2;
    for y in body_y..(body_y + body_size + 1) {
        for dx in 0..body_size {
            for dz in 0..body_size {
                let x = center_x - body_size / 2 + dx;
                let z = center_z - body_size / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Head (on top)
    let head_size = (2.0 * scale).max(1.0) as u8;
    let head_y = body_y + body_size + 1;
    for dy in 0..head_size {
        for dx in 0..head_size {
            for dz in 0..head_size {
                let x = center_x - head_size / 2 + dx;
                let z = center_z - head_size / 2 + dz;
                let y = head_y + dy;
                if x < size && z < size && y < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Beak
    let beak_z = center_z + head_size / 2 + 1;
    if beak_z < size {
        model.add_voxel(Voxel {
            x: center_x,
            y: head_y,
            z: beak_z,
            color_index: 2,
        });
    }

    // Wings (extended)
    let wing_y = body_y + body_size / 2;
    for dx in 1..4 {
        // Left wing
        let left_x = center_x.saturating_sub(body_size / 2 + dx);
        model.add_voxel(Voxel {
            x: left_x,
            y: wing_y,
            z: center_z,
            color_index: 1,
        });

        // Right wing
        let right_x = center_x + body_size / 2 + dx;
        if right_x < size {
            model.add_voxel(Voxel {
                x: right_x,
                y: wing_y,
                z: center_z,
                color_index: 1,
            });
        }
    }

    model
}

/// Generate a fish
pub fn generate_fish(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);
    let scale = body_type.scale_factor();

    // Fish palette - aquatic colors
    let hash = hash_string(seed);
    let color_variant = hash % 3;
    match color_variant {
        0 => model.palette.add_color(0.55, 0.7, 0.6), // Blue
        1 => model.palette.add_color(0.05, 0.9, 0.5), // Orange
        _ => model.palette.add_color(0.3, 0.6, 0.7),  // Green
    }
    model.palette.add_color(0.5, 0.8, 0.8); // Lighter shade

    let center_x = size / 2;
    let center_z = size / 2;

    // Oval body
    let body_length = (6.0 * scale) as u8;
    let body_width = (3.0 * scale) as u8;
    let body_y = size / 2;

    for x in (center_x - body_width / 2)..(center_x + body_width / 2) {
        for y in (body_y - body_width / 2)..(body_y + body_width / 2 + 1) {
            for z in (center_z - body_length / 2)..(center_z + body_length / 2) {
                if x < size && z < size && y < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Tail fin
    let tail_z = center_z - body_length / 2;
    for dx in 0..3 {
        for dy in 0..3 {
            let x = center_x - 1 + dx;
            let y = body_y - 1 + dy;
            if tail_z > 0 && x < size {
                model.add_voxel(Voxel {
                    x,
                    y,
                    z: tail_z - 1,
                    color_index: 1,
                });
            }
        }
    }

    // Dorsal fin
    let fin_y = body_y + body_width / 2 + 1;
    for dz in 1..3 {
        if fin_y < size {
            model.add_voxel(Voxel {
                x: center_x,
                y: fin_y,
                z: center_z + dz,
                color_index: 1,
            });
        }
    }

    model
}

/// Generate a dragon
pub fn generate_dragon(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);
    let scale = body_type.scale_factor();

    // Dragon palette - fiery colors
    let _hash = hash_string(seed);
    model.palette.add_color(0.0, 0.8, 0.3); // Red scales
    model.palette.add_color(0.05, 0.9, 0.4); // Orange accent
    model.palette.add_color(0.15, 0.9, 0.9); // Yellow fire
    model.palette.add_color(0.3, 0.3, 0.3); // Dark gray horns

    let center_x = size / 2;
    let center_z = size / 2;

    // Dragon head (larger)
    let head_size = (5.0 * scale) as u8;
    let head_y = size / 2 + 4;
    for y in head_y..(head_y + head_size) {
        for dx in 0..head_size {
            for dz in 0..head_size {
                let x = center_x - head_size / 2 + dx;
                let z = center_z - head_size / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Horns
    let horn_y = head_y + head_size;
    for dy in 0..3 {
        if horn_y + dy < size {
            model.add_voxel(Voxel {
                x: center_x - 1,
                y: horn_y + dy,
                z: center_z,
                color_index: 3,
            });
            model.add_voxel(Voxel {
                x: center_x + 1,
                y: horn_y + dy,
                z: center_z,
                color_index: 3,
            });
        }
    }

    // Snout
    let snout_z = center_z + head_size / 2;
    for dy in 0..2 {
        for dz in 0..2 {
            if snout_z + dz < size {
                model.add_voxel(Voxel {
                    x: center_x,
                    y: head_y + dy,
                    z: snout_z + dz,
                    color_index: 0,
                });
            }
        }
    }

    // Fire breath
    if snout_z + 3 < size {
        model.add_voxel(Voxel {
            x: center_x,
            y: head_y,
            z: snout_z + 3,
            color_index: 2,
        });
    }

    // Body (thick and long)
    let body_length = (8.0 * scale) as u8;
    let body_width = (4.0 * scale) as u8;
    let body_y = head_y - 2;
    for x in (center_x - body_width / 2)..(center_x + body_width / 2) {
        for y in (body_y - body_width)..(body_y + 1) {
            for z in (center_z - body_length / 2)..(center_z + body_length / 2) {
                if x < size && z < size && y < size {
                    // Add orange accents
                    let color = if (x + y + z) % 4 == 0 { 1 } else { 0 };
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: color,
                    });
                }
            }
        }
    }

    // Wings
    let wing_y = body_y;
    for dx in 1..4 {
        for dy in 0..3 {
            // Left wing
            let left_x = center_x.saturating_sub(body_width / 2 + dx);
            let y = wing_y + dy;
            if y < size {
                model.add_voxel(Voxel {
                    x: left_x,
                    y,
                    z: center_z,
                    color_index: 1,
                });
            }

            // Right wing
            let right_x = center_x + body_width / 2 + dx;
            if right_x < size && y < size {
                model.add_voxel(Voxel {
                    x: right_x,
                    y,
                    z: center_z,
                    color_index: 1,
                });
            }
        }
    }

    model
}

/// Generate a bear
pub fn generate_bear(size: u8, body_type: BodyType, seed: &str) -> VoxelModel {
    let mut model = VoxelModel::new(size, size, size);
    let scale = body_type.scale_factor();

    // Bear palette - brown
    let hash = hash_string(seed);
    let color_variant = hash % 2;
    match color_variant {
        0 => model.palette.add_color(0.08, 0.5, 0.3), // Brown bear
        _ => model.palette.add_color(0.9, 0.9, 0.9),  // Polar bear
    }
    model.palette.add_color(0.05, 0.2, 0.1); // Dark brown (nose)

    let center_x = size / 2;
    let center_z = size / 2;

    // Large round head
    let head_size = (5.0 * scale) as u8;
    let head_y = size / 2 + 3;
    for y in head_y..(head_y + head_size) {
        for dx in 0..head_size {
            for dz in 0..head_size {
                let x = center_x - head_size / 2 + dx;
                let z = center_z - head_size / 2 + dz;
                if x < size && z < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    // Round ears
    let ear_y = head_y + head_size;
    for dy in 0..2 {
        if ear_y + dy < size {
            // Left ear
            model.add_voxel(Voxel {
                x: center_x - 2,
                y: ear_y + dy,
                z: center_z,
                color_index: 0,
            });
            model.add_voxel(Voxel {
                x: center_x - 2,
                y: ear_y + dy,
                z: center_z - 1,
                color_index: 0,
            });
            // Right ear
            model.add_voxel(Voxel {
                x: center_x + 2,
                y: ear_y + dy,
                z: center_z,
                color_index: 0,
            });
            model.add_voxel(Voxel {
                x: center_x + 2,
                y: ear_y + dy,
                z: center_z - 1,
                color_index: 0,
            });
        }
    }

    // Snout
    let snout_z = center_z + head_size / 2;
    for dy in 0..2 {
        for dz in 0..2 {
            if snout_z + dz < size {
                model.add_voxel(Voxel {
                    x: center_x,
                    y: head_y + dy,
                    z: snout_z + dz,
                    color_index: 0,
                });
            }
        }
    }

    // Nose
    if snout_z + 2 < size {
        model.add_voxel(Voxel {
            x: center_x,
            y: head_y,
            z: snout_z + 2,
            color_index: 1,
        });
    }

    // Large bulky body
    let body_length = (7.0 * scale) as u8;
    let body_width = (5.0 * scale) as u8;
    let body_y = head_y - 1;
    for x in (center_x - body_width / 2)..(center_x + body_width / 2) {
        for y in (body_y - body_width)..(body_y + 1) {
            for z in (center_z - body_length / 2)..(center_z + body_length / 2) {
                if x < size && z < size && y < size {
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }
    }

    model
}
