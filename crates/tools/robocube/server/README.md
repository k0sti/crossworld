# Cube3D Occupancy Server

FastAPI server that wraps [Roblox Cube3D](https://github.com/Roblox/cube) for text-to-voxel generation.

## Key Feature: Direct Occupancy Field Access

Unlike the standard Cube3D API that returns meshes, this server directly queries the shape model's **occupancy decoder** at discrete grid points. This returns binary voxel occupancy values that can be directly converted to voxel formats without mesh-to-voxel conversion artifacts.

## Setup

### 1. Clone and Install Cube3D

```bash
git clone https://github.com/Roblox/cube.git
cd cube
pip install -e .[meshlab]
```

### 2. Install Server Dependencies

```bash
cd crates/robocube/server
pip install -r requirements.txt
```

### 3. Download Model Weights

```bash
huggingface-cli download Roblox/cube3d-v0.5 --local-dir ./model_weights
```

### 4. Start Server

```bash
python server.py
```

Or with environment variables:

```bash
ROBOCUBE_HOST=0.0.0.0 ROBOCUBE_PORT=8642 python server.py
```

## API Endpoints

### `GET /health`

Check server status.

Response:
```json
{
  "status": "ready",
  "gpu_available": true,
  "gpu_name": "NVIDIA RTX 4090",
  "model_loaded": true,
  "model_version": "cube3d-v0.5",
  "uptime_secs": 120.5
}
```

### `POST /generate_occupancy`

Generate voxel occupancy field from text prompt.

Request:
```json
{
  "prompt": "A wooden chair",
  "grid_resolution": 64,
  "seed": 42,
  "guidance_scale": 3.0,
  "threshold": 0.0,
  "include_logits": false
}
```

Response:
```json
{
  "resolution": 64,
  "bbox_min": [-0.5, -0.5, -0.5],
  "bbox_max": [0.5, 0.5, 0.5],
  "occupied_voxels": [[10, 20, 30], [11, 20, 30], ...],
  "metadata": {
    "generation_time_secs": 5.2,
    "seed_used": 42,
    "model_version": "cube3d-v0.5"
  }
}
```

## How It Works

1. **Text → Shape Tokens**: GPT model generates shape token IDs from text prompt
2. **Shape Tokens → Latent**: VQ-VAE decoder converts tokens to continuous latent representation
3. **Latent → Occupancy**: Occupancy decoder is queried at each grid point to get occupancy logits
4. **Threshold → Voxels**: Logits are thresholded to get binary occupied/empty voxels

This bypasses the marching cubes mesh extraction step, giving direct access to the underlying voxel representation.

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `ROBOCUBE_HOST` | `0.0.0.0` | Server bind address |
| `ROBOCUBE_PORT` | `8642` | Server port |
| `ROBOCUBE_WORKERS` | `1` | Number of uvicorn workers |
| `CUBE3D_MODEL_PATH` | `./model_weights` | Path to model weights |

## Hardware Requirements

- **GPU**: NVIDIA GPU with 16GB+ VRAM recommended
- **CPU**: Falls back to CPU if CUDA not available (very slow)
- **RAM**: 32GB+ recommended
- **Storage**: ~10GB for model weights

## Troubleshooting

### "CUDA out of memory"

Reduce `grid_resolution` (try 32 instead of 64) or use a smaller batch size.

### "Models not loaded"

Check that model weights are downloaded and `CUBE3D_MODEL_PATH` points to the correct directory.

### Slow generation

Ensure CUDA is being used. CPU inference is ~100x slower than GPU.
