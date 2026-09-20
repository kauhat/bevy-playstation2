#![cfg_attr(target_arch = "mips64", no_std)]
#![cfg_attr(target_arch = "mips64", no_main)]

#[cfg(target_arch = "mips64")]
extern crate alloc;

use bevy::prelude::*;

mod platform;

#[cfg(target_arch = "mips64")]
#[unsafe(no_mangle)]
pub extern "C" fn __start() -> ! {
    // let test = "SHITS".to_string();

    platform::ps2::init();
    panic!("FUCK");

    let mut app = App::new();

    app.add_plugins(platform::PlatformPlugin)
        .add_systems(Update, shared_game_logic);

    // let mut world = bevy_ecs::world::World::new();
    // let mut schedule = bevy_ecs::schedule::Schedule::default();

    // schedule.add_systems(shared_game_logic);
    // schedule.add_systems(platform::cycle_background_color_system);

    loop {
        let test = "POOPS".to_string();

        // platform::ps2::wait_vsync();

        app.update();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _exit(_status: core::ffi::c_int) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_arch = "mips64"))]
fn main() {
    App::new()
        .add_plugins(platform::PlatformPlugin)
        .add_systems(Update, shared_game_logic)
        .run();
}

fn shared_game_logic() {
    // This runs seamlessly on your PC and your PS2 ISO!
}
