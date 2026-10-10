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

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
pub struct GameMesh;

// #[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GameMaterial {
    pub color: Color,
}

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

        // app.init_asset::<Mesh>();
        // app.init_asset::<GameMaterial>();
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

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GameMaterial>>,
) {
    // 2. Create your generic material
    let material_handle = materials.add(GameMaterial {
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