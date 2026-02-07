#!/usr/bin/env bash
#
# XCube Training Script
#
# Downloads ShapeNet data (if needed) and starts training optimized for 24GB GPU.
#
# Usage: ./train.sh [OPTIONS]
#
# Options:
#   --category CATEGORY   ShapeNet category to train (chair, car, plane) [default: chair]
#   --stage STAGE         Training stage (1 or 2) [default: 1]
#   --model MODEL         Model type (vae or diffusion) [default: vae]
#   --batch-size N        Batch size [default: 4]
#   --accum-steps N       Gradient accumulation steps [default: 8]
#   --max-epochs N        Maximum epochs [default: 100]
#   --precision PREC      Training precision (16, bf16, 32) [default: 16]
#   --resume PATH         Resume from checkpoint
#   --skip-download       Skip dataset download
#   --dry-run             Show training command without executing
#   -h, --help            Show this help message
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CROSSWORLD_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Default paths
XCUBE_PATH="${XCUBE_PATH:-$CROSSWORLD_ROOT/external/XCube}"
DATA_DIR="${XCUBE_DATA_DIR:-$CROSSWORLD_ROOT/data/shapenet}"

# Default training parameters (optimized for 24GB GPU)
CATEGORY="chair"
STAGE="1"
MODEL="vae"
BATCH_SIZE="4"
ACCUM_STEPS="8"
MAX_EPOCHS="100"
PRECISION="16"
RESUME=""
SKIP_DOWNLOAD=false
DRY_RUN=false
NO_WANDB=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --category)
            CATEGORY="$2"
            shift 2
            ;;
        --stage)
            STAGE="$2"
            shift 2
            ;;
        --model)
            MODEL="$2"
            shift 2
            ;;
        --batch-size)
            BATCH_SIZE="$2"
            shift 2
            ;;
        --accum-steps)
            ACCUM_STEPS="$2"
            shift 2
            ;;
        --max-epochs)
            MAX_EPOCHS="$2"
            shift 2
            ;;
        --precision)
            PRECISION="$2"
            shift 2
            ;;
        --resume)
            RESUME="$2"
            shift 2
            ;;
        --skip-download)
            SKIP_DOWNLOAD=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --no-wandb)
            NO_WANDB=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "XCube Training Script - Train XCube models on ShapeNet data"
            echo ""
            echo "Options:"
            echo "  --category CATEGORY   ShapeNet category (chair, car, plane) [default: chair]"
            echo "  --stage STAGE         Training stage (1=coarse 16³, 2=fine 128³) [default: 1]"
            echo "  --model MODEL         Model type (vae, diffusion) [default: vae]"
            echo "  --batch-size N        Batch size per GPU [default: 4]"
            echo "  --accum-steps N       Gradient accumulation steps [default: 8]"
            echo "  --max-epochs N        Maximum training epochs [default: 100]"
            echo "  --precision PREC      Training precision (16, bf16, 32) [default: 16]"
            echo "  --resume PATH         Resume training from checkpoint"
            echo "  --skip-download       Skip dataset download"
            echo "  --dry-run             Show command without executing"
            echo "  --no-wandb            Disable Weights & Biases logging"
            echo "  -h, --help            Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                                    # Train Stage 1 VAE on chairs"
            echo "  $0 --category car --model diffusion   # Train diffusion on cars"
            echo "  $0 --stage 2 --batch-size 1           # Train Stage 2 (requires more VRAM)"
            echo ""
            echo "Environment Variables:"
            echo "  XCUBE_PATH      Path to XCube repo [default: \$CROSSWORLD_ROOT/external/XCube]"
            echo "  XCUBE_DATA_DIR  Path to ShapeNet data [default: \$CROSSWORLD_ROOT/data/shapenet]"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                  XCube Training Script                       ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Validate category
if [[ ! "$CATEGORY" =~ ^(chair|car|plane)$ ]]; then
    echo -e "${RED}✗ Invalid category: $CATEGORY${NC}"
    echo "  Valid categories: chair, car, plane"
    exit 1
fi

# Validate stage
if [[ ! "$STAGE" =~ ^(1|2)$ ]]; then
    echo -e "${RED}✗ Invalid stage: $STAGE${NC}"
    echo "  Valid stages: 1 (coarse), 2 (fine)"
    exit 1
fi

# Validate model
if [[ ! "$MODEL" =~ ^(vae|diffusion)$ ]]; then
    echo -e "${RED}✗ Invalid model: $MODEL${NC}"
    echo "  Valid models: vae, diffusion"
    exit 1
fi

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

# Check for XCube
if [ ! -d "$XCUBE_PATH" ]; then
    echo -e "${RED}✗ XCube not found at $XCUBE_PATH${NC}"
    echo "  Run 'just xcube-setup' first to set up XCube"
    exit 1
