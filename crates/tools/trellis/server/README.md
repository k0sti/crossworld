# Trellis.2 Inference Server

FastAPI server wrapping Trellis.2 inference for Rust client integration. Provides image-to-3D generation using the Trellis diffusion model.

## Overview

This server exposes a REST API for the Trellis.2 image-to-3D pipeline, allowing Rust clients (and other HTTP clients) to generate 3D meshes from images without embedding the Python ML stack in the main application.

### Architecture

```
┌──────────────┐      HTTP/JSON      ┌────────────────────┐
│              │ ─────────────────────> │                    │
│ Rust Client  │                      │  FastAPI Server    │
│ (trellis)    │ <───────────────────  │  (Python)          │
│              │     TrellisResult    │                    │
└──────────────┘                      └────────────────────┘
                                               │
                                               │ PyTorch
                                               ▼
                                      ┌────────────────────┐
                                      │ Trellis.2 Pipeline │
                                      │ (HuggingFace)      │
                                      └────────────────────┘
```

## Features

- **Image-to-3D Generation**: Convert images to textured 3D meshes
- **Configurable Quality**: Adjust resolution and sampling steps
- **GPU Acceleration**: Automatic CUDA detection and usage
- **Async Processing**: Non-blocking inference via FastAPI
- **Health Monitoring**: Check server status and model loading state
- **Auto-loading**: Models loaded on startup from HuggingFace

## Installation

### Prerequisites

- Python 3.10+
- `uv` package manager
- NVIDIA GPU with CUDA 11.8+ (recommended, CPU fallback available)
- ~8GB GPU VRAM for 1024px inference, ~16GB for 1536px

### Setup

```bash
# From crossworld root
cd crates/trellis/server

# Run setup script (installs dependencies and downloads model)
./setup.sh

# Or manually:
uv sync
uv pip install -e ../../../external/TRELLIS
```

The setup script will:
1. Clone the Trellis repository to `external/TRELLIS`
2. Install Python dependencies via `uv`
3. Download the Trellis.2-4B model from HuggingFace (~4GB)

### Custom Model

To use a different model:

```bash
export TRELLIS_MODEL_PATH="microsoft/TRELLIS.2-8B"
./setup.sh
```

## Usage

### Starting the Server

```bash
# Via just (recommended)
just trellis-server

# Or manually
cd crates/trellis/server
uv run server.py

# Or with custom configuration
TRELLIS_HOST=0.0.0.0 TRELLIS_PORT=3642 uv run server.py
```

The server will:
1. Start on `http://0.0.0.0:3642` by default
2. Load the Trellis.2 pipeline on startup (may take 30-60 seconds)
3. Expose endpoints at `/health`, `/generate`, and `/docs`

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TRELLIS_MODEL_PATH` | `microsoft/TRELLIS.2-4B` | HuggingFace model name |
| `TRELLIS_HOST` | `0.0.0.0` | Server bind address |
| `TRELLIS_PORT` | `3642` | Server port |
| `TRELLIS_WORKERS` | `1` | Uvicorn worker count (keep at 1 for GPU) |

## API Endpoints

### GET /health

Check server status and model loading state.

**Response:**
```json
{
  "status": "ready",
  "trellis_available": true,
  "gpu_available": true,
  "gpu_name": "NVIDIA A100",
  "model_loaded": true,
  "error": null
}
```

**Status values:**
- `ready`: Server is ready to process requests
- `loading`: Models are still loading (retry in a few seconds)
- `error`: Model loading failed (check `error` field)

### POST /generate

Generate a 3D mesh from an image.

**Request:**
```json
{
  "image": "data:image/png;base64,iVBORw0KG...",
  "seed": 42,
  "resolution": "1024",
  "ss_guidance_strength": 7.5,
  "ss_sampling_steps": 12,
  "shape_slat_guidance_strength": 3.0,
  "shape_slat_sampling_steps": 12,
  "tex_slat_guidance_strength": 3.0,
  "tex_slat_sampling_steps": 12
}
```

**Parameters:**
- `image` (required): Base64-encoded image (with or without data URI prefix)
- `seed` (optional): Random seed for reproducibility (default: 42)
- `resolution` (optional): Image resolution - `"512"`, `"1024"`, or `"1536"` (default: `"1024"`)
- `ss_guidance_strength` (optional): Sparse structure guidance strength (default: 7.5)
- `ss_sampling_steps` (optional): Sparse structure sampling steps (default: 12)
- `shape_slat_guidance_strength` (optional): Shape SLAT guidance strength (default: 3.0)
- `shape_slat_sampling_steps` (optional): Shape SLAT sampling steps (default: 12)
- `tex_slat_guidance_strength` (optional): Texture SLAT guidance strength (default: 3.0)
- `tex_slat_sampling_steps` (optional): Texture SLAT sampling steps (default: 12)

**Response:**
```json
{
  "vertices": [[x, y, z], ...],
  "faces": [[i1, i2, i3], ...],
  "vertex_colors": [[r, g, b], ...],
  "vertex_normals": [[nx, ny, nz], ...]
}
```

**Response fields:**
- `vertices`: Array of vertex positions (Nx3 floats)
- `faces`: Array of triangle faces (Mx3 uint32 indices)
- `vertex_colors`: Per-vertex RGB colors (Nx3 floats, 0.0-1.0 range), optional
- `vertex_normals`: Per-vertex normals (Nx3 floats), optional

### GET /

Server info and documentation links.

### GET /docs

Interactive API documentation (Swagger UI).

## Example: cURL

```bash
# Check health
curl http://localhost:3642/health

