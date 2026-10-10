// Target-specific platform backends.
//
// Each `cfg_if!` covers exactly one platform (an `if`-only chain, which
// `cfg_if!` accepts). Note that `cfg_if!` expands the selected branch's items
// directly into *this* module (it does not create a nested inline module), so
// `pub mod pc;` and `pub mod playstation2;` resolve relative to this file as
// `src/platform/pc.rs` / `src/platform/playstation2.rs`. That avoids the
// "one directory too deep" path that a plain nested `mod { ... }` would cause.

cfg_if::cfg_if! {
    if #[cfg(not(target_vendor = "sony"))] {
        //
        // PC...
        //

        pub mod pc;

        pub use pc::*;
        pub use pc::PcPlatformPlugin as PlatformPlugin;
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_vendor = "sony")] {
        //
        // PlayStation 2...
        //

        pub mod playstation2;

        pub use playstation2::*;
        pub use playstation2::Ps2PlatformPlugin as PlatformPlugin;
    }
}
