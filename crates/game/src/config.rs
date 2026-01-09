use app::lua_config::{extract_u32, mlua, LuaConfig};
use glam::Vec3;
use mlua::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// World configuration from Lua
#[derive(Debug, Clone)]
pub struct WorldConfig {
    pub macro_depth: u32,
    pub micro_depth: u32,
    pub border_depth: u32,
    pub seed: u32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            macro_depth: 3,
            micro_depth: 5,
            border_depth: 1,
            seed: 12345,
        }
    }
}

/// Character mapping for 2D map
#[derive(Debug, Clone)]
pub struct MapChar {
    pub material: String,
    pub is_spawn: bool,
}

/// 2D Map configuration
#[derive(Debug, Clone, Default)]
pub struct MapConfig {
    pub chars: HashMap<char, MapChar>,
    pub layout: Vec<String>,
    pub materials: HashMap<String, u8>,
}

/// Combined game configuration
#[derive(Debug, Clone, Default)]
pub struct GameConfig {
    pub world: WorldConfig,
    pub map: MapConfig,
}

impl GameConfig {
    /// Load configuration from Lua file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut lua_config = LuaConfig::new().map_err(|e| format!("Failed to create Lua config: {}", e))?;

        lua_config
            .load_file(path.as_ref())
            .map_err(|e| format!("Failed to load config file: {}", e))?;

        let world_config = Self::extract_world_config(lua_config.lua())?;
        let map_config = Self::extract_map_config(lua_config.lua())?;

        Ok(Self {
            world: world_config,
            map: map_config,
        })
    }

    /// Extract world configuration from Lua globals
    fn extract_world_config(lua: &Lua) -> Result<WorldConfig, String> {
        let globals = lua.globals();
        let world_table: mlua::Table = globals
            .get("world_config")
            .map_err(|e| format!("Missing world_config table: {}", e))?;

        let macro_depth = extract_u32(&world_table.get("macro_depth").map_err(|e| format!("Missing macro_depth: {}", e))?)
            .map_err(|e| format!("Invalid macro_depth: {}", e))?;
        let micro_depth = extract_u32(&world_table.get("micro_depth").map_err(|e| format!("Missing micro_depth: {}", e))?)
            .map_err(|e| format!("Invalid micro_depth: {}", e))?;
        let border_depth = extract_u32(&world_table.get("border_depth").map_err(|e| format!("Missing border_depth: {}", e))?)
            .map_err(|e| format!("Invalid border_depth: {}", e))?;
        let seed = extract_u32(&world_table.get("seed").map_err(|e| format!("Missing seed: {}", e))?)
            .map_err(|e| format!("Invalid seed: {}", e))?;

        Ok(WorldConfig {
            macro_depth,
            micro_depth,
            border_depth,
            seed,
        })
    }

    /// Extract map configuration from Lua globals
    fn extract_map_config(lua: &Lua) -> Result<MapConfig, String> {
        let globals = lua.globals();

        // Extract map characters
        let map_chars_table: mlua::Table = globals
            .get("map_chars")
            .map_err(|e| format!("Missing map_chars table: {}", e))?;

        let mut chars = HashMap::new();
        for pair in map_chars_table.pairs::<String, mlua::Table>() {
            let (key, value): (String, mlua::Table) = pair.map_err(|e: mlua::Error| format!("Failed to parse map_chars: {}", e))?;
            if key.len() == 1 {
                let ch = key.chars().next().unwrap();
                let mat: String = value.get("mat").map_err(|e: mlua::Error| format!("Missing mat for char '{}': {}", ch, e))?;
                let is_spawn: bool = value.get("spawn").unwrap_or(false);
                chars.insert(ch, MapChar { material: mat, is_spawn });
            }
        }

        // Extract map layout
        let layout_str: String = globals
            .get("map_layout")
            .map_err(|e| format!("Missing map_layout: {}", e))?;
        let layout: Vec<String> = layout_str
            .lines()
            .filter(|line: &&str| !line.trim().is_empty())
            .map(|s: &str| s.to_string())
            .collect();

        // Extract materials
        let materials_table: mlua::Table = globals
            .get("materials")
            .map_err(|e| format!("Missing materials table: {}", e))?;

        let mut materials = HashMap::new();
        for pair in materials_table.pairs::<String, mlua::Value>() {
            let (name, value): (String, mlua::Value) = pair.map_err(|e: mlua::Error| format!("Failed to parse materials: {}", e))?;
            let material_id = match value {
                mlua::Value::Integer(i) => i as u8,
                mlua::Value::Number(n) => n as u8,
                _ => return Err(format!("Invalid material value for '{}'", name)),
            };
            materials.insert(name, material_id);
        }

        Ok(MapConfig {
            chars,
            layout,
            materials,
        })
    }

    /// Apply 2D map to world cube
    /// Maps the 2D layout onto the XZ plane at y=0, scaled by 2^macro_depth
    pub fn apply_map_to_world(&self, world: &mut crossworld_world::NativeWorldCube) -> Option<Vec3> {
        let mut spawn_pos = None;

        let layout = &self.map.layout;
        if layout.is_empty() {
            return spawn_pos;
        }

        // Scale factor: voxels at macro_depth correspond to world units
        // Each voxel at macro_depth = 2^macro_depth world units
        let scale = 1 << self.world.macro_depth;

        // Calculate map dimensions
        let height = layout.len() as i32;
        let width = layout.iter().map(|s| s.len()).max().unwrap_or(0) as i32;

        // Calculate center offset to center map on origin (in world coordinates)
        let offset_x = -(width * scale) / 2;
        let offset_z = -(height * scale) / 2;

        // Iterate through map and place voxels at macro_depth
        for (z_idx, row) in layout.iter().enumerate() {
            for (x_idx, ch) in row.chars().enumerate() {
                if let Some(map_char) = self.map.chars.get(&ch) {
                    // Position in voxel coordinates at macro_depth
                    let x = x_idx as i32;
                    let z = z_idx as i32;
                    let y = 0_i32; // Place at y=0 in voxel coordinates

                    // Get material ID
                    if let Some(&material_id) = self.map.materials.get(&map_char.material) {
                        if material_id > 0 {
                            // Place voxel at macro_depth (matches terrain scale)
                            world.set_voxel_at_depth(x, y, z, self.world.macro_depth, material_id);
                        }
                    }

                    // Track spawn position (in world coordinates)
                    if map_char.is_spawn {
                        let world_x = offset_x as f32 + (x as f32 + 0.5) * scale as f32;
                        let world_z = offset_z as f32 + (z as f32 + 0.5) * scale as f32;
                        spawn_pos = Some(Vec3::new(world_x, 1.0, world_z));
                    }
                }
            }
        }

        spawn_pos
    }
}
