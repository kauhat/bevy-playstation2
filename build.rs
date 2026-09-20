use std::env;

fn main() {
    // Read $PS2SDK path from environment
    let ps2sdk = env::var("PS2SDK").unwrap();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    println!("Arch: {}", target_arch.as_str());

    if target_arch == "mips64" {
        println!("It's MIPS! <3");

        // Tell cargo where to search for PS2SDK static libraries (.a)
        println!("cargo:rustc-link-search=native={}/ee/lib", ps2sdk);
        println!("cargo:rustc-link-arg=-T{}/ee/startup/linkfile", ps2sdk);

        //
        println!("cargo:rustc-link-arg=-Wl,--start-group");
        println!("cargo:rustc-link-arg=-lc");
        println!("cargo:rustc-link-arg=-lcglue");
        println!("cargo:rustc-link-arg=-lkernel");
        println!("cargo:rustc-link-arg=-latomic");
        // println!("cargo:rustc-link-arg=-lpthread");
        println!("cargo:rustc-link-arg=-ldebug");
        println!("cargo:rustc-link-arg=-Wl,--end-group");
    }
}
