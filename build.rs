use std::env;

fn main() {
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();

    if target_vendor == "sony" {
        // Get linkfile and search path from the ps2sdk-sys build script.
        let bin_name = env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME is not set");

        // Retrieve search paths from both ps2sdk-sys and prussia_rt
        // let ps2sdk_link_search =
        //     env::var("DEP_PS2SDK_LINK_SEARCH").expect("DEP_PS2SDK_LINK_SEARCH is not set");

        // let prussia_link_search =
        //     env::var("DEP_PRUSSIA_RT_LINK_SEARCH").expect("DEP_PRUSSIA_RT_LINK_SEARCH is not set");

        // Linker script selection: choose Prussia's linker script or PS2SDK's
        // Option A: Use Prussia's user-linkfile.ld
        // let linkfile = env::var("DEP_PRUSSIA_RT_LINKFILE_PATH").unwrap();

        // Option B: Use PS2SDK's standard linkfile
        let linkfile = env::var("DEP_PS2SDK_LINKFILE_PATH").unwrap();

        // Pass native search paths to the linker
        // println!("cargo:rustc-link-search={prussia_link_search}");
        // println!("cargo:rustc-link-search={ps2sdk_link_search}");

        // println!("{ps2sdk_link_search}");

        // FIX: Pass the static library as a raw argument to force it to the end of the command line
        // println!("cargo:rustc-link-arg=-lprussia-rt");

        // Pass the linker script argument
        println!("cargo:rustc-link-arg-bin={bin_name}=-T{linkfile}");

        // Re-run triggers
        println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINKFILE_PATH");
        println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINK_SEARCH");
        println!("cargo:rerun-if-env-changed=DEP_PRUSSIA_RT_LINKFILE_PATH");
        println!("cargo:rerun-if-env-changed=DEP_PRUSSIA_RT_LINK_SEARCH");
    }
}
