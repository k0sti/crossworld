//! Logic crate - Rule-based transformation system for Crossworld
//!
//! This crate provides a declarative rule system for voxel operations,
//! enabling pattern-based transformations on octree structures.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Rule Engine                           │
//! ├─────────────────────────────────────────────────────────┤
//! │  Rules                                                   │
//! │  ├── Condition patterns (match voxel state)             │
//! │  ├── Priority ordering                                  │
//! │  └── Action dispatch                                    │
//! ├─────────────────────────────────────────────────────────┤
//! │  Actions                                                 │
//! │  ├── SetVoxel - Set material at position                │
//! │  ├── ClearVoxel - Remove voxel at position              │
//! │  ├── Transform - Apply transformation to region         │
//! │  └── Spawn - Create entity at position                  │
//! ├─────────────────────────────────────────────────────────┤
//! │  Transactions                                            │
//! │  ├── Batch multiple actions                             │
//! │  ├── Atomic commit/rollback                             │
//! │  └── Change tracking                                    │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust
//! use logic::{Rule, Condition, Action, RuleEngine};
//! use glam::IVec3;
//!
//! // Create a rule that turns grass to dirt when covered
//! let rule = Rule::new("grass_decay")
//!     .when(Condition::MaterialAt {
//!         offset: IVec3::ZERO,
//!         material: 2, // GRASS
//!     })
//!     .when(Condition::MaterialAt {
//!         offset: IVec3::Y,
//!         material: 1, // Not AIR (any solid above)
//!     })
//!     .then(Action::SetVoxel {
//!         offset: IVec3::ZERO,
//!         material: 3, // DIRT
//!     });
//!
//! // Create rule engine and add rule
//! let mut engine = RuleEngine::new();
//! engine.add_rule(rule);
//! ```

mod action;
mod condition;
mod engine;
mod error;
mod rule;
mod transaction;

#[cfg(feature = "cube")]
mod cube_adapter;

pub use action::Action;
pub use condition::Condition;
pub use engine::{RuleContext, RuleEngine, RuleExecutor};
pub use error::{Error, Result};
pub use rule::{Rule, RuleId};
pub use transaction::{RuleTx, TxChange};

#[cfg(feature = "cube")]
pub use cube_adapter::{CubeAdapter, CubeBuilder};

// Re-export glam for convenience
pub use glam;
