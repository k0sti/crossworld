{
  description = "Crossworld - Multiplayer voxel metaverse";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;  # Required for CUDA packages
        };

        # Rust nightly toolchain with cranelift backend for faster dev builds
        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rustc-codegen-cranelift" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Build dependencies
        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
          rustToolchain
          wasm-pack
          bun
          just
        ];

        # System libraries for all crates (Bevy, renderer, physics, etc.)
        buildInputs = with pkgs; [
          # Audio dependencies (Bevy)
          alsa-lib

          # Input dependencies (Bevy)
          udev

          # Network/crypto
          openssl
          openssl.dev

          # Graphics dependencies
          libGL
          libglvnd
          vulkan-loader
          vulkan-validation-layers

          # Wayland dependencies
          wayland
          wayland-protocols
          libxkbcommon
          egl-wayland

          # X11 dependencies (fallback)
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];

        # Development tools
        devTools = with pkgs; [
          # Fast linkers for Rust
          mold
          clang
          lld

          # Debugging and profiling
          gdb
          valgrind
          heaptrack

          # Python (for XCube server)
          uv
        ];

        # CUDA toolkit for XCube/fVDB compilation and Trellis
        # PyTorch wheel must match this version (see crates/xcube/server/pyproject.toml)
        # Trellis requires CUDA 12.8+ for RTX 5090 support (compute capability 12.0)
        cudaPackages = pkgs.cudaPackages_12;
        cudaDeps = with cudaPackages; [
          # CUDA compiler and runtime
          cuda_nvcc
          cuda_cudart
          cuda_cupti

          # CUDA libraries required by PyTorch and kaolin
          libcublas
          libcufft
          libcurand
          libcusolver
          libcusparse
          cudnn

          # Full toolkit for building from source (e.g., kaolin)
          cudatoolkit
        ];

        # Note: Conda is not included in the nix shell due to packaging issues.
        # Install conda/miniconda manually: https://docs.anaconda.com/miniconda/install/
        # The nix shell will work with system-installed conda.

        # Library path for dynamic libraries
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

        # CUDA library path (for cuda shell)
        CUDA_LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (buildInputs ++ cudaDeps);

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = buildInputs ++ devTools;
          inherit nativeBuildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:$LD_LIBRARY_PATH"
            export PKG_CONFIG_PATH="${pkgs.alsa-lib.dev}/lib/pkgconfig:${pkgs.udev.dev}/lib/pkgconfig:${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

            # Wayland/X11 environment
            export WAYLAND_DISPLAY="''${WAYLAND_DISPLAY:-wayland-1}"
            export XDG_RUNTIME_DIR="''${XDG_RUNTIME_DIR:-/run/user/1000}"

            echo "🦀 Crossworld development environment loaded"
            echo ""
            echo "Toolchain:"
            echo "  Rust: $(rustc --version)"
            echo "  Bun: $(bun --version)"
            echo ""
            echo "Quick start:"
            echo "  just dev       - Start development server (web)"
            echo "  just build     - Production build"
            echo "  just check     - Run all checks"
            echo "  just test      - Run tests"
            echo ""
            echo "Native applications:"
            echo "  just planet    - Run native voxel editor"
            echo "  just proto     - Run physics prototype"
            echo "  just server    - Run game server"
            echo ""
            echo "AI Inference (requires GPU + conda):"
            echo "  Install conda: https://docs.anaconda.com/miniconda/install/"
            echo "  just trellis-setup  - Set up Trellis environment"
            echo "  just trellis-server - Start Trellis server"
            echo ""
            echo "Build optimizations:"
            echo "  ✓ mold linker configured in .cargo/config.toml"
            echo "  ✓ cargo build --profile dev-cranelift  (cranelift backend)"
            echo "  ✓ cargo build --profile fast-dev       (cranelift + opt-level=1)"
            echo ""
          '';
        };

        # CUDA-enabled shell for XCube/fVDB development and Trellis
        devShells.cuda = pkgs.mkShell {
          buildInputs = buildInputs ++ devTools ++ cudaDeps;
          inherit nativeBuildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH="${CUDA_LD_LIBRARY_PATH}:$LD_LIBRARY_PATH"
            export PKG_CONFIG_PATH="${pkgs.alsa-lib.dev}/lib/pkgconfig:${pkgs.udev.dev}/lib/pkgconfig:${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

            # CUDA environment
            export CUDA_HOME="${cudaPackages.cudatoolkit}"
            export CUDA_PATH="${cudaPackages.cudatoolkit}"
            export CUDA_TOOLKIT_ROOT_DIR="${cudaPackages.cudatoolkit}"

            # Add nvcc to PATH
            export PATH="${cudaPackages.cuda_nvcc}/bin:$PATH"

            # PyTorch CUDA arch list (include sm_120 for RTX 5090)
            export TORCH_CUDA_ARCH_LIST="7.0 7.5 8.0 8.6 8.9 9.0 12.0"

            # Force CUDA build for packages like kaolin
            export FORCE_CUDA=1

            # Wayland/X11 environment
            export WAYLAND_DISPLAY="''${WAYLAND_DISPLAY:-wayland-1}"
            export XDG_RUNTIME_DIR="''${XDG_RUNTIME_DIR:-/run/user/1000}"

            echo "🦀 Crossworld development environment loaded (with CUDA)"
            echo ""
            echo "Toolchain:"
            echo "  Rust: $(rustc --version)"
            echo "  Bun: $(bun --version)"
            echo "  CUDA: $(nvcc --version | grep release | sed 's/.*release //' | sed 's/,.*//')"
            echo "  CUDA_HOME: $CUDA_HOME"
            echo ""
            echo "AI Inference Servers:"
            echo "  just trellis-setup - Set up Trellis environment (conda, ~4GB download)"
            echo "  just trellis-server - Start Trellis inference server"
            echo "  just xcube-setup   - Set up XCube environment (uv)"
            echo "  just xcube-server  - Start XCube inference server"
            echo ""
            echo "Quick start:"
            echo "  just dev       - Start development server (web)"
            echo "  just build     - Production build"
            echo "  just check     - Run all checks"
            echo ""
          '';
        };
      }
    );
}
