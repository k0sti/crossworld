#!/usr/bin/env bash
#
# Robocube (Cube3D) Server Setup Script
#
# Sets up the Cube3D inference server environment:
# - Clones Cube3D repository
# - Installs Python dependencies via uv
#
# Usage: ./setup.sh [--skip-deps] [--cube-path PATH]
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
CUBE_PATH="${CUBE_PATH:-$CROSSWORLD_ROOT/external/cube}"

# Flags
SKIP_DEPS=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-deps)
            SKIP_DEPS=true
            shift
            ;;
        --cube-path)
            CUBE_PATH="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-deps         Skip cloning Cube3D (use existing installation)"
            echo "  --cube-path PATH    Custom path for Cube3D repository"
            echo "  -h, --help          Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Robocube (Cube3D) Server Setup Script              ║${NC}"
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

    # Clone Cube3D
    if [ ! -d "$CUBE_PATH" ]; then
        echo -e "${BLUE}Cloning Cube3D...${NC}"
        git clone https://github.com/Roblox/cube.git "$CUBE_PATH"
        echo -e "${GREEN}✓ Cube3D cloned to $CUBE_PATH${NC}"
    else
        echo -e "${GREEN}✓ Cube3D already exists at $CUBE_PATH${NC}"
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

# Install Cube3D into the virtual environment
echo -e "${YELLOW}Installing Cube3D...${NC}"
if [ -d "$CUBE_PATH" ]; then
    echo -e "${BLUE}Installing Cube3D from $CUBE_PATH...${NC}"

    # Check if cube3d is already installed
    if uv run python -c "import cube3d; print('cube3d OK')" 2>/dev/null; then
        echo -e "${GREEN}✓ Cube3D already installed${NC}"
    else
        # Install cube3d package
        uv pip install -e "$CUBE_PATH"
        echo -e "${GREEN}✓ Cube3D installed${NC}"
    fi
else
    echo -e "${YELLOW}⚠ Cube3D not found at $CUBE_PATH - skipping installation${NC}"
    echo "  Run without --skip-deps or set CUBE_PATH to install Cube3D"
fi
echo ""

# Download model weights from Hugging Face
echo -e "${YELLOW}Downloading model weights from Hugging Face...${NC}"
MODEL_WEIGHTS_DIR="$CUBE_PATH/model_weights"

if [ -f "$MODEL_WEIGHTS_DIR/shape_gpt.safetensors" ] && [ -f "$MODEL_WEIGHTS_DIR/shape_tokenizer.safetensors" ]; then
    echo -e "${GREEN}✓ Model weights already downloaded${NC}"
else
    echo -e "${BLUE}Downloading Roblox/cube3d-v0.5 models...${NC}"
    uv run huggingface-cli download Roblox/cube3d-v0.5 --local-dir "$MODEL_WEIGHTS_DIR"
    echo -e "${GREEN}✓ Model weights downloaded${NC}"
fi
echo ""

# Final status
echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    Setup Complete!                           ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "Start the server with:"
echo -e "  ${BLUE}just robocube-server${NC}"
echo ""
echo "Or manually:"
echo -e "  ${BLUE}cd crates/robocube/server && uv run server.py${NC}"
echo ""
