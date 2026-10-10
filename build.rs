use std::env;

fn main() {
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();

    if target_vendor == "sony" {
        println!("cargo:rerun-if-env-changed=DEP_PS2SDK_LINKFILE");
        println!("cargo:rerun-if-env-changed=DEP_PS2SDK_ARGS");

        if let Ok(linkfile) = env::var("DEP_PS2SDK_LINKFILE") {
            println!("cargo:rustc-link-arg=-T{linkfile}");
        }

        if let Ok(link_group) = env::var("DEP_PS2SDK_LINK_GROUP") {
            println!("cargo:rustc-link-arg={link_group}");
        }
    };
}
