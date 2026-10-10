#![cfg_attr(target_vendor = "sony", no_std)]
#![cfg_attr(target_vendor = "sony", no_main)]
#![feature(breakpoint)]
#![allow(unused)]

#[cfg(target_vendor = "sony")]
#[macro_use]
extern crate ps2sdk_sys;

#[cfg(target_vendor = "sony")]
extern crate alloc;

// use core::arch::breakpoint;

// use alloc::boxed::Box;

use bevy::ecs::{
    component::Component,
    entity::Entity,
    query::With,
    schedule::SystemSet,
    system::{Commands, Local, Query, Res, ResMut},
    world::World,
};
use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy::time::TimePlugin;
use core::time::Duration;
// use bevy::core_pipeline::core_3d::Camera3dBundle;
// use bevy::render::mesh::Mesh;
// MaterialMeshBundle is used for any custom 3D material
// use bevy::pbr::MaterialMeshBundle;

mod shared;
mod platform;

//
// PC...
//

#[cfg(not(target_vendor = "sony"))]
fn main() {
    App::new()
        .add_plugins(platform::PlatformPlugin)
        .add_plugins(shared::SharedPlugin)
        .run();
}

//
// PlayStation 2...
//

#[cfg(target_vendor = "sony")]
#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    platform::init();

    println!("Setting up Bevy app...");

    // breakpoint();

    let mut app = App::new()
        .add_plugins(platform::PlatformPlugin)
        .add_plugins(shared::SharedPlugin)
        .run();

    println!("Bevy exited. Reason: {:?}", app);

    loop {}

    0
}


//
//
//

// TODO: Test
// try to allocate a large buffer to test the custom allocator
// (|| allocate_too_much())().unwrap_or_else(|_err: String| {
//     println!("Failed to allocate that much.");
// });

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

fn allocate_too_much() -> Result<()> {
    let _buffers = [
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
