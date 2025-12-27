# File Organization Guide: Trellis2 Model Generation (Rust-Only Architecture)

## Overview

This document provides the file organization for the Trellis2 AI model generation feature using a **Rust-only architecture**:
- **Python backend**: Trellis2 inference and O-Voxel extraction (`services/trellis-server/`)
- **Rust library + viewer**: O-Voxel parsing, conversion, and desktop viewer (`crates/trellis/`)

**No TypeScript/Web UI** - This is a native desktop tool for working with Trellis2-generated models.

## Architecture

```
┌─────────────────────────────────────────────┐
│   Python Backend (services/trellis-server)  │
│   ├── Trellis2 Inference (GPU)              │
│   ├── O-Voxel Extraction                    │
│   └── WebSocket/HTTP API                    │
└─────────────────────────────────────────────┘
                    │ WebSocket/HTTP
                    ▼ O-Voxel JSON
┌─────────────────────────────────────────────┐
│   Rust Native App (crates/trellis)          │
│   ├── O-Voxel Parser (lib)                  │
│   ├── O-Voxel → Octree Converter (lib)      │
│   └── Model Viewer (bin)                    │
│       └── Reuses crates/renderer            │
└─────────────────────────────────────────────┘
```

## Directory Structure

### 1. Python Backend (Unchanged)

```
services/trellis-server/
├── api/
│   ├── __init__.py
│   └── routes.py                   # FastAPI WebSocket & HTTP endpoints
│
├── worker/
│   ├── __init__.py
│   └── inference.py                # Trellis2 model loading & inference
│
├── utils/
│   ├── __init__.py
│   ├── image_processing.py         # Image validation, preprocessing
│   └── ovoxel_extraction.py        # Extract O-Voxel from Trellis2 mesh
│
├── requirements.txt                # Python dependencies
├── Dockerfile                      # Container definition
├── docker-compose.yml              # Local dev environment
└── README.md                       # Deployment guide
```

**Responsibilities**:
- Run Trellis2 inference on GPU
- Extract O-Voxel sparse voxel data from generated mesh
- Serve O-Voxel JSON via WebSocket or HTTP endpoint
- No conversion logic (handled by Rust)

---

### 2. Rust Trellis Crate (New: `crates/trellis/`)

```
crates/trellis/
├── Cargo.toml                      # Dependencies and build config
├── README.md                       # Usage guide
│
├── src/
│   ├── lib.rs                      # Library exports
│   │
│   ├── ovoxel.rs                   # O-Voxel data structures
│   ├── ovoxel_parser.rs            # JSON parsing and validation
│   ├── ovoxel_converter.rs         # O-Voxel → Octree conversion
│   ├── sparse_voxel.rs             # Sparse voxel utilities
│   ├── color_quantizer.rs          # Palette generation (median cut)
│   │
│   ├── client.rs                   # HTTP/WebSocket client for Python backend
│   ├── cache.rs                    # Local file cache for O-Voxel results
│   │
│   ├── main.rs                     # Binary entry point
│   └── viewer/                     # Viewer application modules
│       ├── mod.rs
│       ├── app.rs                  # Main viewer app (egui)
│       ├── ui.rs                   # UI components
│       └── renderer_integration.rs # Integration with crates/renderer
│
├── tests/
│   ├── ovoxel_parsing.rs           # O-Voxel JSON parsing tests
│   ├── ovoxel_conversion.rs        # Conversion integration tests
│   └── test_data/                  # Sample O-Voxel JSON files
│       ├── cube_5k.json
│       ├── sphere_10k.json
│       └── complex_50k.json
│
├── benches/
│   └── conversion.rs               # Performance benchmarks
│
└── examples/
    ├── convert.rs                  # Example: Convert O-Voxel JSON to CSM
    └── fetch_and_convert.rs        # Example: Fetch from server + convert
```

---

## Cargo.toml Configuration

```toml
[package]
name = "trellis"
version = "0.1.0"
edition = "2024"

[lib]
name = "trellis"
path = "src/lib.rs"

[[bin]]
name = "trellis"
path = "src/main.rs"

[dependencies]
# Core
glam = { version = "0.29", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP/WebSocket client
reqwest = { version = "0.12", features = ["json", "blocking"] }
tokio = { version = "1.0", features = ["full"] }
tokio-tungstenite = "0.23"  # WebSocket

# Color quantization (optional)
kiddo = "4.0"  # k-d tree for nearest color lookup

# UI/Rendering (bin only)
egui = "0.29"
egui_glow = "0.29"
egui-winit = "0.29"
glow = "0.14"
glutin = "0.32"
glutin-winit = "0.5"
winit = "0.30"
raw-window-handle = "0.6"
image = "0.25"

# Local crates
cube = { path = "../cube" }
renderer = { path = "../renderer" }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
criterion = "0.5"  # Benchmarks
```

