use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin};

pub struct PcPlatformPlugin;

impl Plugin for PcPlatformPlugin {
    fn build(&self, app: &mut App) {
        // Add standard Bevy window and input systems for desktop testing
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy PC Host Mode".into(),
                resolution: (640, 448).into(),
                ..default()
            }),
            ..default()
        }))
        // .add_systems(Update, handle_pc_input)
        ;
    }
}
