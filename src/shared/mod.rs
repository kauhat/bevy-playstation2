use bevy::prelude::*;
use bevy::time::TimePlugin;
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Local, Query, Res, ResMut},
    },
};

#[derive(Component)]
pub struct GameCamera();

#[derive(Component)]
pub struct GameMesh();

#[derive(Component)]
pub struct GameLight();

// #[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GameMaterial {
    pub color: Color,
}

#[derive(Component)]
pub struct EntityData {
    pub id: u32,
}

#[derive(Component)]
struct RotatingEntity; // Component to indicate entity should be rotated

#[derive(Resource)]
pub struct BackgroundColor(pub Color);

impl Default for BackgroundColor {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

#[derive(Resource)]
pub struct DebugTimers {
    fps_timer: Timer,
    message_timer: Timer,
}

impl Default for DebugTimers {
    fn default() -> Self {
        Self {
            fps_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            message_timer: Timer::from_seconds(5.0, TimerMode::Repeating),
        }
    }
}

/// Tracker resource to throttle spawns and cap the total rate
#[derive(Resource)]
pub struct SpawnTracker {
    pub timer: Timer,
    pub next_id: u32,
}

impl Default for SpawnTracker {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(10.0, TimerMode::Repeating),
            next_id: 0,
        }
    }
}

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TimePlugin>() {
            app.add_plugins(TimePlugin);
        }

        app.add_plugins(FrameTimeDiagnosticsPlugin::default());

        if !app.is_plugin_added::<TransformPlugin>() {
            app.add_plugins(TransformPlugin);
        }

        // if !app.is_plugin_added::<AnimationPlugin>() {
        //     app.add_plugins(AnimationPlugin);
        // }
        app.init_resource::<SpawnTracker>();
        app.init_resource::<DebugTimers>();

        app.insert_resource(BackgroundColor::default());

        app.add_systems(Startup, setup_scene);
        app.add_systems(
            Update,
            (
                rotate_entities,
                count_entities,
                cycle_background_color_system,
                entity_spawner_system,
            ),
        );

        app.add_systems(Update, print_fps);
    }
}

pub fn count_entities(
    entities: Query<Entity>,
    mut last_count: Local<Option<usize>>,
    time: Res<Time>,
    mut timers: ResMut<DebugTimers>
) {
    timers.message_timer.tick(time.delta());

    if timers.message_timer.just_finished() {
        let current_count = entities.iter().len();

        if last_count.is_none_or(|prev| prev != current_count) {
            *last_count = Some(current_count);

            let elapsed = time.elapsed_secs();
            println!("[{elapsed:.2}s] Total entities: {current_count}");
        }
    }
}

fn print_fps(diagnostics: Res<DiagnosticsStore>, time: Res<Time>, mut timers: ResMut<DebugTimers>) {
    timers.fps_timer.tick(time.delta());

    if timers.fps_timer.just_finished() {
        if let Some(fps) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
        {
            println!("FPS: {}", fps);
        }
    }
}

fn setup_scene(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<GameMaterial>>,
) {
    let cube_locations = [
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(-4.0, 0.0, -9.0),
        Vec3::new(3.0, 0.0, -13.0),
        Vec3::new(-1.0, 0.0, -17.0),
    ];

    for location in cube_locations.iter() {
        commands
            .spawn((GameMesh(), Transform::from_translation(*location)))
            .insert(RotatingEntity);
    }

    // Mock Light
    commands.spawn((GameLight(), Transform::from_xyz(2.0, 5.0, 2.0)));

    // Mock Camera
    commands.spawn((GameCamera(), Transform::from_xyz(0.0, 1.0, 0.0)));
}

pub fn entity_spawner_system(
    mut commands: Commands,
    time: Res<Time>,
    mut tracker: ResMut<SpawnTracker>,
) {
    tracker.timer.tick(time.delta());

    if tracker.timer.just_finished() {
        const BATCH_SIZE: usize = 32;

        for i in 0..BATCH_SIZE {
            let id = tracker.next_id;
            tracker.next_id = tracker.next_id.wrapping_add(1);

            // Spread out position based on ID
            let x = ((id % 20) as f32) - 10.0;
            let y = (((id / 20) % 20) as f32) - 10.0;
            let z = -((id / 400) as f32);

            commands.spawn((
                GameMesh(),
                EntityData { id },
                Transform::from_xyz(x, y, z),
                RotatingEntity,
            ));
        }
    }
}

fn rotate_entities(time: Res<Time>, mut query: Query<&mut Transform, With<RotatingEntity>>) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() as f32 / 2.0);
    }
}

fn cycle_background_color_system(mut hue: Local<f32>, mut bg: ResMut<BackgroundColor>) {
    // Advance hue (0.0 to 360.0 degrees for Bevy's Hsva)
    *hue += 3.6;
    if *hue >= 360.0 {
        *hue -= 360.0;
    }

    // Let Bevy handle the HSV to RGB conversion internally
    bg.0 = Color::from(Hsva::new(*hue, 1.0, 0.2, 1.0));
}
