set shell := ["bash", "-uc"]

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
    LD_LIBRARY_PATH="$(nix eval --raw .#runtimeLibsEnv)/lib:${LD_LIBRARY_PATH:-}" cargo run

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
ps2_sys_dir := "./crates/ps2sdk-sys"
ps2_includes := "-I$PS2DEV/ee/mips64r5900el-ps2-elf/include -I$PS2DEV/ps2sdk/common/include -I$PS2DEV/ps2sdk/ee/include -I$PS2DEV/ps2sdk/iop/include -I$PS2DEV/ee/include -I$PS2DEV/gsKit/include -I$PS2DEV/iop/include"
bindgen_flags := "--use-core --no-layout-tests"
clang_flags := "-D_EE -target mips64el-unknown-elf"
blocklist_types := "--blocklist-type u8 --blocklist-type u16 --blocklist-type u32 --blocklist-type u64 --blocklist-type u128 --blocklist-type qword --blocklist-type vu8 --blocklist-type vu16 --blocklist-type vu32 --blocklist-type vu64 --blocklist-type vu128"

ps2sdk-bindings:
    #!/usr/bin/env bash
    set -euo pipefail

    mkdir -p {{ ps2_sys_dir }}/src/bindings

    echo "Generating common bindings..."
    bindgen {{ ps2_sys_dir }}/wrappers/common.h \
        -o {{ ps2_sys_dir }}/src/bindings/common.rs \
        {{ bindgen_flags }} -- \
        {{ clang_flags }} {{ ps2_includes }}

    modules=("kernel" "gskit" "draw")

    for module in "${modules[@]}"; do
        echo "Generating ${module} bindings..."
        bindgen "{{ ps2_sys_dir }}/wrappers/${module}.h" \
            -o "{{ ps2_sys_dir }}/src/bindings/${module}.rs" \
            {{ bindgen_flags }} {{ blocklist_types }} -- \
            {{ clang_flags }} {{ ps2_includes }}
    done

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
