use app::lua_config::{extract_u32, mlua, LuaConfig};
use cube::{io::vox::load_vox_to_cubebox, CubeBox};
use glam::{IVec3, Vec3};
use mlua::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

/// World model configuration
#[derive(Debug, Clone)]
pub struct WorldModelConfig {
    pub pattern: String,
    pub index: usize,
    pub align: Vec3,
    pub position: Vec3,
}

/// Combined game configuration
#[derive(Debug, Clone, Default)]
pub struct GameConfig {
    pub world: WorldConfig,
    pub map: MapConfig,
    pub models: Vec<WorldModelConfig>,
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
        let model_configs = Self::extract_model_configs(lua_config.lua())?;

        Ok(Self {
            world: world_config,
            map: map_config,
            models: model_configs,
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

    /// Extract world model configurations from Lua globals
    fn extract_model_configs(lua: &Lua) -> Result<Vec<WorldModelConfig>, String> {
        let globals = lua.globals();

        // Get world_models table (optional)
        let models_table: Option<mlua::Table> = globals.get("world_models").ok();

        let mut models = Vec::new();

        if let Some(table) = models_table {
            for pair in table.pairs::<mlua::Value, mlua::Table>() {
                let (_, model_table): (mlua::Value, mlua::Table) =
                    pair.map_err(|e: mlua::Error| format!("Failed to parse world_models: {}", e))?;

                let pattern: String = model_table
                    .get("pattern")
                    .map_err(|e| format!("Missing pattern in world_models: {}", e))?;

                let index = extract_u32(
                    &model_table
                        .get("index")
                        .map_err(|e| format!("Missing index in world_models: {}", e))?,
                )
                .map_err(|e| format!("Invalid index: {}", e))? as usize;

                // Extract Vec3 values using LuaConfig's vec3 support
                let align: mlua::Table = model_table
                    .get("align")
                    .map_err(|e| format!("Missing align in world_models: {}", e))?;
                let align_x: f32 = align.get(1).unwrap_or(0.5);
                let align_y: f32 = align.get(2).unwrap_or(0.0);
                let align_z: f32 = align.get(3).unwrap_or(0.5);

                let position: mlua::Table = model_table
                    .get("position")
                    .map_err(|e| format!("Missing position in world_models: {}", e))?;
                let pos_x: f32 = position.get(1).unwrap_or(0.0);
                let pos_y: f32 = position.get(2).unwrap_or(0.0);
                let pos_z: f32 = position.get(3).unwrap_or(0.0);

                models.push(WorldModelConfig {
                    pattern,
                    index,
                    align: Vec3::new(align_x, align_y, align_z),
                    position: Vec3::new(pos_x, pos_y, pos_z),
                });
            }
        }

        Ok(models)
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

    /// Load and apply world models from configuration
    /// Returns a World struct with all models merged
    pub fn apply_models_to_world(
        &self,
        base_cube: cube::Cube<u8>,
        debug: bool,
    ) -> Result<crossworld_world::World, String> {
        use crossworld_world::World;
        use std::time::Instant;

        let total_start = Instant::now();

        // Start with the base world from macro_depth
        let world_create_start = Instant::now();
        let mut world = World::new(base_cube, self.world.macro_depth);
        if debug {
            println!("[Game] World::new took {:?}", world_create_start.elapsed());
        }

        if debug {
            println!("[Game] Loading {} world models", self.models.len());
        }

        // Resolve asset directory
        let mut assets_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assets_path.pop(); // Go from crates/game to crates
        assets_path.pop(); // Go from crates to workspace root
        assets_path.push("assets");
        assets_path.push("models");
        assets_path.push("vox");

        // Get list of available scene models
        let find_start = Instant::now();
        let scene_models = Self::find_scene_models(&assets_path)?;
        if debug {
            println!("[Game] find_scene_models took {:?}", find_start.elapsed());
            println!(
                "[Game] Found {} scene_* models in {}",
                scene_models.len(),
                assets_path.display()
            );
        }

        let mut total_load_time = std::time::Duration::ZERO;
        let mut total_merge_time = std::time::Duration::ZERO;

        // Load and merge each configured model
        for (i, model_config) in self.models.iter().enumerate() {
            let model_path = Self::resolve_model_pattern(
                &assets_path,
                &model_config.pattern,
                &scene_models,
                model_config.index,
            )?;

            if debug {
                println!(
                    "[Game] [{}/{}] Loading model: {}",
                    i + 1,
                    self.models.len(),
                    model_path.display()
                );
                println!("[Game]   Position: {:?}", model_config.position);
                println!("[Game]   Align: {:?}", model_config.align);
            }

            // Load the vox model
            let load_start = Instant::now();
            let model = Self::load_vox_model(&model_path)?;
            let load_duration = load_start.elapsed();
            total_load_time += load_duration;

            // Calculate aligned position
            let aligned_pos =
                Self::calculate_aligned_position(&model, model_config.position, model_config.align);

            if debug {
                println!("[Game]   Model size: {:?}", model.size);
                println!("[Game]   Model depth: {}", model.depth);
                println!("[Game]   Aligned position: {:?}", aligned_pos);
                println!("[Game]   Load time: {:?}", load_duration);
            }

            // Merge into world at macro_depth
            let merge_start = Instant::now();
            world.merge_model(&model, aligned_pos, self.world.macro_depth);
            let merge_duration = merge_start.elapsed();
            total_merge_time += merge_duration;

            if debug {
                println!("[Game]   Merge time: {:?}", merge_duration);
                println!(
                    "[Game]   World scale after merge: 2^{} = {} units",
                    world.scale(),
                    1 << world.scale()
                );
            }
        }

        if debug {
            println!("[Game] All models merged successfully");
            println!(
                "[Game] Final world scale: 2^{} = {} units",
                world.scale(),
                1 << world.scale()
            );
            println!("[Game] === Performance Summary ===");
            println!("[Game]   Total load time: {:?}", total_load_time);
            println!("[Game]   Total merge time: {:?}", total_merge_time);
            println!(
                "[Game]   Total apply_models_to_world: {:?}",
                total_start.elapsed()
            );
        }

        Ok(world)
    }

    /// Find all scene_*.vox models in the assets directory
    fn find_scene_models(assets_path: &Path) -> Result<Vec<String>, String> {
        let entries = std::fs::read_dir(assets_path)
            .map_err(|e| format!("Failed to read assets directory: {}", e))?;

        let mut models = Vec::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("scene_") && name.ends_with(".vox") {
                    models.push(name.to_string());
                }
            }
        }

        models.sort();
        Ok(models)
    }

    /// Resolve a model pattern to an actual file path
    fn resolve_model_pattern(
        assets_path: &Path,
        pattern: &str,
        scene_models: &[String],
        index: usize,
    ) -> Result<PathBuf, String> {
        if pattern.contains('*') {
            // Wildcard pattern - use index to select from matching models
            let prefix = pattern.trim_end_matches('*');
            let matching: Vec<_> = scene_models
                .iter()
                .filter(|name| name.starts_with(prefix))
                .collect();

            if matching.is_empty() {
                return Err(format!("No models match pattern: {}", pattern));
            }

            let selected_index = index % matching.len();
            let selected_model = matching[selected_index];
            Ok(assets_path.join(selected_model))
        } else {
            // Direct filename
            let filename = if pattern.ends_with(".vox") {
                pattern.to_string()
            } else {
                format!("{}.vox", pattern)
            };
            Ok(assets_path.join(filename))
        }
    }

    /// Load a vox model from file
    fn load_vox_model(path: &Path) -> Result<CubeBox<u8>, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read model file {}: {}", path.display(), e))?;

        load_vox_to_cubebox(&bytes)
    }

    /// Calculate aligned position for a model
    /// align: (0,0,0) = bottom-left corner, (0.5,0,0.5) = bottom center, (1,1,1) = top-right corner
    fn calculate_aligned_position(model: &CubeBox<u8>, world_pos: Vec3, align: Vec3) -> IVec3 {
        let offset_x = -(model.size.x as f32 * align.x);
        let offset_y = -(model.size.y as f32 * align.y);
        let offset_z = -(model.size.z as f32 * align.z);

        IVec3::new(
            (world_pos.x + offset_x) as i32,
            (world_pos.y + offset_y) as i32,
            (world_pos.z + offset_z) as i32,
        )
    }
}
