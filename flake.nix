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
        };

        # Rust stable toolchain with wasm target
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Rust nightly toolchain with cranelift backend for faster dev builds
        rustNightly = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rustc-codegen-cranelift" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Build dependencies
        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
          rustToolchain
          rustNightly
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
        ];

        # Library path for dynamic libraries
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = buildInputs ++ devTools;
          inherit nativeBuildInputs;

          RUST_NIGHTLY_PATH = "${rustNightly}/bin";

          shellHook = ''
            export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:$LD_LIBRARY_PATH"
            export PKG_CONFIG_PATH="${pkgs.alsa-lib.dev}/lib/pkgconfig:${pkgs.udev.dev}/lib/pkgconfig:${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

            # Wayland/X11 environment
            export WAYLAND_DISPLAY="''${WAYLAND_DISPLAY:-wayland-1}"
            export XDG_RUNTIME_DIR="''${XDG_RUNTIME_DIR:-/run/user/1000}"

            # Create wrapper functions for nightly toolchain that set RUSTC properly
            cargo-nightly() {
              CARGO="${rustNightly}/bin/cargo" RUSTC="${rustNightly}/bin/rustc" "${rustNightly}/bin/cargo" "$@"
            }
            export -f cargo-nightly

            echo "🦀 Crossworld development environment loaded"
            echo ""
            echo "Toolchain:"
            echo "  Rust stable: $(rustc --version)"
            echo "  Rust nightly: $(${rustNightly}/bin/rustc --version)"
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
            echo "Build optimizations:"
            echo "  ✓ mold linker configured in .cargo/config.toml"
            echo "  ✓ cargo-nightly build --profile dev-cranelift  (cranelift backend)"
            echo "  ✓ cargo-nightly build --profile fast-dev       (cranelift + opt-level=1)"
            echo ""
          '';
        };
      }
    );
}
