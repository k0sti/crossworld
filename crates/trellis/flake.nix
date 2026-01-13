{
  description = "Trellis.2 inference server with CUDA support";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;  # Required for CUDA packages
            cudaSupport = true;
          };
        };

        # CUDA toolkit version matching PyTorch requirements
        cudaPackages = pkgs.cudaPackages_12;

        # Python environment for Trellis
        pythonEnv = pkgs.python310;

        # Build inputs for kaolin compilation
        buildInputs = with pkgs; [
          # CUDA development tools
          cudaPackages.cudatoolkit
          cudaPackages.cuda_nvcc
          cudaPackages.cuda_cudart
          cudaPackages.cuda_cupti
          cudaPackages.libcublas
          cudaPackages.libcufft
          cudaPackages.libcurand
          cudaPackages.libcusolver
          cudaPackages.libcusparse
          cudaPackages.cudnn

          # Build tools
          gcc
          cmake
          ninja
          pkg-config

          # Python build dependencies
          pythonEnv

          # OpenGL for open3d rendering
          libGL
          libGLU
          mesa
          xorg.libX11
          xorg.libXext
          xorg.libXrender
        ];

        # Runtime libraries
        runtimeLibs = with pkgs; [
          stdenv.cc.cc.lib
          zlib
          libGL
          libGLU
          glib
          xorg.libX11
          xorg.libXext
          xorg.libXrender
        ];

        # LD_LIBRARY_PATH for CUDA and runtime libraries
        libraryPath = pkgs.lib.makeLibraryPath (runtimeLibs ++ [
          cudaPackages.cudatoolkit
          cudaPackages.cuda_cudart
          cudaPackages.libcublas
          cudaPackages.libcufft
          cudaPackages.libcurand
          cudaPackages.libcusolver
          cudaPackages.libcusparse
          cudaPackages.cudnn
        ]);

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          shellHook = ''
            echo "🔥 Trellis CUDA Development Environment"
            echo ""
            echo "CUDA Toolkit: ${cudaPackages.cudatoolkit.version}"
            echo "nvcc: $(which nvcc 2>/dev/null || echo 'not in PATH yet')"
            echo ""
            echo "Environment variables:"
            echo "  CUDA_HOME=${cudaPackages.cudatoolkit}"
            echo "  LD_LIBRARY_PATH=${libraryPath}:/run/opengl-driver/lib"
            echo ""
            echo "Setup commands:"
            echo "  just trellis-setup  - Install Python dependencies and build kaolin"
            echo "  just trellis-server - Start Trellis inference server"
            echo ""
            echo "Notes:"
            echo "  • This environment includes nvcc for building kaolin from source"
            echo "  • PyTorch 2.8.0+cu128 supports RTX 5090 (compute capability 12.0)"
            echo "  • kaolin will be built against PyTorch 2.8.0 during setup"
            echo ""

            # Set CUDA environment variables
            export CUDA_HOME="${cudaPackages.cudatoolkit}"
            export CUDA_PATH="${cudaPackages.cudatoolkit}"
            export CUDA_TOOLKIT_ROOT_DIR="${cudaPackages.cudatoolkit}"

            # Add nvcc to PATH
            export PATH="${cudaPackages.cuda_nvcc}/bin:$PATH"

            # Set library paths
            export LD_LIBRARY_PATH="${libraryPath}:/run/opengl-driver/lib:''${LD_LIBRARY_PATH:-}"
            export LIBRARY_PATH="${libraryPath}:''${LIBRARY_PATH:-}"

            # PyTorch CUDA arch list (include sm_120 for RTX 5090)
            export TORCH_CUDA_ARCH_LIST="7.0 7.5 8.0 8.6 8.9 9.0 12.0"

            # Force CUDA build for kaolin
            export FORCE_CUDA=1

            # Verify CUDA is available
            if command -v nvcc >/dev/null 2>&1; then
              echo "✓ nvcc available: $(nvcc --version | grep release | awk '{print $5,$6}')"
            else
              echo "⚠ nvcc not found - kaolin build may fail"
            fi

            # Check if conda is available
            if command -v conda >/dev/null 2>&1; then
              echo "✓ conda available"
            else
              echo "⚠ conda not found - install from https://docs.anaconda.com/miniconda/install/"
            fi
            echo ""
          '';
        };
      }
    );
}
