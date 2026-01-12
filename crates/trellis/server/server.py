#!/usr/bin/env python3
"""
Trellis.2 Inference Server

FastAPI server wrapping Trellis.2 inference for Rust client integration.
Provides image-to-3D generation using the Trellis diffusion model.
"""

import os
import sys
import asyncio
import logging
import base64
from io import BytesIO
from typing import Optional, List
from pathlib import Path

import torch
import numpy as np
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field
from PIL import Image

# Trellis imports
# These assume Trellis is installed or available in PYTHONPATH
try:
    from trellis.pipelines import Trellis2ImageTo3DPipeline
    TRELLIS_AVAILABLE = True
except ImportError as e:
    TRELLIS_AVAILABLE = False
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
    image: str = Field(..., description="Base64-encoded image")
    seed: Optional[int] = Field(42, description="Random seed for reproducibility (null for random)")
    resolution: str = Field("1024", description="Image resolution: 512, 1024, or 1536")
    ss_guidance_strength: float = Field(7.5, ge=0.0, le=20.0, description="Sparse structure guidance strength")
    ss_sampling_steps: int = Field(12, ge=1, le=100, description="Sparse structure sampling steps")
    shape_slat_guidance_strength: float = Field(3.0, ge=0.0, le=20.0, description="Shape SLAT guidance strength")
    shape_slat_sampling_steps: int = Field(12, ge=1, le=100, description="Shape SLAT sampling steps")
    tex_slat_guidance_strength: float = Field(3.0, ge=0.0, le=20.0, description="Texture SLAT guidance strength")
    tex_slat_sampling_steps: int = Field(12, ge=1, le=100, description="Texture SLAT sampling steps")


class TrellisResult(BaseModel):
    """Response body for /generate endpoint"""
    vertices: List[List[float]] = Field(..., description="Mesh vertex positions [[x,y,z], ...]")
    faces: List[List[int]] = Field(..., description="Mesh triangle faces [[i1,i2,i3], ...]")
    vertex_colors: Optional[List[List[float]]] = Field(None, description="Per-vertex RGB colors (0.0-1.0 range)")
    vertex_normals: Optional[List[List[float]]] = Field(None, description="Per-vertex normals [[nx,ny,nz], ...]")


class HealthResponse(BaseModel):
    """Response body for /health endpoint"""
    status: str = Field(..., description="Server status: 'ready', 'loading', or 'error'")
    trellis_available: bool = Field(..., description="Whether Trellis dependencies are available")
    gpu_available: bool = Field(..., description="Whether CUDA GPU is available")
    gpu_name: Optional[str] = Field(None, description="GPU device name")
    model_loaded: bool = Field(..., description="Whether Trellis models are loaded")
    error: Optional[str] = Field(None, description="Error message if status is 'error'")


# Global state
class ServerState:
    """Global server state for model management"""
    def __init__(self):
        self.pipeline = None
        self.loading = False
        self.error = None

    @property
    def models_loaded(self) -> bool:
        return self.pipeline is not None


state = ServerState()
app = FastAPI(
    title="Trellis.2 Inference Server",
    description="FastAPI server for Trellis.2 image-to-3D generation",
    version="0.1.0"
)


# Model loading
async def load_models():
    """Load Trellis.2 pipeline"""
    if state.loading or state.models_loaded:
        return

    state.loading = True
    logger.info("Starting model loading...")

    try:
        if not TRELLIS_AVAILABLE:
            raise RuntimeError(f"Trellis not available: {IMPORT_ERROR}")

        if not torch.cuda.is_available():
            logger.warning("CUDA not available - inference will be slow")

        # Model path (configurable via environment variable)
        model_path = os.getenv("TRELLIS_MODEL_PATH", "microsoft/TRELLIS.2-4B")
        logger.info(f"Loading Trellis pipeline from {model_path}")

        # Load pipeline
        state.pipeline = Trellis2ImageTo3DPipeline.from_pretrained(model_path)

        # Move to GPU if available
        if torch.cuda.is_available():
            state.pipeline = state.pipeline.to("cuda")
            logger.info("Pipeline moved to GPU")

        state.pipeline.eval()
        logger.info("Model loading complete")
        state.error = None

    except Exception as e:
        logger.error(f"Failed to load models: {e}", exc_info=True)
        state.error = str(e)
        state.pipeline = None

    finally:
        state.loading = False


