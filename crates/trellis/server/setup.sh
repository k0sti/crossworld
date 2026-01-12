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

# Check for uv
if ! command -v uv &> /dev/null; then
    echo -e "${RED}✗ uv not found${NC}"
    echo "  Install uv: curl -LsSf https://astral.sh/uv/install.sh | sh"
    echo "  Or via pip: pip install uv"
    exit 1
fi
echo -e "${GREEN}✓ uv found${NC}"

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

    echo ""
fi

# Install Python dependencies
echo -e "${YELLOW}Installing Python dependencies...${NC}"
cd "$SCRIPT_DIR"

# Sync uv dependencies
echo -e "${BLUE}Running uv sync...${NC}"
uv sync

echo -e "${GREEN}✓ Python dependencies installed${NC}"
echo ""

# Configure Trellis Python path
echo -e "${YELLOW}Configuring Trellis...${NC}"
if [ -d "$TRELLIS_PATH" ]; then
    echo -e "${BLUE}Setting up Trellis from $TRELLIS_PATH...${NC}"

    # Check if trellis is importable (using PYTHONPATH)
    if PYTHONPATH="$TRELLIS_PATH:${PYTHONPATH:-}" uv run python -c "from trellis.pipelines import Trellis2ImageTo3DPipeline; print('trellis OK')" 2>/dev/null; then
        echo -e "${GREEN}✓ Trellis is accessible${NC}"
    else
        echo -e "${YELLOW}⚠ Could not import Trellis - you may need to install additional dependencies${NC}"
        echo "  Run TRELLIS setup.sh script: cd $TRELLIS_PATH && . ./setup.sh --basic"
    fi

    # Create a .env file to set PYTHONPATH for the server
    echo "PYTHONPATH=$TRELLIS_PATH" > "$SCRIPT_DIR/.env"
    echo -e "${GREEN}✓ Created .env file with PYTHONPATH${NC}"
else
    echo -e "${YELLOW}⚠ Trellis not found at $TRELLIS_PATH - skipping configuration${NC}"
    echo "  Run without --skip-deps or set TRELLIS_PATH to configure Trellis"
fi
echo ""

# Download model from HuggingFace
echo -e "${YELLOW}Checking model availability...${NC}"
echo -e "${BLUE}Model: $MODEL_NAME${NC}"
echo ""

# Test if model can be loaded (will trigger HuggingFace download if needed)
echo -e "${YELLOW}Testing model loading (this may download ~4GB of data)...${NC}"
if PYTHONPATH="$TRELLIS_PATH:${PYTHONPATH:-}" uv run python -c "
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
    echo "  You may need to install additional Trellis dependencies"
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
echo -e "  ${BLUE}cd crates/trellis/server && uv run server.py${NC}"
echo ""
echo "Server will be available at:"
echo -e "  ${BLUE}http://localhost:8001${NC}"
echo ""
echo "Check health status:"
echo -e "  ${BLUE}curl http://localhost:8001/health${NC}"
echo ""
