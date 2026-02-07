//! Lua scripting engine with StateTree integration
//!
//! Provides a Lua VM wrapper with:
//! - Built-in math types (vec3, quat)
//! - StateTree read/write access
//! - Script loading and hot-reload support

use crate::{Error, Result, StateTree, Value};
use glam::{Quat, Vec3};
use mlua::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// A loaded Lua script
#[derive(Debug, Clone)]
pub struct Script {
    /// Path to the script file
    pub path: PathBuf,
    /// Compiled bytecode (if available)
    pub bytecode: Option<Vec<u8>>,
    /// Last modification time
    pub modified: Option<SystemTime>,
}

impl Script {
    /// Create a new script from a path
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        Self {
            path,
            bytecode: None,
            modified,
        }
    }

    /// Check if the script file has been modified since it was loaded
    pub fn is_modified(&self) -> bool {
        if let Some(original) = self.modified {
            if let Ok(current) = std::fs::metadata(&self.path).and_then(|m| m.modified()) {
                return current > original;
            }
        }
        false
    }
}

/// Execution context for a script
#[derive(Debug, Clone, Default)]
pub struct ScriptContext {
    /// Optional entity ID this script is attached to
    pub entity_id: Option<u64>,
    /// Additional context data
    pub data: std::collections::HashMap<String, Value>,
}

impl ScriptContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with an entity ID
    pub fn with_entity(entity_id: u64) -> Self {
        Self {
            entity_id: Some(entity_id),
            ..Default::default()
        }
    }
}

/// Lua scripting engine
pub struct LuaEngine {
    lua: Lua,
    state_tree: Option<Arc<RwLock<StateTree>>>,
}

impl LuaEngine {
    /// Create a new Lua engine with base types registered
    pub fn new() -> Result<Self> {
        let lua = Lua::new();

        // Register vec3 constructor
        let vec3_fn = lua.create_function(|_, (x, y, z): (LuaValue, LuaValue, LuaValue)| {
            let x = lua_value_to_f64(&x)?;
            let y = lua_value_to_f64(&y)?;
            let z = lua_value_to_f64(&z)?;
            Ok(vec![x, y, z])
        })?;
        lua.globals().set("vec3", vec3_fn)?;

        // Register quat_euler constructor (from euler angles in radians)
        let quat_euler_fn =
            lua.create_function(|_, (x, y, z): (LuaValue, LuaValue, LuaValue)| {
                let x = lua_value_to_f64(&x)? as f32;
                let y = lua_value_to_f64(&y)? as f32;
                let z = lua_value_to_f64(&z)? as f32;
                let q = Quat::from_euler(glam::EulerRot::XYZ, x, y, z);
                Ok(vec![q.x as f64, q.y as f64, q.z as f64, q.w as f64])
            })?;
        lua.globals().set("quat_euler", quat_euler_fn)?;

        // Register quat_identity
        let quat_identity_fn = lua.create_function(|_, ()| Ok(vec![0.0f64, 0.0, 0.0, 1.0]))?;
        lua.globals().set("quat_identity", quat_identity_fn)?;

        Ok(Self {
            lua,
            state_tree: None,
        })
    }

    /// Set the state tree for this engine
    pub fn set_state_tree(&mut self, tree: Arc<RwLock<StateTree>>) -> Result<()> {
        let tree_ref = tree.clone();

        // Create state_get function
        let tree_for_get = tree_ref.clone();
        let state_get = self.lua.create_function(move |_, path: String| {
            let tree = tree_for_get
                .read()
                .map_err(|e| LuaError::RuntimeError(format!("Failed to lock state tree: {}", e)))?;

            match tree.get(&path) {
                Ok(value) => value_to_lua_value(value),
                Err(_) => Ok(LuaValue::Nil),
            }
        })?;
        self.lua.globals().set("state_get", state_get)?;

        // Create state_set function
        let tree_for_set = tree_ref.clone();
        let state_set = self
            .lua
            .create_function(move |_, (path, value): (String, LuaValue)| {
                let mut tree = tree_for_set.write().map_err(|e| {
                    LuaError::RuntimeError(format!("Failed to lock state tree: {}", e))
                })?;

                let val = lua_value_to_value(&value)?;
                tree.set(&path, val)
                    .map_err(|e| LuaError::RuntimeError(format!("Failed to set state: {}", e)))?;
                Ok(())
            })?;
        self.lua.globals().set("state_set", state_set)?;

        self.state_tree = Some(tree);
        Ok(())
    }

    /// Get the underlying Lua state
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Get mutable access to the underlying Lua state
    pub fn lua_mut(&mut self) -> &mut Lua {
        &mut self.lua
    }

    /// Load and execute a Lua file
    pub fn exec_file(&self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.exec_string(&content)
    }

    /// Execute a Lua string
    pub fn exec_string(&self, code: &str) -> Result<()> {
        self.lua.load(code).exec()?;
        Ok(())
    }

    /// Call a Lua function by name
    pub fn call_function<A, R>(&self, name: &str, args: A) -> Result<R>
    where
        A: IntoLuaMulti,
        R: FromLuaMulti,
    {
        let func: LuaFunction = self.lua.globals().get(name)?;
        let result = func.call(args)?;
        Ok(result)
    }

    /// Get a global value from Lua
    pub fn get_global<T: FromLua>(&self, name: &str) -> Result<T> {
        let value = self.lua.globals().get(name)?;
        Ok(value)
    }

