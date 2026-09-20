# Default target JSON specification file
TARGET := "mipsel-sony-ps2.json"
ELF_PATH := "target/mipsel-sony-ps2/release/bevy-ps2"
ISO_PATH := "target/bevy-ps2.iso"

# Default command
default: run-elf

# Compile the release ELF binary for PS2
build-ps2:
    cargo build-ps2-release

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

    pcsx2-qt -batch -earlyconsolelog -elf "{{ ELF_PATH }}";

debug-elf: build-ps2
    @if [ ! -f "{{ ELF_PATH }}" ]; then \
        echo "Error: Could not find compiled ELF at {{ ELF_PATH }}"; \
        exit 1; \
    fi

    realpath {{ ELF_PATH }}

    pcsx2-qt -batch -debugger -earlyconsolelog -elf "{{ ELF_PATH }}";

# Run the ISO in PCSX2.
run-iso: build-iso
    @if [ ! -f "{{ ISO_PATH }}" ]; then \
        echo "Error: Could not find ISO at {{ ISO_PATH }}"; \
        exit 1; \
    fi
    @echo "Launching {{ ISO_PATH }} in PCSX2..."

    pcsx2-qt "{{ ISO_PATH }}";
