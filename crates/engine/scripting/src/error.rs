//! Error types for the scripting system

use thiserror::Error;

/// Result type for scripting operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the scripting system
#[derive(Error, Debug)]
pub enum Error {
    /// KDL parsing error
    #[error("KDL parse error: {0}")]
    KdlParse(#[from] kdl::KdlError),

    /// Lua error
    #[error("Lua error: {0}")]
    Lua(#[from] mlua::Error),

    /// File I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Path not found in state tree
    #[error("Path not found: {0}")]
    PathNotFound(String),

    /// Type conversion error
    #[error("Type error: expected {expected}, got {actual}")]
    TypeError { expected: String, actual: String },

    /// Invalid value
    #[error("Invalid value: {0}")]
    InvalidValue(String),

    /// Script file not found
    #[error("Script not found: {0}")]
    ScriptNotFound(String),
}
