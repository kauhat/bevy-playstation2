# Default target JSON specification file
TARGET := "mipsel-sony-ps2.json"
ELF_PATH := "target/mipsel-sony-ps2/debug/bevy-ps2"

# Default command
default: build

# Ensure rust-src component is present for build-std
setup:
    @if command -v rustup >/dev/null 2>&1; then \
        rustup component add rust-src --toolchain nightly || true; \
    fi

# Compile the release ELF binary for PS2
build PROFILE='release': setup
    cargo +nightly build \
        -Z build-std=core,alloc \
        -Z json-target-spec \
        --target {{ TARGET }} \
        --profile {{ PROFILE }}

# Build and execute directly in PCSX2
run: build
    @if [ ! -f "{{ ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ ELF_PATH }}"; \
        exit 1; \
    fi

    pcsx2-qt -batch -elf "{{ ELF_PATH }}";

debug ELF_PATH='target/mipsel-sony-ps2/debug/bevy-ps2' PROFILE='dev': build
    @if [ ! -f "{{ ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ ELF_PATH }}"; \
        exit 1; \
    fi

    pcsx2-qt -batch -debugger -elf "{{ ELF_PATH }}";
# Clean cargo artifacts
clean:
    cargo clean
