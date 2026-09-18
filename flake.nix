{
  description = "PS2 Bare-Metal Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixgl.url = "github:nix-community/nixGL";
    rust-overlay.url = "github:oxalica/rust-overlay";
    ps2sdk-src = {
      url = "github:ps2dev/ps2sdk";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    nixgl,
    rust-overlay,
    ps2sdk-src,
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

crossPkgs = import nixpkgs {
        inherit system;
        crossSystem = {
          config = "mips64el-none-elf";
          gcc.arch = "mips3"; 
        };
        overlays = [
          (final: prev: {
            # Use overrideAttrs to pass the flag to the configure script
            newlib = prev.newlib.overrideAttrs (old: {
              configureFlags = (old.configureFlags or []) ++ [ "--disable-libgloss" ];
            });
          })
        ];
      };

      mipsCC = crossPkgs.stdenv.cc;
      nixGLPkg = nixgl.packages.${system}.nixGLDefault;

      rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
        targets = ["x86_64-unknown-linux-gnu"];
        extensions = ["rust-src"];
      };

      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };

      ps2sdk = pkgs.stdenv.mkDerivation {
        pname = "ps2sdk";
        version = "latest";
        src = ps2sdk-src;

        nativeBuildInputs = [pkgs.gnumake mipsCC];

        patchPhase = ''
          substituteInPlace ee/Rules.make \
            --replace-quiet "EE_CFLAGS =" "EE_CFLAGS ?=" || true
          substituteInPlace Rules.make \
            --replace-quiet "EE_CFLAGS =" "EE_CFLAGS ?=" || true
        '';

        buildPhase = ''
          export PS2SDK=$out

          # The compiler now natively provides <string.h> via our overlaid newlib sysroot
          export EE_CFLAGS="-mips3 -mgp64 -mlong64 -D_EE -fno-PIC -Wno-error"

          make -j$NIX_BUILD_CORES release \
            EE_CC=mips64el-none-elf-gcc \
            EE_CXX=mips64el-none-elf-g++ \
            EE_AR=mips64el-none-elf-ar \
            EE_LD=mips64el-none-elf-ld \
            EE_AS=mips64el-none-elf-as \
            IOP_CC=mips64el-none-elf-gcc \
            IOP_AR=mips64el-none-elf-ar \
            IOP_AS=mips64el-none-elf-as
        '';

        dontInstall = true;
      };
    in {
      inherit pkgs crossPkgs mipsCC nixGLPkg rustToolchain rustPlatform ps2sdk;
    });
  in {
    devShells = forAllSystems (system: let
      e = env.${system};
    in {
      default = e.pkgs.mkShell {
        buildInputs = [
          e.mipsCC
          e.pkgs.gnumake
          e.pkgs.git
          e.pkgs.just
          e.rustToolchain
          e.pkgs.pcsx2
          e.pkgs.cdrtools
          e.nixGLPkg
        ];

        shellHook = ''
          export PS2SDK="${e.ps2sdk}"
          export PS2SDK_LIBDIR="${e.ps2sdk}/ee/lib"
          export PS2SDK_INCDIR="${e.ps2sdk}/ee/include"
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
        nativeBuildInputs = [e.mipsCC];
        buildPhase = ''
          export PS2SDK="${e.ps2sdk}"
          export PS2SDK_LIBDIR="${e.ps2sdk}/ee/lib"
          export PS2SDK_INCDIR="${e.ps2sdk}/ee/include"

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
