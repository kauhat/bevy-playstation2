//! The Ratchet & Clank level slice drawn on the EE.
//!
//! `build/fixtures.rs` stages a decimated level-0 mesh into the ELF: a `u32` triangle count, then per triangle three
//! corners of `f32` game-unit positions (Z up, the game's own axes) and one RGB colour. The renderer is the classic
//! PS2 path: transform each corner through the camera with plain float maths on the EE FPU, perspective-divide, and
//! hand the surviving triangles to gsKit as flat-shaded 3D triangles.
//!
//! The projection is the game's own (`rc-engine`'s `vu_tfrag_matrix`, `rc-game`'s `GameProjection`): half-angle
//! tangents `0.63` and `0.63 * 0.775`, near `1`, far `745472`, on a 512x448 draw buffer.

use bevy::prelude::Vec3;

/// The embedded slice (`build/fixtures.rs`).
mod data {
    include!(concat!(env!("OUT_DIR"), "/rc_formats_fixture.rs"));
}

/// One corner: game-unit position.
#[derive(Clone, Copy)]
pub struct Corner(pub [f32; 3]);

/// One triangle of the slice.
#[derive(Clone, Copy)]
pub struct Triangle {
    pub corners: [Corner; 3],
    pub rgb: [u8; 3],
}

/// The number of triangles in the embedded slice.
pub fn triangle_count() -> usize {
    data::LEVEL0_MESH_TRIANGLES
}

/// Decodes triangle `index` from the embedded blob, or `None` past the end.
pub fn triangle(index: usize) -> Option<Triangle> {
    if index >= triangle_count() {
        return None;
    }
    let blob = data::LEVEL0_MESH;
    // Header is the `u32` count; each triangle is 36 bytes of positions then 4 bytes of colour.
    let base = 4 + index * 40;
    let f = |at: usize| f32::from_le_bytes([blob[at], blob[at + 1], blob[at + 2], blob[at + 3]]);
    let corner = |k: usize| Corner([f(base + k * 12), f(base + k * 12 + 4), f(base + k * 12 + 8)]);
    Some(Triangle {
        corners: [corner(0), corner(1), corner(2)],
        rgb: [blob[base + 36], blob[base + 37], blob[base + 38]],
    })
}

/// A world-to-screen camera in game coordinates.
#[derive(Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    /// Forward, left, up (game axes), the rows `vu_tfrag_matrix` takes.
    pub rows: [Vec3; 3],
}

/// Half-angle tangents and clip planes of the game's projection.
pub const TAN_X: f32 = 0.63;
pub const TAN_Y: f32 = 0.63 * 0.775;
pub const NEAR: f32 = 1.0;
pub const FAR: f32 = 745_472.0;

impl Camera {
    /// Projects a game-unit point to screen pixels of a `width` x `height` buffer. `None` behind the near plane.
    pub fn project(&self, p: [f32; 3], width: f32, height: f32) -> Option<[f32; 3]> {
        let d = Vec3::from(p) - self.eye;
        let [fwd, left, up] = self.rows;
        // Camera space: x = -left, y = -up, z = forward (as `vu_tfrag_matrix`).
        let cx = -left.dot(d);
        let cy = -up.dot(d);
        let cz = fwd.dot(d);
        if cz < NEAR {
            return None;
        }
        let sx = width * 0.5 / (TAN_X * cz);
        let sy = height * 0.5 / (TAN_Y * cz);
        let x = width * 0.5 + cx * sx;
        let y = height * 0.5 + cy * sy;
        // Linear depth across the near..far range, scaled to the 24-bit Z buffer gsKit tests against.
        let depth = ((cz - NEAR) / (FAR - NEAR)).clamp(0.0, 1.0);
        Some([x, y, depth * 8_388_607.0])
    }
}

/// The start camera: framed on the slice's centre from the south-west (eye) so the whole block shows. Rows are
/// forward, left, up, unit length, with up along +Z (the game's own up).
pub const START: Camera = Camera {
    eye: Vec3::new(168.0, -40.0, 90.0),
    rows: [
        Vec3::new(0.0, 0.967_372, -0.253_359),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.253_359, 0.967_372),
    ],
};
