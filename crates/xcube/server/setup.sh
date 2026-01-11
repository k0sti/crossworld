#!/usr/bin/env bash
#
# XCube Server Setup Script
#
# Sets up the XCube inference server environment:
# - Clones XCube and fVDB repositories
# - Installs Python dependencies via uv
# - Creates checkpoint directory structure
#
# Usage: ./setup.sh [--skip-deps] [--xcube-path PATH] [--fvdb-path PATH]
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
XCUBE_PATH="${XCUBE_PATH:-$CROSSWORLD_ROOT/external/XCube}"
FVDB_PATH="${FVDB_PATH:-$CROSSWORLD_ROOT/external/fVDB}"
CHECKPOINT_DIR="${XCUBE_CHECKPOINT_DIR:-$SCRIPT_DIR/checkpoints}"

# Flags
SKIP_DEPS=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-deps)
            SKIP_DEPS=true
            shift
            ;;
        --xcube-path)
            XCUBE_PATH="$2"
            shift 2
            ;;
        --fvdb-path)
            FVDB_PATH="$2"
            shift 2
            ;;
        --checkpoint-dir)
            CHECKPOINT_DIR="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-deps         Skip cloning XCube/fVDB (use existing installations)"
            echo "  --xcube-path PATH   Custom path for XCube repository"
            echo "  --fvdb-path PATH    Custom path for fVDB repository"
            echo "  --checkpoint-dir    Custom path for model checkpoints"
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
echo -e "${BLUE}║              XCube Server Setup Script                       ║${NC}"
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

    # Clone fVDB
    if [ ! -d "$FVDB_PATH" ]; then
        echo -e "${BLUE}Cloning fVDB...${NC}"
        git clone https://github.com/nv-tlabs/fVDB.git "$FVDB_PATH"
        echo -e "${GREEN}✓ fVDB cloned to $FVDB_PATH${NC}"
    else
        echo -e "${GREEN}✓ fVDB already exists at $FVDB_PATH${NC}"
    fi

    # Clone XCube
    if [ ! -d "$XCUBE_PATH" ]; then
        echo -e "${BLUE}Cloning XCube...${NC}"
        git clone https://github.com/nv-tlabs/XCube.git "$XCUBE_PATH"
        echo -e "${GREEN}✓ XCube cloned to $XCUBE_PATH${NC}"
    else
        echo -e "${GREEN}✓ XCube already exists at $XCUBE_PATH${NC}"
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

# Install fVDB into the virtual environment
echo -e "${YELLOW}Installing fVDB...${NC}"
if [ -d "$FVDB_PATH" ]; then
    echo -e "${BLUE}Installing fVDB from $FVDB_PATH...${NC}"
    echo -e "${YELLOW}Note: fVDB compilation requires CUDA toolkit and may take several minutes${NC}"

    # Check if fvdb is already installed
    if uv run python -c "import fvdb; print('fvdb version:', fvdb.__version__)" 2>/dev/null; then
        echo -e "${GREEN}✓ fVDB already installed${NC}"
    else
        cd "$FVDB_PATH"
        uv pip install .
        cd "$SCRIPT_DIR"
        echo -e "${GREEN}✓ fVDB installed${NC}"
    fi
else
    echo -e "${YELLOW}⚠ fVDB not found at $FVDB_PATH - skipping installation${NC}"
    echo "  Run without --skip-deps or set FVDB_PATH to install fVDB"
fi
echo ""

# Install XCube into the virtual environment
echo -e "${YELLOW}Installing XCube...${NC}"
if [ -d "$XCUBE_PATH" ]; then
    echo -e "${BLUE}Installing XCube from $XCUBE_PATH...${NC}"

    # Check if xcube is already installed
    if uv run python -c "from xcube.models.model import create_model_from_args; print('xcube OK')" 2>/dev/null; then
        echo -e "${GREEN}✓ XCube already installed${NC}"
    else
        cd "$XCUBE_PATH"
        uv pip install -e .
        cd "$SCRIPT_DIR"
        echo -e "${GREEN}✓ XCube installed${NC}"
    fi
else
    echo -e "${YELLOW}⚠ XCube not found at $XCUBE_PATH - skipping installation${NC}"
    echo "  Run without --skip-deps or set XCUBE_PATH to install XCube"
fi
echo ""

# Create checkpoint directory structure
echo -e "${YELLOW}Setting up checkpoint directory...${NC}"
mkdir -p "$CHECKPOINT_DIR/objaverse_coarse"
mkdir -p "$CHECKPOINT_DIR/objaverse_fine"
echo -e "${GREEN}✓ Checkpoint directory created at $CHECKPOINT_DIR${NC}"

# Check for existing checkpoints
COARSE_CKPT="$CHECKPOINT_DIR/objaverse_coarse/last.ckpt"
FINE_CKPT="$CHECKPOINT_DIR/objaverse_fine/last.ckpt"

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

if [ -f "$COARSE_CKPT" ]; then
    echo -e "${GREEN}✓ Coarse checkpoint found${NC}"
else
    echo -e "${YELLOW}⚠ Coarse checkpoint not found${NC}"
fi

if [ -f "$FINE_CKPT" ]; then
    echo -e "${GREEN}✓ Fine checkpoint found (optional)${NC}"
else
    echo -e "${YELLOW}○ Fine checkpoint not found (optional, for higher quality)${NC}"
fi

echo ""

# Final status
if [ -f "$COARSE_CKPT" ]; then
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    Setup Complete!                           ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Start the server with:"
    echo -e "  ${BLUE}just xcube-server${NC}"
    echo ""
    echo "Or manually:"
    echo -e "  ${BLUE}cd crates/xcube/server && uv run server.py${NC}"
else
    echo -e "${YELLOW}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${YELLOW}║              Setup Partially Complete                        ║${NC}"
    echo -e "${YELLOW}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}Action Required: Download model checkpoints${NC}"
    echo ""
    echo "1. Download checkpoints from Google Drive:"
    echo -e "   ${BLUE}https://drive.google.com/drive/folders/1M7K0eLm6aLGIW6wvHpTNQh6hd4s8BkN0${NC}"
    echo ""
    echo "2. Place files in the following structure:"
    echo -e "   ${CHECKPOINT_DIR}/"
    echo "   ├── objaverse_coarse/"
    echo "   │   ├── config.yaml"
    echo "   │   └── last.ckpt"
    echo "   └── objaverse_fine/        (optional)"
    echo "       ├── config.yaml"
    echo "       └── last.ckpt"
    echo ""
    echo "3. Then start the server:"
    echo -e "   ${BLUE}just xcube-server${NC}"
fi

echo ""
