# Default target JSON specification file
TARGET := "mipsel-sony-ps2.json"
ELF_PATH := "target/mipsel-sony-ps2/release/bevy-ps2"
ISO_DIR := "target/iso_root"
ISO_PATH := "target/bevy-ps2.iso"

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
        -Z build-std-features=compiler-builtins-mem \
        -Z json-target-spec \
        --target {{ TARGET }} \
        --profile {{ PROFILE }}

# Build and execute directly in PCSX2
run: build
    @if [ ! -f "{{ ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ ELF_PATH }}"; \
        exit 1; \
    fi

    pcsx2-qt -batch -earlyconsolelog -elf "{{ ELF_PATH }}";

debug ELF_PATH='target/mipsel-sony-ps2/debug/bevy-ps2': (build 'dev')
    @if [ ! -f "{{ ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ ELF_PATH }}"; \
        exit 1; \
    fi

    realpath {{ ELF_PATH }}

    pcsx2-qt -batch -debugger -earlyconsolelog -elf "{{ ELF_PATH }}";

# Create a bootable PS2 ISO
iso: build
    @echo "Preparing ISO directory structure..."
    @mkdir -p {{ ISO_DIR }}

    @echo "Copying ELF..."
    @cp "{{ ELF_PATH }}" "{{ ISO_DIR }}/BOOT.ELF"

    @echo "Generating SYSTEM.CNF..."
    @echo "BOOT2 = cdrom0:\BOOT.ELF;1" > "{{ ISO_DIR }}/SYSTEM.CNF"
    @echo "VER = 1.00" >> "{{ ISO_DIR }}/SYSTEM.CNF"
    @echo "VMODE = NTSC" >> "{{ ISO_DIR }}/SYSTEM.CNF"

    @echo "Building ISO image..."
    mkisofs -l -iso-level 1 -volid "BEVY_PS2" -sysid "PLAYSTATION" -o "{{ ISO_PATH }}" "{{ ISO_DIR }}"
    @echo "Successfully created {{ ISO_PATH }}"

# Run the ISO in PCSX2
run-iso: iso
    @if [ ! -f "{{ ISO_PATH }}" ]; then \
        echo "Error: Could not find ISO at {{ ISO_PATH }}"; \
        exit 1; \
    fi
    @echo "Launching {{ ISO_PATH }} in PCSX2..."

    pcsx2-qt "{{ ISO_PATH }}";

# Clean cargo artifacts
clean:
    cargo clean
