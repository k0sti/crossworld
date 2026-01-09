//! Testbed-specific Lua configuration for scene setup
//!
//! Extends the base Lua configuration from `app` with physics and scene types.

use app::lua_config::{
    extract_u32, extract_u8, lua_val_to_f64, parse_quat, parse_vec3, LuaConfig, QuatConfig,
    Vec3Config,
};
// Re-export from app's lua re-exports
use app::lua_config::mlua::prelude::*;
use std::path::Path;

/// Ground type configuration
#[derive(Debug, Clone)]
pub enum GroundConfig {
    /// Solid cube with material and size_shift (edge = 2^size_shift)
    SolidCube {
        material: u8,
        size_shift: u32,
        center: Vec3Config,
    },
    /// Ground loaded from CSM file
    CsmFile {
        path: String,
        size_shift: u32,
        center: Vec3Config,
    },
}

/// Object configuration in the scene
#[derive(Debug, Clone)]
pub struct ObjectConfig {
    pub position: Vec3Config,
    pub rotation: QuatConfig,
    pub size: Vec3Config,
    pub mass: f32,
    pub material: u8,
}

/// Camera configuration
#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub position: Vec3Config,
    pub look_at: Vec3Config,
}

/// Testbed Lua configuration with scene-specific functions
pub struct TestbedConfig {
    config: LuaConfig,
}

impl TestbedConfig {
    /// Create a new testbed configuration engine
    ///
    /// Registers base types from `app::lua_config` plus:
    /// - `camera` - Camera configuration
    /// - `ground_cube` - Solid cube ground
    /// - `ground_cuboid` - Cuboid ground
    /// - `object` - Scene object
    /// - `scene` - Complete scene
    /// - `quat` - Raw quaternion constructor
    /// - `rand_lcg` - LCG random seed generator
    /// - `rand_01` - Normalized random (0.0-1.0)
    /// - `rand_range` - Ranged random (min-max)
    pub fn new() -> LuaResult<Self> {
        let mut config = LuaConfig::new()?;
        let lua = config.lua_mut();

        // Register quat (raw quaternion constructor)
        let quat_fn = lua.create_function(
            |_, (x, y, z, w): (LuaValue, LuaValue, LuaValue, LuaValue)| {
                Ok(vec![
                    lua_val_to_f64(&x)?,
                    lua_val_to_f64(&y)?,
                    lua_val_to_f64(&z)?,
                    lua_val_to_f64(&w)?,
                ])
            },
        )?;
        lua.globals().set("quat", quat_fn)?;

        // Register camera
        let camera_fn = lua.create_function(|lua, (position, look_at): (LuaTable, LuaTable)| {
            let table = lua.create_table()?;
            table.set("type", "camera")?;
            table.set("position", position)?;
            table.set("look_at", look_at)?;
            Ok(table)
        })?;
        lua.globals().set("camera", camera_fn)?;

        // Register ground_cube
        let ground_cube_fn = lua.create_function(
            |lua, (material, size_shift, center): (LuaValue, LuaValue, LuaTable)| {
                let table = lua.create_table()?;
                table.set("type", "ground_cube")?;
                table.set("material", extract_u8(&material)?)?;
                table.set("size_shift", extract_u32(&size_shift)?)?;
                table.set("center", center)?;
                Ok(table)
            },
        )?;
        lua.globals().set("ground_cube", ground_cube_fn)?;

        // Register ground_csm
        let ground_csm_fn = lua.create_function(
            |lua, (path, size_shift, center): (String, LuaValue, LuaTable)| {
                let table = lua.create_table()?;
                table.set("type", "ground_csm")?;
                table.set("path", path)?;
                table.set("size_shift", extract_u32(&size_shift)?)?;
                table.set("center", center)?;
                Ok(table)
            },
        )?;
        lua.globals().set("ground_csm", ground_csm_fn)?;

        // Register object
        let object_fn = lua.create_function(
            |lua,
             (position, rotation, size, mass, material): (
                LuaTable,
                LuaTable,
                LuaTable,
                LuaValue,
                LuaValue,
            )| {
                let table = lua.create_table()?;
                table.set("type", "object")?;
                table.set("position", position)?;
                table.set("rotation", rotation)?;
                table.set("size", size)?;
                table.set("mass", lua_val_to_f64(&mass)? as f32)?;
                table.set("material", extract_u8(&material)?)?;
                Ok(table)
            },
        )?;
        lua.globals().set("object", object_fn)?;

        // Register scene
        let scene_fn = lua.create_function(
            |lua, (ground, objects, camera): (LuaTable, LuaTable, LuaTable)| {
                let table = lua.create_table()?;
                table.set("type", "scene")?;
                table.set("ground", ground)?;
                table.set("objects", objects)?;
                table.set("camera", camera)?;
                Ok(table)
            },
        )?;
        lua.globals().set("scene", scene_fn)?;

        // Register stateful random number generator
        // LCG: Linear Congruential Generator with constants from Numerical Recipes
        // Initialize the global random seed (default to 0)
        lua.globals().set("_rand_state", 0i64)?;

        // rand_seed(seed) - initialize the random state
        let rand_seed_fn = lua.create_function(|lua, seed: LuaValue| {
            let seed_val = lua_val_to_f64(&seed)? as i64;
            lua.globals().set("_rand_state", seed_val)?;
            Ok(())
        })?;
        lua.globals().set("rand_seed", rand_seed_fn)?;

        // rand_01() - return random float [0, 1) and advance state
        let rand_01_fn = lua.create_function(|lua, ()| {
            const A: u64 = 1664525;
            const C: u64 = 1013904223;
            const M: u64 = 2u64.pow(32);

            let current_seed: i64 = lua.globals().get("_rand_state")?;
            let next = (A.wrapping_mul(current_seed as u64).wrapping_add(C)) % M;
            lua.globals().set("_rand_state", next as i64)?;

            let normalized = next as f64 / M as f64;
            Ok(normalized)
        })?;
        lua.globals().set("rand_01", rand_01_fn)?;

        // rand_range(min, max) - return random float [min, max) and advance state
        let rand_range_fn = lua.create_function(|lua, (min, max): (LuaValue, LuaValue)| {
            const A: u64 = 1664525;
            const C: u64 = 1013904223;
            const M: u64 = 2u64.pow(32);

            let current_seed: i64 = lua.globals().get("_rand_state")?;
            let next = (A.wrapping_mul(current_seed as u64).wrapping_add(C)) % M;
            lua.globals().set("_rand_state", next as i64)?;

            let normalized = next as f64 / M as f64;
            let min_val = lua_val_to_f64(&min)?;
            let max_val = lua_val_to_f64(&max)?;
            let ranged = min_val + normalized * (max_val - min_val);

            Ok(ranged)
        })?;
        lua.globals().set("rand_range", rand_range_fn)?;

        Ok(Self { config })
    }

