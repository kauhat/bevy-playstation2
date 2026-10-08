# Default target JSON specification file
TARGET := "mips64el-sony-ps2.json"
ELF_PATH := "target/mips64el-sony-ps2/debug/bevy-ps2"
ISO_PATH := "target/bevy-ps2.iso"

# Default command
default: run-elf

# Compile the release ELF binary for PS2
build-ps2:
    cargo build-ps2

    mips64r5900el-ps2-elf-readelf -h {{ ELF_PATH }}

# Create a bootable PS2 ISO.
build-iso: build-ps2
    @./scripts/pack-iso.sh "{{ ELF_PATH }}" "{{ ISO_PATH }}"

# Run PC development version.
run-pc:
    cargo run

# Build and execute directly in PCSX2.
run-elf: build-ps2
    @if [ ! -f "{{ ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ ELF_PATH }}"; \
        exit 1; \
    fi

    @echo "Launching {{ ELF_PATH }} in PCSX2..."

    pcsx2-qt -batch -earlyconsolelog -elf "$(realpath {{ ELF_PATH }})";

# Run the ISO in PCSX2.
run-iso: build-iso
    @if [ ! -f "{{ ISO_PATH }}" ]; then \
        echo "Error: Could not find ISO at {{ ISO_PATH }}"; \
        exit 1; \
    fi

    @echo "Launching {{ ISO_PATH }} in PCSX2..."

    pcsx2-qt -batch -earlyconsolelog "$(realpath {{ ISO_PATH }})";

# Build and execute with debugger.
debug: build-ps2
    @if [ ! -f "{{ ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ ELF_PATH }}"; \
        exit 1; \
    fi

    @echo "Debugging {{ ELF_PATH }} in PCSX2..."

    pcsx2-qt -batch -debugger -earlyconsolelog -elf "$(realpath {{ ELF_PATH }})";

# PS2SDK...
ps2sdk-bindings:
    bindgen ./crates/ps2sdk-sys/wrapper.h \
        -o ./crates/ps2sdk-sys/src/bindings.rs \
        --use-core \
        --no-layout-tests \
        -- \
        -D_EE \
        -target mips64el-unknown-elf \
        -I$PS2DEV/ee/mips64r5900el-ps2-elf/include \
        -I$PS2DEV/ps2sdk/common/include \
        -I$PS2DEV/ps2sdk/ee/include \
        -I$PS2DEV/ps2sdk/iop/include \
        -I$PS2DEV/ee/include \
        -I$PS2DEV/gsKit/include \
        -I$PS2DEV/iop/include

# Prussia fork...
prussia_branch := "master"
prussia_prefix := "vendor/prussia"

prussia-remotes:
    -git remote add "prussia-fork" git@github.com:kauhat/prussia.git
    -git remote add "prussia-upstream" git@github.com:Ravenslofty/prussia.git

prussia-subtree-add remote="prussia-fork" branch=prussia_branch:
    git subtree add --prefix={{ prussia_prefix }} {{ remote }} {{ branch }}

prussia-subtree-pull remote="prussia-fork" branch=prussia_branch:
    git subtree pull --prefix={{ prussia_prefix }} {{ remote }} {{ branch }}

prussia-subtree-push remote="prussia-fork" branch=prussia_branch:
    git subtree push --prefix={{ prussia_prefix }} {{ remote }} {{ branch }} 
