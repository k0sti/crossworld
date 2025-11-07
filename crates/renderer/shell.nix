{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
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
    libglvnd
    egl-wayland

    # Build tools
    pkg-config
    cmake
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
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
  ]);

  shellHook = ''
    echo "Renderer development environment loaded"
    echo "Wayland/X11/OpenGL libraries configured"
    echo "Run with: cargo run -p renderer"
  '';
}
