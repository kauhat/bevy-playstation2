#![feature(breakpoint)]
#![cfg_attr(target_vendor = "sony", no_std)]
#![cfg_attr(target_vendor = "sony", no_main)]

#[cfg(target_vendor = "sony")]
#[macro_use]
extern crate ps2sdk_sys;

extern crate alloc;

// use core::arch::breakpoint;

use alloc::boxed::Box;
use bevy::prelude::*;
use bevy::time::TimePlugin;
use bevy::platform::time::Instant;
use core::time::Duration;
use bevy::core_pipeline::core_3d::Camera3dBundle;
use bevy::render::mesh::Mesh;
// MaterialMeshBundle is used for any custom 3D material
use bevy::pbr::MaterialMeshBundle;

mod platform;

#[cfg(not(target_vendor = "sony"))]
fn main() {
    App::new()
        .add_plugins(platform::PlatformPlugin)
        .add_plugins(SharedPlugin)
        .run();
}

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

    println!("Setting up Bevy app...");

    // breakpoint();

    let mut app = App::new()
        .add_plugins(platform::PlatformPlugin)
        .add_plugins(SharedPlugin)
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

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TimePlugin>() {
            app.add_plugins(TimePlugin);
        }

        if !app.is_plugin_added::<TransformPlugin>() {
            app.add_plugins(TransformPlugin);
        }

        // if !app.is_plugin_added::<AnimationPlugin>() {
        //     app.add_plugins(AnimationPlugin);
        // }

        app.add_systems(Startup, setup_scene);
        app.add_systems(Update, rotate_entities);
        app.add_systems(Update, count_entities);

        app.init_asset::<Mesh>();
        app.init_asset::<Ps2BasicMaterial>();
    }
}

#[derive(Component)]
struct RotatingEntity; // Component to indicate entity should be rotated

pub fn count_entities(
    entities: Query<Entity>,
    mut last_count: Local<Option<usize>>,
    time: Res<Time>,
) {
    let current_count = entities.iter().len();

    if last_count.is_none_or(|prev| prev != current_count) {
        *last_count = Some(current_count);

        let elapsed = time.elapsed_secs();
        println!("[{elapsed:.2}s] Total entities: {current_count}");
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Ps2BasicMaterial {
    #[uniform(0)]
    pub color: Color,
}

impl Material for Ps2BasicMaterial {}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<Ps2BasicMaterial>>,
) {
    // 2. Create your generic material
    let material_handle = materials.add(Ps2BasicMaterial {
        color: Color::srgb(0.7, 0.6, 0.7),
    });

    let cube_locations = [
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(-4.0, 0.0, -9.0),
        Vec3::new(3.0, 0.0, -13.0),
        Vec3::new(-1.0, 0.0, -17.0),
    ];

    for location in cube_locations.iter() {
        // 3. Use MaterialMeshBundle instead of PbrBundle
        commands
            .spawn(MaterialMeshBundle {
                mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)), // or shape::Cube::new(1.0) depending on Bevy version
                material: material_handle.clone(),
                transform: Transform::from_translation(*location),
                ..Default::default()
            })
            .insert(RotatingEntity);
    }

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.0, 0.0),
        ..Default::default()
    });
    
    // Note: If your custom material doesn't use lighting, 
    // you don't need to spawn a PointLightBundle at all.
}


fn rotate_entities(time: Res<Time>, mut query: Query<&mut Transform, With<RotatingEntity>>) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() as f32 / 2.0);
    }
}

//
//
//

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