---

## Module Structure

### Library (`src/lib.rs`)

```rust
//! Trellis2 model generation integration
//!
//! This crate provides:
//! - O-Voxel data structure parsing and validation
//! - O-Voxel to Crossworld octree conversion
//! - HTTP/WebSocket client for Trellis2 backend
//! - Model viewer (binary)

mod ovoxel;
mod ovoxel_parser;
mod ovoxel_converter;
mod sparse_voxel;
mod color_quantizer;
mod client;
mod cache;

// Public exports
pub use ovoxel::OVoxel;
pub use ovoxel_parser::parse_ovoxel_json;
pub use ovoxel_converter::convert_ovoxel_to_octree;
pub use client::{TrellisClient, GenerationParams};
pub use cache::OVoxelCache;

// Re-export cube types
pub use cube::Cube;
```

### Binary (`src/main.rs`)

```rust
//! Trellis2 Model Viewer
//!
//! A desktop application for:
//! - Fetching models from Trellis2 backend
//! - Converting O-Voxel to octree
//! - Viewing models with OpenGL renderer
//! - Exporting to CSM format

mod viewer;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
#[command(name = "trellis")]
#[command(about = "Trellis2 model viewer and converter")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch interactive viewer
    View {
        /// Trellis2 server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server: String,
    },

    /// Convert O-Voxel JSON file to CSM
    Convert {
        /// Input O-Voxel JSON file
        input: PathBuf,

        /// Output CSM file
        output: PathBuf,

        /// Octree depth (6 = 64³, 7 = 128³)
        #[arg(short, long, default_value = "6")]
        depth: u8,
    },

    /// Generate model from image (fetches from server)
    Generate {
        /// Image file path
        image: PathBuf,

        /// Text prompt (optional)
        #[arg(short, long)]
        prompt: Option<String>,

        /// Output CSM file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Trellis2 server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server: String,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::View { server } => {
            info!("Launching viewer with server: {}", server);
            viewer::run(server)?;
        }

        Commands::Convert { input, output, depth } => {
            info!("Converting {} to {}", input.display(), output.display());
            convert_file(input, output, depth)?;
        }

        Commands::Generate { image, prompt, output, server } => {
            info!("Generating model from {}", image.display());
            generate_model(image, prompt, output, server)?;
        }
    }

    Ok(())
}

fn convert_file(input: PathBuf, output: PathBuf, depth: u8) -> Result<()> {
    // Load O-Voxel JSON
    let json = std::fs::read_to_string(input)?;
    let ovoxel = trellis::parse_ovoxel_json(&json)?;

    // Convert to octree
    let cube = trellis::convert_ovoxel_to_octree(&ovoxel, depth)?;

    // Save as CSM
    let csm = cube.to_csm(); // Assuming this method exists
    std::fs::write(output, csm)?;

    Ok(())
}

fn generate_model(
    image: PathBuf,
    prompt: Option<String>,
    output: Option<PathBuf>,
    server: String
) -> Result<()> {
    use trellis::{TrellisClient, GenerationParams};

    let client = TrellisClient::new(server);

    let params = GenerationParams {
        image_path: image.clone(),
        prompt,
        seed: None,
    };

    // Blocking generation
    let ovoxel = client.generate_blocking(params)?;

    // Convert to octree
    let cube = trellis::convert_ovoxel_to_octree(&ovoxel, 6)?;

    // Save or display
    if let Some(out) = output {
        let csm = cube.to_csm();
        std::fs::write(out, csm)?;
    } else {
        // Launch viewer with generated model
        // ... viewer logic
    }

    Ok(())
}
```

---

## Key Source Files

### 1. O-Voxel Data Structures (`src/ovoxel.rs`)

```rust
use glam::{Vec3, IVec3};
use serde::{Deserialize, Serialize};

/// O-Voxel sparse voxel representation from Trellis2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OVoxel {
    /// Sparse voxel coordinates [N, 3] (integer)
    pub coords: Vec<IVec3>,

    /// RGB color attributes [N, 3] (float [0, 1])
    pub attrs: Vec<[f32; 3]>,

    /// Size of each voxel
    pub voxel_size: f32,

    /// Axis-aligned bounding box [min, max]
    pub aabb: [Vec3; 2],
}

impl OVoxel {
    /// Validate O-Voxel structure
    pub fn validate(&self) -> Result<(), String> {
        if self.coords.len() != self.attrs.len() {
            return Err(format!(
                "coords length ({}) != attrs length ({})",
                self.coords.len(),
                self.attrs.len()
            ));
        }

        if self.aabb[0].x >= self.aabb[1].x
            || self.aabb[0].y >= self.aabb[1].y
            || self.aabb[0].z >= self.aabb[1].z
        {
            return Err("Invalid AABB: min >= max".into());
        }

        if self.voxel_size <= 0.0 {
            return Err("voxel_size must be positive".into());
        }

        Ok(())
    }

    /// Get number of sparse voxels
    pub fn voxel_count(&self) -> usize {
        self.coords.len()
    }
}
```

