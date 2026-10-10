// Target-specific platform backends.
#![allow(unused)]

//
// PC...
//
cfg_if::cfg_if! {
    if #[cfg(not(target_vendor = "sony"))] {
        pub mod pc;

        pub use pc::*;
        pub use pc::PcPlatformPlugin as PlatformPlugin;
    }
}

//
// PlayStation 2...
//
cfg_if::cfg_if! {
    if #[cfg(target_vendor = "sony")] {
        pub mod playstation2;

        pub use playstation2::*;
        pub use playstation2::Ps2PlatformPlugin as PlatformPlugin;
    }
}
