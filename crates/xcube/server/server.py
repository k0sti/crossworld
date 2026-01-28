#!/usr/bin/env python3
"""
XCube Inference Server

FastAPI server wrapping XCube inference for Rust client integration.
Provides text-to-3D generation using the XCube diffusion model.
"""

import os
import sys
import asyncio
import logging
from typing import Optional, List
from pathlib import Path

import torch
import numpy as np
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field


def test_cuda_works() -> tuple[bool, str | None]:
    """
    Test if CUDA actually works (not just available).
    PyTorch may report CUDA available but kernels may fail on unsupported GPUs.
    Returns (works: bool, error_message: str | None)
    """
    if not torch.cuda.is_available():
        return False, "CUDA not available"

    try:
        # Try a simple tensor operation to verify kernels work
        x = torch.randn(10, 10, device="cuda")
        y = torch.matmul(x, x)
        _ = y.sum().item()  # Force synchronization
        return True, None
    except RuntimeError as e:
        error_msg = str(e)
        if "no kernel image" in error_msg:
            gpu_name = torch.cuda.get_device_name(0)
            return False, f"GPU {gpu_name} not supported by this PyTorch build (missing CUDA kernels)"
        return False, error_msg


# Global CUDA state - tested once at startup
CUDA_WORKS = False
CUDA_ERROR = None


# XCube imports
# These assume XCube is installed or available in PYTHONPATH
try:
    import importlib
    from xcube.utils import exp
    from transformers import CLIPTextModel, AutoProcessor

    def create_model_from_args(config_path, ckpt_path, strict=True):
        """Load XCube model from config and checkpoint."""
        model_yaml_path = Path(config_path)
        model_args = exp.parse_config_yaml(model_yaml_path)
        net_module = importlib.import_module("xcube.models." + model_args.model).Model
        args_ckpt = Path(ckpt_path)
        assert args_ckpt.exists(), f"Checkpoint does not exist: {args_ckpt}"
        net_model = net_module.load_from_checkpoint(args_ckpt, hparams=model_args, strict=strict)
        return net_model.eval()

    def padding_text_emb(text_emb, max_text_len=77):
        """Pad text embedding to fixed length."""
        padded_text_emb = torch.zeros(max_text_len, text_emb.shape[1])
        padded_text_emb[:text_emb.shape[0]] = text_emb
        mask = torch.zeros(max_text_len)
        mask[:text_emb.shape[0]] = 1
        return padded_text_emb, mask.bool()

    def clip_preprocess(clip_tag='l14'):
        """Get CLIP preprocessor."""
        clip_names = {
            'l14': 'openai/clip-vit-large-patch14',
            'h14': 'laion/CLIP-ViT-H-14-laion2B-s32B-b79K'
        }
        return AutoProcessor.from_pretrained(clip_names[clip_tag])

    XCUBE_AVAILABLE = True
except ImportError as e:
    XCUBE_AVAILABLE = False
    IMPORT_ERROR = str(e)

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# API models
class GenerateRequest(BaseModel):
    """Request body for /generate endpoint"""
    prompt: str = Field(..., description="Text prompt describing the 3D object to generate")
    ddim_steps: int = Field(100, ge=1, le=1000, description="Number of DDIM diffusion steps")
    guidance_scale: float = Field(7.5, ge=1.0, le=20.0, description="Classifier-free guidance scale")
    seed: Optional[int] = Field(None, description="Random seed for reproducibility (null for random)")
    use_fine: bool = Field(True, description="Use fine-resolution model (slower but higher quality)")


class XCubeResult(BaseModel):
    """Response body for /generate endpoint"""
    coarse_xyz: List[List[float]] = Field(..., description="Coarse point cloud positions [[x,y,z], ...]")
    coarse_normal: List[List[float]] = Field(..., description="Coarse point cloud normals [[nx,ny,nz], ...]")
    fine_xyz: Optional[List[List[float]]] = Field(None, description="Fine point cloud positions (if use_fine=true)")
    fine_normal: Optional[List[List[float]]] = Field(None, description="Fine point cloud normals (if use_fine=true)")


class HealthResponse(BaseModel):
    """Response body for /health endpoint"""
    status: str = Field(..., description="Server status: 'ready', 'loading', or 'error'")
    xcube_available: bool = Field(..., description="Whether XCube dependencies are available")
    gpu_available: bool = Field(..., description="Whether CUDA GPU is available")
    gpu_name: Optional[str] = Field(None, description="GPU device name")
    model_loaded: bool = Field(..., description="Whether XCube models are loaded")
    error: Optional[str] = Field(None, description="Error message if status is 'error'")


# Global state
class ServerState:
    """Global server state for model management"""
    def __init__(self):
        self.coarse_model = None
        self.fine_model = None
        self.text_encoder = None
        self.clip_preprocess_fn = None
        self.loading = False
        self.error = None

    @property
    def models_loaded(self) -> bool:
        return self.coarse_model is not None and self.text_encoder is not None


