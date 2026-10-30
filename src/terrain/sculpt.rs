//! Hand edits of the live terrain: [`SculptTerrain`], one brush stroke over the bedrock layer.
//!
//! A stroke is a [`Message`]. The terrain plugin queues them and sends one per frame to the
//! render world, which runs `sculpt.wesl` at the head of the terrain's compute pass, so the pack,
//! the leaf bounds and the height readback of the same frame carry the edit: the drawn ground, the
//! rocks, the cover and [`Environment::ground_height`](crate::Environment::ground_height) all
//! follow it. Bedrock moves and sediment stays (`docs/research/terrain-sculpt.md`).

use std::collections::VecDeque;
use std::sync::atomic::Ordering;

use bevy::math::{Vec2, Vec4};
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

use super::TerrainGpu;
use crate::setup::{SetupError, bad};

/// What a stroke does to the ground under it. `w` is the brush weight at a cell
/// ([`sculpt_weight`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SculptKind {
    /// Raise the bedrock by `amount_m · w`.
    Raise {
        /// Height gained at the centre of the plateau, m.
        amount_m: f32,
    },
    /// Lower the bedrock by `amount_m · w`.
    Lower {
        /// Height lost at the centre of the plateau, m.
        amount_m: f32,
    },
    /// Move the surface toward the mean of its eight neighbours by `blend · w`.
    Smooth {
        /// Fraction of the way to the neighbours' mean at full weight, `0..=1`.
        blend: f32,
    },
    /// Move the surface toward a height by `blend · w`.
    Flatten {
        /// The level to flatten to, m in the metre frame (the terrain's `y0` included).
        height_m: f32,
        /// Fraction of the way to `height_m` at full weight, `0..=1`; 1 makes a level disc.
        blend: f32,
    },
    /// Add `amplitude_m · w · n` with `n` a seeded three-octave Perlin noise in `[−1, 1]`.
    Noise {
        /// Peak height change at full weight, m.
        amplitude_m: f32,
        /// Size of the noise's largest feature, m.
        feature_m: f32,
        /// Selects the noise; the same seed gives the same stroke.
        seed: u32,
    },
}

/// One brush stroke on the live [`Terrain`](crate::Terrain), applied on the next frame the
/// terrain is live. A drag is a message per sample; one is applied per frame, in order.
///
/// ```
/// # use bevy_elements::{SculptKind, SculptTerrain};
/// # use bevy::math::Vec2;
/// let hill = SculptTerrain::new(Vec2::new(100.0, -40.0), 60.0, SculptKind::Raise { amount_m: 5.0 });
/// assert!(hill.validate().is_ok());
/// ```
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SculptTerrain {
    /// Centre of the brush, world XZ in the metre frame (what every query takes).
    pub centre_m: Vec2,
    /// Radius, m: nothing at this distance or beyond changes.
    pub radius_m: f32,
    /// The fraction of the radius that takes the full effect, `0.0..1.0`; the weight falls as a
    /// smoothstep from there to the radius, so the rim leaves no crease. Default 0.35.
    pub hardness: f32,
    /// What the stroke does.
    pub kind: SculptKind,
}

/// The hardness a stroke made by [`SculptTerrain::new`] takes.
pub const DEFAULT_SCULPT_HARDNESS: f32 = 0.35;

impl SculptTerrain {
    /// A stroke at `centre_m` of `radius_m` with the default hardness.
    pub fn new(centre_m: Vec2, radius_m: f32, kind: SculptKind) -> Self {
        Self {
            centre_m,
            radius_m,
            hardness: DEFAULT_SCULPT_HARDNESS,
            kind,
        }
    }

    /// Set the hardness.
    pub fn with_hardness(mut self, hardness: f32) -> Self {
        self.hardness = hardness;
        self
    }

    /// Check every field against its bound; the error names the field. A stroke that fails is
    /// dropped (with a warning) by the terrain plugin, so an editor calls this first.
    pub fn validate(&self) -> Result<(), SetupError> {
        if !self.centre_m.is_finite() {
            return Err(bad("centre_m", "must be finite"));
        }
        if !(self.radius_m.is_finite() && self.radius_m > 0.0) {
            return Err(bad("radius_m", "must be finite and above 0"));
        }
        if !(0.0..1.0).contains(&self.hardness) {
            return Err(bad("hardness", "must be in 0.0..1.0"));
        }
        let finite_min = |v: f32| v.is_finite() && v >= 0.0;
        let unit = |v: f32| (0.0..=1.0).contains(&v);
        match self.kind {
            SculptKind::Raise { amount_m } | SculptKind::Lower { amount_m } => {
                if !finite_min(amount_m) {
                    return Err(bad("kind.amount_m", "must be finite and at least 0"));
                }
            }
            SculptKind::Smooth { blend } => {
                if !unit(blend) {
                    return Err(bad("kind.blend", "must be in 0.0..=1.0"));
                }
            }
            SculptKind::Flatten { height_m, blend } => {
                if !height_m.is_finite() {
                    return Err(bad("kind.height_m", "must be finite"));
                }
                if !unit(blend) {
                    return Err(bad("kind.blend", "must be in 0.0..=1.0"));
                }
            }
            SculptKind::Noise {
                amplitude_m,
                feature_m,
                ..
            } => {
                if !finite_min(amplitude_m) {
                    return Err(bad("kind.amplitude_m", "must be finite and at least 0"));
                }
                if !(feature_m.is_finite() && feature_m > 0.0) {
                    return Err(bad("kind.feature_m", "must be finite and above 0"));
                }
            }
        }
        Ok(())
    }
}

