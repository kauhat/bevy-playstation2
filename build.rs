use std::env;

mod fixtures {
    include!("build/fixtures.rs");
}

fn main() {
    // Crane's `buildDepsOnly` compiles a throwaway dummy binary just to warm the dependency
    // cache. It cannot boot through the PS2SDK crt0, so it sets this to ask for a link that
    // skips startup instead.
    //
    // This is deliberately an environment switch rather than a `postPatch` that swaps build.rs
    // out: everything in a Nix sandbox has mtime 0, so cargo's mtime-based freshness check
    // cannot tell the two versions of this script apart and happily reuses the stub's recorded
    // output for the real build. That leaves the final binary with no linker script, entry
    // point 0, and `--gc-sections` discarding the whole program.
    println!("cargo:rerun-if-env-changed=BEVY_PS2_DEPS_ONLY");
    if env::var_os("BEVY_PS2_DEPS_ONLY").is_some() {
        println!("cargo:rustc-link-arg=-nostartfiles");
        println!("cargo:rustc-link-arg=-Wl,--entry=0");
        return;
    }

    // Stage the Ratchet & Clank lumps the PS2 tests parse, and their host-computed answers. Harmless (and quick)
    // when the extracted tree is absent: it just writes empty blobs and a `PRESENT = false` marker.
    fixtures::generate();

    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();

    if target_vendor != "sony" {
        return;
    }

    println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINKFILE");
    println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINK_GROUP");

    if let Ok(linkfile) = env::var("DEP_PS2SDK_LINKFILE") {
        println!("cargo:rustc-link-arg=-T{linkfile}");
    }

    if let Ok(link_group) = env::var("DEP_PS2SDK_LINK_GROUP") {
        println!("cargo:rustc-link-arg={link_group}");
    }
}
