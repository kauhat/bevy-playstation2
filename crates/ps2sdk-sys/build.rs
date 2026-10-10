use std::env;

fn main() {
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    if target_vendor != "sony" {
        return;
    }

    let ps2dev = env::var("PS2DEV").expect("PS2DEV environment variable required");
    let ps2sdk = env::var("PS2SDK").expect("PS2SDK environment variable required");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PS2DEV");
    println!("cargo:rerun-if-env-changed=PS2SDK");

    // Pass search paths to dependent crates
    println!("cargo:rustc-link-search=native={ps2sdk}/ee/lib");
    println!("cargo:rustc-link-search=native={ps2dev}/gsKit/lib");
    println!("cargo:rustc-link-search=native={ps2dev}/ee/mips64r5900el-ps2-elf/lib");

    // Export linkfile and library group metadata to downstream crates
    println!("cargo:linkfile={ps2sdk}/ee/startup/linkfile");
    println!("cargo:args=-Wl,--start-group,-lgskit,-ldmakit,-ldraw,-lgraph,-ldma,-lpad,-lpatches,-lkernel,-lcdvd,-lcglue,-ldebug,-lpthread,-lpthreadglue,-lc,-lm,--end-group");
}