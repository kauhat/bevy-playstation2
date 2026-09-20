#[cfg(not(target_arch = "mips64"))]
mod pc;

#[cfg(target_arch = "mips64")]
pub mod ps2;

#[cfg(target_arch = "mips64")]
pub use ps2::Ps2PlatformPlugin as PlatformPlugin;

#[cfg(not(target_arch = "mips64"))]
pub use pc::PcPlatformPlugin as PlatformPlugin;
