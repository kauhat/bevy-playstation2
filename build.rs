use std::env;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_arch == "mips64" {
        // Get linkfile and search path from the ps2sdk-sys build script.
        let bin_name = env::var("CARGO_PKG_NAME")
            .expect("CARGO_PKG_NAME is not set");
    
        let linkfile = env::var("DEP_PS2SDK_LINKFILE_PATH")
            .expect("DEP_PS2SDK_LINKFILE_PATH is not set");
    
        let link_search = env::var("DEP_PS2SDK_LINK_SEARCH")
            .expect("DEP_PS2SDK_LINK_SEARCH is not set");
    
        println!("cargo:rustc-link-search=native={link_search}");
        println!("cargo:rustc-link-arg-bin={bin_name}=-T{linkfile}");
    
        println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINKFILE_PATH");
        println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINK_SEARCH");
    }
}
