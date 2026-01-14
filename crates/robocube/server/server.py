#!/usr/bin/env python3
"""
Cube3D Inference Server with Occupancy Field Endpoint

FastAPI server wrapping Roblox Cube3D for Rust client integration.
Provides direct access to the occupancy decoder for voxel generation.
"""

import os
import sys
import time
import asyncio
import logging
from typing import Optional, List, Tuple
from pathlib import Path

import torch
import numpy as np
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field

# Cube3D imports
try:
    from cube3d.inference.engine import Engine, EngineFast
    from cube3d.model.autoencoder.grid import generate_dense_grid_points
    CUBE3D_AVAILABLE = True
except ImportError as e:
    CUBE3D_AVAILABLE = False
    IMPORT_ERROR = str(e)

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# =============================================================================
# API Models
# =============================================================================

class OccupancyRequest(BaseModel):
    """Request body for /generate_occupancy endpoint"""
    prompt: str = Field(..., description="Text prompt for 3D generation")
    seed: Optional[int] = Field(None, description="Random seed for reproducibility")
    grid_resolution: int = Field(64, ge=8, le=256, description="Grid resolution (power of 2)")
    guidance_scale: float = Field(3.0, ge=0.0, le=20.0, description="Classifier-free guidance scale")
    top_p: Optional[float] = Field(None, ge=0.0, le=1.0, description="Top-p sampling (None for deterministic)")
    bounding_box_xyz: Optional[List[float]] = Field(None, description="Bounding box aspect ratio [x, y, z]")
    threshold: float = Field(0.0, description="Occupancy threshold")
    include_logits: bool = Field(False, description="Include raw logits in response")


class GenerationMetadata(BaseModel):
    """Metadata about the generation process"""
    generation_time_secs: Optional[float] = None
    seed_used: Optional[int] = None
    model_version: Optional[str] = None


class OccupancyResult(BaseModel):
    """Response body for /generate_occupancy endpoint"""
    resolution: int = Field(..., description="Grid resolution (N x N x N)")
    bbox_min: List[float] = Field(..., description="Bounding box minimum [x, y, z]")
    bbox_max: List[float] = Field(..., description="Bounding box maximum [x, y, z]")
    occupied_voxels: List[List[int]] = Field(..., description="Occupied voxel positions [[x,y,z], ...]")
    logits: Optional[List[float]] = Field(None, description="Raw occupancy logits (resolution^3 values)")
    metadata: Optional[GenerationMetadata] = None


class HealthResponse(BaseModel):
    """Response body for /health endpoint"""
    status: str = Field(..., description="Server status: 'ready', 'loading', or 'error'")
    gpu_available: bool = Field(..., description="Whether CUDA GPU is available")
    gpu_name: Optional[str] = Field(None, description="GPU device name")
    model_loaded: bool = Field(..., description="Whether models are loaded")
    model_version: Optional[str] = Field(None, description="Model version")
    error: Optional[str] = Field(None, description="Error message if status is 'error'")
    uptime_secs: Optional[float] = Field(None, description="Server uptime in seconds")


# =============================================================================
# Server State
# =============================================================================

class ServerState:
    """Global server state for model management"""
    def __init__(self):
        self.engine = None
        self.shape_model = None
        self.gpt_model = None
        self.loading = False
        self.error = None
        self.start_time = time.time()
        self.model_version = "cube3d-v0.5"

    @property
    def models_loaded(self) -> bool:
        return self.engine is not None

    @property
    def uptime(self) -> float:
        return time.time() - self.start_time


state = ServerState()
app = FastAPI(
    title="Cube3D Occupancy Server",
    description="FastAPI server for Cube3D text-to-voxel generation via occupancy field",
    version="0.1.0"
)


# =============================================================================
# Model Loading
# =============================================================================

