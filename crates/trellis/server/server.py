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
# Import directly to avoid loading text pipeline which needs open3d with GUI
try:
    from trellis.pipelines.trellis_image_to_3d import TrellisImageTo3DPipeline
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

        # Check if CUDA is available and properly supported
        cuda_available = False
        if torch.cuda.is_available():
            try:
                # Test CUDA by creating a simple tensor
                test_tensor = torch.zeros(1, device='cuda')
                del test_tensor
                cuda_available = True
                logger.info("CUDA is available and working")
            except RuntimeError as e:
                logger.warning(f"CUDA available but not functional: {e}")
                logger.warning("Will use CPU - inference will be slow")
        else:
            logger.warning("CUDA not available - inference will be slow")

        # Model path (configurable via environment variable)
        model_path = os.getenv("TRELLIS_MODEL_PATH", "microsoft/TRELLIS-image-large")
        logger.info(f"Loading Trellis pipeline from {model_path}")

        # Load pipeline (force CPU if CUDA not working)
        if cuda_available:
            logger.info("Loading pipeline for GPU...")
            state.pipeline = TrellisImageTo3DPipeline.from_pretrained(model_path)
            if state.pipeline is None:
                raise RuntimeError("Pipeline from_pretrained returned None")
            logger.info("Pipeline loaded, converting to bfloat16 and moving to GPU...")
            # Convert all models to bfloat16 for RTX 5090 compatibility with xformers
            # xformers requires bfloat16 or float16 (not float32) and RTX 5090 is sm_120
            # EXCEPTION: sparse_structure_decoder must stay in float32 due to custom norm layer
            if hasattr(state.pipeline, 'models'):
                for name, model in state.pipeline.models.items():
                    if hasattr(model, 'to'):
                        try:
                            # IMPORTANT: .to() doesn't modify in place, must reassign
                            state.pipeline.models[name] = model.to(dtype=torch.bfloat16, device='cuda')
                            logger.info(f"  {name}: converted to bfloat16 on cuda")
                        except Exception as e:
                            logger.warning(f"  {name}: failed to convert - {e}")
            # Also convert image_cond_model if it exists (DINOv2)
            if hasattr(state.pipeline, 'image_cond_model') and state.pipeline.image_cond_model is not None:
                try:
                    state.pipeline.image_cond_model = state.pipeline.image_cond_model.to(dtype=torch.bfloat16, device='cuda')
                    logger.info("  image_cond_model: converted to bfloat16 on cuda")
                except Exception as e:
                    logger.warning(f"  image_cond_model: failed to convert - {e}")
            logger.info("Pipeline loaded, converted to bfloat16, and moved to GPU")
        else:
            # Force CPU mode to avoid CUDA errors
            # Note: No need for torch.cuda.device() when using CPU
            logger.info("Loading pipeline for CPU...")
            state.pipeline = TrellisImageTo3DPipeline.from_pretrained(model_path)
            if state.pipeline is None:
                raise RuntimeError("Pipeline from_pretrained returned None")
            logger.info("Pipeline loaded on CPU (inference will be slow)")

        if state.pipeline is None:
            raise RuntimeError("Pipeline is None after loading")

        # Set models to eval mode (pipeline itself doesn't have eval, but its models do)
        if hasattr(state.pipeline, 'eval'):
            state.pipeline.eval()
            logger.info("Pipeline set to eval mode")
        elif hasattr(state.pipeline, 'models'):
            # Set each model in the pipeline to eval mode
            for model_name, model in state.pipeline.models.items():
                if hasattr(model, 'eval'):
                    model.eval()
            logger.info("Pipeline models set to eval mode")
        else:
            logger.warning("Pipeline has no eval() method, skipping")

        # Monkey-patch encode_image to convert inputs to model dtype (bfloat16)
        # This is necessary because TRELLIS hardcodes .float() but we need bfloat16 for RTX 5090
        if hasattr(state.pipeline, 'encode_image'):
            original_encode_image = state.pipeline.encode_image
            def encode_image_bfloat16(image):
                import torch.nn.functional as F
                from PIL import Image as PILImage
                import numpy as np

                # Convert image to tensor if needed
                if isinstance(image, torch.Tensor):
                    if image.ndim == 3:
                        image = image.unsqueeze(0)
                    assert image.ndim == 4, "Image tensor should be batched (B, C, H, W)"
                elif isinstance(image, list):
                    assert all(isinstance(i, PILImage.Image) for i in image), "Image list should be list of PIL images"
                    image = [i.resize((518, 518), PILImage.LANCZOS) for i in image]
                    image = [np.array(i.convert('RGB')).astype(np.float32) / 255 for i in image]
                    image = [torch.from_numpy(i).permute(2, 0, 1) for i in image]
                    image = torch.stack(image).to(device='cuda', dtype=torch.bfloat16)
                else:
                    raise ValueError(f"Unsupported type of image: {type(image)}")

                # Ensure image is bfloat16
                if image.dtype != torch.bfloat16:
                    image = image.to(dtype=torch.bfloat16)

                # Apply transform and ensure bfloat16
                image = state.pipeline.image_cond_model_transform(image).to(device='cuda', dtype=torch.bfloat16)
                features = state.pipeline.models['image_cond_model'](image, is_training=True)['x_prenorm']
                patchtokens = F.layer_norm(features, features.shape[-1:])
                return patchtokens

            state.pipeline.encode_image = encode_image_bfloat16
            logger.info("Patched encode_image to use bfloat16 for RTX 5090 compatibility")

        # Monkey-patch sparse_structure_sampler._inference_model to use bfloat16 timestamps
        # IMPORTANT: Patch instance methods, not the class, to avoid conflicts between samplers
        import types

        if hasattr(state.pipeline, 'sparse_structure_sampler'):
            sampler = state.pipeline.sparse_structure_sampler
            # Find the original _inference_model from the class
            original_inference_model = None
            for cls in sampler.__class__.__mro__:
                if cls.__name__ == 'FlowEulerSampler' and hasattr(cls, '_inference_model'):
                    original_inference_model = cls._inference_model
                    break

            if original_inference_model:
                def inference_model_bfloat16_sparse(self, model, x_t, t, cond=None, **kwargs):
                    # For sparse structure sampling with dense tensors
                    # Note: sparse_structure_flow doesn't accept **kwargs, only (x, t, cond)
                    t_tensor = torch.tensor([1000 * t] * x_t.shape[0], device=x_t.device, dtype=torch.bfloat16)
                    if cond is not None and cond.shape[0] == 1 and x_t.shape[0] > 1:
                        cond = cond.repeat(x_t.shape[0], *([1] * (len(cond.shape) - 1)))
                    with torch.autocast(device_type='cuda', dtype=torch.bfloat16):
                        return model(x_t, t_tensor, cond)

                # Bind to THIS instance only
                sampler._inference_model = types.MethodType(inference_model_bfloat16_sparse, sampler)
                logger.info("Patched sparse_structure_sampler._inference_model (instance method)")

        if hasattr(state.pipeline, 'slat_sampler'):
            sampler = state.pipeline.slat_sampler
            # Find the original _inference_model from the class
            original_inference_model = None
            for cls in sampler.__class__.__mro__:
                if cls.__name__ == 'FlowEulerSampler' and hasattr(cls, '_inference_model'):
                    original_inference_model = cls._inference_model
                    break

            if original_inference_model:
                def inference_model_bfloat16_slat(self, model, x_t, t, cond=None, **kwargs):
                    # For SLAT sampling with sparse tensors
                    if hasattr(x_t, 'features'):
                        batch_size = 1
                        device = x_t.features.device
                    else:
                        batch_size = x_t.shape[0]
                        device = x_t.device

                    t_tensor = torch.tensor([1000 * t] * batch_size, device=device, dtype=torch.bfloat16)
                    if cond is not None and cond.shape[0] == 1 and batch_size > 1:
                        cond = cond.repeat(batch_size, *([1] * (len(cond.shape) - 1)))

                    # Don't pass kwargs to model - only (x, t, cond) are accepted
                    # Guidance params like neg_cond, cfg_strength are handled by sampler wrapper
                    with torch.autocast(device_type='cuda', dtype=torch.bfloat16):
                        return model(x_t, t_tensor, cond)

                # Bind to THIS instance only
                sampler._inference_model = types.MethodType(inference_model_bfloat16_slat, sampler)
                logger.info("Patched slat_sampler._inference_model (instance method)")

        # Patch custom norm layers to handle bfloat16 weights
        # The norm layers (LayerNorm32/GroupNorm32) call x.float() but expect float32 weights
        # We need to patch them to temporarily convert weights to float32 during forward pass
        decoder = state.pipeline.models.get('sparse_structure_decoder')
        if decoder is not None:
            # Set decoder's dtype attribute to bfloat16 to match the actual weights
            decoder.dtype = torch.bfloat16
            decoder.use_fp16 = False  # We're using bfloat16, not float16

            import torch.nn as nn
            from functools import wraps

            norm_layer_count = 0
            for module in decoder.modules():
                # Check if it's a custom norm layer (has the signature of calling x.float())
                if isinstance(module, (nn.LayerNorm, nn.GroupNorm)):
                    # Save original forward
                    original_forward = module.forward

                    def make_patched_forward(orig_forward, mod):
                        @wraps(orig_forward)
                        def patched_forward(x):
                            # Temporarily convert weights/bias to float32 for this forward pass
                            orig_weight_dtype = mod.weight.dtype if mod.weight is not None else None
                            orig_bias_dtype = mod.bias.dtype if mod.bias is not None else None

                            if mod.weight is not None:
                                mod.weight.data = mod.weight.data.float()
                            if mod.bias is not None:
                                mod.bias.data = mod.bias.data.float()

                            # Call original forward (which calls x.float() internally)
                            result = orig_forward(x)

                            # Restore original dtypes
                            if mod.weight is not None and orig_weight_dtype is not None:
                                mod.weight.data = mod.weight.data.to(orig_weight_dtype)
                            if mod.bias is not None and orig_bias_dtype is not None:
                                mod.bias.data = mod.bias.data.to(orig_bias_dtype)

                            return result
                        return patched_forward

                    module.forward = make_patched_forward(original_forward, module)
                    norm_layer_count += 1

            logger.info(f"Patched {norm_layer_count} norm layers to handle bfloat16 weights")

        # Patch sample_sparse_structure to convert sampler output to bfloat16
        # The sampler outputs float16 from autocast, but decoder weights are bfloat16
        original_sample_sparse_structure = state.pipeline.sample_sparse_structure
        def sample_sparse_structure_bfloat16(self, cond, num_samples=1, sampler_params={}):
            # Replicate original logic from trellis_image_to_3d.py line 176-193
            flow_model = self.models['sparse_structure_flow_model']
            reso = flow_model.resolution
            noise = torch.randn(num_samples, flow_model.in_channels, reso, reso, reso).to(self.device)
            sampler_params = {**self.sparse_structure_sampler_params, **sampler_params}

            # Extract neg_cond separately - sampler needs it but flow model doesn't
            # Don't modify original cond dict - make a copy
            neg_cond = cond.get('neg_cond', None)
            cond_without_neg = {k: v for k, v in cond.items() if k != 'neg_cond'}

            z_s = self.sparse_structure_sampler.sample(
                flow_model,
                noise,
                neg_cond=neg_cond,
                **cond_without_neg,
                **sampler_params,
                verbose=True
            ).samples

            # Convert sampler output from float16 to bfloat16 to match decoder weights
            if hasattr(z_s, 'dtype'):
                z_s = z_s.to(dtype=torch.bfloat16)
            elif hasattr(z_s, 'features'):
                # Sparse tensor - convert features
                z_s = z_s.replace(features=z_s.features.to(dtype=torch.bfloat16))

            # Decode occupancy latent - decoder is in bfloat16
            decoder = self.models['sparse_structure_decoder']
            coords = torch.argwhere(decoder(z_s)>0)[:, [0, 2, 3, 4]].int()
            return coords

        state.pipeline.sample_sparse_structure = types.MethodType(
            sample_sparse_structure_bfloat16,
            state.pipeline
        )
        logger.info("Patched sample_sparse_structure to convert sampler output to bfloat16")

        logger.info("Model loading complete")
        # models_loaded is a property based on pipeline being not None
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
    logger.info(f"  SS: cfg_strength={ss_guidance_strength}, steps={ss_sampling_steps}")
    logger.info(f"  SLAT: cfg_strength={shape_slat_guidance_strength}, steps={shape_slat_sampling_steps}")

    # Prepare sampler parameters as dictionaries
    sparse_structure_sampler_params = {
        "steps": ss_sampling_steps,
        "cfg_strength": ss_guidance_strength,
    }

    slat_sampler_params = {
        "steps": shape_slat_sampling_steps,
        "cfg_strength": shape_slat_guidance_strength,
    }

    # Run pipeline - models are already in bfloat16, samplers handle dtype conversion
    # Do NOT set default dtype globally as it breaks custom norm layers
    result = state.pipeline.run(
        pil_image,
        seed=seed,
        sparse_structure_sampler_params=sparse_structure_sampler_params,
        slat_sampler_params=slat_sampler_params,
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
    port = int(os.getenv("TRELLIS_PORT", "3642"))
    workers = int(os.getenv("TRELLIS_WORKERS", "1"))

    logger.info(f"Starting server on {host}:{port} with {workers} workers")

    uvicorn.run(
        "server:app",
        host=host,
        port=port,
        workers=workers,
        log_level="info"
    )
