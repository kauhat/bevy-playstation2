//! The Ratchet & Clank level as the game draws it, independent of the platform: the decoded tfrags and triangles, the
//! camera that views them, and the byte formats the host writes (`build/fixtures.rs`). The PS2 reads the bytes from
//! the host file system and submits the result to gsKit; the same parsers and camera serve any other target.
//!
//! Formats (little-endian):
//! * per-tfrag file: `f32` base distance `L`, `u32` tfrag count, then per tfrag a sphere (`4 × f32`, raw units),
//!   `u32` base-only flag, and three detail lists (LOD 0, 1, 2), each a `u32` count and 40-byte triangle records.
//! * flat file: `u32` count, then 40-byte triangle records.
//! * triangle record: three corners of `3 × f32` game-unit positions, three colour bytes, one pad byte.

extern crate alloc;

use crate::shared::tfrag_lod::{self, FAR, NEAR, TAN_X, TAN_Y, View};
use alloc::vec::Vec;
use bevy::prelude::Vec3;

/// The bytes of one triangle record.
pub const TRIANGLE_BYTES: usize = 40;

/// One corner: game-unit position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner(pub [f32; 3]);

/// One triangle of the level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub corners: [Corner; 3],
    pub rgb: [u8; 3],
}

/// One tfrag: its bounding sphere (raw units, 1024 per world unit), its base-only flag, and its three detail lists
/// (LOD 0, 1, 2), the same as the game's tfrag block.
#[derive(Clone, Debug, PartialEq)]
pub struct Tfrag {
    pub sphere: [f32; 4],
    pub base_only: bool,
    pub lods: [Vec<Triangle>; 3],
}

/// The level as the game draws it: tfrag by tfrag, so the LOD choice is per tfrag.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    /// The LOD base distance `L` (`TfragBlockHeader::unknown_8`); the switch distances are `6L, 4L, 2L`.
    pub base: f32,
    pub tfrags: Vec<Tfrag>,
    /// Total triangles over all lists, for reports.
    pub triangles: usize,
}

fn read_f32(blob: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([blob[at], blob[at + 1], blob[at + 2], blob[at + 3]])
}

fn read_u32(blob: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([blob[at], blob[at + 1], blob[at + 2], blob[at + 3]])
}

/// Decodes the triangle record at `at`.
fn triangle_at(blob: &[u8], at: usize) -> Triangle {
    let corner = |k: usize| Corner([read_f32(blob, at + k * 12), read_f32(blob, at + k * 12 + 4), read_f32(blob, at + k * 12 + 8)]);
    Triangle {
        corners: [corner(0), corner(1), corner(2)],
        rgb: [blob[at + 36], blob[at + 37], blob[at + 38]],
    }
}

impl Level {
    /// Parses the per-tfrag file. `None` if the sizes do not add up.
    pub fn parse_tfrags(blob: &[u8]) -> Option<Level> {
        if blob.len() < 8 {
            return None;
        }
        let base = read_f32(blob, 0);
        let count = read_u32(blob, 4) as usize;
        let mut at = 8usize;
        let mut tfrags = Vec::with_capacity(count);
        let mut triangles = 0usize;
        for _ in 0..count {
            if at + 20 > blob.len() {
                return None;
            }
            let sphere = [read_f32(blob, at), read_f32(blob, at + 4), read_f32(blob, at + 8), read_f32(blob, at + 12)];
            let base_only = read_u32(blob, at + 16) != 0;
            at += 20;
            let mut lods: [Vec<Triangle>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            for list in lods.iter_mut() {
                if at + 4 > blob.len() {
                    return None;
                }
                let n = read_u32(blob, at) as usize;
                at += 4;
                if at + n * TRIANGLE_BYTES > blob.len() {
                    return None;
                }
                for k in 0..n {
                    list.push(triangle_at(blob, at + k * TRIANGLE_BYTES));
                }
                at += n * TRIANGLE_BYTES;
                triangles += n;
            }
            tfrags.push(Tfrag { sphere, base_only, lods });
        }
        (at == blob.len()).then_some(Level { base, tfrags, triangles })
    }

    /// Parses the flat file as one tfrag at LOD 0 with an unbounded sphere, so it is always drawn. `None` if malformed.
    pub fn parse_flat(blob: &[u8]) -> Option<Level> {
        if blob.len() < 4 {
            return None;
        }
        let count = read_u32(blob, 0) as usize;
        if blob.len() != 4 + count * TRIANGLE_BYTES {
            return None;
        }
        let tris: Vec<Triangle> = (0..count).map(|k| triangle_at(blob, 4 + k * TRIANGLE_BYTES)).collect();
        let tfrag = Tfrag { sphere: [0.0, 0.0, 0.0, f32::MAX], base_only: false, lods: [tris, Vec::new(), Vec::new()] };
        Some(Level { base: 0.0, tfrags: alloc::vec![tfrag], triangles: count })
    }

    /// The LOD switch thresholds `TfragProc` compares against, in raw units.
    pub fn thresholds(&self) -> [i32; 3] {
        tfrag_lod::lod_thresholds(tfrag_lod::lod_distances(self.base))
    }
}

/// A world-to-screen camera in game coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    /// Forward, left, up (game axes), the rows `vu_tfrag_matrix` takes.
    pub rows: [Vec3; 3],
}