/// The brush weight at distance `d_m` from the centre: 1 inside `hardness · radius_m`, a
/// smoothstep down to 0 at `radius_m`, 0 beyond. Mirrors `sculpt_weight` in `sculpt.wesl`.
pub fn sculpt_weight(d_m: f32, radius_m: f32, hardness: f32) -> f32 {
    let t = ((d_m / radius_m - hardness) / (1.0 - hardness)).clamp(0.0, 1.0);
    let s = 1.0 - t;
    s * s * (3.0 - 2.0 * s)
}

/// Uniform of `sculpt.wesl`. Mirrors `SculptParams`.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub(crate) struct SculptParamsGpu {
    origin: Vec2,
    cell: f32,
    pad0: f32,
    size: bevy::math::UVec2,
    pad1: bevy::math::UVec2,
    /// xy: centre. z: radius. w: hardness.
    area: Vec4,
    /// x: kind. y: amount. z: flatten height above y0 | noise feature. w: noise seed.
    shape: Vec4,
}

impl SculptParamsGpu {
    fn new(stroke: &SculptTerrain, grid: &super::TerrainGrid) -> Self {
        let (kind, amount, z, w) = match stroke.kind {
            SculptKind::Raise { amount_m } => (0.0, amount_m, 0.0, 0.0),
            SculptKind::Lower { amount_m } => (1.0, amount_m, 0.0, 0.0),
            SculptKind::Smooth { blend } => (2.0, blend, 0.0, 0.0),
            SculptKind::Flatten { height_m, blend } => (3.0, blend, height_m - grid.y0, 0.0),
            // The seed offsets the noise's third coordinate; 16 bits keep it exact in an f32.
            SculptKind::Noise {
                amplitude_m,
                feature_m,
                seed,
            } => (
                4.0,
                amplitude_m,
                feature_m,
                (seed & 0xffff) as f32 * 17.0 + 0.5,
            ),
        };
        Self {
            origin: grid.origin,
            cell: grid.cell_m,
            pad0: 0.0,
            size: grid.size,
            pad1: bevy::math::UVec2::ZERO,
            area: Vec4::new(
                stroke.centre_m.x,
                stroke.centre_m.y,
                stroke.radius_m,
                stroke.hardness,
            ),
            shape: Vec4::new(kind, amount, z, w),
        }
    }
}

/// Strokes waiting for the render world.
#[derive(Resource, Default)]
pub(crate) struct SculptQueue(VecDeque<SculptTerrain>);

/// Queue this frame's strokes and hand the next one to the render world once it has run the last.
///
/// The request is a plain field of the extracted [`TerrainGpu`] with an epoch; the render world
/// answers through `stroked` only (`AGENTS.md`, pipelined rendering). Nothing is sent before the
/// terrain's first generation, so an early stroke is not lost under the generator.
pub(crate) fn apply_sculpts(
    mut strokes: MessageReader<SculptTerrain>,
    mut queue: ResMut<SculptQueue>,
    gpu: Option<ResMut<TerrainGpu>>,
) {
    for stroke in strokes.read() {
        match stroke.validate() {
            Ok(()) => queue.0.push_back(*stroke),
            Err(error) => warn!("a SculptTerrain stroke is dropped: {error}"),
        }
    }
    let Some(mut gpu) = gpu else {
        return;
    };
    if !gpu.generated.load(Ordering::Relaxed)
        || gpu.stroked.load(Ordering::Relaxed) < gpu.stroke_epoch
    {
        return;
    }
    if let Some(stroke) = queue.0.pop_front() {
        gpu.stroke = Some(SculptParamsGpu::new(&stroke, &gpu.state.grid));
        gpu.stroke_epoch += 1;
        gpu.changes += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_weight_is_a_plateau_then_a_smooth_fall_to_nothing_at_the_radius() {
        assert_eq!(sculpt_weight(0.0, 10.0, 0.5), 1.0);
        assert_eq!(sculpt_weight(5.0, 10.0, 0.5), 1.0);
        assert_eq!(sculpt_weight(10.0, 10.0, 0.5), 0.0);
        assert_eq!(sculpt_weight(25.0, 10.0, 0.5), 0.0);
        let mut last = 1.0;
        for i in 50..=100 {
            let w = sculpt_weight(i as f32 / 10.0, 10.0, 0.5);
            assert!(w <= last, "monotone at {i}: {w} > {last}");
            last = w;
        }
        // Zero slope at the rim: no crease where the brush ends.
        assert!(sculpt_weight(9.99, 10.0, 0.5) < 1e-4);
    }

    #[test]
    fn a_stroke_names_the_field_that_breaks_its_bound() {
        let ok = SculptTerrain::new(Vec2::ZERO, 10.0, SculptKind::Raise { amount_m: 1.0 });
        assert!(ok.validate().is_ok());
        let field = |s: SculptTerrain| s.validate().expect_err("invalid").field;
        assert_eq!(
            field(SculptTerrain {
                radius_m: 0.0,
                ..ok
            }),
            "radius_m"
        );
        assert_eq!(field(ok.with_hardness(1.0)), "hardness");
        assert_eq!(field(ok.with_hardness(-0.1)), "hardness");
        assert_eq!(
            field(SculptTerrain {
                kind: SculptKind::Lower { amount_m: -1.0 },
                ..ok
            }),
            "kind.amount_m"
        );
        assert_eq!(
            field(SculptTerrain {
                kind: SculptKind::Smooth { blend: 1.5 },
                ..ok
            }),
            "kind.blend"
        );
        assert_eq!(
            field(SculptTerrain {
                kind: SculptKind::Noise {
                    amplitude_m: 1.0,
                    feature_m: 0.0,
                    seed: 0
                },
                ..ok
            }),
            "kind.feature_m"
        );
        assert_eq!(
            field(SculptTerrain {
                centre_m: Vec2::new(f32::NAN, 0.0),
                ..ok
            }),
            "centre_m"
        );
    }
}
