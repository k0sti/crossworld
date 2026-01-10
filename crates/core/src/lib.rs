//! Core library for Crossworld - generic components that can be compiled to WASM
//!
//! This crate contains common code that is shared across different applications
//! and can be compiled both natively and to WebAssembly.
//!
//! # Modules
//!
//! - [`camera`]: 3D camera system with orbit and first-person controllers
//! - [`input`]: Input types for controllers, mouse, and cursor handling

pub mod camera;
pub mod input;
