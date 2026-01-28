# Trellis2 Integration Failure Analysis

## Executive Summary

The Trellis2 integration failed due to **two distinct issues**:

1. **Primary Failure**: A Python syntax error in the CPU fallback code path (`torch.cuda.device("cpu")` is invalid)
2. **Root Cause**: PyTorch version incompatibility with RTX 5090's sm_120 compute capability

The integration was ~95% complete with sophisticated workarounds documented, but a last-minute code change introduced a fatal bug that prevented the server from starting.

---

## What Happened

### Timeline

1. **Initial Setup**: Trellis2 Python server created with FastAPI, targeting GPU inference
2. **RTX 5090 Blocker Discovered**: PyTorch 2.4.0+cu118 doesn't support sm_120 (Blackwell architecture)
3. **Upgrade Path Identified**: PyTorch 2.9.1+cu128 with xformers 0.0.33.post2 and rebuilt kaolin
4. **Workarounds Implemented**: Extensive bfloat16 conversion patches, monkey-patching of TRELLIS internals
5. **CPU Fallback Added**: Attempted to add graceful CPU fallback when CUDA unavailable
6. **Fatal Bug Introduced**: Used `torch.cuda.device("cpu")` which is invalid syntax

### The Fatal Error

**Location**: `crates/trellis/server/server.py`, line 134 (in commit ad56f7c)

**Error Message**:
```
ValueError: Expected a cuda device, but got: cpu
  File "crates/trellis/server/server.py", line 134, in load_models
    with torch.cuda.device("cpu"):
```

**Root Cause**: The `torch.cuda.device()` context manager only accepts CUDA device indices (integers or "cuda:N" strings), not the CPU device. This is a fundamental misunderstanding of the PyTorch API.

**What Was Intended**: Force CPU mode for pipeline loading when CUDA isn't functional.

**What Should Have Been Done**: Remove the context manager entirely for CPU mode, or use a different approach like setting device map.

---

## Technical Issues Identified

### Issue 1: Invalid PyTorch API Usage

```python
# WRONG - throws ValueError
with torch.cuda.device("cpu"):
    pipeline = TrellisImageTo3DPipeline.from_pretrained(model_path)

# CORRECT - no context manager needed for CPU
pipeline = TrellisImageTo3DPipeline.from_pretrained(model_path)
# or specify device in pipeline config if supported
```

### Issue 2: PyTorch/CUDA Version Incompatibility

**Problem**: RTX 5090 (sm_120) requires PyTorch 2.9.1+cu128, but the environment had PyTorch 2.4.0+cu118.

**Error**:
```
NVIDIA GeForce RTX 5090 Laptop GPU with CUDA capability sm_120 is not compatible
with the current PyTorch installation.
The current PyTorch install supports CUDA capabilities sm_50 sm_60 sm_70 sm_75 sm_80 sm_86 sm_37 sm_90.
```

### Issue 3: Dependency Chain Complexity

The RTX 5090 requires a precise dependency chain:
- PyTorch 2.9.1+cu128 (for sm_120 support)
- xformers 0.0.33.post2 (built for PyTorch 2.9.1)
- kaolin 0.18.0 (must be rebuilt from source with `IGNORE_TORCH_VER=1`)
- CUDA Toolkit 12.8+

### Issue 4: HuggingFace Model Bug

The `microsoft/TRELLIS-image-large` model has an incorrect checkpoint reference in `pipeline.json`:
- References: `ckpts/slat_flow_img_dit_L_64l8p2_fp16`
- Should be: `ckpts/ss_flow_img_dit_L_16l8_fp16`

Requires manual fix after model download.

### Issue 5: xformers dtype Requirements

xformers 0.0.33.post2 requires bfloat16/float16 inputs (not float32) and only supports compute capability up to 9.0, but RTX 5090 is sm_120. This required extensive monkey-patching in server.py.

---

## Why the Failure Wasn't Caught