    /// Load and evaluate a Lua configuration file
    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        self.config.load_file(path)
    }

    /// Load configuration from a string
    #[cfg(test)]
    pub fn load_string(&mut self, content: &str) -> Result<(), String> {
        self.config.load_string(content)
    }

    /// Extract camera configuration
    pub fn extract_camera(&self, name: &str) -> Result<CameraConfig, String> {
        let table: LuaTable = self.config.extract_value(name)?;
        parse_camera_config(&table)
    }

    /// Extract ground configuration
    pub fn extract_ground(&self, name: &str) -> Result<GroundConfig, String> {
        let table: LuaTable = self.config.extract_value(name)?;
        parse_ground_config(&table)
    }

    /// Extract objects configuration
    pub fn extract_objects(&self, name: &str) -> Result<Vec<ObjectConfig>, String> {
        let table: LuaTable = self.config.extract_value(name)?;
        parse_objects_config(&table)
    }
}

impl Default for TestbedConfig {
    fn default() -> Self {
        Self::new().expect("Failed to create default TestbedConfig")
    }
}

// Parsing functions

fn parse_camera_config(table: &LuaTable) -> Result<CameraConfig, String> {
    let type_str: String = table
        .get("type")
        .map_err(|e| format!("camera missing type field: {}", e))?;

    if type_str != "camera" {
        return Err(format!("Expected camera type, got {}", type_str));
    }

    let position_table: LuaTable = table
        .get("position")
        .map_err(|e| format!("camera missing position: {}", e))?;
    let look_at_table: LuaTable = table
        .get("look_at")
        .map_err(|e| format!("camera missing look_at: {}", e))?;

    let position = parse_vec3(&position_table)
        .map_err(|e| format!("Failed to parse camera position: {}", e))?;
    let look_at =
        parse_vec3(&look_at_table).map_err(|e| format!("Failed to parse camera look_at: {}", e))?;

    Ok(CameraConfig { position, look_at })
}

