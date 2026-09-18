// src/main.rs
#[cfg_attr(target_arch = "mips", no_std)]
#[cfg_attr(target_arch = "mips", no_main)]

extern crate alloc;
use bevy::prelude::*;

mod platform;

#[cfg(target_arch = "mips")]
#[unsafe(no_mangle)]
pub extern "C" fn __start() -> ! {
    let mut app = App::new();
    app.add_plugins(platform::PlatformPlugin)
       .add_systems(Update, shared_game_logic);

    loop {
        app.update();
        // Add VSync lock here for PS2
    }
}

#[cfg(not(target_arch = "mips"))]
fn main() {
    App::new()
        .add_plugins(platform::PlatformPlugin)
        .add_systems(Update, shared_game_logic)
        .run();
}

fn shared_game_logic() {
    // This runs seamlessly on your PC and your PS2 ISO!
}
