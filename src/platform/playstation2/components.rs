
use crate::shared::{BackgroundColor, GameCamera, GameMesh};

#[derive(Component)]
pub struct Ps2Camera();

impl Default for Ps2Camera {
    fn default() -> Self {
        Self()
    }
}

#[derive(Component)]
pub struct Ps2Mesh();

#[derive(Component)]
pub struct Ps2Light();