fn parse_ground_config(table: &LuaTable) -> Result<GroundConfig, String> {
    let type_str: String = table
        .get("type")
        .map_err(|e| format!("ground missing type field: {}", e))?;

    match type_str.as_str() {
        "ground_cube" => {
            let material: u8 = table
                .get("material")
                .map_err(|e| format!("ground_cube missing material: {}", e))?;
            let size_shift: u32 = table
                .get("size_shift")
                .map_err(|e| format!("ground_cube missing size_shift: {}", e))?;
            let center_table: LuaTable = table
                .get("center")
                .map_err(|e| format!("ground_cube missing center: {}", e))?;
            let center = parse_vec3(&center_table)
                .map_err(|e| format!("Failed to parse ground center: {}", e))?;

            Ok(GroundConfig::SolidCube {
                material,
                size_shift,
                center,
            })
        }
        "ground_csm" => {
            let path: String = table
                .get("path")
                .map_err(|e| format!("ground_csm missing path: {}", e))?;
            let size_shift: u32 = table
                .get("size_shift")
                .map_err(|e| format!("ground_csm missing size_shift: {}", e))?;
            let center_table: LuaTable = table
                .get("center")
                .map_err(|e| format!("ground_csm missing center: {}", e))?;
            let center = parse_vec3(&center_table)
                .map_err(|e| format!("Failed to parse ground center: {}", e))?;

            Ok(GroundConfig::CsmFile {
                path,
                size_shift,
                center,
            })
        }
        _ => Err(format!("Unknown ground type: {}", type_str)),
    }
}

fn parse_object_config(table: &LuaTable) -> Result<ObjectConfig, String> {
    let type_str: String = table
        .get("type")
        .map_err(|e| format!("object missing type field: {}", e))?;

    if type_str != "object" {
        return Err(format!("Expected object type, got {}", type_str));
    }

    let position_table: LuaTable = table
        .get("position")
        .map_err(|e| format!("object missing position: {}", e))?;
    let rotation_table: LuaTable = table
        .get("rotation")
        .map_err(|e| format!("object missing rotation: {}", e))?;
    let size_table: LuaTable = table
        .get("size")
        .map_err(|e| format!("object missing size: {}", e))?;
    let mass: f32 = table
        .get("mass")
        .map_err(|e| format!("object missing mass: {}", e))?;
    let material: u8 = table
        .get("material")
        .map_err(|e| format!("object missing material: {}", e))?;

    let position = parse_vec3(&position_table)
        .map_err(|e| format!("Failed to parse object position: {}", e))?;
    let rotation = parse_quat(&rotation_table)
        .map_err(|e| format!("Failed to parse object rotation: {}", e))?;
    let size =
        parse_vec3(&size_table).map_err(|e| format!("Failed to parse object size: {}", e))?;

    Ok(ObjectConfig {
        position,
        rotation,
        size,
        mass,
        material,
    })
}

