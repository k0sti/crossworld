#!/usr/bin/env bash
#
# Trellis.2 Server Setup Script
#
# Sets up the Trellis.2 inference server environment:
# - Clones Trellis repository
# - Installs Python dependencies via uv
# - Downloads model from HuggingFace
#
# Usage: ./setup.sh [--skip-deps] [--trellis-path PATH]
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

# Default paths (can be overridden via environment or arguments)
TRELLIS_PATH="${TRELLIS_PATH:-$CROSSWORLD_ROOT/external/TRELLIS}"
MODEL_NAME="${TRELLIS_MODEL_PATH:-microsoft/TRELLIS.2-4B}"

# Flags
SKIP_DEPS=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-deps)
            SKIP_DEPS=true
            shift
            ;;
        --trellis-path)
            TRELLIS_PATH="$2"
            shift 2
            ;;
        --model)
            MODEL_NAME="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-deps           Skip cloning Trellis (use existing installation)"
            echo "  --trellis-path PATH   Custom path for Trellis repository"
            echo "  --model NAME          Model name from HuggingFace (default: microsoft/TRELLIS.2-4B)"
            echo "  -h, --help            Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Trellis.2 Server Setup Script                     ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

# Check for conda
if ! command -v conda &> /dev/null; then
    echo -e "${RED}✗ conda not found${NC}"
    echo "  Install Miniconda: https://docs.anaconda.com/miniconda/install/"
    echo "  Or Anaconda: https://www.anaconda.com/download"
    exit 1
fi
echo -e "${GREEN}✓ conda found${NC}"

# Check for git
if ! command -v git &> /dev/null; then
    echo -e "${RED}✗ git not found${NC}"
    exit 1
fi
echo -e "${GREEN}✓ git found${NC}"

# Check for CUDA (optional but recommended)
if command -v nvidia-smi &> /dev/null; then
    GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1)
    echo -e "${GREEN}✓ NVIDIA GPU detected: $GPU_NAME${NC}"
else
    echo -e "${YELLOW}⚠ nvidia-smi not found - GPU inference may not work${NC}"
fi

echo ""

# Clone external dependencies
if [ "$SKIP_DEPS" = false ]; then
    echo -e "${YELLOW}Setting up external dependencies...${NC}"

    # Create external directory
    mkdir -p "$CROSSWORLD_ROOT/external"

    # Clone Trellis
    if [ ! -d "$TRELLIS_PATH" ]; then
        echo -e "${BLUE}Cloning Trellis...${NC}"
        git clone https://github.com/microsoft/TRELLIS.git "$TRELLIS_PATH"
        echo -e "${GREEN}✓ Trellis cloned to $TRELLIS_PATH${NC}"
    else
        echo -e "${GREEN}✓ Trellis already exists at $TRELLIS_PATH${NC}"
    fi

    # Initialize git submodules (needed for flexicubes)
    echo -e "${BLUE}Initializing submodules...${NC}"
    cd "$TRELLIS_PATH"
    git submodule update --init --recursive
    cd - > /dev/null
    echo -e "${GREEN}✓ Submodules initialized${NC}"

    echo ""
fi

