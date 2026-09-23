use std::env;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_arch == "mips64" {
        // Get file path from the ps2sdk-sys build script.
        let linkfile = env::var("DEP_PS2SDK_LINKFILE_PATH")
            .expect("Failed to get linkfile path from ps2sdk-sys");

        // Inject it into the final binary link step.
        println!("cargo:rustc-link-arg=-T{}", linkfile);
    }
}