async def load_models():
    """Load Cube3D models"""
    if state.loading or state.models_loaded:
        return

    state.loading = True
    logger.info("Starting model loading...")

    try:
        if not CUBE3D_AVAILABLE:
            raise RuntimeError(f"Cube3D not available: {IMPORT_ERROR}")

        # Check CUDA
        if not torch.cuda.is_available():
            logger.warning("CUDA not available - using CPU (slow)")
            device = torch.device("cpu")
        else:
            device = torch.device("cuda")
            logger.info(f"Using GPU: {torch.cuda.get_device_name(0)}")

        # Model paths (configurable via environment)
        model_dir = Path(os.getenv("CUBE3D_MODEL_PATH", "./model_weights"))
        config_path = model_dir / "config.yaml"
        gpt_ckpt = model_dir / "shape_gpt.safetensors"
        shape_ckpt = model_dir / "shape_tokenizer.safetensors"

        # Config may also be in cube3d/configs/ directory (CUBE3D_CONFIG_PATH)
        if not config_path.exists():
            alt_config = Path(os.getenv("CUBE3D_CONFIG_PATH", ""))
            if alt_config.exists():
                config_path = alt_config
            else:
                raise RuntimeError(f"Config not found: {config_path}")
        if not gpt_ckpt.exists():
            raise RuntimeError(f"GPT checkpoint not found: {gpt_ckpt}")
        if not shape_ckpt.exists():
            raise RuntimeError(f"Shape checkpoint not found: {shape_ckpt}")

        logger.info(f"Loading models from {model_dir}")

        # Use EngineFast on CUDA, Engine on CPU
        if device.type == "cuda":
            try:
                state.engine = EngineFast(
                    str(config_path),
                    str(gpt_ckpt),
                    str(shape_ckpt),
                    device=device
                )
                logger.info("EngineFast loaded successfully")
            except Exception as e:
                logger.warning(f"EngineFast failed: {e}, falling back to Engine")
                state.engine = Engine(
                    str(config_path),
                    str(gpt_ckpt),
                    str(shape_ckpt),
                    device=device
                )
        else:
            state.engine = Engine(
                str(config_path),
                str(gpt_ckpt),
                str(shape_ckpt),
                device=device
            )

        # Get references to sub-models for direct access
        state.shape_model = state.engine.shape_model
        state.gpt_model = state.engine.gpt_model

        logger.info("Model loading complete")
        state.error = None

    except Exception as e:
        logger.error(f"Failed to load models: {e}", exc_info=True)
        state.error = str(e)
        state.engine = None

    finally:
        state.loading = False