# Install Trellis with conda
echo -e "${YELLOW}Installing Trellis and dependencies with conda...${NC}"
if [ -d "$TRELLIS_PATH" ]; then
    echo -e "${BLUE}Running TRELLIS setup.sh...${NC}"

    # Initialize conda in this shell if not already done
    # This is required for TRELLIS setup.sh to activate environments
    if [ -f "$HOME/.local/share/miniconda3/etc/profile.d/conda.sh" ]; then
        source "$HOME/.local/share/miniconda3/etc/profile.d/conda.sh"
    fi

    # Navigate to TRELLIS directory
    cd "$TRELLIS_PATH"

    # Check if trellis conda environment already exists and has PyTorch
    if conda env list | grep -q "^trellis "; then
        echo -e "${GREEN}✓ Conda environment 'trellis' already exists${NC}"

        # Check if PyTorch is installed in the environment
        if conda run -n trellis python -c "import torch" 2>/dev/null; then
            echo -e "${GREEN}✓ PyTorch is installed${NC}"
            echo -e "${BLUE}Activating and updating environment...${NC}"
            # Activate environment and source the setup script
            # --basic: Install basic dependencies (rembg, etc.)
            # --xformers: Install xformers (may fail on CPU-only)
            # --kaolin: Install kaolin (required for flexicubes)
            conda activate trellis
            . ./setup.sh --basic --xformers --kaolin
            conda deactivate
        else
            echo -e "${YELLOW}⚠ PyTorch not found - installing with pip${NC}"
            # Install PyTorch via pip (conda version has iJIT_NotifyEvent issues on NixOS)
            echo -e "${BLUE}Installing PyTorch 2.4.0+cu118 via pip...${NC}"
            conda run -n trellis pip install torch==2.4.0 torchvision==0.19.0 --index-url https://download.pytorch.org/whl/cu118
            echo -e "${BLUE}Running TRELLIS setup for remaining dependencies...${NC}"
            conda activate trellis
            . ./setup.sh --basic --xformers --kaolin
            conda deactivate

            # Fix numpy/opencv version conflicts (kaolin needs numpy<2.0)
            echo -e "${BLUE}Fixing numpy/opencv compatibility...${NC}"
            conda run -n trellis pip install 'numpy<2.0' 'opencv-python-headless<4.8' --force-reinstall -q

            # Install xformers compatible with PyTorch 2.4.0
            echo -e "${BLUE}Installing xformers 0.0.27.post2...${NC}"
            conda run -n trellis pip install xformers==0.0.27.post2 --no-deps -q

            echo -e "${GREEN}✓ Dependencies fixed${NC}"
        fi
    else
        echo -e "${BLUE}Creating new conda environment 'trellis'...${NC}"
        echo -e "${YELLOW}This may take 10-20 minutes to download and install packages${NC}"

        # Create environment with just Python (not PyTorch from conda)
        conda create -n trellis python=3.10 -y

        # Install PyTorch via pip (conda version has iJIT_NotifyEvent issues on NixOS)
        echo -e "${BLUE}Installing PyTorch 2.4.0+cu118 via pip...${NC}"
        conda run -n trellis pip install torch==2.4.0 torchvision==0.19.0 --index-url https://download.pytorch.org/whl/cu118

        # Now run TRELLIS setup for remaining dependencies (skip --new-env since env exists)
        echo -e "${BLUE}Running TRELLIS setup for remaining dependencies...${NC}"
        conda activate trellis
        . ./setup.sh --basic --xformers --kaolin
        conda deactivate

        # Fix numpy/opencv version conflicts (kaolin needs numpy<2.0)
        echo -e "${BLUE}Fixing numpy/opencv compatibility...${NC}"
        conda run -n trellis pip install 'numpy<2.0' 'opencv-python-headless<4.8' --force-reinstall -q

        # Install xformers compatible with PyTorch 2.4.0
        echo -e "${BLUE}Installing xformers 0.0.27.post2...${NC}"
        conda run -n trellis pip install xformers==0.0.27.post2 --no-deps -q

        echo -e "${GREEN}✓ Dependencies fixed${NC}"
    fi

    cd "$SCRIPT_DIR"
    echo -e "${GREEN}✓ Trellis environment configured${NC}"
else
    echo -e "${RED}✗ Trellis not found at $TRELLIS_PATH${NC}"
    echo "  Cannot proceed without TRELLIS repository"
    exit 1
fi
echo ""

# Download model from HuggingFace
echo -e "${YELLOW}Checking model availability...${NC}"
echo -e "${BLUE}Model: $MODEL_NAME${NC}"
echo ""

# Test if model can be loaded (will trigger HuggingFace download if needed)
echo -e "${YELLOW}Testing model loading (this may download ~4GB of data)...${NC}"
if conda run -n trellis python -c "
from trellis.pipelines import Trellis2ImageTo3DPipeline
import torch
print('Loading pipeline...')
pipeline = Trellis2ImageTo3DPipeline.from_pretrained('$MODEL_NAME')
print('Pipeline loaded successfully')
" 2>&1; then
    echo -e "${GREEN}✓ Model loaded successfully${NC}"
else
    echo -e "${YELLOW}⚠ Model loading test failed - server may fail to start${NC}"
    echo "  Check your internet connection and HuggingFace access"
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Final status
echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    Setup Complete!                           ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "Start the server with:"
echo -e "  ${BLUE}just trellis-server${NC}"
echo ""
echo "Or manually:"
echo -e "  ${BLUE}LD_LIBRARY_PATH=/run/opengl-driver/lib:\$LD_LIBRARY_PATH conda run -n trellis --no-capture-output python crates/trellis/server/server.py${NC}"
echo ""
echo "Server will be available at:"
echo -e "  ${BLUE}http://localhost:3642${NC}"
echo ""
echo "Check health status:"
echo -e "  ${BLUE}curl http://localhost:3642/health${NC}"
echo ""
echo "Conda environment: ${BLUE}trellis${NC}"
echo -e "${YELLOW}Note: Set LD_LIBRARY_PATH=/run/opengl-driver/lib for CUDA support${NC}"
echo ""
