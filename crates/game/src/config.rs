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
        let mut lua_config =
            LuaConfig::new().map_err(|e| format!("Failed to create Lua config: {}", e))?;

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

        let macro_depth = extract_u32(
            &world_table
                .get("macro_depth")
                .map_err(|e| format!("Missing macro_depth: {}", e))?,
        )
        .map_err(|e| format!("Invalid macro_depth: {}", e))?;
        let micro_depth = extract_u32(
            &world_table
                .get("micro_depth")
                .map_err(|e| format!("Missing micro_depth: {}", e))?,
        )
        .map_err(|e| format!("Invalid micro_depth: {}", e))?;
        let border_depth = extract_u32(
            &world_table
                .get("border_depth")
                .map_err(|e| format!("Missing border_depth: {}", e))?,
        )
        .map_err(|e| format!("Invalid border_depth: {}", e))?;
        let seed = extract_u32(
            &world_table
                .get("seed")
                .map_err(|e| format!("Missing seed: {}", e))?,
        )
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
            let (key, value): (String, mlua::Table) =
                pair.map_err(|e: mlua::Error| format!("Failed to parse map_chars: {}", e))?;
            if key.len() == 1 {
                let ch = key.chars().next().unwrap();
                let mat: String = value
                    .get("mat")
                    .map_err(|e: mlua::Error| format!("Missing mat for char '{}': {}", ch, e))?;
                let is_spawn: bool = value.get("spawn").unwrap_or(false);
                chars.insert(
                    ch,
                    MapChar {
                        material: mat,
                        is_spawn,
                    },
                );
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
            let (name, value): (String, mlua::Value) =
                pair.map_err(|e: mlua::Error| format!("Failed to parse materials: {}", e))?;
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
    /// Maps the 2D layout onto the XZ plane at y=0, centered at origin
    /// At macro_depth, each voxel = 1 world unit (1 meter)
    ///
    /// The map modifies the terrain:
    /// - '#' (bedrock): Places solid bedrock column from y=-1 to y=1
    /// - ' ' (empty): Carves out the terrain at y=0 (creates walkable floor)
    /// - '^' (spawn): Same as empty but marks spawn point
    pub fn apply_map_to_world(
        &self,
        world: &mut crossworld_world::NativeWorldCube,
        debug: bool,
    ) -> Option<Vec3> {
        let mut spawn_pos = None;

        let layout = &self.map.layout;
        if layout.is_empty() {
            if debug {
                println!("[Game] Map layout is empty, skipping map application");
            }
            return spawn_pos;
        }

        // At macro_depth, coordinates map directly to world units (1 voxel = 1 meter)
        // Calculate map dimensions
        let height = layout.len() as i32;
        let width = layout.iter().map(|s| s.len()).max().unwrap_or(0) as i32;

        // Center offset to place map centered on origin
        let offset_x = -width / 2;
        let offset_z = -height / 2;

        if debug {
            println!("[Game] Applying 2D map to world ground:");
            println!(
                "[Game]   Map dimensions: {}x{} (width x height)",
                width, height
            );
            println!("[Game]   Center offset: ({}, {})", offset_x, offset_z);
            println!("[Game]   Macro depth: {}", self.world.macro_depth);
            println!("[Game]   Materials: {:?}", self.map.materials);
            println!("[Game]   Character mappings: {:?}", self.map.chars);
            println!("[Game]   Layout:");
            for (i, row) in layout.iter().enumerate() {
                println!("[Game]     Row {}: \"{}\"", i, row);
            }
        }

        let mut voxels_placed = 0;
        let mut voxels_cleared = 0;

        // Iterate through map and place/clear voxels at macro_depth
        for (z_idx, row) in layout.iter().enumerate() {
            for (x_idx, ch) in row.chars().enumerate() {
                if let Some(map_char) = self.map.chars.get(&ch) {
                    // Position in voxel coordinates at macro_depth
                    // Each coordinate = 1 world unit
                    // We need to convert to octree coordinates which are [0, 2^depth)
                    let map_x = x_idx as i32 + offset_x;
                    let map_z = z_idx as i32 + offset_z;

                    // Convert world coords to octree coords
                    // At macro_depth, the world spans [-half_size, half_size) in world units
                    // Octree coords are [0, 2^depth)
                    let half_size = (1 << self.world.macro_depth) / 2;
                    let octree_x = map_x + half_size;
                    let octree_z = map_z + half_size;
                    let octree_y = half_size; // y=0 in world coords = half_size in octree coords

                    // Validate coordinates are within bounds
                    let max_coord = (1 << self.world.macro_depth) - 1;
                    if octree_x < 0 || octree_x > max_coord || octree_z < 0 || octree_z > max_coord
                    {
                        if debug {
                            println!("[Game]   Skipping out-of-bounds: world ({}, 0, {}) -> octree ({}, {}, {})",
                                map_x, map_z, octree_x, octree_y, octree_z);
                        }
                        continue;
                    }

                    // Get material ID
                    if let Some(&material_id) = self.map.materials.get(&map_char.material) {
                        if material_id > 0 {
                            // Place solid voxel - create a column for walls
                            // Place at y=-1, y=0, y=1 (relative to world origin)
                            for dy in -1..=1 {
                                let y = octree_y + dy;
                                if y >= 0 && y <= max_coord {
                                    world.set_voxel_at_depth(
                                        octree_x,
                                        y,
                                        octree_z,
                                        self.world.macro_depth,
                                        material_id,
                                    );
                                    voxels_placed += 1;
                                }
                            }
                            if debug {
                                println!("[Game]   Placed wall at world ({}, 0, {}) -> octree ({}, {}, {}) mat={}",
                                    map_x, map_z, octree_x, octree_y, octree_z, material_id);
                            }
                        } else {
                            // Empty material (air) - clear the voxel at y=0 for walkable floor
                            world.set_voxel_at_depth(
                                octree_x,
                                octree_y,
                                octree_z,
                                self.world.macro_depth,
                                0,
                            );
                            // Also clear above for headroom
                            if octree_y < max_coord {
                                world.set_voxel_at_depth(
                                    octree_x,
                                    octree_y + 1,
                                    octree_z,
                                    self.world.macro_depth,
                                    0,
                                );
                            }
                            voxels_cleared += 1;
                            if debug {
                                println!("[Game]   Cleared floor at world ({}, 0, {}) -> octree ({}, {}, {})",
                                    map_x, map_z, octree_x, octree_y, octree_z);
                            }
                        }
                    }

                    // Track spawn position (in world coordinates)
                    if map_char.is_spawn {
                        spawn_pos = Some(Vec3::new(map_x as f32 + 0.5, 1.0, map_z as f32 + 0.5));
                        if debug {
                            println!(
                                "[Game]   Spawn point set at world ({:.1}, 1.0, {:.1})",
                                map_x as f32 + 0.5,
                                map_z as f32 + 0.5
                            );
                        }
                    }
                }
            }
        }

        if debug {
            println!("[Game]   Total voxels placed: {}", voxels_placed);
            println!("[Game]   Total voxels cleared: {}", voxels_cleared);
        }

        spawn_pos
    }
}
