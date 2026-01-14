# Robocube

Rust client and conversion library for [Roblox Cube3D](https://github.com/Roblox/cube) text-to-3D generation.

Converts Cube3D output meshes to Crossworld's CSM (CubeScript Model) voxel format.

## Architecture

```
Text Prompt
    ↓
Cube3D Python Server (FastAPI)
    ↓ HTTP POST /generate
Robocube Rust Client
    ↓
OBJ Mesh (vertices, faces, colors)
    ↓
Voxelization (mesh → discrete grid)
    ↓
Cube<u8> Octree
    ↓
CSM Text Format
```

## Installation

### Python Server Setup

The Cube3D inference requires a Python server wrapping the Roblox cube3d library:

```bash
cd crates/robocube/server
pip install -e .[meshlab]
pip install torch torchvision --index-url https://download.pytorch.org/whl/cu124

# Download model weights
huggingface-cli download Roblox/cube3d-v0.5 --local-dir ./model_weights

# Start server
python server.py
```

### Rust CLI

```bash
cargo build --release -p robocube
```

## Usage

### CLI

```bash
# Generate CSM from text prompt
robocube generate "A wooden chair" --output chair.csm

# Check server health
robocube health --server http://localhost:8642

# Generate with custom parameters
robocube generate "A red mushroom" \
    --output mushroom.csm \
    --depth 6 \
    --seed 42 \
    --server http://localhost:8642
```

### Library

```rust
use robocube::{RobocubeClient, GenerationRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RobocubeClient::new("http://localhost:8642");

    // Check server status
    let status = client.health_check().await?;
    println!("Server status: {}", status.status);

    // Generate from text prompt
    let request = GenerationRequest::new("A wooden chair")
        .with_seed(42)
        .with_bounding_box([1.0, 1.0, 1.0]);

    let result = client.generate(&request).await?;

    // Convert to CSM format
    let csm = robocube::convert::robocube_to_csm(&result)?;
    std::fs::write("chair.csm", csm)?;

    Ok(())
}
```

## Server API

### Endpoints

- `GET /health` - Server health check
- `POST /generate` - Generate 3D mesh from text prompt

### Request Format

```json
{
    "prompt": "A wooden chair",
    "seed": 42,
    "ddim_steps": 50,
    "guidance_scale": 7.5,
    "bounding_box_xyz": [1.0, 1.0, 1.0]
}
```

### Response Format

```json
{
    "vertices": [[0.1, 0.2, 0.3], ...],
    "faces": [[0, 1, 2], ...],
    "vertex_colors": [[1.0, 0.5, 0.0], ...]
}
```

## Conversion Pipeline

1. **Mesh Voxelization**: Triangle mesh → discrete voxel grid
   - Surface sampling with barycentric coordinates
   - Adaptive sample count based on triangle area
   - Optional interior flood fill

2. **Color Mapping**: Vertex colors → R2G3B2 material indices
   - Nearest triangle interpolation
   - 7-bit color encoding (128-255 range)

3. **Octree Construction**: Voxel grid → hierarchical octree
   - Efficient spatial partitioning
   - Automatic depth selection

4. **CSM Serialization**: Octree → text format
   - Human-readable output
   - Version control friendly

## Configuration

### Environment Variables

- `ROBOCUBE_SERVER_URL`: Default server URL (default: `http://localhost:8642`)
- `CUDA_VISIBLE_DEVICES`: GPU device for inference

### Server Configuration

- `--host`: Bind address (default: `0.0.0.0`)
- `--port`: Server port (default: `8642`)
- `--model-path`: Path to model weights

## Hardware Requirements

- **GPU**: NVIDIA GPU with 16GB+ VRAM recommended
- **Memory**: 32GB+ RAM recommended
- **Storage**: ~10GB for model weights

## License

MIT OR Apache-2.0
