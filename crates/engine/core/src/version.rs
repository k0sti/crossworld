//! Crossworld product version.
//!
//! All workspace crates share the version in the root `VERSION` file /
//! `[workspace.package]` section of `Cargo.toml`.

/// Product version string (SEMVER), synced across the workspace.
pub const CROSSWORLD_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// WASM binding for the product version string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = crossworldVersion)]
pub fn crossworld_version() -> String {
    CROSSWORLD_VERSION.to_string()
}
