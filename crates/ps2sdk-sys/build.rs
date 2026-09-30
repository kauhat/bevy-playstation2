use std::env;

fn main() {
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    
    if target_vendor != "sony" {
        println!("Unexpected target vendor: {}", target_vendor);
        return;
    }

    let ps2sdk =
        env::var("PS2SDK").expect("PS2SDK environment variable is required for MIPS builds");

    let libdir = format!("{ps2sdk}/ee/lib");
    let linkfile = format!("{ps2sdk}/ee/startup/linkfile");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PS2SDK");

    // These apply to ps2sdk-sys itself.
    println!("cargo:rustc-link-search=native={libdir}");
    println!("cargo:rustc-link-arg=-T{linkfile}");

    // These are metadata for the direct dependent.
    println!("cargo:linkfile_path={linkfile}");
    println!("cargo:link_search={libdir}");
}