impl Camera {
    /// This camera turned about the vertical axis by `angle` radians (positive turns left), eye fixed.
    pub fn yawed(self, angle: f32) -> Camera {
        let (sin, cos) = (libm::sinf(angle), libm::cosf(angle));
        let turn = |v: Vec3| Vec3::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos, v.z);
        Camera { eye: self.eye, rows: [turn(self.rows[0]), turn(self.rows[1]), self.rows[2]] }
    }

    /// The view the game's tfrag LOD path tests against: the eye and the sphere are in raw units (1024 per game unit),
    /// the axes unchanged.
    pub fn view(&self) -> View {
        let [fwd, left, up] = self.rows;
        let raw = |v: Vec3| (v * 1024.0).to_array();
        View { eye: raw(self.eye), fwd: fwd.to_array(), left: left.to_array(), up: up.to_array() }
    }

    /// Projects a game-unit point to screen pixels of a `width` x `height` buffer. `None` behind the near plane.
    /// The depth is linear over the near..far range, scaled to the 24-bit Z buffer.
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
        let depth = ((cz - NEAR) / (FAR - NEAR)).clamp(0.0, 1.0);
        Some([x, y, depth * 8_388_607.0])
    }
}

/// The start camera: inside the level's footprint, looking along +Y, so the tfrags split across the three detail lists
/// and the culling has something to remove. Up is along +Z, the game's own up.
pub const START: Camera = Camera {
    eye: Vec3::new(160.0, 150.0, 30.0),
    rows: [Vec3::new(0.0, 1.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)],
};

#[cfg(test)]
mod tests {
    use super::*;

    fn tri(x: f32, rgb: [u8; 3]) -> [u8; TRIANGLE_BYTES] {
        let mut b = [0u8; TRIANGLE_BYTES];
        for (k, v) in [x, 0.0, 0.0, x + 1.0, 0.0, 0.0, x, 1.0, 0.0].iter().enumerate() {
            b[k * 4..k * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        b[36..39].copy_from_slice(&rgb);
        b
    }

    #[test]
    fn flat_file_round_trips_one_triangle() {
        let mut blob = 1u32.to_le_bytes().to_vec();
        blob.extend_from_slice(&tri(5.0, [1, 2, 3]));
        let level = Level::parse_flat(&blob).expect("well formed");
        assert_eq!(level.triangles, 1);
        let t = level.tfrags[0].lods[0][0];
        assert_eq!(t.corners[1], Corner([6.0, 0.0, 0.0]));
        assert_eq!(t.rgb, [1, 2, 3]);
    }

    #[test]
    fn flat_file_rejects_a_bad_size() {
        let mut blob = 2u32.to_le_bytes().to_vec();
        blob.extend_from_slice(&tri(0.0, [0; 3]));
        assert!(Level::parse_flat(&blob).is_none());
    }

    #[test]
    fn tfrag_file_round_trips_sphere_flag_and_lists() {
        let mut blob = 12.0f32.to_le_bytes().to_vec();
        blob.extend_from_slice(&1u32.to_le_bytes());
        for v in [1.0f32, 2.0, 3.0, 4.0] {
            blob.extend_from_slice(&v.to_le_bytes());
        }
        blob.extend_from_slice(&1u32.to_le_bytes());
        blob.extend_from_slice(&1u32.to_le_bytes());
        blob.extend_from_slice(&tri(0.0, [9, 9, 9]));
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        let level = Level::parse_tfrags(&blob).expect("well formed");
        assert_eq!(level.base, 12.0);
        assert_eq!(level.triangles, 1);
        assert_eq!(level.tfrags[0].sphere, [1.0, 2.0, 3.0, 4.0]);
        assert!(level.tfrags[0].base_only);
        assert_eq!(level.tfrags[0].lods[0].len(), 1);
        assert!(level.tfrags[0].lods[1].is_empty());
    }

    #[test]
    fn tfrag_file_rejects_truncation() {
        let mut blob = 12.0f32.to_le_bytes().to_vec();
        blob.extend_from_slice(&1u32.to_le_bytes());
        assert!(Level::parse_tfrags(&blob).is_none());
    }

    #[test]
    fn thresholds_follow_the_base_distance() {
        let level = Level { base: 12.0, tfrags: Vec::new(), triangles: 0 };
        assert_eq!(level.thresholds(), [73_728, 49_152, 24_576]);
    }

    #[test]
    fn projection_keeps_the_view_axis_on_screen_centre() {
        let p = START.project([160.0, 200.0, 30.0], 640.0, 448.0).expect("in front");
        assert!((p[0] - 320.0).abs() < 1e-3);
        assert!((p[1] - 224.0).abs() < 1e-3);
    }

    #[test]
    fn projection_rejects_points_behind_the_camera() {
        assert!(START.project([160.0, 100.0, 30.0], 640.0, 448.0).is_none());
    }

    #[test]
    fn yawing_moves_forward_and_keeps_the_eye() {
        let turned = START.yawed(core::f32::consts::FRAC_PI_2);
        assert_eq!(turned.eye, START.eye);
        assert!((turned.rows[0].x + 1.0).abs() < 1e-5, "forward turned to -X: {:?}", turned.rows[0]);
    }
}