### 2. O-Voxel Parser (`src/ovoxel_parser.rs`)

```rust
use crate::ovoxel::OVoxel;
use serde_json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Parse O-Voxel JSON string
pub fn parse_ovoxel_json(json: &str) -> Result<OVoxel, ParseError> {
    let ovoxel: OVoxel = serde_json::from_str(json)?;

    ovoxel.validate()
        .map_err(ParseError::Validation)?;

    Ok(ovoxel)
}
```

### 3. O-Voxel Converter (`src/ovoxel_converter.rs`)

```rust
use crate::ovoxel::OVoxel;
use crate::color_quantizer::{Palette, quantize_colors};
use cube::Cube;
use glam::{Vec3, IVec3};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Empty O-Voxel data")]
    EmptyData,

    #[error("Invalid depth: {0} (must be 1-8)")]
    InvalidDepth(u8),
}

/// Convert O-Voxel to Crossworld octree
pub fn convert_ovoxel_to_octree(
    ovoxel: &OVoxel,
    target_depth: u8,
) -> Result<Cube<u8>, ConversionError> {
    if ovoxel.voxel_count() == 0 {
        return Err(ConversionError::EmptyData);
    }

    if target_depth == 0 || target_depth > 8 {
        return Err(ConversionError::InvalidDepth(target_depth));
    }

    // Build color palette
    let colors: Vec<[f32; 3]> = ovoxel.attrs.clone();
    let palette = quantize_colors(&colors, 256);

    // Create octree
    let grid_size = 1 << target_depth;
    let mut octree = Cube::empty(target_depth);

    // Calculate bounds
    let aabb_size = ovoxel.aabb[1] - ovoxel.aabb[0];
    let scale = grid_size as f32 / aabb_size.max_element();

    // Map each sparse voxel to octree
    for (coord, attr) in ovoxel.coords.iter().zip(&ovoxel.attrs) {
        // Normalize to [0, 1]
        let normalized = (coord.as_vec3() - ovoxel.aabb[0]) / aabb_size;

        // Scale to octree space
        let octree_coord = (normalized * grid_size as f32).as_ivec3();

        // Clamp to bounds
        let clamped = octree_coord.clamp(
            IVec3::ZERO,
            IVec3::splat(grid_size - 1),
        );

        // Assign material index
        let material_idx = palette.nearest_index(attr);
        octree.set_voxel(
            clamped.x as u32,
            clamped.y as u32,
            clamped.z as u32,
            material_idx,
        );
    }

    // Optimize octree
    octree.optimize();

    Ok(octree)
}
```

### 4. Trellis Client (`src/client.rs`)

```rust
use crate::ovoxel::OVoxel;
use anyhow::Result;
use reqwest::blocking::Client;
use std::path::PathBuf;
use std::time::Duration;

pub struct TrellisClient {
    server_url: String,
    client: Client,
}

pub struct GenerationParams {
    pub image_path: PathBuf,
    pub prompt: Option<String>,
    pub seed: Option<u64>,
}

impl TrellisClient {
    pub fn new(server_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))  // 5 min timeout
            .build()
            .expect("Failed to create HTTP client");

        Self { server_url, client }
    }

    /// Check server health
    pub fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/health", self.server_url);
        let resp = self.client.get(&url).send()?;
        Ok(resp.status().is_success())
    }

    /// Generate model (blocking)
    pub fn generate_blocking(&self, params: GenerationParams) -> Result<OVoxel> {
        // Read image file
        let image_data = std::fs::read(&params.image_path)?;
        let image_b64 = base64::encode(&image_data);

        // Build request
        let url = format!("{}/api/generate", self.server_url);
        let body = serde_json::json!({
            "image": image_b64,
            "prompt": params.prompt,
            "seed": params.seed,
        });

        // Send request
        let resp = self.client
            .post(&url)
            .json(&body)
            .send()?;

        // Parse response
        let ovoxel_json: serde_json::Value = resp.json()?;
        let ovoxel: OVoxel = serde_json::from_value(ovoxel_json)?;

        Ok(ovoxel)
    }

    // Async version with WebSocket (for viewer with progress)
    // pub async fn generate_async(...) -> Result<OVoxel> { ... }
}
```

### 5. Viewer App (`src/viewer/app.rs`)

