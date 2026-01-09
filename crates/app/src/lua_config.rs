//! Base Lua configuration module for application scene setup
//!
//! Provides Lua 5.4 scripting support with base types like vectors
//! and quaternions. Application-specific types (scenes, physics, etc.)
//! should be registered by the application.
//!
//! # Example Lua Configuration
//!
//! ```lua
//! -- Create vectors and quaternions
//! pos = vec3(0, 6, -3)
//! rot = quat_euler(0.1, 0.2, 0.3)
//! ```

use glam::{Quat, Vec3};
use std::path::Path;

// Re-export mlua crate for use by dependent crates
pub use mlua;
use mlua::prelude::*;

/// Configuration for a 3D vector
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec3Config {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3Config {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

impl From<Vec3Config> for Vec3 {
    fn from(config: Vec3Config) -> Self {
        config.to_vec3()
    }
}

/// Configuration for a quaternion rotation
#[derive(Debug, Clone, Copy)]
pub struct QuatConfig {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for QuatConfig {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
}

impl QuatConfig {
    pub fn from_euler(x: f32, y: f32, z: f32) -> Self {
        let q = Quat::from_euler(glam::EulerRot::XYZ, x, y, z);
        Self {
            x: q.x,
            y: q.y,
            z: q.z,
            w: q.w,
        }
    }

    pub fn to_quat(&self) -> Quat {
        Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}

impl From<QuatConfig> for Quat {
    fn from(config: QuatConfig) -> Self {
        config.to_quat()
    }
}

/// Lua configuration engine wrapper
///
/// Provides base functionality for Lua scripting with common types.
/// Applications can extend this by registering additional functions.
pub struct LuaConfig {
    lua: Lua,
}

/// Convert a Lua value to f64, handling both integers and numbers
pub fn lua_val_to_f64(val: &LuaValue) -> LuaResult<f64> {
    match val {
        LuaValue::Number(n) => Ok(*n),
        LuaValue::Integer(i) => Ok(*i as f64),
        _ => Err(LuaError::FromLuaConversionError {
            from: val.type_name(),
            to: "f64".to_string(),
            message: Some("expected number or integer".to_string()),
        }),
    }
}

/// Parse a Lua table as Vec3Config (expects 3 elements: x, y, z)
pub fn parse_vec3(table: &LuaTable) -> LuaResult<Vec3Config> {
    // Lua tables are 1-indexed
    let x = extract_f32(table, 1)?;
    let y = extract_f32(table, 2)?;
    let z = extract_f32(table, 3)?;
    Ok(Vec3Config::new(x, y, z))
}

/// Parse a Lua table as QuatConfig (expects 4 elements: x, y, z, w)
pub fn parse_quat(table: &LuaTable) -> LuaResult<QuatConfig> {
    // Lua tables are 1-indexed
    let x = extract_f32(table, 1)?;
    let y = extract_f32(table, 2)?;
    let z = extract_f32(table, 3)?;
    let w = extract_f32(table, 4)?;
    Ok(QuatConfig { x, y, z, w })
}

/// Extract f32 from a Lua table at given index (1-indexed)
pub fn extract_f32(table: &LuaTable, index: i32) -> LuaResult<f32> {
    let val: LuaValue = table.get(index)?;
    match val {
        LuaValue::Number(n) => Ok(n as f32),
        LuaValue::Integer(i) => Ok(i as f32),
        _ => Err(LuaError::FromLuaConversionError {
            from: val.type_name(),
            to: "f32".to_string(),
            message: Some(format!("expected number at index {}", index)),
        }),
    }
}

/// Extract u8 from a Lua value
pub fn extract_u8(val: &LuaValue) -> LuaResult<u8> {
    match val {
        LuaValue::Integer(i) => {
            if (0..=255).contains(i) {
                Ok(*i as u8)
            } else {
                Err(LuaError::FromLuaConversionError {
                    from: "integer",
                    to: "u8".to_string(),
                    message: Some(format!("value {} out of u8 range", i)),
                })
            }
        }
        LuaValue::Number(n) => {
            let i = *n as i64;
            if (0..=255).contains(&i) {
                Ok(i as u8)
            } else {
                Err(LuaError::FromLuaConversionError {
                    from: "number",
                    to: "u8".to_string(),
                    message: Some(format!("value {} out of u8 range", n)),
                })
            }
        }
        _ => Err(LuaError::FromLuaConversionError {
            from: val.type_name(),
            to: "u8".to_string(),
            message: Some("expected integer or number".to_string()),
        }),
    }
}

/// Extract u32 from a Lua value
pub fn extract_u32(val: &LuaValue) -> LuaResult<u32> {
    match val {
        LuaValue::Integer(i) => {
            if *i >= 0 {
                Ok(*i as u32)
            } else {
                Err(LuaError::FromLuaConversionError {
                    from: "integer",
                    to: "u32".to_string(),
                    message: Some(format!("value {} cannot be negative", i)),
                })
            }
        }
        LuaValue::Number(n) => {
            let i = *n as i64;
            if i >= 0 {
                Ok(i as u32)
            } else {
                Err(LuaError::FromLuaConversionError {
                    from: "number",
                    to: "u32".to_string(),
                    message: Some(format!("value {} cannot be negative", n)),
                })
            }
        }
        _ => Err(LuaError::FromLuaConversionError {
            from: val.type_name(),
            to: "u32".to_string(),
            message: Some("expected integer or number".to_string()),
        }),
    }
}

impl LuaConfig {
    /// Create a new Lua configuration engine with base types registered
    ///
    /// Registers:
    /// - `vec3` - Create 3D vectors
    /// - `quat_euler` - Create quaternions from Euler angles
    /// - `quat_identity` - Identity quaternion
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();

        // Register vec3 constructor - accepts any numeric type
        let vec3_fn = lua.create_function(|_, (x, y, z): (LuaValue, LuaValue, LuaValue)| {
            let x_num = lua_val_to_f64(&x)?;
            let y_num = lua_val_to_f64(&y)?;
            let z_num = lua_val_to_f64(&z)?;
            Ok(vec![x_num, y_num, z_num])
        })?;
        lua.globals().set("vec3", vec3_fn)?;

        // Register quat_euler constructor (from euler angles in radians)
        let quat_euler_fn =
            lua.create_function(|_, (x, y, z): (LuaValue, LuaValue, LuaValue)| {
                let x_f = lua_val_to_f64(&x)? as f32;
                let y_f = lua_val_to_f64(&y)? as f32;
                let z_f = lua_val_to_f64(&z)? as f32;
                let q = Quat::from_euler(glam::EulerRot::XYZ, x_f, y_f, z_f);
                Ok(vec![q.x as f64, q.y as f64, q.z as f64, q.w as f64])
            })?;
        lua.globals().set("quat_euler", quat_euler_fn)?;

        // Register quat_identity
        let quat_identity_fn = lua.create_function(|_, ()| Ok(vec![0.0, 0.0, 0.0, 1.0]))?;
        lua.globals().set("quat_identity", quat_identity_fn)?;

        Ok(Self { lua })
    }

    /// Get mutable access to the underlying Lua state
    ///
    /// Use this to register additional functions for your application.
    pub fn lua_mut(&mut self) -> &mut Lua {
        &mut self.lua
    }

    /// Get immutable access to the underlying Lua state
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Load and evaluate a Lua configuration file
    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {:?}: {}", path, e))?;

        self.lua
            .load(&content)
            .exec()
            .map_err(|e| format!("Failed to evaluate config: {}", e))?;

        Ok(())
    }

    /// Load configuration from a string
    pub fn load_string(&mut self, content: &str) -> Result<(), String> {
        self.lua
            .load(content)
            .exec()
            .map_err(|e| format!("Failed to evaluate config: {}", e))?;
        Ok(())
    }

    /// Extract a value by name from the Lua global table
    pub fn extract_value<T: FromLua>(&self, name: &str) -> Result<T, String> {
        self.lua
            .globals()
            .get::<T>(name)
            .map_err(|e| format!("Failed to extract '{}': {}", name, e))
    }
}

impl Default for LuaConfig {
    fn default() -> Self {
        Self::new().expect("Failed to create default LuaConfig")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_creation() {
        let mut config = LuaConfig::new().unwrap();
        config.load_string("test_vec = vec3(1.0, 2.0, 3.0)").unwrap();

        let table: LuaTable = config.extract_value("test_vec").unwrap();
        let vec = parse_vec3(&table).unwrap();
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 2.0);
        assert_eq!(vec.z, 3.0);
    }

    #[test]
    fn test_vec3_with_integers() {
        let mut config = LuaConfig::new().unwrap();
        config.load_string("test_vec = vec3(1, 2, 3)").unwrap();

        let table: LuaTable = config.extract_value("test_vec").unwrap();
        let vec = parse_vec3(&table).unwrap();
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 2.0);
        assert_eq!(vec.z, 3.0);
    }

    #[test]
    fn test_quat_euler() {
        let mut config = LuaConfig::new().unwrap();
        config
            .load_string("test_quat = quat_euler(0.0, 0.0, 0.0)")
            .unwrap();

        let table: LuaTable = config.extract_value("test_quat").unwrap();
        let quat = parse_quat(&table).unwrap();
        // Identity quaternion
        assert!((quat.w - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_quat_identity() {
        let mut config = LuaConfig::new().unwrap();
        config.load_string("test_quat = quat_identity()").unwrap();

        let table: LuaTable = config.extract_value("test_quat").unwrap();
        let quat = parse_quat(&table).unwrap();
        assert_eq!(quat.x, 0.0);
        assert_eq!(quat.y, 0.0);
        assert_eq!(quat.z, 0.0);
        assert_eq!(quat.w, 1.0);
    }

    #[test]
    fn test_vec3_to_glam() {
        let vec_config = Vec3Config::new(1.0, 2.0, 3.0);
        let vec: Vec3 = vec_config.into();
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 2.0);
        assert_eq!(vec.z, 3.0);
    }

    #[test]
    fn test_quat_to_glam() {
        let quat_config = QuatConfig::default();
        let quat: Quat = quat_config.into();
        assert_eq!(quat.x, 0.0);
        assert_eq!(quat.y, 0.0);
        assert_eq!(quat.z, 0.0);
        assert_eq!(quat.w, 1.0);
    }
}
