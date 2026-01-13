# Trellis.2 Server Setup Notes

## RTX 5090 Support

This document describes the fixes required to run Trellis.2 inference server on RTX 5090 (Blackwell architecture, compute capability 12.0).

## Dependency Versions

The following versions are required for RTX 5090 support:

- **PyTorch**: 2.9.1+cu128 (supports sm_120, compatible with xformers)
- **xformers**: 0.0.33.post2 (built for PyTorch 2.9.1+cu128)
- **kaolin**: 0.18.0 (built from source with `IGNORE_TORCH_VER=1`)
- **CUDA Toolkit**: 12.8+ (for kaolin compilation)

## Known Issues and Fixes

### 1. Pipeline.json Checkpoint Reference Bug

**Problem**: The `microsoft/TRELLIS-image-large` model from HuggingFace has an incorrect checkpoint reference in `pipeline.json`.

**Error**:
```
Repository Not Found for url: https://huggingface.co/ckpts/slat_flow_img_dit_L_64l8p2_fp16/resolve/main/.json
```

**Root Cause**: Line 10 in `pipeline.json` references a checkpoint that doesn't exist in the downloaded model.

**Fix**: Edit the downloaded pipeline.json file:

```bash
# Location: ~/.cache/huggingface/hub/models--microsoft--TRELLIS-image-large/snapshots/*/pipeline.json

# Change line 10 from:
"slat_flow_model": "ckpts/slat_flow_img_dit_L_64l8p2_fp16"

# To:
"slat_flow_model": "ckpts/ss_flow_img_dit_L_16l8_fp16"
```

**Available Checkpoints in TRELLIS-image-large**:
- `ckpts/ss_dec_conv3d_16l8_fp16` ✓
- `ckpts/ss_flow_img_dit_L_16l8_fp16` ✓
- `ckpts/slat_dec_gs_swin8_B_64l8gs32_fp16` ✓
- `ckpts/slat_dec_rf_swin8_B_64l8r16_fp16` ✓
- `ckpts/slat_dec_mesh_swin8_B_64l8m256c_fp16` ✓
- `ckpts/slat_flow_img_dit_L_64l8p2_fp16` ✗ (referenced but doesn't exist)

### 2. PyTorch/xformers Version Compatibility

**Problem**: xformers 0.0.33.post2 is built for PyTorch 2.9.1, but earlier setup used PyTorch 2.8.0 or 2.4.0.

**Error**:
```
WARNING[XFORMERS]: xFormers can't load C++/CUDA extensions. xFormers was built for:
    PyTorch 2.9.1+cu128 with CUDA 1208 (you have 2.8.0+cu128)
ImportError: undefined symbol: _ZNK3c106SymInt22maybe_as_int_slow_pathEv
```

**Fix**: Upgrade to PyTorch 2.9.1+cu128:
```bash
conda run -n trellis pip install torch==2.9.1+cu128 torchvision --index-url https://download.pytorch.org/whl/cu128
```

### 3. kaolin ABI Compatibility

**Problem**: kaolin 0.18.0 officially only supports PyTorch <= 2.8.0, causing ABI errors with PyTorch 2.9.1.

**Error**:
```
ImportError: /tmp/kaolin_build/kaolin/_C.so: undefined symbol: _ZNK3c106SymInt6sym_neERKS0_
```

**Fix**: Rebuild kaolin from source with version check override:
```bash
IGNORE_TORCH_VER=1 conda run -n trellis pip install git+https://github.com/NVIDIAGameWorks/kaolin.git@v0.18.0 --no-build-isolation
```

This takes 10-20 minutes to compile.

### 4. RTX 5090 Compute Capability Support

**Problem**: PyTorch 2.8.0 and earlier don't support RTX 5090's sm_120 compute capability.

**Error**:
```
NVIDIA GeForce RTX 5090 with CUDA capability sm_120 is not compatible with the current PyTorch installation.
The current PyTorch install supports CUDA capabilities sm_50 sm_60 sm_70 sm_75 sm_80 sm_86 sm_37 sm_90.
```

**Fix**: Use PyTorch 2.9.1+cu128 which includes sm_120 support.

## Dependency Chain Explanation

The dependency chain that led to these specific versions:

1. **RTX 5090** requires PyTorch with sm_120 support → **PyTorch 2.9.1+**
2. **TRELLIS sparse modules** require xformers or flash_attn → **xformers**
3. **xformers 0.0.33+** requires PyTorch 2.9.1+ → **PyTorch 2.9.1+cu128**
4. **kaolin 0.18.0** officially supports PyTorch <= 2.8.0 → **Build from source with override**

## Environment Variables

Required environment variables for running the server:

```bash
# Use xformers attention backend (required for sparse modules)
export ATTN_BACKEND=xformers

# Add TRELLIS to Python path
export PYTHONPATH=external/TRELLIS:${PYTHONPATH:-}

# Add GPU driver libraries for CUDA support
export LD_LIBRARY_PATH=/run/opengl-driver/lib:${LD_LIBRARY_PATH:-}

# Model path (corrected model with fixed pipeline.json)
export TRELLIS_MODEL_PATH=microsoft/TRELLIS-image-large
```

## Verification

To verify the setup works:

```bash
nix develop .#cuda --command bash -c "
export ATTN_BACKEND=xformers
export PYTHONPATH=external/TRELLIS:\${PYTHONPATH:-}
export LD_LIBRARY_PATH=/run/opengl-driver/lib:\${LD_LIBRARY_PATH:-}

conda run -n trellis python -c '
import torch
print(f\"PyTorch: {torch.__version__}\")
print(f\"CUDA available: {torch.cuda.is_available()}\")
if torch.cuda.is_available():
    print(f\"Device: {torch.cuda.get_device_name(0)}\")

from trellis.pipelines import TrellisImageTo3DPipeline
pipeline = TrellisImageTo3DPipeline.from_pretrained(\"microsoft/TRELLIS-image-large\")
print(\"✓ Pipeline loaded successfully\")
'
"
```

Expected output:
```
PyTorch: 2.9.1+cu128
CUDA available: True
Device: NVIDIA GeForce RTX 5090 Laptop GPU
[SPARSE] Backend: spconv, Attention: xformers
[ATTENTION] Using backend: xformers
✓ Pipeline loaded successfully
```

## Updated Setup Script

The `crates/trellis/server/setup.sh` script has been updated to:
- Use PyTorch 2.9.1+cu128 instead of 2.4.0+cu118
- Install xformers 0.0.33.post2 instead of 0.0.27.post2
- Rebuild kaolin 0.18.0 from source with `IGNORE_TORCH_VER=1`
- Provide instructions for fixing the pipeline.json bug if model loading fails

## References

- PyTorch CUDA support: https://pytorch.org/get-started/locally/
- xformers releases: https://github.com/facebookresearch/xformers/releases
- kaolin GitHub: https://github.com/NVIDIAGameWorks/kaolin
- TRELLIS repository: https://github.com/microsoft/TRELLIS
- RTX 5090 specs: Compute capability 12.0 (Blackwell architecture)
