use std::env;

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

    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();

    if target_vendor != "sony" {
        return;
    }

    println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINKFILE");
    println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINK_GROUP");
    println!("cargo:rerun-if-env-changed=DEP_PRUSSIA_RT_LINK_SEARCH");
    println!("cargo:rerun-if-env-changed=DEP_PRUSSIA_RT_LINK_LIB");

    if let Ok(linkfile) = env::var("DEP_PS2SDK_LINKFILE") {
        println!("cargo:rustc-link-arg=-T{linkfile}");
    }

    if let Ok(link_group) = env::var("DEP_PS2SDK_LINK_GROUP") {
        println!("cargo:rustc-link-arg={link_group}");
    }

    // PruSSia's assembly helpers (CoP0 register access, atomic 64-bit load/store).
    //
    // `prussia_rt`'s build script already asks for `-lprussia-rt`, but cargo puts a dependency's
    // native libraries ahead of the objects that reference them, and GNU ld only scans an archive
    // once. Re-emitting it as a link arg lands it at the end of the linker's library list, which
    // is the same trick the PS2SDK library group above uses.
    if let (Ok(search), Ok(lib)) = (
        env::var("DEP_PRUSSIA_RT_LINK_SEARCH"),
        env::var("DEP_PRUSSIA_RT_LINK_LIB"),
    ) {
        println!("cargo:rustc-link-search=native={search}");
        println!("cargo:rustc-link-arg=-Wl,-Bstatic,-l{lib},-Bdynamic");
    }
}