state = ServerState()
app = FastAPI(
    title="XCube Inference Server",
    description="FastAPI server for XCube text-to-3D generation",
    version="0.1.0"
)


# Model loading
async def load_models():
    """Load XCube models and CLIP text encoder"""
    global CUDA_WORKS, CUDA_ERROR

    if state.loading or state.models_loaded:
        return

    state.loading = True
    logger.info("Starting model loading...")

    try:
        if not XCUBE_AVAILABLE:
            raise RuntimeError(f"XCube not available: {IMPORT_ERROR}")

        # Test if CUDA actually works (not just available)
        CUDA_WORKS, CUDA_ERROR = test_cuda_works()
        if CUDA_WORKS:
            logger.info(f"CUDA working - using GPU: {torch.cuda.get_device_name(0)}")
        elif torch.cuda.is_available():
            logger.warning(f"CUDA detected but not working: {CUDA_ERROR}")
            logger.warning("Falling back to CPU - inference will be slow")
        else:
            logger.warning("CUDA not available - inference will be slow")

        # Model paths (configurable via environment variables)
        checkpoint_dir = Path(os.getenv("XCUBE_CHECKPOINT_DIR", "./checkpoints"))
        config_coarse = checkpoint_dir / "objaverse_coarse" / "config.yaml"
        ckpt_coarse = checkpoint_dir / "objaverse_coarse" / "last.ckpt"
        config_fine = checkpoint_dir / "objaverse_fine" / "config.yaml"
        ckpt_fine = checkpoint_dir / "objaverse_fine" / "last.ckpt"

        # Verify checkpoint files exist
        for path in [config_coarse, ckpt_coarse]:
            if not path.exists():
                raise FileNotFoundError(f"Missing checkpoint file: {path}")

        logger.info(f"Loading coarse model from {ckpt_coarse}")
        state.coarse_model = create_model_from_args(str(config_coarse), str(ckpt_coarse))
        state.coarse_model = state.coarse_model.cuda() if CUDA_WORKS else state.coarse_model
        state.coarse_model.eval()

        # Fine model is optional
        if config_fine.exists() and ckpt_fine.exists():
            logger.info(f"Loading fine model from {ckpt_fine}")
            state.fine_model = create_model_from_args(str(config_fine), str(ckpt_fine))
            state.fine_model = state.fine_model.cuda() if CUDA_WORKS else state.fine_model
            state.fine_model.eval()
        else:
            logger.warning("Fine model checkpoints not found - fine generation disabled")

        # Load CLIP text encoder
        logger.info("Loading CLIP text encoder")
        clip_model_name = "openai/clip-vit-large-patch14"
        state.text_encoder = CLIPTextModel.from_pretrained(clip_model_name)
        state.text_encoder = state.text_encoder.cuda() if CUDA_WORKS else state.text_encoder
        state.text_encoder.eval()

        state.clip_preprocess_fn = clip_preprocess()  # Call to get AutoProcessor instance

        logger.info("Model loading complete")
        state.error = None

    except Exception as e:
        logger.error(f"Failed to load models: {e}", exc_info=True)
        state.error = str(e)
        state.coarse_model = None
        state.fine_model = None
        state.text_encoder = None

    finally:
        state.loading = False


def encode_text(prompt: str, max_text_len: int = 77):
    """Encode text prompt using CLIP text encoder"""
    inputs = state.clip_preprocess_fn(
        text=[prompt],
        return_tensors="pt",
        padding=True,
        max_length=max_text_len,
        truncation=True
    )

    # Move to GPU if CUDA works
    if CUDA_WORKS:
        inputs = {k: v.cuda() for k, v in inputs.items()}

    with torch.no_grad():
        text_embed_sd_model = state.text_encoder.text_model(**inputs)
        text_emb = text_embed_sd_model.last_hidden_state[0]
        text_emb, mask = padding_text_emb(text_emb, max_text_len=max_text_len)

    return text_emb, mask


