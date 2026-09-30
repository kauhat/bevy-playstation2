#[cfg(not(target_vendor = "sony"))]
mod pc;

#[cfg(target_vendor = "sony")]
pub mod ps2;

#[cfg(target_vendor = "sony")]
pub use ps2::Ps2PlatformPlugin as PlatformPlugin;

#[cfg(not(target_vendor = "sony"))]
pub use pc::PcPlatformPlugin as PlatformPlugin;
