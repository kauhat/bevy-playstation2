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
          # nixgl.overlays.default
          rust-overlay.overlays.default
        ];
      };

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

      lwipSrc = pkgs.fetchFromGitHub {
        owner = "lwip-tcpip";
        repo = "lwip";
        rev = "d08f4773edd0182b7910fc8f046eed82ffcd67c9";
        hash = "sha256-2eUtb4w2s/C/i9mXTLDs6PYhWxLAye2ElPfktadH89Y=";
      };

      fatfsSrc = pkgs.fetchFromGitHub {
        owner = "fjtrujy";
        repo = "FatFs";
        rev = "cac3314968129ee0e2c019ae48e5c9cd262ceb4e";
        hash = "sha256-nHwekmfIXImbopFUlXQchmooo8FY+1xeSHj1UYtpdRA=";
      };

      # TODO: Broken, but not needed?
      ps2sdk = pkgs.stdenv.mkDerivation {
        pname = "ps2sdk";
        version = "2.0.0";

        src = pkgs.fetchFromGitHub {
          owner = "ps2dev";
          repo = "ps2sdk";
          rev = "be12adcb7c8aa6065202e8360d39fe3298eadad3";
          hash = "sha256-rYIom3ZLz0qne7ckaS1N7KBOvgxK3GbTszn/g1h7Ufw=";
        };

        nativeBuildInputs = [
          ps2dev
          pkgs.pkg-config
          pkgs.bash
        ];

        postPatch = ''
          # 1. Unpack external dependencies
          mkdir -p common/external_deps/lwip common/external_deps/fatfs
          cp -r ${lwipSrc}/. common/external_deps/lwip/
          cp -r ${fatfsSrc}/. common/external_deps/fatfs/
          chmod -R +w common/external_deps/lwip common/external_deps/fatfs

          # 2. Setup FatFs include structure
          mkdir -p common/external_deps/fatfs/source/include
          cp -r common/external_deps/fatfs/source/*.h common/external_deps/fatfs/source/include/ 2>/dev/null || true

          # 3. Neutralize upstream sample Makefiles in external_deps so they don't compile diskio.c
          echo "all:" > common/external_deps/fatfs/Makefile
          echo "clean:" >> common/external_deps/fatfs/Makefile
          echo "all:" > common/external_deps/lwip/Makefile
          echo "clean:" >> common/external_deps/lwip/Makefile

          # 4. Enable exFAT in FatFs (fixes 64-bit fsize shift overflow)
          sed -i 's/#define\s\+FF_FS_EXFAT\s\+0/#define FF_FS_EXFAT 1/' common/external_deps/fatfs/source/ffconf.h
          sed -i 's/#define\s\+FF_FS_EXFAT\s\+0/#define FF_FS_EXFAT 1/' common/external_deps/fatfs/source/include/ffconf.h

          # 5. Un-static internal FatFs functions in ff.c
          sed -i 's/^static DWORD get_fat/DWORD get_fat/' common/external_deps/fatfs/source/ff.c
          sed -i 's/^static QWORD clst2sect/QWORD clst2sect/' common/external_deps/fatfs/source/ff.c
          sed -i 's/^static DWORD clst2sect/DWORD clst2sect/' common/external_deps/fatfs/source/ff.c

          # 6. Append matching function prototypes to ff.h
          cat <<'EOF' >> common/external_deps/fatfs/source/include/ff.h

          /* Required by PS2SDK bdmfs_fatfs */
          DWORD get_fat (FFOBJID* obj, DWORD clst);
          QWORD clst2sect (FATFS* fs, DWORD clst);
          EOF
          cp common/external_deps/fatfs/source/include/ff.h common/external_deps/fatfs/source/ff.h

          # 7. Stub download_dependencies.sh script
          cat <<'EOF' > download_dependencies.sh
          #!${pkgs.bash}/bin/bash
          exit 0
          EOF
          chmod +x download_dependencies.sh

          patchShebangs .
        '';

        preBuild = ''
          export PS2DEV="${ps2dev}"
          export PS2SDK="$out"
          export PS2SDKSRC="$(pwd)"
          export PATH="$PATH:$PS2DEV/bin:$PS2DEV/ee/bin:$PS2DEV/iop/bin:$PS2DEV/dvp/bin:$PS2SDK/bin"
        '';

        buildPhase = ''
          runHook preBuild
          make build
          runHook postBuild
        '';

        installPhase = ''
          runHook preInstall
          mkdir -p $out
          make release
          runHook postInstall
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
    in {
      inherit pkgs rustToolchain craneLib ps2dev ps2sdk runtimeLibs runtimeLibsEnv;
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
            e.ps2dev
            # e.ps2sdk
            e.pkgs.gnumake
            e.pkgs.git
            e.pkgs.just
            e.rustToolchain
            e.pkgs.rust-bindgen
            e.pkgs.pcsx2
            e.pkgs.cdrtools
            e.pkgs.alejandra
            # e.nixGLPkg
          ]
          ++ e.runtimeLibs;

        shellHook = ''
          export PS2DEV="${e.ps2dev}"
          export PS2SDK="$PS2DEV/ps2sdk"
          export GSKIT="$PS2DEV/gsKit"
          export PATH=$PATH:$PS2DEV/bin:$PS2DEV/ee/bin:$PS2DEV/iop/bin:$PS2DEV/dvp/bin:$PS2SDK/bin
          export PS2SDK_LIBDIR="$PS2SDK/ee/lib"
          export PS2SDK_INCDIR="$PS2SDK/ee/include"

          export PKG_CONFIG_PATH="${e.pkgs.wayland.dev}/lib/pkgconfig:${e.pkgs.libxkbcommon.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
          # export LD_LIBRARY_PATH="${e.pkgs.lib.makeLibraryPath e.runtimeLibs}:$LD_LIBRARY_PATH"
          echo "PS2 SDK Build Environment Loaded for ${system}!"
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

      elf = e.craneLib.buildPackage {
        inherit src;
        pname = "bevy-ps2-elf";
        version = "0.1.0";

        cargoVendorDir = e.craneLib.vendorMultipleCargoDeps {
          inherit (e.craneLib.findCargoFiles src) cargoConfigs;
          cargoLockList = [
            ./Cargo.lock
            "${e.rustToolchain.passthru.availableComponents.rust-src}/lib/rustlib/src/rust/library/Cargo.lock"
          ];
        };
        
        cargoExtraArgs = "--target ${./mips64el-sony-ps2.json} -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec";

        nativeBuildInputs = [e.pkgs.pkg-config];
        buildInputs = [e.ps2dev];

        preBuild = ''
          export PS2DEV="${e.ps2dev}"
          export PS2SDK="$PS2DEV/ps2sdk"
          export GSKIT="$PS2DEV/gsKit"
          export PATH=$PATH:$PS2DEV/bin:$PS2DEV/ee/bin:$PS2DEV/iop/bin:$PS2DEV/dvp/bin:$PS2SDK/bin
          export PS2SDK_LIBDIR="$PS2SDK/ee/lib"
          export PS2SDK_INCDIR="$PS2SDK/ee/include"
        '';

        installPhaseCommand = ''
          mkdir -p $out/bin
          cp target/mips64el-sony-ps2/release/bevy-ps2 $out/bin/BOOT.ELF
        '';

        doCheck = false;
      };

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
    in {
      # Build PS2 ISO and run in PCSX2.
      default = {
        type = "app";
        program = "${e.pkgs.writeShellScriptBin "run-iso" ''
          set -e
          ISO_PATH="${self.packages.${system}.iso}/bevy-ps2.iso"
          exec ${e.pkgs.pcsx2}/bin/pcsx2-qt -disc "$ISO_PATH"
        ''}/bin/run-iso";
      };

      # Build PS2 ISO and run in PCSX2 (with debugger.)
      debugger = {
        type = "app";
        program = "${e.pkgs.writeShellScriptBin "run-iso-debug" ''
          set -e
          ISO_PATH="${self.packages.${system}.iso}/bevy-ps2.iso"
          exec ${e.pkgs.pcsx2}/bin/pcsx2-qt -debugger -disc "$ISO_PATH"
        ''}/bin/run-iso-debug";
      };

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
