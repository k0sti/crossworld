# XCube Integration

Rust client library and Python inference server for [XCube](https://github.com/nv-tlabs/XCube), NVIDIA's text-to-3D diffusion model. Enables AI-powered voxel generation in Crossworld.

## Prerequisites

### Hardware Requirements

- **GPU**: NVIDIA GPU with Ampere architecture or newer (RTX 30-series, A100, etc.)
  - fVDB requires compute capability 8.0+ (Ampere)
- **VRAM**: 8GB minimum (16GB+ recommended for fine model)
- **RAM**: 16GB minimum, 32GB+ recommended

### Software Requirements

- **Linux**: Ubuntu 20.04+ or similar
- **CUDA**: 11.8 or newer
- **Python**: 3.10 (recommended)
- **cuDNN**: Compatible with your CUDA version

## Server Setup

### Quick Setup (Recommended)

Use the setup script to automate most of the installation:

```bash
# From crossworld root:
just xcube-setup

# Or run the script directly:
crates/xcube/server/setup.sh
```

**NixOS Users**: Enter the CUDA-enabled development shell first:

```bash
# Enter CUDA shell (includes nvcc, cudnn, CUDA_HOME)
nix develop .#cuda

# Then run setup
just xcube-setup
```

The setup script will:
1. Clone XCube and fVDB repositories to `external/`
2. Install Python dependencies via uv
3. Install fVDB and XCube into the virtual environment
4. Create checkpoint directory structure
5. Provide instructions for downloading model checkpoints

#### Setup Options

```bash
# Skip cloning (use existing XCube/fVDB installations)
just xcube-setup --skip-deps

# Use custom paths
just xcube-setup --xcube-path /path/to/XCube --fvdb-path /path/to/fVDB

# Custom checkpoint directory
just xcube-setup --checkpoint-dir /path/to/checkpoints
```

### Manual Setup

If you prefer manual installation, follow these steps:

#### 1. Clone XCube Repository

```bash
git clone https://github.com/nv-tlabs/XCube.git
cd XCube
```

#### 2. Install fVDB Framework

fVDB is NVIDIA's sparse voxel framework required by XCube. It's a feature branch of OpenVDB:

```bash
# Clone OpenVDB
git clone https://github.com/AcademySoftwareFoundation/openvdb.git
cd openvdb

# Fetch and checkout the fVDB feature branch
git fetch origin pull/1808/head:feature/fvdb
git checkout feature/fvdb

# Replace setup.py with XCube's patched version
rm fvdb/setup.py && cp /path/to/XCube/assets/setup.py fvdb/

# Build and install (requires CUDA toolkit, may take several minutes)
cd fvdb && pip install .
```

#### 3. Install XCube Dependencies

```bash
cd /path/to/XCube

# Install Python dependencies
pip install -r requirements.txt

# Install XCube in development mode
pip install -e .
```

#### 4. Install Server Dependencies

The server uses [uv](https://github.com/astral-sh/uv) for Python package management (included in the Nix flake):

```bash
cd /path/to/crossworld/crates/xcube/server

# uv will automatically create a virtual environment and install dependencies
# when you first run the server (see step 6)

# Or install dependencies manually:
uv sync
```

### Download Model Checkpoints

Download pretrained XCube checkpoints from [Google Drive](https://drive.google.com/drive/folders/1PEh0ofpSFcgH56SZtu6iQPC8xAxzhmke):

```bash
# Create checkpoints directory
mkdir -p checkpoints

# Expected structure after download:
checkpoints/
├── objaverse_coarse/
│   ├── config.yaml
│   └── last.ckpt
└── objaverse_fine/        # Optional, for higher quality
    ├── config.yaml
    └── last.ckpt
```

### Start the Server

```bash
# From crossworld root (recommended):
just xcube-server

# Or manually from crates/xcube/server directory:
uv run server.py
```

The server runs on `http://0.0.0.0:8000` by default.

#### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XCUBE_HOST` | `0.0.0.0` | Server bind address |
| `XCUBE_PORT` | `8000` | Server port |
| `XCUBE_WORKERS` | `1` | Number of worker processes |
| `XCUBE_CHECKPOINT_DIR` | `./checkpoints` | Path to model checkpoints |

## Testing the Server

### Health Check

```bash
curl http://localhost:8000/health
```

Expected response when ready:
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

### Using the Python Test Client

```bash
cd crates/xcube/server
python test_client.py
```

### Generate a 3D Object

```bash
# Quick generation (coarse only, ~10 seconds)
just xcube-generate "a wooden chair"

# Or manually with curl:
curl -X POST http://localhost:8000/generate \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "a wooden chair",
    "ddim_steps": 50,
    "guidance_scale": 7.5,
    "use_fine": false
  }'
```

## Example Commands

```bash
# Set up XCube environment (clone repos, install deps)
just xcube-setup

# Start the XCube inference server
just xcube-server

# Generate a simple object (fast preview)
just xcube-generate "a red sports car"

# Full API documentation
open http://localhost:8000/docs
```

## Troubleshooting

### CUDA Out of Memory (OOM)

**Symptoms**: `CUDA out of memory` error during inference

**Solutions**:
1. Set `use_fine: false` in requests (halves VRAM usage)
2. Reduce `ddim_steps` to 50 or fewer
3. Use a GPU with more VRAM (16GB+ recommended)
4. Close other GPU applications

### Missing Checkpoint Files

**Symptoms**: `Missing checkpoint file: checkpoints/objaverse_coarse/last.ckpt`

**Solutions**:
1. Download checkpoints from the [Google Drive link](https://drive.google.com/drive/folders/1PEh0ofpSFcgH56SZtu6iQPC8xAxzhmke)
2. Verify directory structure matches expected layout
3. Set `XCUBE_CHECKPOINT_DIR` environment variable if using custom path

### XCube Not Available

**Symptoms**: `XCube not available` in health check

**Solutions**:
```bash
# Ensure XCube is installed
cd /path/to/XCube
pip install -e .

# Verify installation
python -c "from xcube.models.model import create_model_from_args; print('OK')"
```

### fVDB Import Errors

**Symptoms**: `ImportError: No module named 'fvdb'` or CUDA capability errors

**Solutions**:
1. Verify GPU is Ampere or newer (RTX 30-series, A100, etc.)
2. Reinstall fVDB with correct CUDA version:
   ```bash
   cd /path/to/fVDB
   pip uninstall fvdb
   pip install .
   ```
3. Check CUDA version matches PyTorch:
   ```bash
   python -c "import torch; print(torch.version.cuda)"
   nvidia-smi
   ```

### Slow Inference (CPU Fallback)

**Symptoms**: Inference takes minutes instead of seconds

**Solutions**:
1. Check GPU detection:
   ```bash
   python -c "import torch; print(torch.cuda.is_available())"
   ```
2. Reinstall PyTorch with CUDA:
   ```bash
   pip uninstall torch torchvision
   pip install torch torchvision --index-url https://download.pytorch.org/whl/cu118
   ```

## API Reference

See [`server/README.md`](server/README.md) for complete API documentation including:
- Endpoint specifications
- Request/response formats
- Python and Rust client examples
- Performance tuning guide
- Production deployment instructions

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Crossworld Integration                  │
├─────────────────────────────────────────────────────────┤
│  xcube crate (Rust)                                      │
│  ├── XCubeClient: HTTP client for inference server      │
│  ├── types: XCubeModel, XCubeResponse                   │
│  └── convert: Point cloud → CSM/voxel conversion        │
├─────────────────────────────────────────────────────────┤
│  XCube Server (Python)                                   │
│  ├── FastAPI REST API                                   │
│  ├── CLIP text encoding                                 │
│  └── XCube diffusion inference                          │
├─────────────────────────────────────────────────────────┤
│  XCube (NVIDIA)                                          │
│  ├── fVDB sparse voxel storage                          │
│  └── Diffusion model for text-to-3D                     │
└─────────────────────────────────────────────────────────┘
```

## References

- [XCube Paper](https://research.nvidia.com/labs/dir/xcube/) - NVIDIA Research
- [XCube Repository](https://github.com/nv-tlabs/XCube) - Official code
- [fVDB](https://github.com/nv-tlabs/fVDB) - Sparse voxel framework
