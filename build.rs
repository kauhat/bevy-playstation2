use std::env;

fn main() {
    // Read $PS2SDK path from environment
    let ps2sdk = env::var("PS2SDK").unwrap();

    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    println!("Arch: {}", target_arch.as_str());

    if (target_arch.as_str() == "mips") {
        println!("It's MIPS! <3");

        // Tell cargo where to search for PS2SDK static libraries (.a)
        println!("cargo:rustc-link-search=native={}/ee/lib", ps2sdk);
        println!("cargo:rustc-link-arg=-T{}/ee/startup/linkfile", ps2sdk);

        // // Link required PS2SDK C libraries (e.g., libkernel.a, libdebug.a)
        println!("cargo:rustc-link-lib=static=kernel");
        println!("cargo:rustc-link-lib=static=debug");
        println!("cargo:rustc-link-lib=static=c"); // PS2 glue libc
    }
}