```rust
use renderer::{CpuTracer, Renderer};
use egui::Context as EguiContext;
use cube::Cube;

pub struct TrellisViewerApp {
    // Trellis client
    server_url: String,

    // Current model
    current_cube: Option<Cube<u8>>,

    // Renderer (reuse from crates/renderer)
    renderer: Option<Renderer>,

    // UI state
    image_path: String,
    prompt: String,
    generating: bool,
    progress: f32,
}

impl TrellisViewerApp {
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            current_cube: None,
            renderer: None,
            image_path: String::new(),
            prompt: String::new(),
            generating: false,
            progress: 0.0,
        }
    }

    pub fn ui(&mut self, ctx: &EguiContext) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Trellis2 Model Viewer");

            // Input controls
            ui.horizontal(|ui| {
                ui.label("Image:");
                ui.text_edit_singleline(&mut self.image_path);
                if ui.button("Browse...").clicked() {
                    // File picker
                }
            });

            ui.horizontal(|ui| {
                ui.label("Prompt:");
                ui.text_edit_singleline(&mut self.prompt);
            });

            // Generate button
            if ui.button("Generate").clicked() && !self.generating {
                self.start_generation();
            }

            // Progress bar
            if self.generating {
                ui.add(egui::ProgressBar::new(self.progress));
            }

            // Model display (reuse renderer)
            if let Some(cube) = &self.current_cube {
                // Render using crates/renderer logic
                self.render_cube(ui, cube);
            }
        });
    }

    fn start_generation(&mut self) {
        // Spawn async task to generate
        // Update progress via channel
        // Load result when complete
    }

    fn render_cube(&mut self, ui: &mut egui::Ui, cube: &Cube<u8>) {
        // Reuse rendering from crates/renderer
        // Similar to CubeRendererApp in renderer/src/egui_app.rs
    }
}
```

---

## Usage Examples

### CLI Usage

```bash
# View mode (interactive)
cargo run --bin trellis -- view --server http://localhost:8000

# Convert O-Voxel JSON to CSM
cargo run --bin trellis -- convert input.json output.csm --depth 6

# Generate model from image
cargo run --bin trellis -- generate photo.jpg \
    --prompt "low poly robot" \
    --output robot.csm \
    --server http://localhost:8000
```

### Library Usage

```rust
use trellis::{parse_ovoxel_json, convert_ovoxel_to_octree};

// Parse O-Voxel JSON
let json = std::fs::read_to_string("model.json")?;
let ovoxel = parse_ovoxel_json(&json)?;

// Convert to octree
let cube = convert_ovoxel_to_octree(&ovoxel, 6)?;

// Use cube...
```

---

## Integration with Existing Crates

### Reuse from `crates/renderer`

```rust
// In crates/trellis/src/viewer/renderer_integration.rs
use renderer::{Renderer, CpuTracer};
use cube::Cube;

pub fn create_renderer_for_cube(cube: &Cube<u8>) -> Renderer {
    // Reuse renderer creation logic from crates/renderer
    let renderer = Renderer::new();
    renderer.load_cube(cube);
    renderer
}
```

### Reuse from `crates/cube`

```rust
// Already using Cube<u8> from cube crate
use cube::Cube;

// Convert O-Voxel and get Cube
let cube: Cube<u8> = convert_ovoxel_to_octree(&ovoxel, 6)?;
```

---

## Build and Run

### Build

```bash
# Build library
cargo build --package trellis

# Build binary (viewer)
cargo build --package trellis --bin trellis

# Release build
cargo build --package trellis --release
```

### Run

```bash
# Run viewer
cargo run --package trellis --bin trellis -- view

# Run with custom server
cargo run --package trellis --bin trellis -- view --server http://my-server:8000
```

### Add to Workspace

Update root `Cargo.toml`:
```toml
[workspace]
members = [
    "crates/cube",
    "crates/world",
    "crates/physics",
    "crates/renderer",
    "crates/assets",
    "crates/worldtool",
    "crates/server",
    "crates/trellis",  # ADD THIS
]
```

---

## Summary

| Component | Location | Purpose |
|-----------|----------|---------|
| Python backend | `services/trellis-server/` | Trellis2 inference, O-Voxel extraction |
| O-Voxel parser | `crates/trellis/src/ovoxel.rs` | Data structures |
| Converter | `crates/trellis/src/ovoxel_converter.rs` | Sparse → dense octree |
| HTTP client | `crates/trellis/src/client.rs` | Fetch from backend |
| Viewer app | `crates/trellis/src/viewer/` | Desktop GUI (egui) |
| Binary | `crates/trellis/src/main.rs` | CLI tool + viewer |

**No TypeScript/Web** - Pure Rust native application reusing existing renderer infrastructure.
