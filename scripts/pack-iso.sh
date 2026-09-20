#!/usr/bin/env bash
set -e

# Arguments or defaults
ELF_IN=${1:-"target/mipsel-sony-ps2/release/bevy-ps2"}
ISO_OUT=${2:-"target/bevy-ps2.iso"}
ISO_ROOT="target/iso_root"

echo "Preparing ISO directory structure..."
mkdir -p "$ISO_ROOT"

# Copy the compiled ELF
cp "$ELF_IN" "$ISO_ROOT/BOOT.ELF"

# Generate SYSTEM.CNF if not present in the workspace
if [ -f "SYSTEM.CNF" ]; then
    cp SYSTEM.CNF "$ISO_ROOT/SYSTEM.CNF"
else
    printf "BOOT2 = cdrom0:\\BOOT.ELF;1\r\nVER = 1.00\r\nVMODE = NTSC\r\n" > "$ISO_ROOT/SYSTEM.CNF"
fi

echo "Building ISO image..."
mkisofs -l -iso-level 1 -volid "BEVY_PS2" -sysid "PLAYSTATION" -o "$ISO_OUT" "$ISO_ROOT"

echo "Successfully created $ISO_OUT"