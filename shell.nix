{ pkgs ? import <nixpkgs> { } }:

with pkgs;

mkShell rec {
  nativeBuildInputs = [ pkg-config mold-wrapped clang lld rustup trunk ];
  buildInputs = [
    udev
    alsa-lib
    vulkan-loader
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr # To use the x11 feature
    libxkbcommon
    wayland # To use the wayland feature
    wabt
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;

  shellHook = ''
    # export RUSTFLAGS="-C link-arg=-fuse-ld=$(which mold)"
    alias rundev='cargo run --features bevy/dynamic_linking'
  '';
}

