# XCube Inference Server

FastAPI server wrapping [XCube](https://github.com/nv-tlabs/XCube) inference for Rust client integration. Provides text-to-3D generation using XCube's diffusion model.

## Overview

This server exposes XCube's text-to-3D generation capabilities via a REST API, allowing Rust clients (or any HTTP client) to generate 3D voxel point clouds from text prompts. The server manages model loading, CLIP text encoding, and two-stage (coarse + fine) diffusion inference.

### Key Features

- **FastAPI REST API** with automatic OpenAPI documentation
- **Two-stage generation**: Coarse (fast) and fine (high-quality) point clouds
- **CLIP text encoding**: Converts text prompts to embeddings for conditioning
- **Health monitoring**: GPU status, model loading state, error reporting
- **Configurable inference**: Adjustable DDIM steps, guidance scale, random seed

## Requirements

### Hardware

- **GPU**: NVIDIA GPU with CUDA support (Ampere generation or newer recommended)
  - XCube requires fVDB which needs Ampere+ architecture (RTX 30-series, A100, etc.)
  - Minimum 8GB VRAM (16GB+ recommended for fine model)
- **CPU**: Multi-core processor (4+ cores recommended)
- **RAM**: 16GB minimum, 32GB+ recommended

### Software

- **Python**: 3.8+ (3.10 recommended)
- **CUDA**: 11.8 or newer
- **cuDNN**: Compatible with your CUDA version

## Installation

### 1. Clone XCube Repository

```bash
# Clone XCube from NVIDIA's repository
git clone https://github.com/nv-tlabs/XCube.git
cd XCube
```

### 2. Install XCube Dependencies

Follow XCube's installation instructions to install fVDB and other requirements:

```bash
# Install fVDB (requires CMake and CUDA)
# See https://github.com/nv-tlabs/fVDB for detailed instructions

# Install Python dependencies
pip install -r requirements.txt

# Install XCube in development mode
pip install -e .
```

### 3. Install Server Dependencies

```bash
cd /path/to/crossworld/crates/xcube/server
pip install -r requirements.txt
```

### 4. Download Model Checkpoints

Download pretrained XCube checkpoints from the [official Google Drive](https://drive.google.com/drive/folders/XCube_checkpoints):

```bash
# Create checkpoints directory
mkdir -p checkpoints

# Download and extract checkpoints
# You need:
# - objaverse_coarse/config.yaml
# - objaverse_coarse/last.ckpt
# - objaverse_fine/config.yaml (optional)
# - objaverse_fine/last.ckpt (optional)

# Example structure:
checkpoints/
├── objaverse_coarse/
│   ├── config.yaml
│   └── last.ckpt
└── objaverse_fine/
    ├── config.yaml
    └── last.ckpt
```

### 5. Set Checkpoint Path (Optional)

By default, the server looks for checkpoints in `./checkpoints`. To use a different path:

```bash
export XCUBE_CHECKPOINT_DIR=/path/to/your/checkpoints
```

## Usage

### Starting the Server

```bash
# Basic usage (default: http://0.0.0.0:8000)
python server.py

# Custom host/port
XCUBE_HOST=127.0.0.1 XCUBE_PORT=8080 python server.py

# Production mode with multiple workers
XCUBE_WORKERS=4 python server.py
```

### Configuration via Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XCUBE_HOST` | `0.0.0.0` | Server bind address |
| `XCUBE_PORT` | `8000` | Server port |
| `XCUBE_WORKERS` | `1` | Number of worker processes |
| `XCUBE_CHECKPOINT_DIR` | `./checkpoints` | Path to model checkpoints |

## API Endpoints

### Health Check: `GET /health`

Returns server status, GPU info, and model loading state.

**Response:**
```json
{
  "status": "ready",
  "xcube_available": true,
  "gpu_available": true,
  "gpu_name": "NVIDIA GeForce RTX 4090",
  "model_loaded": true,
  "error": null
}
```

**Status values:**
- `"ready"`: Models loaded, server ready for inference
- `"loading"`: Models currently loading (wait and retry)
- `"error"`: Model loading failed (check `error` field)

### Generate 3D Object: `POST /generate`

Generate a 3D point cloud from a text prompt.

**Request Body:**
```json
{
  "prompt": "a wooden chair",
  "ddim_steps": 100,
  "guidance_scale": 7.5,
  "seed": null,
  "use_fine": true
}
```

**Parameters:**
- `prompt` (string, required): Text description of the 3D object
- `ddim_steps` (int, 1-1000): Number of diffusion steps (higher = slower but better quality)
  - 50: Fast preview (5-10 seconds)
  - 100: Balanced quality (10-20 seconds)
  - 250: High quality (30-60 seconds)
- `guidance_scale` (float, 1.0-20.0): Classifier-free guidance strength (higher = closer to prompt)
  - 3.0-5.0: Low guidance (more creative)
  - 7.5: Balanced (recommended)
  - 12.0-15.0: High guidance (strict adherence to prompt)
- `seed` (int, optional): Random seed for reproducibility (null for random)
- `use_fine` (bool): Whether to run fine-resolution model (true = slower but higher detail)

**Response:**
```json
{
  "coarse_xyz": [[x, y, z], ...],
  "coarse_normal": [[nx, ny, nz], ...],
  "fine_xyz": [[x, y, z], ...],
  "fine_normal": [[nx, ny, nz], ...]
}
```

**Fields:**
- `coarse_xyz`: Coarse point cloud positions (always present)
- `coarse_normal`: Coarse point cloud normals (always present)
- `fine_xyz`: Fine point cloud positions (null if `use_fine=false` or fine model unavailable)
- `fine_normal`: Fine point cloud normals (null if `use_fine=false` or fine model unavailable)

### Interactive API Documentation

- **Swagger UI**: http://localhost:8000/docs
- **ReDoc**: http://localhost:8000/redoc

## Example Usage

### Python Client

```python
import requests

# Generate a 3D object
response = requests.post(
    "http://localhost:8000/generate",
    json={
        "prompt": "a vintage red sports car",
        "ddim_steps": 100,
        "guidance_scale": 7.5,
        "use_fine": True
    }
)

result = response.json()
print(f"Generated {len(result['coarse_xyz'])} coarse points")
print(f"Generated {len(result['fine_xyz'])} fine points")

# Save to file (example: convert to PLY format)
import numpy as np

def save_ply(xyz, normals, filename):
    points = np.array(xyz)
    normals = np.array(normals)

    with open(filename, 'w') as f:
        f.write("ply\n")
        f.write("format ascii 1.0\n")
        f.write(f"element vertex {len(points)}\n")
        f.write("property float x\n")
        f.write("property float y\n")
        f.write("property float z\n")
        f.write("property float nx\n")
        f.write("property float ny\n")
        f.write("property float nz\n")
        f.write("end_header\n")

        for (x, y, z), (nx, ny, nz) in zip(points, normals):
            f.write(f"{x} {y} {z} {nx} {ny} {nz}\n")

save_ply(result['fine_xyz'], result['fine_normal'], 'output.ply')
```

### cURL

```bash
# Check server health
curl http://localhost:8000/health

# Generate 3D object
curl -X POST http://localhost:8000/generate \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "a blue ceramic vase",
    "ddim_steps": 100,
    "guidance_scale": 7.5,
    "use_fine": true
  }' \
  -o output.json
```

### Rust Client (Example)

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GenerateRequest {
    prompt: String,
    ddim_steps: u32,
    guidance_scale: f32,
    seed: Option<u64>,
    use_fine: bool,
}

#[derive(Deserialize)]
struct XCubeResult {
    coarse_xyz: Vec<[f32; 3]>,
    coarse_normal: Vec<[f32; 3]>,
    fine_xyz: Option<Vec<[f32; 3]>>,
    fine_normal: Option<Vec<[f32; 3]>>,
}

async fn generate_object(prompt: &str) -> Result<XCubeResult, reqwest::Error> {
    let client = reqwest::Client::new();
    let request = GenerateRequest {
        prompt: prompt.to_string(),
        ddim_steps: 100,
        guidance_scale: 7.5,
        seed: None,
        use_fine: true,
    };

    let response = client
        .post("http://localhost:8000/generate")
        .json(&request)
        .send()
        .await?;

    response.json::<XCubeResult>().await
}
```

## Performance Considerations

### Inference Time

On an NVIDIA RTX 4090:
- **Coarse only** (50 steps): ~5-10 seconds
- **Coarse only** (100 steps): ~10-20 seconds
- **Coarse + Fine** (100 steps): ~30-60 seconds
- **Coarse + Fine** (250 steps): ~2-3 minutes

Times scale roughly linearly with DDIM steps and inversely with GPU compute power.

### Memory Usage

- **Coarse model**: ~4GB VRAM
- **Fine model**: ~8GB VRAM (loaded on demand)
- **CLIP text encoder**: ~1GB VRAM
- **Total**: 10-12GB VRAM peak during fine generation

### Optimization Tips

1. **Batch inference**: Process multiple prompts in parallel (requires code modification)
2. **Cache models**: Keep server running to avoid repeated loading (20-60 second startup)
3. **Adjust steps**: Use 50-75 steps for previews, 100-150 for production
4. **Skip fine model**: Set `use_fine=false` for 2-3x speedup
5. **Use smaller guidance**: Lower `guidance_scale` can sometimes improve speed without quality loss

## Troubleshooting

### "XCube not available" Error

**Cause**: XCube Python package not installed or not in PYTHONPATH.

**Solution**:
```bash
# Install XCube in development mode
cd /path/to/XCube
pip install -e .
```

### "CUDA not available" Warning

**Cause**: PyTorch not compiled with CUDA support or CUDA drivers missing.

**Solution**:
```bash
# Reinstall PyTorch with CUDA support
pip uninstall torch torchvision
pip install torch torchvision --index-url https://download.pytorch.org/whl/cu118
```

### "Missing checkpoint file" Error

**Cause**: Model checkpoints not downloaded or `XCUBE_CHECKPOINT_DIR` incorrect.

**Solution**:
1. Download checkpoints from XCube Google Drive
2. Verify directory structure matches expected layout
3. Set `XCUBE_CHECKPOINT_DIR` environment variable if using custom path

### Out of Memory (OOM) Errors

**Cause**: GPU VRAM insufficient for model size.

**Solution**:
1. Use `use_fine=false` to skip fine model (halves VRAM usage)
2. Reduce batch size (requires code modification)
3. Use a GPU with more VRAM (16GB+ recommended)
4. Enable gradient checkpointing (requires XCube code modification)

### Slow Inference on CPU

**Cause**: CUDA not available, falling back to CPU inference.

**Solution**:
1. Install CUDA drivers and CUDA-enabled PyTorch
2. Verify `torch.cuda.is_available()` returns `True`
3. Check `/health` endpoint for `gpu_available` status

## Development

### Running Tests

```bash
# Install dev dependencies
pip install pytest httpx

# Run tests
pytest test_server.py
```

### Code Structure

```
server.py
├── ServerState: Global model state management
├── load_models(): Async model initialization
├── encode_text(): CLIP text encoding
├── run_inference(): XCube inference pipeline
└── API endpoints:
    ├── GET /health
    ├── POST /generate
    └── GET / (info)
```

### Extending the Server

**Adding new endpoints**:
```python
@app.post("/batch_generate")
async def batch_generate(prompts: List[str]):
    # Process multiple prompts in parallel
    pass
```

**Custom model configurations**:
```python
# Modify load_models() to support different XCube variants
config_path = os.getenv("XCUBE_CONFIG", "custom_config.yaml")
```

## Production Deployment

### Using Gunicorn

```bash
# Install gunicorn
pip install gunicorn

# Run with multiple workers (note: each worker loads models, high memory usage)
gunicorn server:app \
  --workers 2 \
  --worker-class uvicorn.workers.UvicornWorker \
  --bind 0.0.0.0:8000 \
  --timeout 300
```

### Docker Deployment

```dockerfile
FROM nvidia/cuda:11.8.0-cudnn8-runtime-ubuntu22.04

# Install Python and dependencies
RUN apt-get update && apt-get install -y python3 python3-pip
COPY requirements.txt .
RUN pip3 install -r requirements.txt

# Copy XCube installation (assumes built separately)
COPY --from=xcube-builder /opt/xcube /opt/xcube
ENV PYTHONPATH=/opt/xcube:$PYTHONPATH

# Copy server code and checkpoints
COPY server.py /app/
COPY checkpoints /app/checkpoints

WORKDIR /app
EXPOSE 8000

CMD ["python3", "server.py"]
```

### Monitoring

- **Logs**: Server logs to stdout (capture with systemd, Docker logs, etc.)
- **Metrics**: Add Prometheus instrumentation (e.g., `prometheus-fastapi-instrumentator`)
- **Health checks**: Use `/health` endpoint for load balancer health monitoring

## License

This server code is provided as part of the Crossworld project. XCube is licensed under NVIDIA Source Code License (see XCube repository for details).

## References

- **XCube Paper**: [NVIDIA Research](https://research.nvidia.com/labs/dir/xcube/)
- **XCube Repository**: https://github.com/nv-tlabs/XCube
- **FastAPI Documentation**: https://fastapi.tiangolo.com/
- **fVDB**: https://github.com/nv-tlabs/fVDB