# Generate from image
curl -X POST http://localhost:3642/generate \
  -H "Content-Type: application/json" \
  -d '{
    "image": "data:image/png;base64,iVBORw0KGgoAAAANS...",
    "seed": 42,
    "resolution": "1024"
  }' | jq
```

## Example: Python Client

```python
import requests
import base64
from pathlib import Path

# Read and encode image
image_path = Path("input.png")
image_data = base64.b64encode(image_path.read_bytes()).decode()

# Generate mesh
response = requests.post(
    "http://localhost:3642/generate",
    json={
        "image": image_data,
        "seed": 42,
        "resolution": "1024"
    }
)

result = response.json()
print(f"Generated {len(result['vertices'])} vertices, {len(result['faces'])} faces")
```

## Example: Rust Client

```rust
use trellis::{TrellisClient, GenerationRequest, Resolution};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TrellisClient::new("http://localhost:3642");

    // Check health
    let status = client.health_check().await?;
    println!("Server status: {}", status.status);

    // Generate mesh
    let image_data = std::fs::read("input.png")?;
    let base64_image = base64::encode(&image_data);

    let request = GenerationRequest::new(base64_image)
        .with_resolution(Resolution::R1024)
        .with_seed(42);

    let result = client.generate(&request).await?;
    println!("Generated {} vertices, {} faces",
             result.vertex_count(), result.face_count());

    Ok(())
}
```

## Performance Characteristics

### Inference Time

| Resolution | GPU (A100) | GPU (RTX 3090) | CPU |
|------------|------------|----------------|-----|
| 512px      | ~15s       | ~30s           | ~5min |
| 1024px     | ~30s       | ~60s           | ~10min |
| 1536px     | ~60s       | ~120s          | ~20min |

### Memory Usage

| Resolution | GPU VRAM | System RAM |
|------------|----------|------------|
| 512px      | ~4GB     | ~8GB       |
| 1024px     | ~8GB     | ~16GB      |
| 1536px     | ~16GB    | ~32GB      |

### Output Mesh Complexity

| Resolution | Typical Vertices | Typical Faces |
|------------|------------------|---------------|
| 512px      | ~10k-50k         | ~20k-100k     |
| 1024px     | ~50k-200k        | ~100k-400k    |
| 1536px     | ~100k-500k       | ~200k-1M      |

## Troubleshooting

### Models not loading

**Symptom:** `/health` returns `status: "error"`

**Solutions:**
1. Check internet connection (models download from HuggingFace)
2. Verify disk space (~4GB required for model)
3. Check logs: `uv run server.py` for detailed errors
4. Try manual model download:
   ```python
   from trellis.pipelines import Trellis2ImageTo3DPipeline
   pipeline = Trellis2ImageTo3DPipeline.from_pretrained("microsoft/TRELLIS.2-4B")
   ```

### CUDA out of memory

**Symptom:** Inference fails with "CUDA out of memory"

**Solutions:**
1. Reduce resolution: Use `"512"` or `"1024"` instead of `"1536"`
2. Close other GPU applications
3. Reduce sampling steps (trade quality for memory)
4. Use CPU inference (set `CUDA_VISIBLE_DEVICES=""`)

### Slow inference

**Symptom:** Generation takes minutes instead of seconds

**Solutions:**
1. Verify GPU is being used: Check `/health` for `gpu_available: true`
2. Check CUDA installation: `nvidia-smi`
3. Reduce sampling steps for faster (lower quality) output
4. Use lower resolution

### Import errors

**Symptom:** `ModuleNotFoundError: No module named 'trellis'`

**Solutions:**
1. Re-run setup: `./setup.sh`
2. Manually install Trellis:
   ```bash
   cd ../../../external/TRELLIS
   uv pip install -e .
   ```
3. Check TRELLIS repository was cloned correctly

## Development

### Running Tests

```bash
# Unit tests (if available)
uv run pytest

# Manual API test
curl http://localhost:3642/health
```

### Logging

Set log level via environment variable:

```bash
TRELLIS_LOG_LEVEL=DEBUG uv run server.py
```

### Hot Reload (Development)

```bash
uv run uvicorn server:app --reload --host 0.0.0.0 --port 3642
```

## Deployment

### Docker

```dockerfile
FROM nvidia/cuda:11.8.0-cudnn8-runtime-ubuntu22.04

# Install Python and uv
RUN apt-get update && apt-get install -y python3.10 python3-pip curl
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Copy server code
WORKDIR /app
COPY crates/trellis/server /app
COPY external/TRELLIS /app/external/TRELLIS

# Install dependencies
RUN uv sync
RUN uv pip install -e /app/external/TRELLIS

# Expose port
EXPOSE 3642

# Run server
CMD ["uv", "run", "server.py"]
```

Build and run:

```bash
docker build -t trellis-server .
docker run --gpus all -p 3642:3642 trellis-server
```

### Systemd Service

```ini
[Unit]
Description=Trellis.2 Inference Server
After=network.target

[Service]
Type=simple
User=trellis
WorkingDirectory=/opt/crossworld/crates/trellis/server
ExecStart=/home/trellis/.local/bin/uv run server.py
Restart=on-failure
Environment="TRELLIS_MODEL_PATH=microsoft/TRELLIS.2-4B"
Environment="TRELLIS_PORT=3642"

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable trellis-server
sudo systemctl start trellis-server
```

## References

- [Trellis GitHub](https://github.com/microsoft/TRELLIS)
- [Trellis.2 Paper](https://arxiv.org/abs/2412.01506)
- [HuggingFace Model](https://huggingface.co/microsoft/TRELLIS.2-4B)
- [FastAPI Documentation](https://fastapi.tiangolo.com/)

## License

This server implementation follows Crossworld's license. Trellis.2 model is subject to Microsoft's license terms.
