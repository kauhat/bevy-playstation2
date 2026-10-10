// Target-specific platform backends.
//
// NOTE: `pc` and `playstation2` must be declared directly in this file (not
// inside an inline module) so that the compiler resolves them to
// `src/platform/pc.rs` / `src/platform/playstation2.rs`. External module
// declarations nested in an inline module get looked up one directory deeper
// (e.g. `src/platform/platform_impl/pc.rs`), which does not exist here.

//
// PC...
//

#[cfg(not(target_vendor = "sony"))]
pub mod pc;

#[cfg(not(target_vendor = "sony"))]
pub use pc::*;

#[cfg(not(target_vendor = "sony"))]
pub use pc::PcPlatformPlugin as PlatformPlugin;

//
// PlayStation 2...
//

#[cfg(target_vendor = "sony")]
pub mod playstation2;

#[cfg(target_vendor = "sony")]
pub use playstation2::*;

#[cfg(target_vendor = "sony")]
pub use playstation2::Ps2PlatformPlugin as PlatformPlugin;
