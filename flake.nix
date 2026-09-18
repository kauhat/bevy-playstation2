{
  description = "PS2 Bare-Metal Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixgl.url = "github:nix-community/nixGL";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    nixgl,
    rust-overlay,
  }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

    env = forAllSystems (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          nixgl.overlays.default
          rust-overlay.overlays.default
        ];
      };

      # Exact SHA256 hashes and asset names for ps2dev v2.0.0
      ps2devSrc =
        {
          "x86_64-linux" = {
            asset = "ubuntu-latest";
            hash = "sha256:c8e5dedccf62084d476894e88cd0451b57f1ed288e741eb3c1263010aaffc028";
          };
          "aarch64-linux" = {
            asset = "ubuntu-24.04-arm";
            hash = "sha256:ed69330f235d421754196acf866f8590e5f92e8b92571f1c7e05e853900e7622";
          };
          "x86_64-darwin" = {
            asset = "macos-15-intel";
            hash = "sha256:833d2dc1e3cbfa2091047e4539e1bd0bd2836668f886a1c8a75e6e09deb56bef";
          };
          "aarch64-darwin" = {
            asset = "macos-latest";
            hash = "sha256:6f6df659d2d49738f21f53a53c361a74182d0e4c2495e31e76972ff818bc21cb";
          };
        }.${
          system
        };

      ps2dev = pkgs.stdenv.mkDerivation {
        pname = "ps2dev";
        version = "2.0.0";

        src = pkgs.fetchurl {
          url = "https://github.com/ps2dev/ps2dev/releases/download/v2.0.0/ps2dev-${ps2devSrc.asset}.tar.gz";
          sha256 = ps2devSrc.hash;
        };

        # autoPatchelfHook runs on Linux; macOS binaries are already dynamically linked for Mach-O
        nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [pkgs.autoPatchelfHook];

        # Injected the missing multiprecision math libraries required by GCC
        buildInputs = [
          pkgs.zlib
          pkgs.stdenv.cc.cc.lib
          pkgs.gmp
          pkgs.mpfr
          pkgs.libmpc
        ];

        installPhase = ''
          mkdir -p $out
          cp -r * $out/
        '';
      };

      nixGLPkg = nixgl.packages.${system}.nixGLDefault;

      rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
        targets = ["x86_64-unknown-linux-gnu"];
        extensions = ["rust-src"];
      };

      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
    in {
      inherit pkgs nixGLPkg rustToolchain rustPlatform ps2dev;
    });
  in {
    devShells = forAllSystems (system: let
      e = env.${system};
    in {
      default = e.pkgs.mkShell {
        buildInputs = [
          e.ps2dev
          e.pkgs.gnumake
          e.pkgs.git
          e.pkgs.just
          e.rustToolchain
          e.pkgs.pcsx2
          e.pkgs.cdrtools
          e.nixGLPkg
        ];

        shellHook = ''
          export PS2SDK=${e.ps2dev}/ps2sdk
          export GSKIT=${e.ps2dev}/gsKit
          export PATH=$PATH:${e.ps2dev}/bin:${e.ps2dev}/ee/bin:${e.ps2dev}/iop/bin:${e.ps2dev}/dvp/bin:$PS2SDK/bin
          export PS2SDK_LIBDIR="$PS2SDK/ee/lib"
          export PS2SDK_INCDIR="$PS2SDK/ee/include"
          echo "PS2 SDK Build Environment Loaded for ${system}!"
        '';
      };
    });

    packages = forAllSystems (system: let
      e = env.${system};
    in {
      pc = e.rustPlatform.buildRustPackage {
        pname = "bevy-ps2-pc";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = [e.pkgs.pkg-config];
        buildInputs = [e.pkgs.udev e.pkgs.alsa-lib e.pkgs.vulkan-loader e.pkgs.wayland];
      };

      elf = e.rustPlatform.buildRustPackage {
        pname = "bevy-ps2-elf";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = [e.ps2dev];
        buildPhase = ''
          export PS2SDK="${e.ps2dev}/ps2sdk"
          export PS2SDK_LIBDIR="$PS2SDK/ee/lib"
          export PS2SDK_INCDIR="$PS2SDK/ee/include"

          export PATH=$PATH:${e.ps2dev}/bin:${e.ps2dev}/ee/bin:${e.ps2dev}/iop/bin:${e.ps2dev}/dvp/bin:$PS2SDK/bin

          cargo build -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem -Z json-target-spec --target mipsel-sony-ps2.json --release
        '';
        installPhase = ''
          mkdir -p $out/bin
          cp target/mipsel-sony-ps2/release/bevy-ps2 $out/bin/BOOT.ELF
        '';
        doCheck = false;
      };

      iso = e.pkgs.stdenv.mkDerivation {
        pname = "bevy-ps2-iso";
        version = "0.1.0";
        srcs = [
          self.packages.${system}.elf
          ./.
        ];
        sourceRoot = ".";
        nativeBuildInputs = [e.pkgs.cdrtools];
        dontUnpack = false;

        buildPhase = ''
          ./scripts/pack-iso.sh "bin/BOOT.ELF" "bevy-ps2.iso"
        '';
        installPhase = ''
          mkdir -p $out
          cp bevy-ps2.iso $out/
        '';
      };

      default = self.packages.${system}.iso;
    });

    apps = forAllSystems (system: let
      e = env.${system};
    in {
      default = {
        type = "app";
        program = "${e.pkgs.writeShellScriptBin "run-iso" ''
          set -e
          ISO_PATH="${self.packages.${system}.iso}/bevy-ps2.iso"
          exec nixGL pcsx2-qt -disc "$ISO_PATH"
        ''}/bin/run-iso";
      };

      debugger = {
        type = "app";
        program = "${e.pkgs.writeShellScriptBin "run-iso-debug" ''
          set -e
          ISO_PATH="${self.packages.${system}.iso}/bevy-ps2.iso"
          exec nixGL pcsx2-qt -debugger -disc "$ISO_PATH"
        ''}/bin/run-iso-debug";
      };
    });
  };
}