1. **No Automated Testing**: No unit tests for the Python server code
2. **Manual Testing Gap**: The bug was introduced in the CPU fallback path, which wasn't tested since the primary target was GPU
3. **Complex Error Flow**: The CUDA incompatibility triggered the fallback path, which then failed with a different error
4. **Documentation Focus**: Extensive documentation was created, but the actual code change that implemented the CPU fallback was flawed

---

## What Went Well

Despite the failure, significant progress was made:

1. **Comprehensive Documentation**: TRELLIS_SETUP_RTX5090.md (715 lines) documents every dependency, issue, and workaround
2. **SDPA Backend Support**: Successfully patched TRELLIS to use PyTorch's native SDPA instead of flash-attn
3. **bfloat16 Conversion**: Sophisticated patches to convert all models and inputs to bfloat16 for RTX 5090 compatibility
4. **Architecture Design**: Clear architecture for Rust client + Python server integration

---

## How to Avoid This in the Future

### 1. Add Python Unit Tests

Create `crates/trellis/server/test_server.py`:
```python
import pytest
import torch

def test_cpu_fallback_loading():
    """Test that pipeline can load in CPU mode"""
    # Mock CUDA as unavailable
    original_is_available = torch.cuda.is_available
    torch.cuda.is_available = lambda: False

    try:
        from server import load_models, state
        # This should not raise
        asyncio.run(load_models())
        assert state.error is None or "cpu" not in state.error.lower()
    finally:
        torch.cuda.is_available = original_is_available
```

### 2. Test Both Code Paths

When adding fallback logic, explicitly test:
- Primary path (GPU available and working)
- Fallback path (GPU unavailable)
- Error path (GPU available but broken)

### 3. Use Type Hints and Static Analysis

Add mypy or pyright to catch API misuse:
```python
# mypy would flag this as torch.cuda.device expects Device, not str "cpu"
with torch.cuda.device("cpu"):  # Type error!
```

### 4. Review PyTorch API Documentation

The `torch.cuda.device()` context manager documentation clearly states:
> Context-manager that changes the selected device.

It's for selecting between CUDA devices, not for switching to CPU.

### 5. CI/CD Pipeline

Add GitHub Actions workflow:
```yaml
jobs:
  test-trellis-server:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.10'
      - run: pip install pytest torch --extra-index-url https://download.pytorch.org/whl/cpu
      - run: pytest crates/trellis/server/
```

---

## Design Outline: Correct Implementation

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                 Trellis Server (Python)                  │
├─────────────────────────────────────────────────────────┤
│  Device Detection                                        │
│  ├── Check CUDA availability                            │
│  ├── Test CUDA functionality (create tensor)            │
│  └── Determine device: "cuda" or "cpu"                  │
├─────────────────────────────────────────────────────────┤
│  Pipeline Loading                                        │
│  ├── Load from HuggingFace (no device context needed)   │
│  ├── Move models to device                              │
│  └── Apply dtype conversions (bfloat16 for RTX 5090)    │
├─────────────────────────────────────────────────────────┤
│  FastAPI Server                                          │
│  ├── /health - Return device info, model status         │
│  ├── /generate - Run inference                          │
│  └── /docs - OpenAPI documentation                      │
└─────────────────────────────────────────────────────────┘
```

### Fixed CPU Fallback Code

```python
async def load_models():
    """Load Trellis.2 pipeline"""
    state.loading = True

    try:
        # Detect device
        device = "cpu"
        if torch.cuda.is_available():
            try:
                # Test CUDA functionality
                test = torch.zeros(1, device='cuda')
                del test
                device = "cuda"
                logger.info("Using CUDA device")
            except RuntimeError as e:
                logger.warning(f"CUDA available but not functional: {e}")
                logger.warning("Falling back to CPU - inference will be slow")
        else:
            logger.warning("CUDA not available - using CPU")

        # Load pipeline (no device context manager needed)
        model_path = os.getenv("TRELLIS_MODEL_PATH", "microsoft/TRELLIS-image-large")
        logger.info(f"Loading pipeline from {model_path}")

        state.pipeline = TrellisImageTo3DPipeline.from_pretrained(model_path)

        if state.pipeline is None:
            raise RuntimeError("Pipeline from_pretrained returned None")

        # Move to device and apply dtype conversions
        if device == "cuda":
            # Apply bfloat16 conversions for RTX 5090 compatibility
            apply_bfloat16_patches(state.pipeline)
        else:
            # CPU mode - keep float32 for compatibility
            logger.info("Running in CPU mode (float32)")

        state.device = device
        state.error = None
        logger.info(f"Model loading complete on {device}")

    except Exception as e:
        logger.error(f"Failed to load models: {e}", exc_info=True)
        state.error = str(e)
        state.pipeline = None
    finally:
        state.loading = False
