# Default target JSON file
TARGET := "mipsel-sony-ps2.json"

# Default build command
default: build

# Compile the release ELF binary for PS2
build:
	cargo +nightly build \
		-Z build-std=core,alloc \
		-Z json-target-spec \
		--target {{TARGET}} \
		--release

# Optional: Run debug build
debug:
	cargo +nightly build \
		-Z build-std=core,alloc \
		-Z json-target-spec \
		--target {{TARGET}}

# Clean cargo artifacts
clean:
	cargo clean