    /// Set a global value in Lua
    pub fn set_global<T: IntoLua>(&self, name: &str, value: T) -> Result<()> {
        self.lua.globals().set(name, value)?;
        Ok(())
    }

    /// Load a script file (for hot-reload tracking)
    pub fn load_script(&self, path: &Path) -> Result<Script> {
        let content = std::fs::read_to_string(path)
            .map_err(|_| Error::ScriptNotFound(path.display().to_string()))?;

        self.lua.load(&content).exec()?;

        Ok(Script::new(path))
    }

    /// Reload a script if it has been modified
    pub fn reload_if_modified(&self, script: &mut Script) -> Result<bool> {
        if script.is_modified() {
            let content = std::fs::read_to_string(&script.path)?;
            self.lua.load(&content).exec()?;

            // Update modification time
            script.modified = std::fs::metadata(&script.path)
                .ok()
                .and_then(|m| m.modified().ok());

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Default for LuaEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default LuaEngine")
    }
}

// Helper functions for Lua value conversion

/// Convert a Lua value to f64
fn lua_value_to_f64(val: &LuaValue) -> LuaResult<f64> {
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

/// Convert a Lua value to our Value type
fn lua_value_to_value(val: &LuaValue) -> LuaResult<Value> {
    match val {
        LuaValue::Nil => Ok(Value::Null),
        LuaValue::Boolean(b) => Ok(Value::Bool(*b)),
        LuaValue::Integer(i) => Ok(Value::Int(*i)),
        LuaValue::Number(n) => Ok(Value::Float(*n)),
        LuaValue::String(s) => Ok(Value::String(s.to_str()?.to_string())),
        LuaValue::Table(t) => {
            // Check if it's an array (sequential integer keys starting from 1)
            let len = t.raw_len();
            if len > 0 {
                let mut arr = Vec::with_capacity(len);
                for i in 1..=len {
                    let v: LuaValue = t.raw_get(i)?;
                    arr.push(lua_value_to_value(&v)?);
                }
                Ok(Value::Array(arr))
            } else {
                // It's a map
                let mut map = std::collections::HashMap::new();
                for pair in t.pairs::<String, LuaValue>() {
                    let (k, v) = pair?;
                    map.insert(k, lua_value_to_value(&v)?);
                }
                Ok(Value::Map(map))
            }
        }
        _ => Err(LuaError::FromLuaConversionError {
            from: val.type_name(),
            to: "Value".to_string(),
            message: Some("unsupported Lua type".to_string()),
        }),
    }
}

/// Convert our Value type to a Lua value
/// Note: String and complex types return Nil because we don't have access to the Lua state
/// For full support, use the Lua state directly
fn value_to_lua_value(val: &Value) -> LuaResult<LuaValue> {
    match val {
        Value::Null => Ok(LuaValue::Nil),
        Value::Bool(b) => Ok(LuaValue::Boolean(*b)),
        Value::Int(i) => Ok(LuaValue::Integer(*i)),
        Value::Float(f) => Ok(LuaValue::Number(*f)),
        // String and complex types need Lua state to create, return nil
        _ => Ok(LuaValue::Nil),
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

/// Parse a Lua table as Vec3 (expects 3 elements)
pub fn parse_vec3(table: &LuaTable) -> LuaResult<Vec3> {
    let x = extract_f32(table, 1)?;
    let y = extract_f32(table, 2)?;
    let z = extract_f32(table, 3)?;
    Ok(Vec3::new(x, y, z))
}

/// Parse a Lua table as Quat (expects 4 elements)
pub fn parse_quat(table: &LuaTable) -> LuaResult<Quat> {
    let x = extract_f32(table, 1)?;
    let y = extract_f32(table, 2)?;
    let z = extract_f32(table, 3)?;
    let w = extract_f32(table, 4)?;
    Ok(Quat::from_xyzw(x, y, z, w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_engine_basic() {
        let engine = LuaEngine::new().unwrap();
        engine.exec_string("x = 42").unwrap();

        let x: i64 = engine.get_global("x").unwrap();
        assert_eq!(x, 42);
    }

    #[test]
    fn test_vec3_in_lua() {
        let engine = LuaEngine::new().unwrap();
        engine.exec_string("v = vec3(1, 2, 3)").unwrap();

        let v: LuaTable = engine.get_global("v").unwrap();
        let vec = parse_vec3(&v).unwrap();
        assert_eq!(vec, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_quat_euler_in_lua() {
        let engine = LuaEngine::new().unwrap();
        engine.exec_string("q = quat_identity()").unwrap();

        let q: LuaTable = engine.get_global("q").unwrap();
        let quat = parse_quat(&q).unwrap();
        assert!((quat.w - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_state_tree_integration() {
        let mut engine = LuaEngine::new().unwrap();
        let tree = Arc::new(RwLock::new(StateTree::new()));

        // Set initial value
        tree.write()
            .unwrap()
            .set("app.value", Value::from(10))
            .unwrap();

        engine.set_state_tree(tree.clone()).unwrap();

        // Read from Lua
        engine
            .exec_string(
                r#"
                local val = state_get("app.value")
                state_set("app.doubled", val * 2)
            "#,
            )
            .unwrap();

        // Verify the change
        let doubled = tree.read().unwrap().get_i64("app.doubled").unwrap();
        assert_eq!(doubled, 20);
    }
}