```

### Dependency Requirements

For RTX 5090 (sm_120) support, these exact versions are required:

| Package | Version | Notes |
|---------|---------|-------|
| PyTorch | 2.9.1+cu128 | First version with sm_120 |
| xformers | 0.0.33.post2 | Must match PyTorch version |
| kaolin | 0.18.0 | Build from source: `IGNORE_TORCH_VER=1` |
| CUDA | 12.8+ | For kernel compilation |
| Python | 3.10.x | TRELLIS requirement |

### Setup Script Updates

Update `crates/trellis/server/setup.sh`:
```bash
#!/bin/bash
set -euo pipefail

# Create conda environment
conda create -n trellis python=3.10 -y

# Install PyTorch 2.9.1 with CUDA 12.8 (supports sm_120)
conda run -n trellis pip install \
    torch==2.9.1+cu128 \
    torchvision==0.24.1+cu128 \
    --index-url https://download.pytorch.org/whl/cu128

# Install xformers matching PyTorch version
conda run -n trellis pip install xformers==0.0.33.post2

# Build kaolin from source (bypasses version check)
IGNORE_TORCH_VER=1 conda run -n trellis pip install \
    git+https://github.com/NVIDIAGameWorks/kaolin.git@v0.18.0 \
    --no-build-isolation

# Install remaining dependencies
conda run -n trellis pip install \
    fastapi uvicorn pydantic pillow \
    'numpy<2.0' 'opencv-python-headless<4.8'
```

### Testing Checklist

Before deploying, verify:

1. [ ] PyTorch version is 2.9.1+cu128: `python -c "import torch; print(torch.__version__)"`
2. [ ] CUDA is available: `python -c "import torch; print(torch.cuda.is_available())"`
3. [ ] RTX 5090 detected: `python -c "import torch; print(torch.cuda.get_device_name(0))"`
4. [ ] Pipeline loads: `python -c "from trellis.pipelines import TrellisImageTo3DPipeline; p = TrellisImageTo3DPipeline.from_pretrained('microsoft/TRELLIS-image-large')"`
5. [ ] Server starts: `just trellis-server` and check `curl http://localhost:8001/health`
6. [ ] Inference works: Send test image to `/generate` endpoint

---

## Conclusion

The Trellis2 integration failure was caused by a simple but fatal Python API error (`torch.cuda.device("cpu")`) introduced while adding CPU fallback support. The underlying RTX 5090 compatibility issues were well-documented and had working solutions, but the final code change that implemented the fallback path was incorrect.

**Key Lessons**:
1. Test all code paths, especially error handling and fallback logic
2. Add automated tests for Python server code
3. Use static type analysis (mypy/pyright) to catch API misuse
4. Review API documentation before using unfamiliar functions

**Next Steps**:
1. Fix the CPU fallback code by removing the invalid `torch.cuda.device("cpu")` context manager
2. Verify PyTorch 2.9.1+cu128 is installed in the conda environment
3. Add unit tests for server startup and device detection
4. Re-test the complete inference pipeline
