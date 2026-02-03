//! Geospatial services for Crossworld
//!
//! This crate provides geospatial coordinate handling, height map queries,
//! satellite/terrain imagery, and OpenStreetMap data integration.
//!
//! # Modules
//!
//! - [`coords`]: Coordinate types for geographic and projected positions
//! - [`area`]: Area definitions for querying regions of the world
//! - [`height`]: Height map queries and terrain elevation data
//! - [`imagery`]: Satellite and terrain imagery fetching
//! - [`osm`]: OpenStreetMap data integration (planned)

pub mod area;
pub mod coords;
pub mod height;
pub mod imagery;
pub mod osm;

pub use area::Area;
pub use coords::{GeoCoord, WorldCoord};
pub use height::{HeightMap, HeightProvider};
pub use imagery::{ImageProvider, TileImage};