def decode_base64_image(base64_str: str) -> Image.Image:
    """
    Decode base64 string to PIL Image.

    Args:
        base64_str: Base64-encoded image (with or without data URI prefix)

    Returns:
        PIL Image
    """
    # Remove data URI prefix if present (e.g., "data:image/png;base64,")
    if "base64," in base64_str:
        base64_str = base64_str.split("base64,")[1]

    # Decode base64 to bytes
    image_bytes = base64.b64decode(base64_str)

    # Open image with PIL
    image = Image.open(BytesIO(image_bytes))

    # Convert to RGB if needed (remove alpha channel)
    if image.mode != "RGB":
        image = image.convert("RGB")

    return image


def preprocess_image(image: Image.Image, resolution: int) -> Image.Image:
    """
    Preprocess image for Trellis.2 inference.

    Args:
        image: PIL Image
        resolution: Target resolution (512, 1024, or 1536)

    Returns:
        Preprocessed PIL Image
    """
    # Resize image to target resolution (maintain aspect ratio, then center crop)
    width, height = image.size
    target_size = (resolution, resolution)

    # Calculate scale to cover target size
    scale = max(resolution / width, resolution / height)
    new_width = int(width * scale)
    new_height = int(height * scale)

    # Resize with high-quality resampling
    image = image.resize((new_width, new_height), Image.Resampling.LANCZOS)

    # Center crop to target size
    left = (new_width - resolution) // 2
    top = (new_height - resolution) // 2
    image = image.crop((left, top, left + resolution, top + resolution))

    return image


def extract_mesh_data(result) -> TrellisResult:
    """
    Extract mesh data from Trellis pipeline output.

    Args:
        result: Trellis pipeline output (MeshWithVoxel or similar)

    Returns:
        TrellisResult with vertices, faces, colors, normals
    """
    # Extract mesh data (adjust based on actual Trellis.2 output format)
    # The exact structure depends on the Trellis.2 API

    # Assuming result has mesh attribute with vertices and faces
    mesh = result.mesh if hasattr(result, 'mesh') else result

    # Extract vertices (Nx3 array of positions)
    vertices = mesh.vertices.cpu().numpy() if torch.is_tensor(mesh.vertices) else mesh.vertices
    vertices = vertices.tolist()

    # Extract faces (Mx3 array of vertex indices)
    faces = mesh.faces.cpu().numpy() if torch.is_tensor(mesh.faces) else mesh.faces
    faces = faces.tolist()

    # Extract colors if available (Nx3 array of RGB values)
    vertex_colors = None
    if hasattr(mesh, 'vertex_colors') and mesh.vertex_colors is not None:
        colors = mesh.vertex_colors.cpu().numpy() if torch.is_tensor(mesh.vertex_colors) else mesh.vertex_colors
        vertex_colors = colors.tolist()

    # Extract normals if available (Nx3 array of normals)
    vertex_normals = None
    if hasattr(mesh, 'vertex_normals') and mesh.vertex_normals is not None:
        normals = mesh.vertex_normals.cpu().numpy() if torch.is_tensor(mesh.vertex_normals) else mesh.vertex_normals
        vertex_normals = normals.tolist()

    return TrellisResult(
        vertices=vertices,
        faces=faces,
        vertex_colors=vertex_colors,
        vertex_normals=vertex_normals
    )


