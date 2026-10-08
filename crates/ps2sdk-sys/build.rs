use std::env;

fn main() {
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();

    if target_vendor != "sony" {
        println!("Unexpected target vendor: {}", target_vendor);
        return;
    }

    let ps2dev_root =
        env::var("PS2DEV").expect("PS2DEV environment variable is required for MIPS builds");

    let ps2sdk =
        env::var("PS2SDK").expect("PS2SDK environment variable is required for MIPS builds");

    let linkfile = format!("{ps2sdk}/ee/startup/linkfile");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PS2SDK");
    println!("cargo:rerun-if-env-changed=PS2DEV");

    // Search paths for PS2SDK, gsKit, and EE toolchain libs
    println!("cargo:rustc-link-search=native={ps2sdk}/ee/lib");
    println!("cargo:rustc-link-search=native={ps2sdk}/iop/lib");
    println!("cargo:rustc-link-search=native={ps2dev_root}/gsKit/lib");

    // Linker script
    println!("cargo:rustc-link-arg=-T{linkfile}");

    // Core PS2SDK libraries (link order matters for GCC/ld dependencies!)
    println!("cargo:rustc-link-lib=static=gskit");
    println!("cargo:rustc-link-lib=static=dmakit");
    println!("cargo:rustc-link-lib=static=draw");
    println!("cargo:rustc-link-lib=static=graph");
    println!("cargo:rustc-link-lib=static=dma");
    println!("cargo:rustc-link-lib=static=kernel");

    // Metadata for downstream dependents (bevy-ps2 root crate)
    println!("cargo:linkfile_path={linkfile}");
    println!("cargo:link_search={ps2sdk}/ee/lib");
    println!("cargo:link_search={ps2dev_root}/gsKit/lib");
}
