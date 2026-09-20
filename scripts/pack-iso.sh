#!/usr/bin/env bash
set -e

# Arguments or defaults
ELF_IN=${1:-"target/mipsel-sony-ps2/release/bevy-ps2"}
ISO_OUT=${2:-"target/bevy-ps2.iso"}
ISO_ROOT="target/iso_root"

echo "Preparing ISO directory structure..."
mkdir -p "$ISO_ROOT"

# Copy the ELF and our static SYSTEM.CNF
cp "$ELF_IN" "$ISO_ROOT/BOOT.ELF"
cp SYSTEM.CNF "$ISO_ROOT/SYSTEM.CNF"

echo "Building ISO image..."
mkisofs -l -iso-level 1 -volid "BEVY_PS2" -sysid "PLAYSTATION" -o "$ISO_OUT" "$ISO_ROOT"

echo "Successfully created $ISO_OUT"