fi
echo -e "${GREEN}✓ XCube found at $XCUBE_PATH${NC}"

# Check for XCube training venv and dependencies
XCUBE_VENV="$XCUBE_PATH/.venv"
if [ ! -d "$XCUBE_VENV" ]; then
    echo -e "${YELLOW}Creating XCube training environment...${NC}"
    uv venv "$XCUBE_VENV" --python 3.11
fi

# Check if training dependencies are installed
FVDB_PATH="${FVDB_PATH:-$CROSSWORLD_ROOT/external/fVDB}"

if ! "$XCUBE_VENV/bin/python" -c "import omegaconf" 2>/dev/null; then
    echo -e "${YELLOW}Installing XCube training dependencies...${NC}"
    # Use uv pip to install into the venv
    VIRTUAL_ENV="$XCUBE_VENV" uv pip install \
        torch torchvision --index-url https://download.pytorch.org/whl/cu128
    VIRTUAL_ENV="$XCUBE_VENV" uv pip install \
        omegaconf \
        "pytorch-lightning==1.9.4" \
        wandb \
        loguru \
        pyyaml \
        packaging \
        numpy \
        scipy \
        trimesh \
        open3d \
        einops \
        transformers \
        diffusers \
        accelerate \
        python-pycg \
        flatten-dict \
        rich \
        tqdm \
        matplotlib \
        randomname
    # torch-scatter requires torch at build time, so install with --no-build-isolation
    VIRTUAL_ENV="$XCUBE_VENV" uv pip install --no-build-isolation torch-scatter
    echo -e "${GREEN}✓ XCube training dependencies installed${NC}"
fi

# Check if fVDB is installed (required for training)
if ! "$XCUBE_VENV/bin/python" -c "import fvdb" 2>/dev/null; then
    echo -e "${YELLOW}Installing fVDB (this may take several minutes)...${NC}"
    if [ ! -d "$FVDB_PATH/fvdb" ]; then
        echo -e "${RED}✗ fVDB not found at $FVDB_PATH${NC}"
        echo "  Run 'just xcube-setup' first to clone and configure fVDB"
        exit 1
    fi

    # Check for nvcc (required for fVDB build)
    if ! command -v nvcc &> /dev/null; then
        echo -e "${RED}✗ CUDA toolkit (nvcc) not found${NC}"
        echo "  fVDB requires CUDA development toolkit to compile."
        echo "  On NixOS: nix develop .#cuda"
        echo "  On Ubuntu: sudo apt install nvidia-cuda-toolkit"
        exit 1
    fi

    # Set CUDA_HOME if not set
    if [ -z "$CUDA_HOME" ]; then
        NVCC_PATH=$(which nvcc)
        export CUDA_HOME=$(dirname $(dirname "$NVCC_PATH"))
        echo -e "${BLUE}Auto-detected CUDA_HOME: $CUDA_HOME${NC}"
    fi

    # Install build dependencies
    VIRTUAL_ENV="$XCUBE_VENV" uv pip install setuptools cmake ninja gitpython

    # Set TORCH_CUDA_ARCH_LIST for GPU
    GPU_COMPUTE_CAP=$(nvidia-smi --query-gpu=compute_cap --format=csv,noheader 2>/dev/null | head -1 | tr -d '.')
    if [ -n "$GPU_COMPUTE_CAP" ]; then
        case "$GPU_COMPUTE_CAP" in
            120) ARCH_LIST="8.9+PTX" ;;
            *)   ARCH_LIST="${GPU_COMPUTE_CAP:0:1}.${GPU_COMPUTE_CAP:1}+PTX" ;;
        esac
        export TORCH_CUDA_ARCH_LIST="$ARCH_LIST"
        echo -e "${BLUE}Set TORCH_CUDA_ARCH_LIST=$ARCH_LIST${NC}"
    fi

    # Install fVDB
    VIRTUAL_ENV="$XCUBE_VENV" uv pip install --no-build-isolation "$FVDB_PATH/fvdb"
    echo -e "${GREEN}✓ fVDB installed${NC}"
else
    echo -e "${GREEN}✓ XCube training environment ready${NC}"
fi

# Check for CUDA
if ! command -v nvidia-smi &> /dev/null; then
    echo -e "${RED}✗ nvidia-smi not found - GPU required for training${NC}"
    exit 1
fi

GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1)
GPU_MEM=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader 2>/dev/null | head -1)
echo -e "${GREEN}✓ GPU detected: $GPU_NAME ($GPU_MEM)${NC}"

