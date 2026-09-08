{
  description = "PS2 Bare-Metal Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # Cross-compilation tools for MIPS
          pkgsCross.mips-embedded.stdenv.cc
          cmake
          gnumake
          git

          # Rust nightly toolchain for custom mipsel-sony-ps2 target
          cargo
          rustc
        ];

        shellHook = ''
          echo "PS2 SDK Build Environment Loaded!"
          echo "Target Architecture: mipsel-sony-ps2"
        '';
      };
    };
}
