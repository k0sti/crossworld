use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub index: u8,
    pub id: &'static str,
    pub color: Vec3,
}

pub const MATERIAL_REGISTRY: [Material; 128] = [
    Material {
        index: 0,
        id: "empty",
        color: Vec3::new(0.000, 0.000, 0.000),
    },
    Material {
        index: 1,
        id: "set_empty",
        color: Vec3::new(0.000, 0.000, 0.000),
    },
    Material {
        index: 2,
        id: "glass",
        color: Vec3::new(1.000, 1.000, 1.000),
    },
    Material {
        index: 3,
        id: "ice",
        color: Vec3::new(0.816, 1.000, 1.000),
    },
    Material {
        index: 4,
        id: "water_surface",
        color: Vec3::new(0.000, 0.498, 1.000),
    },
    Material {
        index: 5,
        id: "slime",
        color: Vec3::new(0.000, 1.000, 0.000),
    },
    Material {
        index: 6,
        id: "honey",
        color: Vec3::new(1.000, 0.647, 0.000),
    },
    Material {
        index: 7,
        id: "crystal",
        color: Vec3::new(1.000, 0.000, 1.000),
    },
    Material {
        index: 8,
        id: "force_field",
        color: Vec3::new(0.000, 1.000, 1.000),
    },
    Material {
        index: 9,
        id: "portal",
        color: Vec3::new(0.667, 0.000, 1.000),
    },
    Material {
        index: 10,
        id: "mist",
        color: Vec3::new(0.800, 0.800, 0.800),
    },
    Material {
        index: 11,
        id: "stained_glass_red",
        color: Vec3::new(1.000, 0.000, 0.000),
    },
    Material {
        index: 12,
        id: "stained_glass_green",
        color: Vec3::new(0.000, 1.000, 0.000),
    },
    Material {
        index: 13,
        id: "stained_glass_blue",
        color: Vec3::new(0.000, 0.000, 1.000),
    },
    Material {
        index: 14,
        id: "stained_glass_yellow",
        color: Vec3::new(1.000, 1.000, 0.000),
    },
    Material {
        index: 15,
        id: "transparent_15",
        color: Vec3::new(0.502, 0.502, 0.502),
    },
    Material {
        index: 16,
        id: "hard_ground",
        color: Vec3::new(0.400, 0.267, 0.200),
    },
    Material {
        index: 17,
        id: "water",
        color: Vec3::new(0.000, 0.314, 0.624),
    },
    Material {
        index: 18,
        id: "dirt",
        color: Vec3::new(0.545, 0.271, 0.075),
    },
    Material {
        index: 19,
        id: "grass",
        color: Vec3::new(0.227, 0.490, 0.227),
    },
    Material {
        index: 20,
        id: "stone",
        color: Vec3::new(0.502, 0.502, 0.502),
    },
    Material {
        index: 21,
        id: "cobblestone",
        color: Vec3::new(0.431, 0.431, 0.431),
    },
    Material {
        index: 22,
        id: "sand",
        color: Vec3::new(0.929, 0.788, 0.686),
    },
    Material {
        index: 23,
        id: "sandstone",
        color: Vec3::new(0.788, 0.655, 0.439),
    },
    Material {
        index: 24,
        id: "gravel",
        color: Vec3::new(0.533, 0.533, 0.533),
    },
    Material {
        index: 25,
        id: "clay",
        color: Vec3::new(0.627, 0.627, 0.627),
    },
    Material {
        index: 26,
        id: "snow",
        color: Vec3::new(1.000, 1.000, 1.000),
    },
    Material {
        index: 27,
        id: "ice_solid",
        color: Vec3::new(0.690, 0.878, 1.000),
    },
    Material {
        index: 28,
        id: "obsidian",
        color: Vec3::new(0.102, 0.059, 0.180),
    },
    Material {
        index: 29,
        id: "netherrack",
        color: Vec3::new(0.545, 0.000, 0.000),
    },
    Material {
        index: 30,
        id: "granite",
        color: Vec3::new(0.612, 0.365, 0.239),
    },
    Material {
        index: 31,
        id: "diorite",
        color: Vec3::new(0.749, 0.749, 0.749),
    },
    Material {
        index: 32,
        id: "andesite",
        color: Vec3::new(0.427, 0.427, 0.427),
    },
    Material {
        index: 33,
        id: "marble",
        color: Vec3::new(0.910, 0.910, 0.910),
    },
    Material {
        index: 34,
        id: "limestone",
        color: Vec3::new(0.855, 0.816, 0.753),
    },
    Material {
        index: 35,
        id: "basalt",
        color: Vec3::new(0.169, 0.169, 0.169),
    },
    Material {
        index: 36,
        id: "wood_oak",
        color: Vec3::new(0.627, 0.510, 0.427),
    },
    Material {
        index: 37,
        id: "wood_spruce",
        color: Vec3::new(0.420, 0.333, 0.208),
    },
    Material {
        index: 38,
        id: "wood_birch",
        color: Vec3::new(0.843, 0.796, 0.553),
    },
    Material {
        index: 39,
        id: "wood_jungle",
        color: Vec3::new(0.545, 0.435, 0.278),
    },
    Material {
        index: 40,
        id: "wood_acacia",
        color: Vec3::new(0.722, 0.408, 0.243),
    },
    Material {
        index: 41,
        id: "wood_dark_oak",
        color: Vec3::new(0.290, 0.220, 0.161),
    },
    Material {
        index: 42,
        id: "planks_oak",
        color: Vec3::new(0.769, 0.651, 0.447),
    },
    Material {
        index: 43,
        id: "planks_spruce",
        color: Vec3::new(0.486, 0.365, 0.243),
    },
    Material {
        index: 44,
        id: "planks_birch",
        color: Vec3::new(0.890, 0.851, 0.659),
    },
    Material {
        index: 45,
        id: "leaves",
        color: Vec3::new(0.176, 0.314, 0.086),
    },
    Material {
        index: 46,
        id: "leaves_birch",
        color: Vec3::new(0.365, 0.561, 0.227),
    },
    Material {
        index: 47,
        id: "leaves_spruce",
        color: Vec3::new(0.239, 0.376, 0.188),
    },
    Material {
        index: 48,
        id: "coal",
        color: Vec3::new(0.102, 0.102, 0.102),
    },
    Material {
        index: 49,
        id: "iron",
        color: Vec3::new(0.847, 0.847, 0.847),
    },
    Material {
        index: 50,
        id: "gold",
        color: Vec3::new(1.000, 0.843, 0.000),
    },
    Material {
        index: 51,
        id: "copper",
        color: Vec3::new(0.722, 0.451, 0.200),
    },
    Material {
        index: 52,
        id: "silver",
        color: Vec3::new(0.753, 0.753, 0.753),
    },
    Material {
        index: 53,
        id: "bronze",
        color: Vec3::new(0.804, 0.498, 0.196),
    },
    Material {
        index: 54,
        id: "steel",
        color: Vec3::new(0.565, 0.565, 0.627),
    },
    Material {
        index: 55,
        id: "titanium",
        color: Vec3::new(0.529, 0.525, 0.506),
    },
    Material {
        index: 56,
        id: "brick",
        color: Vec3::new(0.545, 0.227, 0.227),
    },
    Material {
        index: 57,
        id: "concrete",
        color: Vec3::new(0.620, 0.620, 0.620),
    },
    Material {
        index: 58,
        id: "concrete_white",
        color: Vec3::new(0.933, 0.933, 0.933),
    },
    Material {
        index: 59,
        id: "concrete_black",
        color: Vec3::new(0.118, 0.118, 0.118),
    },
    Material {
        index: 60,
        id: "asphalt",
        color: Vec3::new(0.200, 0.200, 0.200),
    },
    Material {
        index: 61,
        id: "rubber",
        color: Vec3::new(0.169, 0.169, 0.169),
    },
    Material {
        index: 62,
        id: "plastic",
        color: Vec3::new(0.667, 0.667, 0.667),
    },
    Material {
        index: 63,
        id: "ceramic",
        color: Vec3::new(0.878, 0.816, 0.753),
    },
    Material {
        index: 64,
        id: "skin_light",
        color: Vec3::new(1.000, 0.835, 0.706),
    },
    Material {
        index: 65,
        id: "skin_medium",
        color: Vec3::new(0.875, 0.690, 0.549),
    },
    Material {
        index: 66,
        id: "skin_tan",
        color: Vec3::new(0.788, 0.510, 0.314),
    },
    Material {
        index: 67,
        id: "skin_brown",
        color: Vec3::new(0.545, 0.353, 0.235),
    },
    Material {
        index: 68,
        id: "skin_dark",
        color: Vec3::new(0.365, 0.227, 0.102),
    },
    Material {
        index: 69,
        id: "leather_brown",
        color: Vec3::new(0.435, 0.306, 0.216),
    },
    Material {
        index: 70,
        id: "leather_black",
        color: Vec3::new(0.180, 0.149, 0.125),
    },
    Material {
        index: 71,
        id: "leather_tan",
        color: Vec3::new(0.749, 0.627, 0.533),
    },
    Material {
        index: 72,
        id: "fabric_white",
        color: Vec3::new(0.941, 0.941, 0.941),
    },
    Material {
        index: 73,
        id: "fabric_red",
        color: Vec3::new(0.863, 0.078, 0.235),
    },
    Material {
        index: 74,
        id: "fabric_blue",
        color: Vec3::new(0.118, 0.565, 1.000),
    },
    Material {
        index: 75,
        id: "fabric_green",
        color: Vec3::new(0.133, 0.545, 0.133),
    },
    Material {
        index: 76,
        id: "fabric_yellow",
        color: Vec3::new(1.000, 0.843, 0.000),
    },
    Material {
        index: 77,
        id: "fabric_purple",
        color: Vec3::new(0.545, 0.000, 0.545),
    },
    Material {
        index: 78,
        id: "fabric_orange",
        color: Vec3::new(1.000, 0.549, 0.000),
    },
    Material {
        index: 79,
        id: "fabric_pink",
        color: Vec3::new(1.000, 0.412, 0.706),
    },
    Material {
        index: 80,
        id: "fabric_black",
        color: Vec3::new(0.110, 0.110, 0.110),
    },
    Material {
        index: 81,
        id: "wool_white",
        color: Vec3::new(0.878, 0.878, 0.878),
    },
    Material {
        index: 82,
        id: "wool_gray",
        color: Vec3::new(0.502, 0.502, 0.502),
    },
    Material {
        index: 83,
        id: "wool_red",
        color: Vec3::new(0.702, 0.192, 0.173),
    },
    Material {
        index: 84,
        id: "wool_blue",
        color: Vec3::new(0.235, 0.267, 0.667),
    },
    Material {
        index: 85,
        id: "sponge",
        color: Vec3::new(0.800, 0.800, 0.333),
    },
    Material {
        index: 86,
        id: "moss",
        color: Vec3::new(0.349, 0.490, 0.208),
    },
    Material {
        index: 87,
        id: "mushroom_red",
        color: Vec3::new(1.000, 0.000, 0.000),
    },
    Material {
        index: 88,
        id: "mushroom_brown",
        color: Vec3::new(0.608, 0.463, 0.325),
    },
    Material {
        index: 89,
        id: "coral",
        color: Vec3::new(1.000, 0.498, 0.314),
    },
    Material {
        index: 90,
        id: "bamboo",
        color: Vec3::new(0.561, 0.737, 0.561),
    },
    Material {
        index: 91,
        id: "cactus",
        color: Vec3::new(0.345, 0.490, 0.243),
    },
    Material {
        index: 92,
        id: "vine",
        color: Vec3::new(0.243, 0.424, 0.145),
    },
    Material {
        index: 93,
        id: "pumpkin",
        color: Vec3::new(1.000, 0.502, 0.000),
    },
    Material {
        index: 94,
        id: "melon",
        color: Vec3::new(0.439, 0.702, 0.255),
    },
    Material {
        index: 95,
        id: "hay",
        color: Vec3::new(0.831, 0.686, 0.216),
    },
    Material {
        index: 96,
        id: "bone",
        color: Vec3::new(0.929, 0.902, 0.839),
    },
    Material {
        index: 97,
        id: "flesh",
        color: Vec3::new(1.000, 0.502, 0.502),
    },
    Material {
        index: 98,
        id: "slime_green",
        color: Vec3::new(0.000, 1.000, 0.000),
    },
    Material {
        index: 99,
        id: "magma",
        color: Vec3::new(1.000, 0.271, 0.000),
    },
    Material {
        index: 100,
        id: "lava_rock",
        color: Vec3::new(0.545, 0.000, 0.000),
    },
    Material {
        index: 101,
        id: "ash",
        color: Vec3::new(0.376, 0.314, 0.314),
    },
    Material {
        index: 102,
        id: "charcoal",
        color: Vec3::new(0.184, 0.184, 0.184),
    },
    Material {
        index: 103,
        id: "sulfur",
        color: Vec3::new(1.000, 1.000, 0.000),
    },
    Material {
        index: 104,
        id: "salt",
        color: Vec3::new(0.941, 0.941, 0.941),
    },
    Material {
        index: 105,
        id: "sugar",
        color: Vec3::new(1.000, 1.000, 1.000),
    },
    Material {
        index: 106,
        id: "paper",
        color: Vec3::new(0.980, 0.941, 0.902),
    },
    Material {
        index: 107,
        id: "cardboard",
        color: Vec3::new(0.667, 0.533, 0.400),
    },
    Material {
        index: 108,
        id: "wax",
        color: Vec3::new(1.000, 0.953, 0.816),
    },
    Material {
        index: 109,
        id: "tar",
        color: Vec3::new(0.059, 0.059, 0.059),
    },
    Material {
        index: 110,
        id: "oil",
        color: Vec3::new(0.235, 0.188, 0.125),
    },
    Material {
        index: 111,
        id: "paint_red",
        color: Vec3::new(1.000, 0.000, 0.000),
    },
    Material {
        index: 112,
        id: "paint_green",
        color: Vec3::new(0.000, 1.000, 0.000),
    },
    Material {
        index: 113,
        id: "paint_blue",
        color: Vec3::new(0.000, 0.000, 1.000),
    },
    Material {
        index: 114,
        id: "paint_white",
        color: Vec3::new(1.000, 1.000, 1.000),
    },
    Material {
        index: 115,
        id: "paint_black",
        color: Vec3::new(0.000, 0.000, 0.000),
    },
    Material {
        index: 116,
        id: "glowstone",
        color: Vec3::new(1.000, 1.000, 0.627),
    },
    Material {
        index: 117,
        id: "redstone",
        color: Vec3::new(1.000, 0.000, 0.000),
    },
    Material {
        index: 118,
        id: "emerald",
        color: Vec3::new(0.314, 0.784, 0.471),
    },
    Material {
        index: 119,
        id: "diamond",
        color: Vec3::new(0.725, 0.949, 1.000),
    },
    Material {
        index: 120,
        id: "ruby",
        color: Vec3::new(0.878, 0.067, 0.373),
    },
    Material {
        index: 121,
        id: "sapphire",
        color: Vec3::new(0.059, 0.322, 0.729),
    },
    Material {
        index: 122,
        id: "amethyst",
        color: Vec3::new(0.600, 0.400, 0.800),
    },
    Material {
        index: 123,
        id: "topaz",
        color: Vec3::new(1.000, 0.784, 0.486),
    },
    Material {
        index: 124,
        id: "pearl",
        color: Vec3::new(1.000, 0.937, 0.835),
    },
    Material {
        index: 125,
        id: "quartz",
        color: Vec3::new(1.000, 1.000, 1.000),
    },
    Material {
        index: 126,
        id: "amber",
        color: Vec3::new(1.000, 0.749, 0.000),
    },
    Material {
        index: 127,
        id: "reserved_127",
        color: Vec3::new(0.533, 0.533, 0.533),
    },
];

