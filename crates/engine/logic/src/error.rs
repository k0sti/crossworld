//! Error types for the logic crate

use thiserror::Error;

/// Result type alias for logic operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the rule engine
#[derive(Debug, Error)]
pub enum Error {
    /// Rule not found by ID
    #[error("Rule not found: {0}")]
    RuleNotFound(String),

    /// Duplicate rule ID
    #[error("Duplicate rule ID: {0}")]
    DuplicateRule(String),

    /// Invalid condition configuration
    #[error("Invalid condition: {0}")]
    InvalidCondition(String),

    /// Invalid action configuration
    #[error("Invalid action: {0}")]
    InvalidAction(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Circular rule dependency detected
    #[error("Circular rule dependency detected: {0}")]
    CircularDependency(String),
}
