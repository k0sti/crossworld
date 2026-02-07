//! Game configuration loading from KDL and Lua files
//!
//! Configuration is loaded in two phases:
//! 1. KDL (app.kdl) - Static configuration (world params, materials, map layout)
//! 2. Lua (world.lua) - Dynamic/procedural configuration (model generation)

use cube::{io::vox::load_vox_to_cubebox, CubeBox};
use glam::{IVec3, Vec3};
use scripting::{extract_u32, KdlReader, LuaEngine, StateTree};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Default depth scale offset for loaded vox models.
/// Negative value makes models smaller: -3 means 2^3 = 8x smaller.
const DEFAULT_MODEL_DEPTH_SCALE: i32 = -3;

/// World configuration
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
    /// Depth scale offset: negative = smaller, positive = larger
    /// Default is -3 (8x smaller)
    pub scale: i32,
}

/// Model generation configuration (from KDL)
#[derive(Debug, Clone)]
pub struct ModelGenerationConfig {
    pub pattern: String,
    pub count: u32,
    pub radius_x: f32,
    pub radius_z: f32,
    pub y: f32,
    pub align: Vec3,
    pub scale: i32,
}

impl Default for ModelGenerationConfig {
    fn default() -> Self {
        Self {
            pattern: "scene_*".to_string(),
            count: 10,
            radius_x: 50.0,
            radius_z: 50.0,
            y: 0.0,
            align: Vec3::new(0.5, 0.0, 0.5),
            scale: DEFAULT_MODEL_DEPTH_SCALE,
        }
    }
}

/// Combined game configuration
#[derive(Debug, Clone, Default)]
pub struct GameConfig {
    pub world: WorldConfig,
    pub map: MapConfig,
    /// Model generation config (used during construction to generate models)
    #[allow(dead_code)]
    pub model_gen: ModelGenerationConfig,
    pub models: Vec<WorldModelConfig>,
}

impl GameConfig {
    /// Load configuration from KDL file, then optionally execute Lua for procedural generation
    pub fn from_kdl_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let tree = KdlReader::from_file(path.as_ref())
            .map_err(|e| format!("Failed to load KDL config: {}", e))?;

