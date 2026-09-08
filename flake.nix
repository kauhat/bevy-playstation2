{
  description = "PS2 Bare-Metal Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {inherit system;};

    # 32-bit Little-Endian MIPS bare-metal cross-compilation toolchain
    mipsPkgs = pkgs.pkgsCross.mipsel-linux-gnu;
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = [
        mipsPkgs.stdenv.cc
        pkgs.cmake
        pkgs.gnumake
        pkgs.git
        pkgs.just
        pkgs.rustup
        pkgs.pcsx2
      ];

      shellHook = ''
        echo "PS2 SDK Build Environment Loaded!"
        echo "Cross Compiler: mipsel-unknown-linux-gnu-gcc"
      '';
    };

    apps.${system}.default = {
      type = "app";
      program = "${pkgs.writeShellScriptBin "run-ps2" ''
        set -e
        echo "Building PS2 ELF..."
        ${pkgs.just}/bin/just build

        # Locate the output ELF binary
        ELF_PATH="target/mipsel-sony-ps2/release/bevy-ps2"

        if [ ! -f "$ELF_PATH" ]; then
          echo "Error: Could not find compiled ELF at $ELF_PATH"
          exit 1
        fi

        echo "Launching $ELF_PATH in PCSX2..."
        exec ${pkgs.pcsx2}/bin/pcsx2-qt -elf "$ELF_PATH"
      ''}/bin/run-ps2";
    };
  };
}
