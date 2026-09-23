use std::env;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    if target_arch != "mips64" {
        println!("Unexpected target architecture: {}", target_arch);
        return;
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PS2SDK");

    // Read $PS2SDK path from environment
    let ps2sdk =
        env::var("PS2SDK").expect("PS2SDK environment variable is required for MIPS builds");

    // PS2SDK static libraries and linker script...
    println!("cargo:rustc-link-search=native={}/ee/lib", ps2sdk);
    println!("cargo:rustc-link-arg=-T{}/ee/startup/linkfile", ps2sdk);

    // Export the linker script to dependents.
    println!("cargo:linkfile_path=-T{}/ee/startup/linkfile", ps2sdk);

    // PS2SDK linker script...
    // println!("cargo:rustc-link-arg={}/ee/startup/src/crt0.o", ps2sdk);

    // Dead code elimination
    // println!("cargo:rustc-link-arg=-Wl,--gc-sections");
}
