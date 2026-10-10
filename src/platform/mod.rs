
#[cfg(not(target_vendor = "sony"))]
mod platform_impl {
    pub mod pc;
    pub use pc::*;
    pub use pc::PcPlatformPlugin as PlatformPlugin;
}

#[cfg(target_vendor = "sony")]
mod platform_impl {
    pub mod playstation2;
    pub use playstation2::*;
    pub use playstation2::Ps2PlatformPlugin as PlatformPlugin;
}

// Export the target platform.
pub use platform_impl::*;