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

    # Clone XCube first (use SSH URL for better auth handling)
    if [ ! -d "$XCUBE_PATH" ]; then
        echo -e "${BLUE}Cloning XCube...${NC}"
        git clone git@github.com:nv-tlabs/XCube.git "$XCUBE_PATH"
        echo -e "${GREEN}✓ XCube cloned to $XCUBE_PATH${NC}"
    else
        echo -e "${GREEN}✓ XCube already exists at $XCUBE_PATH${NC}"
    fi

    # Clone fVDB from OpenVDB repository (fVDB is a feature branch)
    if [ ! -d "$FVDB_PATH" ]; then
        echo -e "${BLUE}Cloning OpenVDB (for fVDB feature branch)...${NC}"
        git clone git@github.com:AcademySoftwareFoundation/openvdb.git "$FVDB_PATH"
        cd "$FVDB_PATH"
        echo -e "${BLUE}Fetching fVDB feature branch...${NC}"
        git fetch origin pull/1808/head:feature/fvdb
        git checkout feature/fvdb
        # Replace setup.py with XCube's version
        if [ -f "$XCUBE_PATH/assets/setup.py" ]; then
            rm -f fvdb/setup.py
            cp "$XCUBE_PATH/assets/setup.py" fvdb/
            echo -e "${GREEN}✓ Patched fVDB setup.py from XCube${NC}"
        fi
        cd "$SCRIPT_DIR"
        echo -e "${GREEN}✓ fVDB cloned and configured at $FVDB_PATH${NC}"
    else
        echo -e "${GREEN}✓ fVDB already exists at $FVDB_PATH${NC}"
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
if [ -d "$FVDB_PATH/fvdb" ]; then
    echo -e "${BLUE}Installing fVDB from $FVDB_PATH/fvdb...${NC}"
    echo -e "${YELLOW}Note: fVDB compilation requires CUDA toolkit and may take several minutes${NC}"

    # Check if fvdb is already installed
    if uv run python -c "import fvdb; print('fvdb version:', fvdb.__version__)" 2>/dev/null; then
        echo -e "${GREEN}✓ fVDB already installed${NC}"
    else
        # Check for CUDA toolkit (nvcc is required for building)
        if ! command -v nvcc &> /dev/null; then
            echo -e "${RED}✗ CUDA toolkit (nvcc) not found${NC}"
            echo ""
            echo -e "${YELLOW}fVDB requires the CUDA development toolkit to compile.${NC}"
            echo ""
            echo "On NixOS (with this project's flake), enter the CUDA shell:"
            echo -e "  ${BLUE}nix develop .#cuda${NC}"
            echo ""
            echo "Then run this script again."
            echo ""
            echo "On Ubuntu/Debian:"
            echo -e "  ${BLUE}sudo apt install nvidia-cuda-toolkit${NC}"
            echo ""
            echo -e "${YELLOW}Skipping fVDB installation...${NC}"
        else
            # CUDA toolkit found, check CUDA_HOME
            if [ -z "$CUDA_HOME" ]; then
                # Try to auto-detect CUDA_HOME from nvcc location
                NVCC_PATH=$(which nvcc)
                export CUDA_HOME=$(dirname $(dirname "$NVCC_PATH"))
                echo -e "${BLUE}Auto-detected CUDA_HOME: $CUDA_HOME${NC}"
            fi

            # Install build dependencies first (fVDB setup.py needs these but doesn't declare them)
            echo -e "${BLUE}Installing fVDB build dependencies...${NC}"
            uv pip install setuptools requests cmake ninja gitpython

            # Patch c-blosc CMakeLists.txt for modern CMake compatibility
            # CMake 3.30+ removed support for cmake_minimum_required < 3.5
            BLOSC_CMAKE="$FVDB_PATH/fvdb/external/c-blosc/CMakeLists.txt"
            if grep -q "VERSION 2.8.12" "$BLOSC_CMAKE" 2>/dev/null; then
                echo -e "${BLUE}Patching c-blosc CMakeLists.txt for modern CMake...${NC}"
                sed -i 's/cmake_minimum_required(VERSION 2.8.12)/cmake_minimum_required(VERSION 3.5)/' "$BLOSC_CMAKE"
            fi

            # Set CUDA architecture for PyTorch extensions
            # Detect GPU compute capability and map to known architectures
            GPU_COMPUTE_CAP=$(nvidia-smi --query-gpu=compute_cap --format=csv,noheader 2>/dev/null | head -1 | tr -d '.')
            if [ -n "$GPU_COMPUTE_CAP" ]; then
                # Map newer architectures to supported ones with PTX fallback
                case "$GPU_COMPUTE_CAP" in
                    120) ARCH_LIST="8.9+PTX" ;;  # Blackwell -> Ada PTX fallback
                    *)   ARCH_LIST="${GPU_COMPUTE_CAP:0:1}.${GPU_COMPUTE_CAP:1}+PTX" ;;
                esac
                export TORCH_CUDA_ARCH_LIST="$ARCH_LIST"
                echo -e "${BLUE}Set TORCH_CUDA_ARCH_LIST=$ARCH_LIST (GPU compute cap: $GPU_COMPUTE_CAP)${NC}"
            fi

            # Install from fVDB directory with --no-build-isolation (uses installed deps)
            uv pip install --no-build-isolation "$FVDB_PATH/fvdb"
            echo -e "${GREEN}✓ fVDB installed${NC}"
        fi
    fi
else
    echo -e "${YELLOW}⚠ fVDB not found at $FVDB_PATH/fvdb - skipping installation${NC}"
    echo "  Run without --skip-deps or set FVDB_PATH to install fVDB"
fi
echo ""

# Configure XCube (no pip install - it's a module)
echo -e "${YELLOW}Configuring XCube...${NC}"
if [ -d "$XCUBE_PATH" ]; then
    # XCube doesn't have setup.py - it's used via PYTHONPATH
    # Check if xcube module is accessible
    if PYTHONPATH="$XCUBE_PATH:${PYTHONPATH:-}" uv run python -c "from xcube.models.model import create_model_from_args; print('xcube OK')" 2>/dev/null; then
        echo -e "${GREEN}✓ XCube module accessible${NC}"
    else
        echo -e "${YELLOW}⚠ XCube module import test failed - may need additional dependencies${NC}"
    fi

    # Create a .pth file in the venv to add XCube to path permanently
    SITE_PACKAGES=$(uv run python -c "import site; print(site.getsitepackages()[0])")
    echo "$XCUBE_PATH" > "$SITE_PACKAGES/xcube.pth"
    echo -e "${GREEN}✓ XCube path configured in venv ($SITE_PACKAGES/xcube.pth)${NC}"
else
    echo -e "${YELLOW}⚠ XCube not found at $XCUBE_PATH - skipping configuration${NC}"
    echo "  Run without --skip-deps or set XCUBE_PATH to configure XCube"
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
    echo -e "   ${BLUE}https://drive.google.com/drive/folders/1PEh0ofpSFcgH56SZtu6iQPC8xAxzhmke${NC}"
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
