//! Re-export message types from the network crate.
//!
//! This module provides backwards compatibility for existing code that
//! imports from `crate::messages`. New code should import directly from
//! `crossworld_network`.

// Allow unused imports since these are re-exports for external API consumers
#[allow(unused_imports)]
pub use crossworld_network::{
    AnimationState, CompactPosition, PlayerIdentity, PlayerState, ReliableMessage,
    UnreliableMessage,
};
