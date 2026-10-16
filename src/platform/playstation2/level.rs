//! The Ratchet & Clank level slice drawn on the EE.
//!
//! `build/fixtures.rs` stages a decimated level-0 mesh into the ELF: a `u32` triangle count, then per triangle three
//! corners of `f32` game-unit positions (Z up, the game's own axes) and one RGB colour. The renderer is the classic
//! PS2 path: transform each corner through the camera with plain float maths on the EE FPU, perspective-divide, and
//! hand the surviving triangles to gsKit as flat-shaded 3D triangles.
//!
//! The projection is the game's own (`rc-engine`'s `vu_tfrag_matrix`, `rc-game`'s `GameProjection`): half-angle
//! tangents `0.63` and `0.63 * 0.775`, near `1`, far `745472`, on a 512x448 draw buffer.

extern crate alloc;

use bevy::prelude::Vec3;

/// The embedded slice (`build/fixtures.rs`), used when the host file is not there.
mod data {
    include!(concat!(env!("OUT_DIR"), "/rc_formats_fixture.rs"));
}

/// The whole lit level, decoded on the host from the extracted tree and read by the EE at start-up
/// (`build/fixtures.rs` `write_full_level`). `host:` reaches it through the `levels` link the build puts beside the ELF.
const HOST_LEVEL: &str = "host:levels/00/level0_tris.bin";

/// One corner: game-unit position.
#[derive(Clone, Copy)]
pub struct Corner(pub [f32; 3]);

/// One triangle of the level.
#[derive(Clone, Copy)]
pub struct Triangle {
    pub corners: [Corner; 3],
    pub rgb: [u8; 3],
}

impl Triangle {
    /// The box around the three corners, in game units.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for corner in &self.corners {
            let p = Vec3::from(corner.0);
            min = min.min(p);
            max = max.max(p);
        }
        (min, max)
    }
}

/// The triangle records, in the format both sources share: a `u32` count, then 40 bytes per triangle (nine `f32`
/// positions, three colour bytes, one pad byte).
pub struct Level {
    bytes: alloc::vec::Vec<u8>,
    count: usize,
}

impl Level {
    /// Loads the host file if it opens and is well formed; otherwise the embedded slice. Returns the reason it fell back,
    /// for the caller to report.
    pub fn load() -> (Level, Option<&'static str>) {
        let fallback = |why| (Level { bytes: data::LEVEL0_MESH.to_vec(), count: data::LEVEL0_MESH_TRIANGLES }, Some(why));
        let Some(bytes) = read_host(HOST_LEVEL) else {
            return fallback("host file did not open");
        };
        if bytes.len() < 4 {
            return fallback("host file is too short");
        }
        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if bytes.len() != 4 + count * 40 {
            return fallback("host file size does not match its triangle count");
        }
        (Level { bytes, count }, None)
    }

    pub fn triangle_count(&self) -> usize {
        self.count
    }

    /// Decodes triangle `index`, or `None` past the end.
    pub fn triangle(&self, index: usize) -> Option<Triangle> {
        if index >= self.count {
            return None;
        }
        let blob = &self.bytes;
        let base = 4 + index * 40;
        let f = |at: usize| f32::from_le_bytes([blob[at], blob[at + 1], blob[at + 2], blob[at + 3]]);
        let corner = |k: usize| Corner([f(base + k * 12), f(base + k * 12 + 4), f(base + k * 12 + 8)]);
        Some(Triangle {
            corners: [corner(0), corner(1), corner(2)],
            rgb: [blob[base + 36], blob[base + 37], blob[base + 38]],
        })
    }
}

/// Reads a whole host file through `fioRead` in 64 KB pieces. `None` if it does not open.
fn read_host(path: &str) -> Option<alloc::vec::Vec<u8>> {
    use ps2sdk_sys::fileio::{fioClose, fioInit, fioLseek, fioOpen, fioRead, FIO_O_RDONLY, FIO_SEEK_END, FIO_SEEK_SET};

    const CHUNK: usize = 64 * 1024;
    let mut name = alloc::vec::Vec::with_capacity(path.len() + 1);
    name.extend_from_slice(path.as_bytes());
    name.push(0);

    unsafe {
        fioInit();
        let fd = fioOpen(name.as_ptr().cast(), FIO_O_RDONLY as i32);
        if fd < 0 {
            return None;
        }
        let size = fioLseek(fd, 0, FIO_SEEK_END as i32);
        if size <= 0 || fioLseek(fd, 0, FIO_SEEK_SET as i32) != 0 {
            fioClose(fd);
            return None;
        }
        let mut bytes = alloc::vec![0u8; size as usize];
        let mut done = 0usize;
        while done < bytes.len() {
            let want = (bytes.len() - done).min(CHUNK);
            let got = fioRead(fd, bytes[done..].as_mut_ptr().cast(), want as i32);
            if got <= 0 {
                fioClose(fd);
                return None;
            }
            done += got as usize;
        }
        fioClose(fd);
        Some(bytes)
    }
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

/// One of the six view planes, as `n·p + d >= 0` for points inside (the form Bevy's `Frustum` uses).
#[derive(Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

/// The view volume as six planes, built from the camera the same way `Camera::project` maps points, so a point is inside
/// exactly when it would be drawn. Bevy's `Frustum::intersects_obb` tests boxes against planes of this kind; it lives in
/// `bevy_camera`, which the EE build does not link, so the test is written out here.
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Box-versus-frustum: the box is outside if, for some plane, its farthest corner lies behind it.
    pub fn intersects_box(&self, min: Vec3, max: Vec3) -> bool {
        let centre = (min + max) * 0.5;
        let half = (max - min) * 0.5;
        self.planes.iter().all(|plane| {
            let radius = half.x * plane.normal.x.abs() + half.y * plane.normal.y.abs() + half.z * plane.normal.z.abs();
            plane.normal.dot(centre) + plane.d + radius > 0.0
        })
    }
}

impl Camera {
    /// The six planes of this camera's view volume: near, far, the two x sides and the two y sides.
    pub fn frustum(&self) -> Frustum {
        let [fwd, left, up] = self.rows;
        // A point's camera coordinates are (cx, cy, cz) = (-left·d, -up·d, fwd·d) with d = p - eye.
        let plane_x = |sign: f32| {
            // Bounds |cx| <= TAN_X * cz * sign-side: cz * TAN_X + sign * cx >= 0.
            Plane { normal: fwd * TAN_X - left * sign, d: -(self.eye.dot(fwd * TAN_X - left * sign)) }
        };
        let plane_y = |sign: f32| Plane { normal: fwd * TAN_Y - up * sign, d: -(self.eye.dot(fwd * TAN_Y - up * sign)) };
        Frustum {
            planes: [
                Plane { normal: fwd, d: -(self.eye.dot(fwd)) - NEAR },
                Plane { normal: -fwd, d: self.eye.dot(fwd) + FAR },
                plane_x(1.0),
                plane_x(-1.0),
                plane_y(1.0),
                plane_y(-1.0),
            ],
        }
    }
}

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
