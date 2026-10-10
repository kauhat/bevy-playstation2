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
struct RotatingEntity; // Component to indicate entity should be rotated

#[derive(Resource)]
pub struct BackgroundColor(pub Color);

impl Default for BackgroundColor {
    fn default() -> Self {
        Self(Color::BLACK)
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

        app.insert_resource(BackgroundColor::default());
        app.add_systems(Startup, setup_scene);
        app.add_systems(
            Update,
            (
                rotate_entities,
                count_entities,
                cycle_background_color_system,
            ),
        );

        app.add_systems(Update, print_fps);

        // app.init_asset::<Mesh>();
        // app.init_asset::<GameMaterial>();
    }
}

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

// TODO: not displaying
fn print_fps(diagnostics: Res<DiagnosticsStore>) {
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    {
        println!("FPS: {}", fps);
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

    // Mock Light (acting purely as a transform point in space)
    commands.spawn((GameLight(), Transform::from_xyz(2.0, 5.0, 2.0)));

    // Mock Camera
    commands.spawn((GameCamera(), Transform::from_xyz(0.0, 1.0, 0.0)));
    // Note: If your custom material doesn't use lighting,
    // you don't need to spawn a PointLightBundle at all.
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
