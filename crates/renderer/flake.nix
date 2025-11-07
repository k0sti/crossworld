{
  description = "WebGL2 Cube Octree Renderer";

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
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Common libraries needed for windowing and graphics
        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
        ];

        buildInputs = with pkgs; [
          # Wayland dependencies
          wayland
          wayland-protocols
          libxkbcommon

          # X11 dependencies (fallback)
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr

          # OpenGL/Vulkan
          libGL
          vulkan-loader

          # Additional libraries
          libglvnd
          egl-wayland
        ];

        # Runtime library path
        runtimeLibs = with pkgs; [
          wayland
          wayland-protocols
          libxkbcommon
          libGL
          vulkan-loader
          libglvnd
          egl-wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          packages = with pkgs; [
            rustToolchain
            cargo
            rustc
            rust-analyzer
          ];

          # Set up library paths for runtime
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;

          # Wayland and X11 environment
          WAYLAND_DISPLAY = "wayland-1";
          XDG_RUNTIME_DIR = "/run/user/1000";

          shellHook = ''
            echo "Renderer development environment"
            echo "Rust version: $(rustc --version)"
            echo "LD_LIBRARY_PATH configured for Wayland/X11/OpenGL"
            echo ""
            echo "Run with: cargo run -p renderer"
          '';
        };

        # Note: For building the package, use the workspace flake at the root
        # This devShell is for development within the renderer crate
      }
    );
}
