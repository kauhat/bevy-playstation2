#![feature(breakpoint)]
#![cfg_attr(target_vendor = "sony", no_std)]
#![cfg_attr(target_vendor = "sony", no_main)]

#[cfg(target_vendor = "sony")]
#[macro_use]
extern crate ps2sdk_sys;
extern crate alloc;

use alloc::boxed::Box;
use bevy::prelude::*;
use core::arch::breakpoint;

mod platform;

#[cfg(target_vendor = "sony")]
#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    platform::ps2::init();

    println!("Hello, I'm a Playstation 2 Rust program!");

    // simple_allocation();

    // try to allocate a large buffer to test the custom allocator
    // (|| allocate_too_much())().unwrap_or_else(|_err: String| {
    //     println!("Failed to allocate that much.");
    // });

    // let mut app = App::empty();

    println!("Setting up Bevy app...");

    // breakpoint();

    let mut app = App::new()
        .add_plugins(platform::PlatformPlugin)
        .add_systems(Update, shared_game_logic)
        .run();
        
    // app.run();

    // let mut world = bevy_ecs::world::World::new();
    // let mut schedule = bevy_ecs::schedule::Schedule::default();

    // schedule.add_systems(shared_game_logic);
    // schedule.add_systems(platform::cycle_background_color_system);

    println!("Bevy exited. Reason: {:?}", app);
    
    loop {}
    
    0
}

// #[unsafe(no_mangle)]
// pub extern "C" fn _exit(_status: core::ffi::c_int) -> ! {
//     loop {
//         core::hint::spin_loop();
//     }
// }

#[cfg(not(target_vendor = "sony"))]
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
    // commands.spawn((Name::new("LargeBufferEntity"), LargeDataBuffer::default()));
}

fn allocate_too_much() -> Result<()> {
    let buffers = vec![
        LargeDataBuffer::default(),
        LargeDataBuffer::default(),
        LargeDataBuffer::default(),
        LargeDataBuffer::default(),
    ];

    Ok(())
}

fn simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);

    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}
