# Bevy Playstation 2

Using some PS2SDK/PS2DEV bindings.

- https://github.com/ps2dev/ps2sdk
- https://github.com/ps2dev/gsKit

...and maybe using some rusty implementations with Prussia

- https://github.com/Ravenslofty/prussia



## Development environent

Use `nix develop` or [direnv](https://direnv.net/) to set up an environment with libraries and tools from the [PS2 homebrew toolchain](https://github.com/ps2dev/ps2dev).

Once your environment is set up, use [just] to run tasks.

```bash
# Use ps2-cargo wrapper for the usual tasks...
just ps2-cargo build
just ps2-cargo run
just ps2-cargo check
just ps2-cargo fix
just ps2-cargo test

# Use regular cargo to target PC
cargo run

# Run in PCSX2...
just ps2-run-elf
just ps2-run-iso
just ps2-debug-elf
```


## Nix build

### PC targets

-   ```
    # Build and run PC engine
    nix run .#pc
    ```

- `nix build .#pc`

### Console targets

-   ```bash
    # Build and run in PCSX2
    nix run
    ```

-   ```bash
    # Build and run in PCSX2 (with debugger)
    nix run .#debugger
    ```

- `nix build .#elf`
- `nix build .#iso`

nix builds seem to fail while linking.

## Targets

I have no idea what I'm doing, I've gotten this far with a lot of trial and error.

### [[./mips64el-sony-ps2.json]]

Builds and links against PS2SDK libs.

### [[./mipsel-sony-ps2.json]]

Fails to link. maybe should try using Prussia's [[./vendor/prussia/ps2.json]]

## Goals

- A functional (no_std) port of Bevy for PS2 hardware
- basic PS2 renderer
- PC version for debug, with standard render pipeline

[just]: https://just.systems/man/en/introduction.html