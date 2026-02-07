//! Scripting and configuration system for Crossworld
//!
//! This crate provides:
//! - **StateTree**: A hierarchical state tree with named paths based on KDL structure
//! - **KDL Reader**: Parse KDL configuration files into StateTree
//! - **Lua Engine**: Lua VM wrapper with Crossworld bindings
//!
//! # Example
//!
//! ```rust,ignore
//! use scripting::{StateTree, KdlReader, LuaEngine};
//!
//! // Load configuration from KDL
//! let tree = KdlReader::from_file("config/app.kdl")?;
//!
//! // Access values by path
//! let macro_depth = tree.get_u32("app.scene.world.macro_depth")?;
//!
//! // Lua scripts can read/write the tree
//! let mut lua = LuaEngine::new()?;
//! lua.set_state_tree(tree);
//! lua.exec_file("scripts/init.lua")?;
//! ```

mod error;
mod kdl_reader;
mod lua_engine;
mod state_tree;
mod value;

pub use error::{Error, Result};
pub use kdl_reader::KdlReader;
pub use lua_engine::{
    extract_f32, extract_u32, extract_u8, parse_quat, parse_vec3, LuaEngine, Script, ScriptContext,
};
pub use state_tree::{SourceLocation, StateNode, StateTree};
pub use value::Value;

// Re-export mlua for downstream crates
pub use mlua;
