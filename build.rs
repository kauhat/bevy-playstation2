use std::env;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_arch == "mips64" {
        // Get linkfile and search path from the ps2sdk-sys build script.
        let linkfile = env::var("DEP_PS2SDK_LINKFILE_PATH")
            .expect("Failed to get linkfile path from ps2sdk-sys");
    
        let link_search = env::var("DEP_PS2SDK_LINK_SEARCH")
            .expect("Failed to get link search path from ps2sdk-sys");
    
        // These apply to the final bevy-ps2 binary.
        println!("cargo:rustc-link-search=native={link_search}");
        println!("cargo:rustc-link-arg=-T{linkfile}");
    
        println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINKFILE_PATH");
        println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINK_SEARCH");
    }
}