def generate_occupancy_field(
    prompt: str,
    resolution: int,
    seed: Optional[int] = None,
    guidance_scale: float = 3.0,
    top_p: Optional[float] = None,
    bounding_box_xyz: Optional[Tuple[float, float, float]] = None,
    threshold: float = 0.0,
    include_logits: bool = False,
) -> OccupancyResult:
    """
    Generate occupancy field from text prompt.

    Directly queries the shape model's occupancy decoder at discrete grid points
    instead of extracting a mesh.
    """
    start_time = time.time()

    if not state.models_loaded:
        raise RuntimeError("Models not loaded")

    device = next(state.shape_model.parameters()).device

    # Set seed for reproducibility
    if seed is not None:
        torch.manual_seed(seed)
        np.random.seed(seed)
        if torch.cuda.is_available():
            torch.cuda.manual_seed_all(seed)

    # Step 1: Run GPT to generate shape tokens
    logger.info(f"Generating shape tokens for: {prompt[:50]}...")

    with torch.no_grad():
        # Generate shape tokens using Engine's run_gpt
        # This handles text encoding and GPT generation internally
        shape_ids = state.engine.run_gpt(
            [prompt],
            use_kv_cache=True,  # Faster generation
            guidance_scale=guidance_scale,
            top_p=top_p,
            bounding_box_xyz=bounding_box_xyz,
        )

        logger.info(f"Generated {shape_ids.shape[1]} shape tokens")

        # Step 2: Decode shape tokens to latent representation
        # Clamp indices to valid codebook range
        num_codes = state.shape_model.cfg.num_codes
        shape_ids = shape_ids.clamp(0, num_codes - 1)

        # Get latent representation from shape decoder
        latents = state.shape_model.decode_indices(shape_ids)

        logger.info(f"Decoded to latent shape: {latents.shape}")

        # Step 3: Generate grid points and query occupancy decoder
        # Use cube3d's grid generation function
        bbox_min = np.array([-1.05, -1.05, -1.05], dtype=np.float32)
        bbox_max = np.array([1.05, 1.05, 1.05], dtype=np.float32)

        # resolution_base is log2 of resolution: 32->5, 64->6, 128->7
        resolution_base = np.log2(resolution)

        grid_points_np, grid_size, _ = generate_dense_grid_points(
            bbox_min,
            bbox_max,
            resolution_base,
            indexing='ij'
        )
        grid_points = torch.from_numpy(grid_points_np).to(device)

        logger.info(f"Querying occupancy at {grid_points.shape[0]} points...")

        # Query occupancy decoder in batches to manage memory
        batch_size = 100000
        num_points = grid_points.shape[0]
        all_logits = []

        for i in range(0, num_points, batch_size):
            batch = grid_points[i:i + batch_size].unsqueeze(0)  # [1, batch, 3]
            logits = state.shape_model.query(batch, latents)  # [1, batch]
            all_logits.append(logits.squeeze(0).cpu())

        logits_tensor = torch.cat(all_logits, dim=0)
        logits_np = logits_tensor.numpy()

        logger.info(f"Occupancy range: [{logits_np.min():.3f}, {logits_np.max():.3f}]")

        # Step 4: Threshold to get occupied voxels
        occupied_mask = logits_np > threshold
        occupied_indices = np.where(occupied_mask)[0]

        # Convert flat indices to 3D coordinates
        # Grid is stored in row-major order with 'ij' indexing
        # grid_size is [nx, ny, nz] where n = 2^resolution_base + 1
        nx, ny, nz = grid_size
        occupied_voxels = []
        for idx in occupied_indices:
            # 'ij' indexing: y varies fastest, then z, then x
            y = idx % ny
            z = (idx // ny) % nz
            x = idx // (ny * nz)
            occupied_voxels.append([int(x), int(y), int(z)])

        logger.info(f"Found {len(occupied_voxels)} occupied voxels ({100*len(occupied_voxels)/num_points:.1f}%)")

    generation_time = time.time() - start_time

    # Return actual grid resolution (number of cells, not points)
    actual_resolution = grid_size[0] - 1  # grid_size is points, not cells

    return OccupancyResult(
        resolution=actual_resolution,
        bbox_min=bbox_min.tolist(),
        bbox_max=bbox_max.tolist(),
        occupied_voxels=occupied_voxels,
        logits=logits_np.tolist() if include_logits else None,
        metadata=GenerationMetadata(
            generation_time_secs=generation_time,
            seed_used=seed,
            model_version=state.model_version,
        )
    )


# =============================================================================
# API Endpoints
# =============================================================================

@app.on_event("startup")
async def startup_event():
    """Initialize server on startup"""
    logger.info("Cube3D Occupancy Server starting...")
    asyncio.create_task(load_models())


@app.get("/health", response_model=HealthResponse)
async def health():
    """Health check endpoint"""
    gpu_available = torch.cuda.is_available()
    gpu_name = torch.cuda.get_device_name(0) if gpu_available else None

    status = "ready" if state.models_loaded else ("loading" if state.loading else "error")

    return HealthResponse(
        status=status,
        gpu_available=gpu_available,
        gpu_name=gpu_name,
        model_loaded=state.models_loaded,
        model_version=state.model_version if state.models_loaded else None,
        error=state.error,
        uptime_secs=state.uptime,
    )


@app.post("/generate_occupancy", response_model=OccupancyResult)
async def generate_occupancy(request: OccupancyRequest):
    """
    Generate occupancy field from text prompt.

    Directly queries the shape model's occupancy decoder at discrete grid points,
    returning binary voxel occupancy instead of a mesh. This is more suitable for
    voxel-based applications.
    """
    if not state.models_loaded:
        if state.loading:
            raise HTTPException(status_code=503, detail="Models still loading - try again shortly")
        else:
            raise HTTPException(status_code=503, detail=f"Models failed to load: {state.error}")

    # Validate resolution is power of 2
    if not (request.grid_resolution & (request.grid_resolution - 1) == 0):
        raise HTTPException(status_code=400, detail="grid_resolution must be a power of 2")

    try:
        bbox = tuple(request.bounding_box_xyz) if request.bounding_box_xyz else None

        result = await asyncio.to_thread(
            generate_occupancy_field,
            prompt=request.prompt,
            resolution=request.grid_resolution,
            seed=request.seed,
            guidance_scale=request.guidance_scale,
            top_p=request.top_p,
            bounding_box_xyz=bbox,
            threshold=request.threshold,
            include_logits=request.include_logits,
        )
        return result

    except Exception as e:
        logger.error(f"Occupancy generation failed: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail=f"Generation failed: {str(e)}")


@app.get("/")
async def root():
    """Root endpoint"""
    return JSONResponse(
        content={
            "message": "Cube3D Occupancy Server",
            "version": "0.1.0",
            "docs": "/docs",
            "health": "/health",
        }
    )


# =============================================================================
# Main
# =============================================================================

if __name__ == "__main__":
    import uvicorn

    host = os.getenv("ROBOCUBE_HOST", "0.0.0.0")
    port = int(os.getenv("ROBOCUBE_PORT", "8642"))
    workers = int(os.getenv("ROBOCUBE_WORKERS", "1"))

    logger.info(f"Starting server on {host}:{port}")

    uvicorn.run(
        "server:app",
        host=host,
        port=port,
        workers=workers,
        log_level="info"
    )
