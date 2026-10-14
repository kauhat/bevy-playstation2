{
  description = "PS2 Bare-Metal Bevy Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    rust-overlay,
  }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

    env = forAllSystems (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlays.default
        ];
      };

      ps2devToolchainSrc =
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

      ps2devToolchain = pkgs.stdenv.mkDerivation {
        pname = "ps2dev";
        version = "2.0.0";

        src = pkgs.fetchurl {
          url = "https://github.com/ps2dev/ps2dev/releases/download/v2.0.0/ps2dev-${ps2devToolchainSrc.asset}.tar.gz";
          sha256 = ps2devToolchainSrc.hash;
        };

        nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [pkgs.autoPatchelfHook];

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

      rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
        extensions = ["rust-src"];
      };

      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      runtimeLibs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux (with pkgs; [
        udev
        alsa-lib
        libxkbcommon
        wayland
        wayland-protocols
        vulkan-loader
        libGL
        libX11
        libXcursor
        libXi
        libXrandr
        libdisplay-info
        llvmPackages.llvm
      ]);

      # Symlink all runtime libraries into a single path containing /lib
      runtimeLibsEnv = pkgs.symlinkJoin {
        name = "bevy-runtime-libs";
        paths = runtimeLibs;
      };

      hasPcsx2 = (pkgs ? pcsx2) && (pkgs.pcsx2.meta.available or false);
    in {
      inherit pkgs rustToolchain craneLib ps2devToolchain runtimeLibs runtimeLibsEnv hasPcsx2;
    });
  in {
    formatter = forAllSystems (
      system: let
        e = env.${system};
      in
        e.pkgs.alejandra
    );

    devShells = forAllSystems (system: let
      e = env.${system};
    in {
      default = e.pkgs.mkShell {
        nativeBuildInputs = [e.pkgs.pkg-config];

        buildInputs =
          [
            e.ps2devToolchain
            e.rustToolchain
            e.pkgs.gnumake
            e.pkgs.git
            e.pkgs.just
            e.pkgs.rust-bindgen
            e.pkgs.cdrtools
            e.pkgs.alejandra
          ]
          ++ e.pkgs.lib.optional e.hasPcsx2 e.pkgs.pcsx2
          ++ e.runtimeLibs;

        shellHook = ''
          export PS2DEV="${e.ps2devToolchain}"
          export PS2SDK="$PS2DEV/ps2sdk"
          export GSKIT="$PS2DEV/gsKit"
          export PATH=$PATH:$PS2DEV/bin:$PS2DEV/ee/bin:$PS2DEV/iop/bin:$PS2DEV/dvp/bin:$PS2SDK/bin
          export PKG_CONFIG_PATH="${e.pkgs.wayland.dev}/lib/pkgconfig:${e.pkgs.libxkbcommon.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

          echo "PS2 SDK Build Environment Loaded for ${system}!"
          ${e.pkgs.lib.optionalString (!e.hasPcsx2) ''
            echo "Note: PCSX2 is not available for ${system} in nixpkgs and was omitted from the shell."
          ''}
        '';
      };
    });

    packages = forAllSystems (system: let
      e = env.${system};
      src = e.craneLib.cleanCargoSource ./.;
    in {
      pc = e.craneLib.buildPackage {
        inherit src;
        pname = "bevy-ps2-pc";
        version = "0.1.0";

        nativeBuildInputs = [e.pkgs.pkg-config];
        buildInputs = e.runtimeLibs;
      };

      elf = let
        cargoVendorDir = e.craneLib.vendorMultipleCargoDeps {
          inherit (e.craneLib.findCargoFiles src) cargoConfigs;
          cargoLockList = [
            ./Cargo.lock
            "${e.rustToolchain.passthru.availableComponents.rust-src}/lib/rustlib/src/rust/library/Cargo.lock"
          ];
        };

        commonArgs = {
          inherit src cargoVendorDir;
          pname = "bevy-ps2-elf";
          version = "0.1.0";

          cargoExtraArgs = "--target ./mips64el-sony-ps2.json -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec";

          nativeBuildInputs = [e.pkgs.pkg-config];
          buildInputs = [e.ps2devToolchain];

          PS2DEV = "${e.ps2devToolchain}";
          PS2SDK = "${e.ps2devToolchain}/ps2sdk";
          GSKIT = "${e.ps2devToolchain}/gsKit";

          preBuild = ''
            export PATH=$PATH:$PS2DEV/bin:$PS2DEV/ee/bin:$PS2DEV/iop/bin:$PS2DEV/dvp/bin:$PS2SDK/bin
            
            cp -f ${./mips64el-sony-ps2.json} mips64el-sony-ps2.json
          '';

          doCheck = false;
        };

        cargoArtifacts = e.craneLib.buildDepsOnly (commonArgs
          // {
            # TODO: Is this a hack?
            # Crane stubs out build.rs and main.rs in buildDepsOnly, so skip
            # crt0/entry  when linking the throwaway dummy binary without
            # altering dependency RUSTFLAGS.
            postPatch = ''
              cat > build.rs <<'EOF'
              fn main() {
                  println!("cargo:rustc-link-arg=-nostartfiles");
                  println!("cargo:rustc-link-arg=-Wl,--entry=0");
              }
              EOF
              sed -i 's|^build = .*|build = "build.rs"|' Cargo.toml
            '';
          });
      in
        e.craneLib.buildPackage (commonArgs
          // {
            # Pass the pre-built third-party dependencies
            inherit cargoArtifacts;

            installPhaseCommand = ''
              mkdir -p $out/bin
              
              cp target/mips64el-sony-ps2/release/bevy-ps2 $out/bin/BOOT.ELF
            '';
          });

      iso = e.pkgs.stdenv.mkDerivation {
        pname = "bevy-ps2-iso";
        version = "0.1.0";
        dontUnpack = true;

        nativeBuildInputs = [e.pkgs.cdrtools];

        buildPhase = ''
          bash ${./scripts/pack-iso.sh} "${self.packages.${system}.elf}/bin/BOOT.ELF" "bevy-ps2.iso"
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
    in
      e.pkgs.lib.optionalAttrs e.hasPcsx2 {
        # Build PS2 ISO and run in PCSX2.
        default = {
          type = "app";
          program = "${e.pkgs.writeShellScriptBin "run-iso" ''
            set -e
            ISO_PATH="${self.packages.${system}.iso}/bevy-ps2.iso"
            exec ${e.pkgs.pcsx2}/bin/pcsx2-qt "$ISO_PATH"
          ''}/bin/run-iso";
        };

        # Build PS2 ISO and run in PCSX2 (with debugger.)
        debugger = {
          type = "app";
          program = "${e.pkgs.writeShellScriptBin "run-iso-debug" ''
            set -e
            ISO_PATH="${self.packages.${system}.iso}/bevy-ps2.iso"
            exec ${e.pkgs.pcsx2}/bin/pcsx2-qt -debugger "$ISO_PATH"
          ''}/bin/run-iso-debug";
        };
      }
      // {
        # Run PC executable.
        pc = {
          type = "app";
          program = "${e.pkgs.writeShellScriptBin "run-pc" ''
            set -e
            exec ${self.packages.${system}.pc}/bin/bevy-ps2
          ''}/bin/run-pc";
        };
      });
  };
}