// Commonly used material indices for terrain generation
pub const HARD_GROUND: u8 = 16;
pub const WATER: u8 = 17;
pub const DIRT: u8 = 18;
pub const GRASS: u8 = 19;
pub const STONE: u8 = 20;
pub const COBBLESTONE: u8 = 21;
pub const SAND: u8 = 22;
pub const SANDSTONE: u8 = 23;
pub const GRAVEL: u8 = 24;
pub const CLAY: u8 = 25;
pub const SNOW: u8 = 26;
pub const ICE_SOLID: u8 = 27;
pub const NETHERRACK: u8 = 29;
pub const GRANITE: u8 = 30;
pub const ANDESITE: u8 = 32;
pub const LIMESTONE: u8 = 34;
pub const BASALT: u8 = 35;
pub const COAL: u8 = 48;
pub const IRON: u8 = 49;

/// Get material color for a voxel value
///
/// Supports both terrain materials (0-127) and R2G3B2 encoded colors (128-255).
pub fn get_material_color(value: i32) -> Vec3 {
    if value < 0 {
        return Vec3::ZERO;
    }

    // Values 0-127: Use registry
    if value < 128 {
        return MATERIAL_REGISTRY[value as usize].color;
    }

    // Values 128-255: R2G3B2 encoded colors
    if value <= 255 {
        return decode_r2g3b2(value as u8);
    }

    // Invalid value
    Vec3::ZERO
}

