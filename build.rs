use std::env;

fn main() {
    // Read $PS2SDK path from environment
    let ps2sdk = env::var("PS2SDK").unwrap();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    println!("Arch: {}", target_arch.as_str());

    if target_arch == "mips64" {
        println!("It's MIPS! <3");

        // PS2SDK static libraries...
        println!("cargo:rustc-link-search=native={}/ee/lib", ps2sdk);

        // PS2SDK linker script...
        println!("cargo:rustc-link-arg=-T{}/ee/startup/linkfile", ps2sdk);
    }
}