fn parse_objects_config(table: &LuaTable) -> Result<Vec<ObjectConfig>, String> {
    let mut objects = Vec::new();

    // Lua tables are 1-indexed
    let mut index = 1;
    while let Ok(obj_table) = table.get::<LuaTable>(index) {
        objects.push(parse_object_config(&obj_table)?);
        index += 1;
    }

    Ok(objects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_config() {
        let mut config = TestbedConfig::new().unwrap();
        config
            .load_string(
                r#"
                test_camera = camera(
                    vec3(0, 6, -3),
                    vec3(0, 0, 4)
                )
            "#,
            )
            .unwrap();

        let camera = config.extract_camera("test_camera").unwrap();
        assert_eq!(camera.position.x, 0.0);
        assert_eq!(camera.position.y, 6.0);
        assert_eq!(camera.position.z, -3.0);
        assert_eq!(camera.look_at.x, 0.0);
        assert_eq!(camera.look_at.y, 0.0);
        assert_eq!(camera.look_at.z, 4.0);
    }

    #[test]
    fn test_ground_cube_config() {
        let mut config = TestbedConfig::new().unwrap();
        config
            .load_string("test_ground = ground_cube(32, 3, vec3(0, -4, 0))")
            .unwrap();

        let ground = config.extract_ground("test_ground").unwrap();
        match ground {
            GroundConfig::SolidCube {
                material,
                size_shift,
                center,
            } => {
                assert_eq!(material, 32);
                assert_eq!(size_shift, 3);
                assert_eq!(center.y, -4.0);
            }
            _ => panic!("Expected SolidCube"),
        }
    }

    #[test]
    fn test_ground_csm_config() {
        let mut config = TestbedConfig::new().unwrap();
        config
            .load_string(r#"test_ground = ground_csm("terrain.csm", 3, vec3(0, -4, 0))"#)
            .unwrap();

        let ground = config.extract_ground("test_ground").unwrap();
        match ground {
            GroundConfig::CsmFile {
                path,
                size_shift,
                center,
            } => {
                assert_eq!(path, "terrain.csm");
                assert_eq!(size_shift, 3);
                assert_eq!(center.y, -4.0);
            }
            _ => panic!("Expected CsmFile"),
        }
    }

    #[test]
    fn test_object_config() {
        let mut config = TestbedConfig::new().unwrap();
        config
            .load_string(
                r#"
                test_object = object(
                    vec3(1, 2, 3),
                    quat_identity(),
                    vec3(0.5, 0.5, 0.5),
                    1.0,
                    32
                )
            "#,
            )
            .unwrap();

        let objects_table: LuaTable = config.config.extract_value("test_object").unwrap();
        let obj = parse_object_config(&objects_table).unwrap();
        assert_eq!(obj.position.x, 1.0);
        assert_eq!(obj.mass, 1.0);
        assert_eq!(obj.material, 32);
    }

    #[test]
    fn test_objects_list() {
        let mut config = TestbedConfig::new().unwrap();
        config
            .load_string(
                r#"
                test_objects = {
                    object(vec3(0, 0, 0), quat_identity(), vec3(1, 1, 1), 1.0, 32),
                    object(vec3(1, 0, 0), quat_identity(), vec3(1, 1, 1), 2.0, 33)
                }
            "#,
            )
            .unwrap();

        let objects = config.extract_objects("test_objects").unwrap();
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].mass, 1.0);
        assert_eq!(objects[1].mass, 2.0);
    }

    #[test]
    fn test_rand_seed_and_01() {
        let mut config = TestbedConfig::new().unwrap();
        config
            .load_string(
                r#"
                rand_seed(42)
                val1 = rand_01()
                val2 = rand_01()
                val3 = rand_01()
            "#,
            )
            .unwrap();

        let val1: f64 = config.config.extract_value("val1").unwrap();
        let val2: f64 = config.config.extract_value("val2").unwrap();
        let val3: f64 = config.config.extract_value("val3").unwrap();

        // Should produce deterministic sequence between 0 and 1
        assert!(val1 >= 0.0 && val1 < 1.0);
        assert!(val2 >= 0.0 && val2 < 1.0);
        assert!(val3 >= 0.0 && val3 < 1.0);
        assert_ne!(val1, val2);
        assert_ne!(val2, val3);
    }

    #[test]
    fn test_rand_range() {
        let mut config = TestbedConfig::new().unwrap();
        config
            .load_string(
                r#"
                rand_seed(42)
                val1 = rand_range(-5.0, 5.0)
                val2 = rand_range(-5.0, 5.0)
                val3 = rand_range(-5.0, 5.0)
            "#,
            )
            .unwrap();

        let val1: f64 = config.config.extract_value("val1").unwrap();
        let val2: f64 = config.config.extract_value("val2").unwrap();
        let val3: f64 = config.config.extract_value("val3").unwrap();

        // All values should be in range
        assert!(val1 >= -5.0 && val1 < 5.0);
        assert!(val2 >= -5.0 && val2 < 5.0);
        assert!(val3 >= -5.0 && val3 < 5.0);
        // Should be different (deterministic but advancing)
        assert_ne!(val1, val2);
        assert_ne!(val2, val3);
    }

    #[test]
    fn test_quat() {
        let mut config = TestbedConfig::new().unwrap();
        config
            .load_string("test_quat = quat(0.1, 0.2, 0.3, 0.4)")
            .unwrap();

        let table: LuaTable = config.config.extract_value("test_quat").unwrap();
        let quat = parse_quat(&table).unwrap();
        assert!((quat.x - 0.1).abs() < 0.001);
        assert!((quat.y - 0.2).abs() < 0.001);
        assert!((quat.z - 0.3).abs() < 0.001);
        assert!((quat.w - 0.4).abs() < 0.001);
    }
}