@torch.no_grad()
def run_inference(
    prompt: str,
    ddim_steps: int = 100,
    guidance_scale: float = 7.5,
    seed: Optional[int] = None,
    use_fine: bool = True
) -> XCubeResult:
    """
    Run XCube inference on a text prompt.

    Args:
        prompt: Text description of the 3D object
        ddim_steps: Number of DDIM sampling steps
        guidance_scale: Classifier-free guidance strength
        seed: Random seed (None for random)
        use_fine: Whether to run fine-resolution model

    Returns:
        XCubeResult with point cloud data
    """
    if not state.models_loaded:
        raise RuntimeError("Models not loaded - check /health endpoint")

    # Set random seed for reproducibility
    if seed is not None:
        torch.manual_seed(seed)
        np.random.seed(seed)

    # Encode text prompt
    logger.info(f"Encoding prompt: '{prompt}'")
    text_emb, text_mask = encode_text(prompt)

    cond_dict = {
        'text_embed': text_emb.unsqueeze(0),  # Add batch dimension
        'text_embed_mask': text_mask.unsqueeze(0)
    }

    # Coarse generation
    logger.info(f"Running coarse generation (DDIM steps: {ddim_steps}, guidance: {guidance_scale})")
    res_coarse, output_x_coarse = state.coarse_model.evaluation_api(
        batch_size=1,
        use_ddim=True,
        ddim_step=ddim_steps,
        cond_dict=cond_dict,
        guidance_scale=guidance_scale
    )

    # Extract coarse point cloud
    coarse_xyz = output_x_coarse.grid.grid_to_world(
        output_x_coarse.grid[0].ijk.float()
    ).jdata.cpu().numpy()

    coarse_normal = res_coarse.normal_features[-1].feature[0].jdata.cpu().numpy()

    result = XCubeResult(
        coarse_xyz=coarse_xyz.tolist(),
        coarse_normal=coarse_normal.tolist(),
        fine_xyz=None,
        fine_normal=None
    )

    # Fine generation (optional)
    if use_fine and state.fine_model is not None:
        logger.info("Running fine generation")
        res_fine, output_x_fine = state.fine_model.evaluation_api(
            grids=output_x_coarse.grid,
            res_coarse=res_coarse,
            cond_dict=cond_dict,
            guidance_scale=guidance_scale
        )

        fine_xyz = output_x_fine.grid.grid_to_world(
            output_x_fine.grid[0].ijk.float()
        ).jdata.cpu().numpy()

        fine_normal = res_fine.normal_features[-1].feature[0].jdata.cpu().numpy()

        result.fine_xyz = fine_xyz.tolist()
        result.fine_normal = fine_normal.tolist()

    elif use_fine:
        logger.warning("Fine model requested but not available")

    logger.info(f"Inference complete - coarse points: {len(coarse_xyz)}, "
                f"fine points: {len(result.fine_xyz) if result.fine_xyz else 0}")

    return result


# API endpoints
@app.on_event("startup")
async def startup_event():
    """Initialize server on startup"""
    logger.info("XCube Inference Server starting...")
    asyncio.create_task(load_models())


@app.get("/health", response_model=HealthResponse)
async def health():
    """
    Health check endpoint - returns server status and GPU info
    """
    gpu_detected = torch.cuda.is_available()
    gpu_name = torch.cuda.get_device_name(0) if gpu_detected else None

    # gpu_available means CUDA actually works (kernels compatible)
    # If CUDA is detected but doesn't work, include the error
    status = "ready" if state.models_loaded else ("loading" if state.loading else "error")
    error = state.error
    if CUDA_ERROR and not error:
        error = CUDA_ERROR

    return HealthResponse(
        status=status,
        xcube_available=XCUBE_AVAILABLE,
        gpu_available=CUDA_WORKS,
        gpu_name=gpu_name if CUDA_WORKS else f"{gpu_name} (unsupported)" if gpu_name else None,
        model_loaded=state.models_loaded,
        error=error
    )


@app.post("/generate", response_model=XCubeResult)
async def generate(request: GenerateRequest):
    """
    Generate 3D point cloud from text prompt using XCube.

    This endpoint runs the XCube diffusion model to generate a 3D object
    from a text description. The output is a point cloud with positions
    and normals at coarse and (optionally) fine resolutions.

    Example request:
    ```json
    {
        "prompt": "a wooden chair",
        "ddim_steps": 100,
        "guidance_scale": 7.5,
        "seed": null,
        "use_fine": true
    }
    ```
    """
    if not state.models_loaded:
        if state.loading:
            raise HTTPException(status_code=503, detail="Models still loading - try again in a moment")
        else:
            raise HTTPException(status_code=503, detail=f"Models failed to load: {state.error}")

    try:
        result = await asyncio.to_thread(
            run_inference,
            prompt=request.prompt,
            ddim_steps=request.ddim_steps,
            guidance_scale=request.guidance_scale,
            seed=request.seed,
            use_fine=request.use_fine
        )
        return result

    except Exception as e:
        logger.error(f"Inference failed: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail=f"Inference failed: {str(e)}")


@app.get("/")
async def root():
    """Root endpoint - redirect to docs"""
    return JSONResponse(
        content={
            "message": "XCube Inference Server",
            "version": "0.1.0",
            "docs": "/docs",
            "health": "/health"
        }
    )


if __name__ == "__main__":
    import uvicorn

    # Configuration from environment variables
    host = os.getenv("XCUBE_HOST", "0.0.0.0")
    port = int(os.getenv("XCUBE_PORT", "8000"))
    workers = int(os.getenv("XCUBE_WORKERS", "1"))

    logger.info(f"Starting server on {host}:{port} with {workers} workers")

    uvicorn.run(
        "server:app",
        host=host,
        port=port,
        workers=workers,
        log_level="info"
    )
