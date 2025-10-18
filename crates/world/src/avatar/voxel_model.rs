use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A single voxel in 3D space
#[derive(Clone, Copy, Debug)]
pub struct Voxel {
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub color_index: u8,
}

/// Color palette for voxel model
#[derive(Clone, Debug)]
pub struct VoxelPalette {
    colors: Vec<[f32; 3]>,
}

impl VoxelPalette {
    pub fn new() -> Self {
        Self { colors: Vec::new() }
    }

    pub fn add_color(&mut self, r: f32, g: f32, b: f32) {
        self.colors.push([r, g, b]);
    }

    pub fn get_color(&self, index: u8) -> [f32; 3] {
        self.colors
            .get(index as usize)
            .copied()
            .unwrap_or([1.0, 1.0, 1.0])
    }

    /// Apply user-specific color shift to palette based on hash
    pub fn customize_for_user(&self, user_hash: &str) -> Self {
        let hash_value = hash_string(user_hash);
        let hue_shift = ((hash_value % 360) as f32) / 360.0;

        let mut customized = Self { colors: Vec::new() };

        for color in &self.colors {
            let shifted = apply_hue_shift(*color, hue_shift);
            customized.colors.push(shifted);
        }

        customized
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.colors.len()
    }
}

/// Voxel model containing voxels and palette
#[derive(Clone, Debug)]
pub struct VoxelModel {
    pub size_x: u8,
    pub size_y: u8,
    pub size_z: u8,
    pub voxels: Vec<Voxel>,
    pub palette: VoxelPalette,
}

impl VoxelModel {
    pub fn new(size_x: u8, size_y: u8, size_z: u8) -> Self {
        Self {
            size_x,
            size_y,
            size_z,
            voxels: Vec::new(),
            palette: VoxelPalette::new(),
        }
    }

    pub fn add_voxel(&mut self, voxel: Voxel) {
        if voxel.x < self.size_x && voxel.y < self.size_y && voxel.z < self.size_z {
            self.voxels.push(voxel);
        }
    }

    /// Check if voxel exists at position
    pub fn has_voxel_at(&self, x: u8, y: u8, z: u8) -> bool {
        self.voxels.iter().any(|v| v.x == x && v.y == y && v.z == z)
    }

    /// Get voxel at position
    #[allow(dead_code)]
    pub fn get_voxel_at(&self, x: u8, y: u8, z: u8) -> Option<&Voxel> {
        self.voxels
            .iter()
            .find(|v| v.x == x && v.y == y && v.z == z)
    }

    /// Create a simple humanoid voxel model for testing
    ///
    /// Model specifications:
    /// - Grid size: 16x32x16 voxels (width, height, depth)
    /// - World size: 1.6 x 3.2 x 1.6 units (at 0.1 voxel size)
    /// - Origin: (0, 0, 0) at grid corner, model centered at (8, 0, 8)
    /// - Height: 28 voxels from feet (y=0) to head top (y=28)
    /// - Colors: skin, shirt, pants, shoes
    pub fn create_simple_humanoid() -> Self {
        let mut model = Self::new(16, 32, 16);

        // Setup basic palette
        model.palette.add_color(0.8, 0.6, 0.4); // 0: Skin
        model.palette.add_color(0.2, 0.4, 0.8); // 1: Shirt
        model.palette.add_color(0.1, 0.2, 0.6); // 2: Pants
        model.palette.add_color(0.4, 0.3, 0.2); // 3: Shoes

        let center_x = 8;
        let center_z = 8;

        // Head (4x4x4)
        for y in 24..28 {
            for dx in 0..4 {
                for dz in 0..4 {
                    let x = center_x - 2 + dx;
                    let z = center_z - 2 + dz;
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }

        // Torso (4x6x3)
        for y in 14..20 {
            for dx in 0..4 {
                for dz in 0..3 {
                    let x = center_x - 2 + dx;
                    let z = center_z - 1 + dz;
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 1,
                    });
                }
            }
        }

        // Left Arm (2x6x2)
        for y in 14..20 {
            for dx in 0..2 {
                for dz in 0..2 {
                    let x = center_x - 4 + dx;
                    let z = center_z - 1 + dz;
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 1,
                    });
                }
            }
        }

        // Right Arm (2x6x2)
        for y in 14..20 {
            for dx in 0..2 {
                for dz in 0..2 {
                    let x = center_x + 2 + dx;
                    let z = center_z - 1 + dz;
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 1,
                    });
                }
            }
        }

        // Left Leg (2x8x2)
        for y in 0..8 {
            for dx in 0..2 {
                for dz in 0..2 {
                    let x = center_x - 2 + dx;
                    let z = center_z - 1 + dz;
                    let color = if y < 2 { 3 } else { 2 };
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: color,
                    });
                }
            }
        }

        // Right Leg (2x8x2)
        for y in 0..8 {
            for dx in 0..2 {
                for dz in 0..2 {
                    let x = center_x + dx;
                    let z = center_z - 1 + dz;
                    let color = if y < 2 { 3 } else { 2 };
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: color,
                    });
                }
            }
        }

        // Neck (2x4x2) - connect head to torso
        for y in 20..24 {
            for dx in 0..2 {
                for dz in 0..2 {
                    let x = center_x - 1 + dx;
                    let z = center_z - 1 + dz;
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 0,
                    });
                }
            }
        }

        // Hips (4x6x3) - connect torso to legs
        for y in 8..14 {
            for dx in 0..4 {
                for dz in 0..3 {
                    let x = center_x - 2 + dx;
                    let z = center_z - 1 + dz;
                    model.add_voxel(Voxel {
                        x,
                        y,
                        z,
                        color_index: 2,
                    });
                }
            }
        }

        model
    }
}

/// Hash a string to a u32 value
fn hash_string(s: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    (hasher.finish() & 0xFFFFFFFF) as u32
}

/// Apply hue shift to RGB color
fn apply_hue_shift(rgb: [f32; 3], shift: f32) -> [f32; 3] {
    // Convert RGB to HSL
    let (h, s, l) = rgb_to_hsl(rgb);

    // Shift hue
    let new_h = (h + shift) % 1.0;

    // Convert back to RGB
    hsl_to_rgb(new_h, s, l)
}

/// Convert RGB to HSL
fn rgb_to_hsl(rgb: [f32; 3]) -> (f32, f32, f32) {
    let r = rgb[0];
    let g = rgb[1];
    let b = rgb[2];

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let l = (max + min) / 2.0;

    if delta == 0.0 {
        return (0.0, 0.0, l);
    }

    let s = if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    let h = if max == r {
        ((g - b) / delta + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };

    (h, s, l)
}

/// Convert HSL to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    if s == 0.0 {
        return [l, l, l];
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };

    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    [r, g, b]
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}
