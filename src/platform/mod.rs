#[cfg(target_arch = "mips")]
mod ps2;
#[cfg(not(target_arch = "mips"))]
mod pc;

#[cfg(target_arch = "mips")]
pub use ps2::Ps2PlatformPlugin as PlatformPlugin;

#[cfg(not(target_arch = "mips"))]
pub use pc::PcPlatformPlugin as PlatformPlugin;