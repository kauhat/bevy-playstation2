set shell := ["bash", "-uc"]

PS2_TARGET := "mips64el-sony-ps2.json"
PS2_ELF_PATH := "target/mips64el-sony-ps2/debug/bevy-ps2"
PS2_ISO_PATH := "target/bevy-ps2.iso"

default: ps2-run-elf

#
# PS2 tasks...
#

ps2-cargo +ARGS="":
    CARGO_BUILD_TARGET={{ PS2_TARGET }} \
    CARGO_UNSTABLE_BUILD_STD=core,alloc \
    CARGO_UNSTABLE_BUILD_STD_FEATURES=compiler-builtins-mem \
    cargo {{ ARGS }}

# Compile the release ELF binary for PS2
ps2-build:
    @just ps2-cargo build

    # Verify that the ELF was built successfully.
    mips64r5900el-ps2-elf-readelf -h {{ PS2_ELF_PATH }}

# Create a bootable PS2 ISO.
ps2-pack-iso: ps2-build
    ./scripts/pack-iso.sh "{{ PS2_ELF_PATH }}" "{{ PS2_ISO_PATH }}"

# Build and execute directly in PCSX2.
ps2-run-elf: ps2-build
    #!/usr/bin/env bash
    set -euxo pipefail

    if [ ! -f "{{ PS2_ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ PS2_ELF_PATH }}"; \
        exit 1; \
    fi

    echo "Launching {{ PS2_ELF_PATH }} in PCSX2..."

    pcsx2-qt -batch -earlyconsolelog -elf "$(realpath {{ PS2_ELF_PATH }})";

# Build and execute with debugger.
ps2-debug-elf: ps2-build
    #!/usr/bin/env bash
    set -euxo pipefail

    if [ ! -f "{{ PS2_ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ PS2_ELF_PATH }}"; \
        exit 1; \
    fi

    echo "Debugging {{ PS2_ELF_PATH }} in PCSX2..."

    pcsx2-qt -batch -debugger -earlyconsolelog -elf "$(realpath {{ PS2_ELF_PATH }})";

# Run the ISO in PCSX2.
ps2-run-iso: ps2-pack-iso
    #!/usr/bin/env bash
    set -euxo pipefail

    if [ ! -f "{{ PS2_ISO_PATH }}" ]; then \
        echo "Error: Could not find ISO at {{ PS2_ISO_PATH }}"; \
        exit 1; \
    fi

    echo "Launching {{ PS2_ISO_PATH }} in PCSX2..."

    pcsx2-qt -batch -earlyconsolelog "$(realpath {{ PS2_ISO_PATH }})";

#
# PC tasks...
#

pc-run:
    LD_LIBRARY_PATH="$(nix eval --raw .#runtimeLibsEnv)/lib:${LD_LIBRARY_PATH:-}" cargo run

#
# PS2SDK and other PS2DEV bindings...
#

CRATE_PS2SDK_SYS := "./crates/ps2sdk-sys"

PS2DEV_INCLUDES := "-I$PS2DEV/ee/mips64r5900el-ps2-elf/include -I$PS2DEV/ps2sdk/common/include -I$PS2DEV/ps2sdk/ee/include -I$PS2DEV/ps2sdk/iop/include -I$PS2DEV/ee/include -I$PS2DEV/gsKit/include -I$PS2DEV/iop/include"
PS2DEV_FLAGS_BINDGEN := "--use-core --no-layout-tests"
PS2DEV_FLAGS_CLANG := "-D_EE -target mips64el-unknown-elf"
blocklist_types := "--blocklist-type u8 --blocklist-type u16 --blocklist-type u32 --blocklist-type u64 --blocklist-type u128 --blocklist-type qword --blocklist-type vu8 --blocklist-type vu16 --blocklist-type vu32 --blocklist-type vu64 --blocklist-type vu128"

ps2sdk-bindings:
    #!/usr/bin/env bash
    set -euo pipefail

    mkdir -p {{ CRATE_PS2SDK_SYS }}/src/bindings

    echo "Generating common bindings..."
    bindgen {{ CRATE_PS2SDK_SYS }}/wrappers/common.h \
        -o {{ CRATE_PS2SDK_SYS }}/src/bindings/common.rs \
        {{ PS2DEV_FLAGS_BINDGEN }} -- \
        {{ PS2DEV_FLAGS_CLANG }} {{ PS2DEV_INCLUDES }}

    modules=("kernel" "gskit" "draw")

    for module in "${modules[@]}"; do
        echo "Generating ${module} bindings..."
        bindgen "{{ CRATE_PS2SDK_SYS }}/wrappers/${module}.h" \
            -o "{{ CRATE_PS2SDK_SYS }}/src/bindings/${module}.rs" \
            {{ PS2DEV_FLAGS_BINDGEN }} {{ blocklist_types }} -- \
            {{ PS2DEV_FLAGS_CLANG }} {{ PS2DEV_INCLUDES }}
    done

#
# Prussia fork...
#

PRUSSIA_BRANCH := "master"
PRUSSIA_PREFIX := "vendor/prussia"

prussia-remotes:
    -git remote add "prussia-fork" git@github.com:kauhat/prussia.git
    -git remote add "prussia-upstream" git@github.com:Ravenslofty/prussia.git

prussia-subtree-add remote="prussia-fork" branch=PRUSSIA_BRANCH:
    git subtree add --prefix={{ PRUSSIA_PREFIX }} {{ remote }} {{ branch }}

prussia-subtree-pull remote="prussia-fork" branch=PRUSSIA_BRANCH:
    git subtree pull --prefix={{ PRUSSIA_PREFIX }} {{ remote }} {{ branch }}

prussia-subtree-push remote="prussia-fork" branch=PRUSSIA_BRANCH:
    git subtree push --prefix={{ PRUSSIA_PREFIX }} {{ remote }} {{ branch }} 
