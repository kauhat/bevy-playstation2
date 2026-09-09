{
  description = "PS2 Bare-Metal Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixgl.url = "github:nix-community/nixGL";
  };

  outputs = {
    self,
    nixpkgs,
    nixgl,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [nixgl.overlays.default];
    };

    # Direct reference to nixGLDefault package from the flake input
    nixGLPkg = nixgl.packages.${system}.nixGLDefault;

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
        pkgs.cdrtools
        nixGLPkg
      ];

      shellHook = ''
        echo "PS2 SDK Build Environment Loaded!"
        echo "Cross Compiler: mipsel-unknown-linux-gnu-gcc"
      '';
    };

    apps.${system}.default = {
      type = "app";
      buildInputs = [
        pkgs.just
        pkgs.pcsx2
        nixGLPkg
      ];
      program = "${pkgs.writeShellScriptBin "run-ps2" ''
        set -e

        # Ensure pcsx2 and nixGL are in the PATH
        export PATH="${pkgs.pcsx2}/bin:${nixGLPkg}/bin:$PATH"

        # Execute using nixGL wrapper
        exec nixGL ${pkgs.just}/bin/just run
      ''}/bin/run-ps2";
    };
  };
}
