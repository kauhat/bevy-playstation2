{
  pkgs ? null,
  stdenv ? pkgs.stdenv,
  fetchFromGitHub ? pkgs.fetchFromGitHub,
  pkg-config ? pkgs.pkg-config,
  bash ? pkgs.bash,
  ps2devToolchain,
  ...
}: let
  lwipSrc = fetchFromGitHub {
    owner = "lwip-tcpip";
    repo = "lwip";
    rev = "d08f4773edd0182b7910fc8f046eed82ffcd67c9";
    hash = "sha256-2eUtb4w2s/C/i9mXTLDs6PYhWxLAye2ElPfktadH89Y=";
  };

  fatfsSrc = fetchFromGitHub {
    owner = "fjtrujy";
    repo = "FatFs";
    rev = "cac3314968129ee0e2c019ae48e5c9cd262ceb4e";
    hash = "sha256-nHwekmfIXImbopFUlXQchmooo8FY+1xeSHj1UYtpdRA=";
  };
in
  stdenv.mkDerivation {
    pname = "ps2sdk";
    version = "2.0.0";

    src = fetchFromGitHub {
      owner = "ps2dev";
      repo = "ps2sdk";
      rev = "be12adcb7c8aa6065202e8360d39fe3298eadad3";
      hash = "sha256-rYIom3ZLz0qne7ckaS1N7KBOvgxK3GbTszn/g1h7Ufw=";
    };

    nativeBuildInputs = [
      ps2devToolchain
      pkg-config
      bash
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
      #!${bash}/bin/bash
      exit 0
      EOF
      chmod +x download_dependencies.sh

      patchShebangs .
    '';

    preBuild = ''
      export PS2DEV="${ps2devToolchain}"
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
  }
