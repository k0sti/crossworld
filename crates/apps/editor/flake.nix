{
  description = "Crossworld Planet - Native voxel editor built with Bevy";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain (stable by default, nightly available)
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Bevy system dependencies following:
        # https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md
        bevyDeps = with pkgs; [
          # Build dependencies
          pkg-config

          # Audio dependencies
          alsa-lib

          # Graphics dependencies
          vulkan-loader
          vulkan-validation-layers

          # X11 dependencies
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          libxkbcommon

          # Wayland dependencies
          wayland
          wayland-protocols

          # Input dependencies
          udev
        ];

        # Development tools
        devTools = with pkgs; [
          # Rust toolchain
          rustToolchain

          # Fast linker for Linux
          mold
          clang
          lld

          # Build tools
          just
          bun

          # WebTransport / QUIC tools
          openssl

          # Optional: debugging and profiling
          gdb
          valgrind
          heaptrack
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = bevyDeps ++ devTools;

          # Set library path for dynamic linking
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath bevyDeps;

          # Set PKG_CONFIG_PATH for pkg-config to find ALSA and other libraries
          PKG_CONFIG_PATH = "${pkgs.alsa-lib.dev}/lib/pkgconfig:${pkgs.udev.dev}/lib/pkgconfig";

          # Environment variables for Bevy
          shellHook = ''
            echo "Crossworld Voxel Editor"
            echo "  Rust: $(rustc --version)"
            echo "  Cargo: $(cargo --version)"
            echo ""
            echo "To build and run the editor:"
            echo "  cargo run --bin editor         (from project root)"
            echo "  just editor                     (from project root)"
            echo ""
            echo "Build optimizations enabled:"
            echo "  - mold linker (configured in ../../.cargo/config.toml)"
            echo "  - Use 'cargo +nightly' for cranelift codegen backend"
            echo ""
          '';
        };
      }
    );
}
