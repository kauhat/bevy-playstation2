use std::env;

fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    if target_arch != "mips64" {
        println!("Unexpected target architecture: {}", target_arch);
    }

    // Read $PS2SDK path from environment
    let ps2sdk = env::var("PS2SDK").unwrap();

    // PS2SDK static libraries...
    println!("cargo:rustc-link-search=native={}/ee/lib", ps2sdk);

    // PS2SDK linker script...
    println!("cargo:rustc-link-arg=-T{}/ee/startup/linkfile", ps2sdk);

    //
    println!("cargo:rustc-link-arg=-Wl,--start-group");
    println!("cargo:rustc-link-lib=static=cglue");
    println!("cargo:rustc-link-lib=static=c");
    println!("cargo:rustc-link-lib=static=kernel");
    println!("cargo:rustc-link-lib=static=debug");
    println!("cargo:rustc-link-lib=static=atomic");
    println!("cargo:rustc-link-lib=static=pthreadglue");
    println!("cargo:rustc-link-lib=static=pthread");
    println!("cargo:rustc-link-arg=-Wl,--end-group");

    println!("cargo:rerun-if-changed=build.rs");
}
