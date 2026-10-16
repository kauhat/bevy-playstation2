//! Tfrag culling and level-of-detail as the game does it: `TfragProc` (boot 0x233fb0) decides, per tfrag and per
//! frame, whether to draw it and which of its three detail lists to draw. Ported from `rc-engine`'s `tfrag_lod.rs`
//! and `rc-formats`' `Tfrag::draw_mode`, with the frame-independent rules here so every target can use them.
//!
//! Units are the game's raw units (1024 = 1 world unit) for the sphere and the thresholds, matching the reference.

/// Near plane of the game's view, in raw units (`tfrag_lod::NEAR`).
pub const NEAR: f32 = 32.0;
/// Far plane of the game's view, in raw units (`tfrag_lod::FAR`).
pub const FAR: f32 = 745_472.0;
/// The horizontal half-field-of-view tangent of the game's projection.
pub const TAN_X: f32 = 0.63;
/// The vertical tangent: horizontal × 0.775 on NTSC.
pub const TAN_Y: f32 = 0.63 * 0.775;

/// What `TfragProc` decided for one tfrag this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawMode {
    /// Not drawn: off screen, beyond the far plane, or behind the near plane.
    Culled,
    /// Partly outside the guard band: drawn on the clipping path, LOD 0 with no morph.
    Clip,
    /// Drawn with one of the detail lists.
    Lod(Lod),
}

/// The detail list a drawn tfrag uses. `Lod0` is the finest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lod {
    Lod0,
    Lod1,
    Lod2,
}

/// The game's LOD distances from a tfrag block header's `unknown_8` (the base distance `L`): `6L`, `4L`, `2L`.
pub fn lod_distances(base: f32) -> [f32; 3] {
    [base * 6.0, base * 4.0, base + base]
}

/// `trunc(D * 1024)` for each distance, the integer thresholds `TfragProc` compares against.
pub fn lod_thresholds(distances: [f32; 3]) -> [i32; 3] {
    distances.map(|d| (d * 1024.0) as i32)
}

/// Picks the detail list from the tfrag's own sphere, as `Tfrag::draw_mode` does. `depth` is the sphere centre's
/// distance along the view, `radius` its radius, both in raw units. `base_only` marks LOD-2-only tfrags.
///
/// The reference has six modes: the morph and collapse modes draw the list of the tier they blend from, so the EE
/// picks the list here and leaves the vertex morph to the vertex shader's job (not modelled on the EE yet).
pub fn draw_lod(depth: f32, radius: f32, base_only: bool, thresholds: [i32; 3]) -> Lod {
    let [d0, d1, _] = thresholds;
    let far = (depth + radius) as i32;
    let near = (depth - radius) as i32;
    if base_only || near >= d0 {
        Lod::Lod2
    } else if far >= d0 || near >= d1 {
        // Lod1Collapse (straddles D0) or Lod1Morph (inside [D1, D0)).
        Lod::Lod1
    } else {
        // Lod0Collapse, Lod0Morph and Lod0 all draw the LOD 0 list.
        Lod::Lod0
    }
}

/// A tfrag's culling sphere (raw units, centre and radius).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub centre: [f32; 3],
    pub radius: f32,
}

/// The six planes of the view volume, as `n·p + d >= 0` for inside points. Built from the eye and the three camera
/// axes, in the same convention as `TfragProc` (forward `fwd`, left `left`, up `up`).
pub struct View {
    pub eye: [f32; 3],
    pub fwd: [f32; 3],
    pub left: [f32; 3],
    pub up: [f32; 3],
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

impl View {
    /// `TfragProc`'s view-space depth and offsets of `p`: `(x, y, z)` with `z` forward.
    fn camera_space(&self, p: [f32; 3]) -> [f32; 3] {
        let d = sub(p, self.eye);
        [-dot(self.left, d), -dot(self.up, d), dot(self.fwd, d)]
    }

    /// Culls and picks the mode for one tfrag: the sphere tests of `TfragProc` (near/far, then each side of the
    /// frustum), then the LOD rule. The guard-band clip test and the occlusion test are not modelled yet.
    pub fn tfrag_mode(&self, sphere: Sphere, base_only: bool, thresholds: [i32; 3]) -> DrawMode {
        let p = self.camera_space(sphere.centre);
        let r = sphere.radius;
        let (far, near) = (r + p[2], -r + p[2]);
        if 512_000.0 - near < 0.0 || NEAR - far >= 0.0 {
            return DrawMode::Culled;
        }
        let (ax, ay) = (p[0].abs(), p[1].abs());
        let (kx, ky) = (libm::sqrtf(1.0 + TAN_X * TAN_X), libm::sqrtf(1.0 + TAN_Y * TAN_Y));
        if TAN_X * p[2] - (ax - r * kx) < 0.0 || TAN_Y * p[2] - (ay - r * ky) < 0.0 {
            return DrawMode::Culled;
        }
        DrawMode::Lod(draw_lod(p[2], r, base_only, thresholds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> View {
        View {
            eye: [168.0 * 1024.0, -40.0 * 1024.0, 90.0 * 1024.0],
            fwd: [0.0, 0.967_372, -0.253_359],
            left: [-1.0, 0.0, 0.0],
            up: [0.0, 0.253_359, 0.967_372],
        }
    }

    #[test]
    fn distances_follow_the_base_distance() {
        assert_eq!(lod_distances(12.0), [72.0, 48.0, 24.0]);
        assert_eq!(lod_thresholds(lod_distances(12.0)), [73_728, 49_152, 24_576]);
    }

    #[test]
    fn lod_by_distance_matches_draw_mode() {
        let th = lod_thresholds(lod_distances(12.0));
        let at = |d: f32| draw_lod(d * 1024.0, 4096.0, false, th);
        assert_eq!(at(60.0), Lod::Lod1);
        assert_eq!(at(80.0), Lod::Lod2);
        assert_eq!(at(10.0), Lod::Lod0);
        assert_eq!(draw_lod(0.0, 0.0, true, th), Lod::Lod2);
    }

    #[test]
    fn a_sphere_on_the_view_axis_is_drawn_and_one_behind_is_culled() {
        let view = camera();
        let th = lod_thresholds(lod_distances(12.0));
        let ahead = [168.0 * 1024.0, -40.0 * 1024.0 + 200.0 * 1024.0, 90.0 * 1024.0];
        let behind = [168.0 * 1024.0, -40.0 * 1024.0 - 200.0 * 1024.0, 90.0 * 1024.0];
        let s = |c| Sphere { centre: c, radius: 1024.0 };
        assert_ne!(view.tfrag_mode(s(ahead), false, th), DrawMode::Culled);
        assert_eq!(view.tfrag_mode(s(behind), false, th), DrawMode::Culled);
    }

    #[test]
    fn a_sphere_far_to_the_side_is_culled() {
        let view = camera();
        let th = lod_thresholds(lod_distances(12.0));
        let side = [168.0 * 1024.0 + 5000.0 * 1024.0, -40.0 * 1024.0 + 100.0 * 1024.0, 90.0 * 1024.0];
        assert_eq!(view.tfrag_mode(Sphere { centre: side, radius: 1024.0 }, false, th), DrawMode::Culled);
    }
}
