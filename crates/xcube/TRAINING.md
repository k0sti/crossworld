# XCube Training Guide

This guide explains how to train XCube models on ShapeNet data using a 24GB GPU.

## Prerequisites

### Hardware
- NVIDIA GPU with 24GB+ VRAM (RTX 3090, RTX 4090, RTX 5090, A100, etc.)
- 32GB+ system RAM recommended
- 250GB+ free disk space for dataset

### Software
- CUDA toolkit with `nvcc` compiler
- On NixOS: `nix develop .#cuda`
- On Ubuntu: `sudo apt install nvidia-cuda-toolkit`

## Quick Start

```bash
# 1. Enter CUDA environment (NixOS)
nix develop .#cuda

# 2. Set up XCube (clones repos, configures fVDB)
just xcube-setup

# 3. Start training (downloads 224GB dataset on first run)
just xcube-train
```

## Training Options

```bash
just xcube-train [OPTIONS]

Options:
  --category CATEGORY   ShapeNet category: chair, car, plane [default: chair]
  --stage STAGE         Training stage: 1 (coarse 16³) or 2 (fine 128³) [default: 1]
  --model MODEL         Model type: vae or diffusion [default: vae]
  --batch-size N        Batch size per GPU [default: 4]
  --accum-steps N       Gradient accumulation steps [default: 8]
  --max-epochs N        Maximum training epochs [default: 100]
  --precision PREC      Training precision: 16, bf16, 32 [default: 16]
  --resume PATH         Resume training from checkpoint
  --skip-download       Skip dataset download
  --no-wandb            Disable Weights & Biases logging
  --dry-run             Show command without executing
```

## Training Stages

XCube uses a two-stage hierarchical approach:

### Stage 1: Coarse (16³ dense)
- Resolution: 16×16×16 voxels
- Dense representation
- Faster training, lower memory
- **Recommended for 24GB GPU**

```bash
# Train VAE (autoencoder)
just xcube-train --stage 1 --model vae

# Train diffusion model (requires trained VAE)
just xcube-train --stage 1 --model diffusion
```

### Stage 2: Fine (128³ sparse)
- Resolution: 128×128×128 voxels
- Sparse representation
- Higher quality, more memory intensive
- **May require reduced batch size on 24GB GPU**

```bash
# Train with reduced batch size for 24GB GPU
just xcube-train --stage 2 --model vae --batch-size 1 --accum-steps 32
```

## Training Categories

ShapeNet categories available:
- `chair` - Office chairs, dining chairs, etc.
- `car` - Vehicles
- `plane` - Aircraft

```bash
just xcube-train --category car
just xcube-train --category plane
```

## Memory Optimization for 24GB GPU

The default settings are optimized for 24GB:

| Stage | Model | Batch Size | Accum Steps | Effective BS |
|-------|-------|------------|-------------|--------------|
| 1 | VAE | 4 | 8 | 32 |
| 1 | Diffusion | 4 | 8 | 32 |
| 2 | VAE | 1 | 32 | 32 |
| 2 | Diffusion | 1 | 64 | 64 |

If you encounter OOM errors:
```bash
# Reduce batch size
just xcube-train --batch-size 2 --accum-steps 16

# Use full precision (slower but sometimes more stable)
just xcube-train --precision 32 --batch-size 2
```

## Dataset

The training script automatically downloads the ShapeNet dataset from HuggingFace:
- **URL**: https://huggingface.co/datasets/xrenaa/XCube-Shapenet-Dataset
- **Size**: ~224GB
- **Location**: `data/shapenet/`

### Manual Download

If automatic download fails due to rate limits:

1. Visit https://huggingface.co/datasets/xrenaa/XCube-Shapenet-Dataset
2. Download files manually
3. Extract to `data/shapenet/`
4. Create marker file: `touch data/shapenet/.download_complete`
5. Run with `--skip-download`

### Rate Limiting

The download script handles HuggingFace rate limits automatically:
- Retries up to 10 times with exponential backoff
- Resumes partial downloads
- Verifies download completion before training

## Logging

### Weights & Biases (default)
```bash
# Login to wandb first
wandb login

# Training logs to wandb
just xcube-train
```

### Disable Logging
```bash
just xcube-train --no-wandb
```

### TensorBoard
Logs are saved to `external/XCube/logs/` directory.

## Checkpoints

Checkpoints are saved to:
```
external/XCube/checkpoints/
├── shapenet/
│   ├── chair/
│   │   ├── coarse_vae/
│   │   │   └── last.ckpt
│   │   └── coarse_diffusion/
│   │       └── last.ckpt
│   ├── car/
│   └── plane/
```

### Resume Training
```bash
just xcube-train --resume external/XCube/checkpoints/shapenet/chair/coarse_vae/last.ckpt
```

## Training Pipeline

Complete training pipeline for a category:

```bash
# 1. Train Stage 1 VAE
just xcube-train --category chair --stage 1 --model vae --max-epochs 100

# 2. Train Stage 1 Diffusion (uses VAE checkpoint)
just xcube-train --category chair --stage 1 --model diffusion --max-epochs 100

# 3. Train Stage 2 VAE (optional, for higher resolution)
just xcube-train --category chair --stage 2 --model vae --batch-size 1

# 4. Train Stage 2 Diffusion (optional)
just xcube-train --category chair --stage 2 --model diffusion --batch-size 1
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XCUBE_PATH` | `external/XCube` | Path to XCube repository |
| `XCUBE_DATA_DIR` | `data/shapenet` | Path to ShapeNet dataset |
| `FVDB_PATH` | `external/fVDB` | Path to fVDB repository |
| `CUDA_HOME` | Auto-detected | CUDA toolkit path |

## Troubleshooting

### "nvcc not found"
Enter the CUDA nix shell:
```bash
nix develop .#cuda
```

### "fVDB not found"
Run setup first:
```bash
just xcube-setup
```

### Out of Memory (OOM)
Reduce batch size:
```bash
just xcube-train --batch-size 2 --accum-steps 16
```

### Rate Limited
The script retries automatically. If persistent:
1. Wait 5 minutes
2. Try again
3. Or download manually (see Dataset section)

### "ModuleNotFoundError"
Delete the venv and re-run:
```bash
rm -rf external/XCube/.venv
just xcube-train
```

### NVIDIA library not found
Set library path (NixOS):
```bash
export LD_LIBRARY_PATH="/run/opengl-driver/lib:$LD_LIBRARY_PATH"
```

## Using Trained Models

After training, use checkpoints with the inference server:

```bash
# Copy checkpoints to server directory
cp -r external/XCube/checkpoints/shapenet/chair/coarse_vae \
      crates/xcube/server/checkpoints/objaverse_coarse

# Start server
just xcube-server
```

## References

- [XCube Paper](https://research.nvidia.com/labs/dir/xcube/)
- [XCube Repository](https://github.com/nv-tlabs/XCube)
- [fVDB Repository](https://github.com/AcademySoftwareFoundation/openvdb) (feature/fvdb branch)
- [ShapeNet Dataset](https://huggingface.co/datasets/xrenaa/XCube-Shapenet-Dataset)
