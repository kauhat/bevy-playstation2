#![cfg_attr(target_arch = "mips64", no_std)]
#![cfg_attr(target_arch = "mips64", no_main)]

#[cfg(target_arch = "mips64")]
extern crate alloc;

use bevy::prelude::*;

mod platform;

#[cfg(target_arch = "mips64")]
#[unsafe(no_mangle)]
pub extern "C" fn __start() -> ! {
    platform::ps2::init();

    platform::ps2::ps2_print("Hello I'm a Playstation 2 Rust app".to_string());

    // panic!("starts");
    // let mut app = App::new();
    platform::ps2::ps2_print("hmm".to_string());

    // app.add_plugins(platform::PlatformPlugin)
    //     .add_systems(Update, shared_game_logic);

    let mut world = bevy_ecs::world::World::new();
    let mut schedule = bevy_ecs::schedule::Schedule::default();

    // schedule.add_systems(shared_game_logic);
    // schedule.add_systems(platform::cycle_background_color_system);

    loop {
        // platform::ps2::wait_vsync();

        // app.update();

        panic!("did a loop");
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


const FOUR_MB: usize = 4 * 1024 * 1024;

#[derive(Component)]
pub struct LargeDataBuffer {
    // Heap-allocated raw byte buffer to avoid overflowing the EE stack
    pub data: Box<[u8; FOUR_MB]>,
}

impl Default for LargeDataBuffer {
    fn default() -> Self {
        Self {
            // Allocates directly on the heap using a zeroed box
            data: vec![0u8; FOUR_MB]
                .into_boxed_slice()
                .try_into()
                .expect("Failed to allocate 4MB buffer"),
        }
    }
}

fn shared_game_logic(mut commands: Commands) {
    commands.spawn((
        Name::new("LargeBufferEntity"),
        LargeDataBuffer::default(),
    ));
}