# Check Python environment
if ! command -v uv &> /dev/null; then
    echo -e "${RED}✗ uv not found${NC}"
    echo "  Install uv: curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi
echo -e "${GREEN}✓ uv found${NC}"

echo ""

# Download dataset if needed
if [ "$SKIP_DOWNLOAD" = false ]; then
    echo -e "${YELLOW}Checking ShapeNet dataset...${NC}"

    # Marker file to indicate complete download
    DOWNLOAD_COMPLETE_MARKER="$DATA_DIR/.download_complete"

    # Check if download is complete (marker file exists)
    if [ -f "$DOWNLOAD_COMPLETE_MARKER" ]; then
        echo -e "${GREEN}✓ ShapeNet data found at $DATA_DIR (verified complete)${NC}"
    else
        if [ -d "$DATA_DIR" ] && [ "$(ls -A "$DATA_DIR" 2>/dev/null)" ]; then
            echo -e "${YELLOW}⚠ Partial download detected at $DATA_DIR${NC}"
            echo -e "${BLUE}Resuming download...${NC}"
        else
            echo -e "${BLUE}Downloading ShapeNet dataset from HuggingFace...${NC}"
            mkdir -p "$DATA_DIR"
        fi

        echo -e "${YELLOW}Note: Dataset is ~224GB, this may take a while${NC}"
        echo -e "${YELLOW}If rate-limited, the script will retry automatically${NC}"
        echo ""

        # Download with retry logic for rate limiting
        MAX_RETRIES=10
        RETRY_COUNT=0
        DOWNLOAD_SUCCESS=false

        while [ "$DOWNLOAD_SUCCESS" = false ] && [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
            cd "$SCRIPT_DIR"

            # Use Python with huggingface_hub (supports resume automatically)
            # Verify download by comparing local file count with expected count from API
            if uv run python -c "
import sys
import os
import warnings
from pathlib import Path
from huggingface_hub import snapshot_download, list_repo_files, HfApi
from huggingface_hub.utils import HfHubHTTPError

DATA_DIR = '$DATA_DIR'
REPO_ID = 'xrenaa/XCube-Shapenet-Dataset'

# Suppress the 'returning local dir' warning - we'll verify ourselves
warnings.filterwarnings('ignore', message='.*remote repo cannot be accessed.*')

print('Downloading XCube-Shapenet-Dataset (resumable)...')
try:
    # First, get expected file count from the API
    api = HfApi()
    try:
        repo_files = list_repo_files(REPO_ID, repo_type='dataset')
        expected_count = len([f for f in repo_files if not f.startswith('.')])
        print(f'Expected files: {expected_count}')
    except HfHubHTTPError as e:
        if '429' in str(e):
            print('Rate limited while checking file list. Will retry...', file=sys.stderr)
            sys.exit(42)
        raise

    # Download/resume
    snapshot_download(
        repo_id=REPO_ID,
        repo_type='dataset',
        local_dir=DATA_DIR,
        local_dir_use_symlinks=False,
        resume_download=True,
        max_workers=4,
    )

    # Verify download is complete by counting files
    local_files = list(Path(DATA_DIR).rglob('*'))
    local_file_count = len([f for f in local_files if f.is_file() and not f.name.startswith('.')])
    print(f'Local files: {local_file_count}')

    if local_file_count < expected_count:
        print(f'Download incomplete: {local_file_count}/{expected_count} files', file=sys.stderr)
        sys.exit(43)  # Special exit code for incomplete download

    print('Download complete and verified!')
    sys.exit(0)

except HfHubHTTPError as e:
    if '429' in str(e):
        print('Rate limited. Will retry after cooldown...', file=sys.stderr)
        sys.exit(42)
    else:
        print(f'Download error: {e}', file=sys.stderr)
        sys.exit(1)
except Exception as e:
    print(f'Download error: {e}', file=sys.stderr)
    sys.exit(1)
"; then
                DOWNLOAD_SUCCESS=true
            else
                EXIT_CODE=$?
                if [ $EXIT_CODE -eq 42 ] || [ $EXIT_CODE -eq 43 ]; then
                    RETRY_COUNT=$((RETRY_COUNT + 1))
                    if [ $EXIT_CODE -eq 42 ]; then
                        WAIT_TIME=$((60 * RETRY_COUNT))  # Exponential backoff: 1min, 2min, 3min...
                        if [ $WAIT_TIME -gt 300 ]; then
                            WAIT_TIME=300  # Cap at 5 minutes
                        fi
                        echo -e "${YELLOW}Rate limited (attempt $RETRY_COUNT/$MAX_RETRIES). Waiting ${WAIT_TIME}s before retry...${NC}"
                        sleep $WAIT_TIME
                    else
                        # Exit code 43: incomplete download, retry immediately
                        echo -e "${YELLOW}Download incomplete (attempt $RETRY_COUNT/$MAX_RETRIES). Retrying...${NC}"
                        sleep 5  # Brief pause before retry
                    fi
                else
                    echo -e "${RED}✗ Download failed with error code $EXIT_CODE${NC}"
                    exit 1
                fi
            fi
        done

        if [ "$DOWNLOAD_SUCCESS" = true ]; then
            # Create marker file to indicate complete download
            touch "$DOWNLOAD_COMPLETE_MARKER"
            echo -e "${GREEN}✓ ShapeNet dataset downloaded to $DATA_DIR${NC}"
        else
            echo -e "${RED}✗ Download failed after $MAX_RETRIES retries${NC}"
            echo "  Try again later or download manually from:"
            echo "  https://huggingface.co/datasets/xrenaa/XCube-Shapenet-Dataset"
            exit 1
        fi
    fi
else
    echo -e "${YELLOW}Skipping dataset download (--skip-download)${NC}"
    if [ ! -d "$DATA_DIR" ]; then
        echo -e "${RED}✗ Data directory not found: $DATA_DIR${NC}"
        exit 1
    fi
fi

echo ""

# Determine config file based on stage and model
if [ "$STAGE" = "1" ]; then
    RESOLUTION="16x16x16"
    DENSITY="dense"
else
    RESOLUTION="128x128x128"
    DENSITY="sparse"
fi

CONFIG_FILE="configs/shapenet/${CATEGORY}/train_${MODEL}_${RESOLUTION}_${DENSITY}.yaml"

echo -e "${YELLOW}Training Configuration:${NC}"
echo "  Category:      $CATEGORY"
echo "  Stage:         $STAGE ($RESOLUTION $DENSITY)"
echo "  Model:         $MODEL"
echo "  Config:        $CONFIG_FILE"
echo "  Batch size:    $BATCH_SIZE"
echo "  Accum steps:   $ACCUM_STEPS"
echo "  Effective BS:  $((BATCH_SIZE * ACCUM_STEPS))"
echo "  Max epochs:    $MAX_EPOCHS"
echo "  Precision:     $PRECISION"
echo "  Data dir:      $DATA_DIR"
if [ -n "$RESUME" ]; then
    echo "  Resume from:   $RESUME"
fi
echo ""

# Build training command (use XCube's venv)
TRAIN_CMD="$XCUBE_VENV/bin/python train.py $CONFIG_FILE"
TRAIN_CMD="$TRAIN_CMD --gpus 1"
TRAIN_CMD="$TRAIN_CMD --batch_size $BATCH_SIZE"
TRAIN_CMD="$TRAIN_CMD --accumulate_grad_batches $ACCUM_STEPS"
TRAIN_CMD="$TRAIN_CMD --max_epochs $MAX_EPOCHS"

# Add precision flag
if [ "$PRECISION" = "16" ]; then
    TRAIN_CMD="$TRAIN_CMD --precision 16-mixed"
elif [ "$PRECISION" = "bf16" ]; then
    TRAIN_CMD="$TRAIN_CMD --precision bf16-mixed"
fi

# Add wandb name or disable logging
WNAME="${CATEGORY}_${MODEL}_${RESOLUTION}_bs${BATCH_SIZE}x${ACCUM_STEPS}"
if [ "$NO_WANDB" = true ]; then
    TRAIN_CMD="$TRAIN_CMD --logger_type none"
else
    TRAIN_CMD="$TRAIN_CMD --wname $WNAME"
fi

# Add resume checkpoint if specified
if [ -n "$RESUME" ]; then
    TRAIN_CMD="$TRAIN_CMD --resume $RESUME"
fi

# Add data path override (XCube uses _shapenet_path)
TRAIN_CMD="$TRAIN_CMD --_shapenet_path $DATA_DIR"

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}Training command:${NC}"
echo ""
echo "  cd $XCUBE_PATH && \\"
echo "  PYTHONPATH=. $TRAIN_CMD"
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"

if [ "$DRY_RUN" = true ]; then
    echo ""
    echo -e "${YELLOW}Dry run mode - command not executed${NC}"
    exit 0
fi

echo ""
echo -e "${GREEN}Starting training...${NC}"
echo ""

# Change to XCube directory and run training
cd "$XCUBE_PATH"

# Set up environment
export PYTHONPATH="$XCUBE_PATH:${PYTHONPATH:-}"

# Add NVIDIA library path for NixOS
export LD_LIBRARY_PATH="/run/opengl-driver/lib:${LD_LIBRARY_PATH:-}"

# Run training
exec $TRAIN_CMD