@torch.no_grad()
def run_inference(
    image: str,
    seed: Optional[int] = 42,
    resolution: str = "1024",
    ss_guidance_strength: float = 7.5,
    ss_sampling_steps: int = 12,
    shape_slat_guidance_strength: float = 3.0,
    shape_slat_sampling_steps: int = 12,
    tex_slat_guidance_strength: float = 3.0,
    tex_slat_sampling_steps: int = 12,
) -> TrellisResult:
    """
    Run Trellis.2 inference on an image.

    Args:
        image: Base64-encoded image
        seed: Random seed (None for random)
        resolution: Image resolution (512, 1024, or 1536)
        ss_guidance_strength: Sparse structure guidance strength
        ss_sampling_steps: Sparse structure sampling steps
        shape_slat_guidance_strength: Shape SLAT guidance strength
        shape_slat_sampling_steps: Shape SLAT sampling steps
        tex_slat_guidance_strength: Texture SLAT guidance strength
        tex_slat_sampling_steps: Texture SLAT sampling steps

    Returns:
        TrellisResult with mesh data
    """
    if not state.models_loaded:
        raise RuntimeError("Models not loaded - check /health endpoint")

    # Set random seed for reproducibility
    if seed is not None:
        torch.manual_seed(seed)
        np.random.seed(seed)
        if torch.cuda.is_available():
            torch.cuda.manual_seed_all(seed)

    # Decode and preprocess image
    logger.info("Decoding base64 image")
    pil_image = decode_base64_image(image)

    resolution_int = int(resolution)
    if resolution_int not in [512, 1024, 1536]:
        raise ValueError(f"Invalid resolution: {resolution}. Must be 512, 1024, or 1536")

    logger.info(f"Preprocessing image to {resolution_int}x{resolution_int}")
    pil_image = preprocess_image(pil_image, resolution_int)

    # Run Trellis pipeline
    logger.info(f"Running Trellis inference (seed: {seed})")
    logger.info(f"  SS: guidance={ss_guidance_strength}, steps={ss_sampling_steps}")
    logger.info(f"  Shape SLAT: guidance={shape_slat_guidance_strength}, steps={shape_slat_sampling_steps}")
    logger.info(f"  Texture SLAT: guidance={tex_slat_guidance_strength}, steps={tex_slat_sampling_steps}")

    result = state.pipeline.run(
        pil_image,
        seed=seed,
        ss_guidance_strength=ss_guidance_strength,
        ss_sampling_steps=ss_sampling_steps,
        shape_slat_guidance_strength=shape_slat_guidance_strength,
        shape_slat_sampling_steps=shape_slat_sampling_steps,
        tex_slat_guidance_strength=tex_slat_guidance_strength,
        tex_slat_sampling_steps=tex_slat_sampling_steps,
    )

    # Extract mesh data
    logger.info("Extracting mesh data from result")
    mesh_result = extract_mesh_data(result)

    logger.info(f"Inference complete - vertices: {len(mesh_result.vertices)}, faces: {len(mesh_result.faces)}")
    return mesh_result


# API endpoints
@app.on_event("startup")
async def startup_event():
    """Initialize server on startup"""
    logger.info("Trellis.2 Inference Server starting...")
    asyncio.create_task(load_models())


@app.get("/health", response_model=HealthResponse)
async def health():
    """
    Health check endpoint - returns server status and GPU info
    """
    gpu_available = torch.cuda.is_available()
    gpu_name = torch.cuda.get_device_name(0) if gpu_available else None

    status = "ready" if state.models_loaded else ("loading" if state.loading else "error")

    return HealthResponse(
        status=status,
        trellis_available=TRELLIS_AVAILABLE,
        gpu_available=gpu_available,
        gpu_name=gpu_name,
        model_loaded=state.models_loaded,
        error=state.error
    )


@app.post("/generate", response_model=TrellisResult)
async def generate(request: GenerateRequest):
    """
    Generate 3D mesh from image using Trellis.2.

    This endpoint runs the Trellis.2 diffusion model to generate a 3D mesh
    from an input image. The output is a textured mesh with vertices, faces,
    colors, and normals.

    Example request:
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
    """
    if not state.models_loaded:
        if state.loading:
            raise HTTPException(status_code=503, detail="Models still loading - try again in a moment")
        else:
            raise HTTPException(status_code=503, detail=f"Models failed to load: {state.error}")

    try:
        result = await asyncio.to_thread(
            run_inference,
            image=request.image,
            seed=request.seed,
            resolution=request.resolution,
            ss_guidance_strength=request.ss_guidance_strength,
            ss_sampling_steps=request.ss_sampling_steps,
            shape_slat_guidance_strength=request.shape_slat_guidance_strength,
            shape_slat_sampling_steps=request.shape_slat_sampling_steps,
            tex_slat_guidance_strength=request.tex_slat_guidance_strength,
            tex_slat_sampling_steps=request.tex_slat_sampling_steps,
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
            "message": "Trellis.2 Inference Server",
            "version": "0.1.0",
            "docs": "/docs",
            "health": "/health"
        }
    )


if __name__ == "__main__":
    import uvicorn

    # Configuration from environment variables
    host = os.getenv("TRELLIS_HOST", "0.0.0.0")
    port = int(os.getenv("TRELLIS_PORT", "8001"))
    workers = int(os.getenv("TRELLIS_WORKERS", "1"))

    logger.info(f"Starting server on {host}:{port} with {workers} workers")

    uvicorn.run(
        "server:app",
        host=host,
        port=port,
        workers=workers,
        log_level="info"
    )