/// Decode R2G3B2 color encoding to RGB
///
/// Encoding: (r << 5) | (g << 2) | b
/// where index = value - 128
fn decode_r2g3b2(value: u8) -> Vec3 {
    let bits = value.saturating_sub(128);
    let r_bits = (bits >> 5) & 0b11;
    let g_bits = (bits >> 2) & 0b111;
    let b_bits = bits & 0b11;

    // Convert to normalized RGB values
    let r = match r_bits {
        0 => 0.0,
        1 => 0.286, // 0x49/255
        2 => 0.573, // 0x92/255
        3 => 0.859, // 0xDB/255
        _ => 0.0,
    };
    let g = match g_bits {
        0 => 0.0,
        1 => 0.141, // 0x24/255
        2 => 0.286, // 0x49/255
        3 => 0.427, // 0x6D/255
        4 => 0.573, // 0x92/255
        5 => 0.714, // 0xB6/255
        6 => 0.859, // 0xDB/255
        7 => 1.0,   // 0xFF/255
        _ => 0.0,
    };
    let b = match b_bits {
        0 => 0.0,
        1 => 0.286, // 0x49/255
        2 => 0.573, // 0x92/255
        3 => 0.859, // 0xDB/255
        _ => 0.0,
    };

    Vec3::new(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_registry() {
        assert_eq!(get_material_color(0), Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(get_material_color(1), Vec3::new(0.0, 0.0, 0.0)); // set_empty is black/transparent
        assert_eq!(get_material_color(2), Vec3::new(1.0, 1.0, 1.0)); // glass
        assert_eq!(get_material_color(117), Vec3::new(1.0, 0.0, 0.0)); // redstone
    }

    #[test]
    fn test_r2g3b2_encoding() {
        // Red: r=3, g=0, b=0 => (3 << 5) + (0 << 2) + 0 = 96 => 128+96 = 224
        let red = get_material_color(224);
        assert!(red.x > 0.8);
        assert!(red.y < 0.1);
        assert!(red.z < 0.1);
    }
}