        Self::from_state_tree(&tree, path.as_ref().parent())
    }

    /// Load configuration from a StateTree (parsed KDL)
    pub fn from_state_tree(tree: &StateTree, _config_dir: Option<&Path>) -> Result<Self, String> {
        let world_config = Self::extract_world_config(tree)?;
        let map_config = Self::extract_map_config(tree)?;
        let model_gen = Self::extract_model_gen_config(tree)?;

        // Generate models using LCG PRNG (same algorithm as Lua version)
        let models = Self::generate_models(&model_gen, world_config.seed);

        Ok(Self {
            world: world_config,
            map: map_config,
            model_gen,
            models,
        })
    }

    /// Load configuration from Lua file (legacy support)
    pub fn from_lua_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let engine = LuaEngine::new().map_err(|e| format!("Failed to create Lua: {}", e))?;

        engine
            .exec_file(path.as_ref())
            .map_err(|e| format!("Failed to load Lua config: {}", e))?;

        Self::from_lua_engine(&engine)
    }

    /// Extract configuration from Lua engine globals
    fn from_lua_engine(engine: &LuaEngine) -> Result<Self, String> {
        use scripting::mlua::prelude::*;

        let lua = engine.lua();
        let globals = lua.globals();

        // Extract world_config
        let world_table: LuaTable = globals
            .get("world_config")
            .map_err(|e| format!("Missing world_config: {}", e))?;

        let world_config = WorldConfig {
            macro_depth: extract_u32(
                &world_table
                    .get("macro_depth")
                    .map_err(|e| format!("Missing macro_depth: {}", e))?,
            )
            .map_err(|e| format!("Invalid macro_depth: {}", e))?,
            micro_depth: extract_u32(
                &world_table
                    .get("micro_depth")
                    .map_err(|e| format!("Missing micro_depth: {}", e))?,
            )
            .map_err(|e| format!("Invalid micro_depth: {}", e))?,
            border_depth: extract_u32(
                &world_table
                    .get("border_depth")
                    .map_err(|e| format!("Missing border_depth: {}", e))?,
            )
            .map_err(|e| format!("Invalid border_depth: {}", e))?,
            seed: extract_u32(
                &world_table
                    .get("seed")
                    .map_err(|e| format!("Missing seed: {}", e))?,
            )
            .map_err(|e| format!("Invalid seed: {}", e))?,
        };

        // Extract map config
        let map_config = Self::extract_map_config_from_lua(lua)?;

        // Extract world_models (generated by Lua)
        let models = Self::extract_model_configs_from_lua(lua)?;

        Ok(Self {
            world: world_config,
            map: map_config,
            model_gen: ModelGenerationConfig::default(),
            models,
        })
    }

    /// Extract world configuration from StateTree
    fn extract_world_config(tree: &StateTree) -> Result<WorldConfig, String> {
        let world_node = tree
            .get_node("app.scene.world")
            .ok_or("Missing app.scene.world node")?;

        let macro_depth = world_node
            .child("macro_depth")
            .and_then(|n| n.value.as_u32().ok())
            .unwrap_or(3);

        let micro_depth = world_node
            .child("micro_depth")
            .and_then(|n| n.value.as_u32().ok())
            .unwrap_or(5);

        let border_depth = world_node
            .child("border_depth")
            .and_then(|n| n.value.as_u32().ok())
            .unwrap_or(1);

        let seed = world_node
            .child("seed")
            .and_then(|n| n.value.as_u32().ok())
            .unwrap_or(12345);

        Ok(WorldConfig {
            macro_depth,
            micro_depth,
            border_depth,
            seed,
        })
    }

    /// Extract map configuration from StateTree
    fn extract_map_config(tree: &StateTree) -> Result<MapConfig, String> {
        let mut chars = HashMap::new();
        let mut materials = HashMap::new();
        let mut layout = Vec::new();

        // Extract materials
        if let Some(materials_node) = tree.get_node("app.scene.materials") {
            for name in materials_node.child_names() {
                if let Some(child) = materials_node.child(name) {
                    if let Ok(id) = child.value.as_u32() {
                        materials.insert(name.to_string(), id as u8);
                    }
                }
            }
        }

        // Extract character mappings
        if let Some(chars_node) = tree.get_node("app.scene.map.chars") {
            for name in chars_node.child_names() {
                if let Some(child) = chars_node.child(name) {
                    let mat = child
                        .attr("mat")
                        .and_then(|v| v.as_str().ok())
                        .unwrap_or("empty")
                        .to_string();

                    let is_spawn = child
                        .attr("is_spawn")
                        .and_then(|v| v.as_bool().ok())
                        .unwrap_or(false);

                    // Map character name to actual char
                    let ch = match name {
                        "space" => ' ',
                        "wall" => '#',
                        "spawn" => '^',
                        _ => continue,
                    };

                    chars.insert(
                        ch,
                        MapChar {
                            material: mat,
                            is_spawn,
                        },
                    );
                }
            }
        }

        // Extract layout
        if let Some(layout_node) = tree.get_node("app.scene.map.layout") {
            if let Ok(layout_str) = layout_node.value.as_str() {
                layout = layout_str
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        Ok(MapConfig {
            chars,
            layout,
            materials,
        })
    }

    /// Extract model generation config from StateTree
    fn extract_model_gen_config(tree: &StateTree) -> Result<ModelGenerationConfig, String> {
        let models_node = match tree.get_node("app.scene.models") {
            Some(node) => node,
            None => return Ok(ModelGenerationConfig::default()),
        };

        let pattern = models_node
            .child("pattern")
            .and_then(|n| n.value.as_str().ok())
            .unwrap_or("scene_*")
            .to_string();

        let count = models_node
            .child("count")
            .and_then(|n| n.value.as_u32().ok())
            .unwrap_or(10);

        let radius_x = models_node
            .child("radius_x")
            .and_then(|n| n.value.as_f32().ok())
            .unwrap_or(50.0);

        let radius_z = models_node
            .child("radius_z")
            .and_then(|n| n.value.as_f32().ok())
            .unwrap_or(50.0);

        let y = models_node
            .child("y")
            .and_then(|n| n.value.as_f32().ok())
            .unwrap_or(0.0);

        let align = models_node
            .child("align")
            .and_then(|n| n.value.as_vec3().ok())
            .unwrap_or(Vec3::new(0.5, 0.0, 0.5));

        let scale = models_node
            .child("scale")
            .and_then(|n| n.value.as_i64().ok())
            .unwrap_or(DEFAULT_MODEL_DEPTH_SCALE as i64) as i32;

        Ok(ModelGenerationConfig {
            pattern,
            count,
            radius_x,
            radius_z,
            y,
            align,
            scale,
        })
    }

    /// Generate models using LCG PRNG (same algorithm as Lua version)
    fn generate_models(config: &ModelGenerationConfig, seed: u32) -> Vec<WorldModelConfig> {
        let mut models = Vec::new();
        let mut rng = LcgRng::new(seed as u64);

        for i in 0..config.count {
            let x = (rng.next_f32() * 2.0 - 1.0) * config.radius_x;
            let z = (rng.next_f32() * 2.0 - 1.0) * config.radius_z;

            models.push(WorldModelConfig {
                pattern: config.pattern.clone(),
                index: i as usize,
                align: config.align,
                position: Vec3::new(x, config.y, z),
                scale: config.scale,
            });
        }

        models
    }

    /// Extract map config from Lua globals
    fn extract_map_config_from_lua(lua: &scripting::mlua::Lua) -> Result<MapConfig, String> {
        use scripting::mlua::prelude::*;

        let globals = lua.globals();

        // Extract map characters
        let map_chars_table: LuaTable = globals
            .get("map_chars")
            .map_err(|e| format!("Missing map_chars: {}", e))?;

        let mut chars = HashMap::new();
        for pair in map_chars_table.pairs::<String, LuaTable>() {
            let (key, value) = pair.map_err(|e| format!("Failed to parse map_chars: {}", e))?;
            if key.len() == 1 {
                let ch = key.chars().next().unwrap();
                let mat: String = value
                    .get("mat")
                    .map_err(|e| format!("Missing mat: {}", e))?;
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

        // Extract layout
        let layout_str: String = globals
            .get("map_layout")
            .map_err(|e| format!("Missing map_layout: {}", e))?;
        let layout: Vec<String> = layout_str
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.to_string())
            .collect();

        // Extract materials
        let materials_table: LuaTable = globals
            .get("materials")
            .map_err(|e| format!("Missing materials: {}", e))?;

        let mut materials = HashMap::new();
        for pair in materials_table.pairs::<String, LuaValue>() {
            let (name, value) = pair.map_err(|e| format!("Failed to parse materials: {}", e))?;
            let material_id = match value {
                LuaValue::Integer(i) => i as u8,
                LuaValue::Number(n) => n as u8,
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

    /// Extract world model configs from Lua globals
    fn extract_model_configs_from_lua(
        lua: &scripting::mlua::Lua,
    ) -> Result<Vec<WorldModelConfig>, String> {
        use scripting::mlua::prelude::*;

        let globals = lua.globals();
        let models_table: Option<LuaTable> = globals.get("world_models").ok();

        let mut models = Vec::new();

        if let Some(table) = models_table {
            for pair in table.pairs::<LuaValue, LuaTable>() {
                let (_, model_table) =
                    pair.map_err(|e| format!("Failed to parse world_models: {}", e))?;

                let pattern: String = model_table
                    .get("pattern")
                    .map_err(|e| format!("Missing pattern: {}", e))?;

                let index = extract_u32(
                    &model_table
                        .get("index")
                        .map_err(|e| format!("Missing index: {}", e))?,
                )
                .map_err(|e| format!("Invalid index: {}", e))? as usize;

                let align: LuaTable = model_table
                    .get("align")
                    .map_err(|e| format!("Missing align: {}", e))?;
                let align_vec = Vec3::new(
                    align.get(1).unwrap_or(0.5),
                    align.get(2).unwrap_or(0.0),
                    align.get(3).unwrap_or(0.5),
                );

                let position: LuaTable = model_table
                    .get("position")
                    .map_err(|e| format!("Missing position: {}", e))?;
                let pos_vec = Vec3::new(
                    position.get(1).unwrap_or(0.0),
                    position.get(2).unwrap_or(0.0),
                    position.get(3).unwrap_or(0.0),
                );

                let scale: i32 = model_table
                    .get("scale")
                    .unwrap_or(DEFAULT_MODEL_DEPTH_SCALE);

                models.push(WorldModelConfig {
                    pattern,
                    index,
                    align: align_vec,
                    position: pos_vec,
                    scale,
                });
            }
        }

        Ok(models)
    }

    /// Apply 2D map to world cube
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

        let height = layout.len() as i32;
        let width = layout.iter().map(|s| s.len()).max().unwrap_or(0) as i32;

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
        }

        let mut voxels_placed = 0;
        let mut voxels_cleared = 0;

        for (z_idx, row) in layout.iter().enumerate() {
            for (x_idx, ch) in row.chars().enumerate() {
                if let Some(map_char) = self.map.chars.get(&ch) {
                    let map_x = x_idx as i32 + offset_x;
                    let map_z = z_idx as i32 + offset_z;

                    let half_size = (1 << self.world.macro_depth) / 2;
                    let octree_x = map_x + half_size;
                    let octree_z = map_z + half_size;
                    let octree_y = half_size;

                    let max_coord = (1 << self.world.macro_depth) - 1;
                    if octree_x < 0 || octree_x > max_coord || octree_z < 0 || octree_z > max_coord
                    {
                        continue;
                    }

                    if let Some(&material_id) = self.map.materials.get(&map_char.material) {
                        if material_id > 0 {
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
                        } else {
                            world.set_voxel_at_depth(
                                octree_x,
                                octree_y,
                                octree_z,
                                self.world.macro_depth,
                                0,
                            );
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
                        }
                    }

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
    pub fn apply_models_to_world(
        &self,
        base_cube: cube::Cube<u8>,
        debug: bool,
    ) -> Result<crossworld_world::World, String> {
        use crossworld_world::World;
        use std::time::Instant;

        let total_start = Instant::now();

        let world_create_start = Instant::now();
        let mut world = World::new(base_cube, self.world.macro_depth);
        if debug {
            println!("[Game] World::new took {:?}", world_create_start.elapsed());
        }

        if debug {
            println!("[Game] Loading {} world models", self.models.len());
        }

        let mut assets_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assets_path.pop(); // crates/apps/game -> crates/apps
        assets_path.pop(); // crates/apps -> crates
        assets_path.pop(); // crates -> workspace root
        assets_path.push("assets");
        assets_path.push("models");
        assets_path.push("vox");

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
            }

            let load_start = Instant::now();
            let model = Self::load_vox_model(&model_path, model_config.scale)?;
            let load_duration = load_start.elapsed();
            total_load_time += load_duration;

            let aligned_pos =
                Self::calculate_aligned_position(&model, model_config.position, model_config.align);

            let merge_start = Instant::now();
            world.merge_model(&model, aligned_pos, self.world.macro_depth);
            let merge_duration = merge_start.elapsed();
            total_merge_time += merge_duration;

            if debug {
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

    fn resolve_model_pattern(
        assets_path: &Path,
        pattern: &str,
        scene_models: &[String],
        index: usize,
    ) -> Result<PathBuf, String> {
        if pattern.contains('*') {
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
            let filename = if pattern.ends_with(".vox") {
                pattern.to_string()
            } else {
                format!("{}.vox", pattern)
            };
            Ok(assets_path.join(filename))
        }
    }

    fn load_vox_model(path: &Path, scale: i32) -> Result<CubeBox<u8>, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read model file {}: {}", path.display(), e))?;

        let mut model = load_vox_to_cubebox(&bytes)?;

        let scaled_depth = (model.depth as i32 + scale).max(0) as u32;
        model.depth = scaled_depth;

        Ok(model)
    }

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

/// Simple LCG PRNG (same algorithm as Lua version for reproducibility)
struct LcgRng {
    seed: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self { seed }
    }

    fn next(&mut self) -> u64 {
        const A: u64 = 1664525;
        const C: u64 = 1013904223;
        const M: u64 = 1 << 32;
        self.seed = (A.wrapping_mul(self.seed).wrapping_add(C)) % M;
        self.seed
    }

    fn next_f32(&mut self) -> f32 {
        self.next() as f32 / (1u64 << 32) as f32
    }
}
