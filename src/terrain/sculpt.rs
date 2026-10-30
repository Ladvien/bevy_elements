//! Hand edits of the live terrain: [`SculptTerrain`], one brush stroke over the bedrock layer.
//!
//! A stroke is a [`Message`]. The terrain plugin queues them and sends one per frame to the
//! render world, which runs `sculpt.wesl` at the head of the terrain's compute pass, so the pack,
//! the leaf bounds and the height readback of the same frame carry the edit: the drawn ground, the
//! rocks, the cover and [`Environment::terrain_height`](crate::Environment::terrain_height) all
//! follow it. Bedrock moves and sediment stays (`docs/research/terrain-sculpt.md`).

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use bevy::math::{IVec2, Rect, UVec2, Vec2, Vec4};
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

use super::{TerrainGpu, TerrainGrid, TerrainSnapshot};
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

    /// The stroke that undoes this one, for the kinds that only add a delta: a raise is undone by
    /// a lower of the same amount, centre, radius and hardness, and the reverse. `None` for
    /// `Smooth`, `Flatten` and `Noise`, whose result depends on the ground they met; an editor
    /// undoes those from a snapshot of the heights taken before the stroke.
    ///
    /// ```
    /// # use bevy_elements::{SculptKind, SculptTerrain};
    /// # use bevy::math::Vec2;
    /// let hill = SculptTerrain::new(Vec2::ZERO, 30.0, SculptKind::Raise { amount_m: 4.0 });
    /// let undo = hill.inverse().unwrap();
    /// assert_eq!(undo.kind, SculptKind::Lower { amount_m: 4.0 });
    /// assert_eq!(undo.inverse(), Some(hill));
    /// ```
    pub fn inverse(&self) -> Option<Self> {
        let kind = match self.kind {
            SculptKind::Raise { amount_m } => SculptKind::Lower { amount_m },
            SculptKind::Lower { amount_m } => SculptKind::Raise { amount_m },
            SculptKind::Smooth { .. } | SculptKind::Flatten { .. } | SculptKind::Noise { .. } => {
                return None;
            }
        };
        Some(Self { kind, ..*self })
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

/// Put the ground of a region back as a [`TerrainSnapshot`] held it: the undo of a stroke that
/// has no [`SculptTerrain::inverse`] (Smooth, Flatten, Noise), or of any stroke, without
/// respawning the terrain and so without losing the water, the cover maps or the trails.
///
/// Every cell whose centre lies in `region_m` takes the snapshot's bedrock (`surface − sediment`)
/// and sediment; every other cell is left as it is. The restore is applied in order with
/// [`SculptTerrain`] strokes, one edit per frame, and the drawn ground, the rocks, the cover and
/// [`Environment::terrain_height`](crate::Environment::terrain_height) follow like they follow a
/// stroke. Take the snapshot with
/// [`Environment::terrain_snapshot`](crate::Environment::terrain_snapshot) before the stroke; one
/// whose grid differs from the live terrain's (another terrain, or a readback coarser than the
/// simulation above 1024² cells) is dropped with a warning. Water is not restored.
///
/// ```
/// # use bevy_elements::{RestoreTerrain, TerrainGrid, TerrainSnapshot};
/// # use bevy::math::{Rect, UVec2, Vec2};
/// # use std::sync::Arc;
/// let grid = TerrainGrid { origin: Vec2::ZERO, cell_m: 1.0, size: UVec2::splat(8), y0: 0.0 };
/// let before = TerrainSnapshot {
///     grid,
///     surface: Arc::from(vec![0.0; 64]),
///     sediment: Arc::from(vec![0.0; 64]),
///     version: 0,
/// };
/// let undo = RestoreTerrain::new(before, Rect::new(2.0, 2.0, 6.0, 6.0));
/// assert!(undo.validate().is_ok());
/// ```
#[derive(Message, Clone, Debug)]
pub struct RestoreTerrain {
    /// The ground to restore from.
    pub snapshot: TerrainSnapshot,
    /// The region to restore, world XZ in the metre frame; cells whose centres lie in it change.
    pub region_m: Rect,
}

impl RestoreTerrain {
    /// Restore `region_m` from `snapshot`.
    pub fn new(snapshot: TerrainSnapshot, region_m: Rect) -> Self {
        Self { snapshot, region_m }
    }

    /// Check the region and the snapshot's arrays; the error names the field. A restore that
    /// fails is dropped (with a warning) by the terrain plugin, so an editor calls this first.
    pub fn validate(&self) -> Result<(), SetupError> {
        let r = self.region_m;
        if !(r.min.is_finite() && r.max.is_finite()) {
            return Err(bad("region_m", "must be finite"));
        }
        if !(r.max.x > r.min.x && r.max.y > r.min.y) {
            return Err(bad("region_m", "must have a positive width and depth"));
        }
        let cells = self.snapshot.grid.size.x as usize * self.snapshot.grid.size.y as usize;
        if self.snapshot.surface.len() != cells || self.snapshot.sediment.len() != cells {
            return Err(bad(
                "snapshot",
                "surface and sediment must hold one value per cell of its grid",
            ));
        }
        Ok(())
    }
}

/// The cells of `grid` whose centres lie in `region`, as `(first, past_the_last)` per axis; `None`
/// when no centre does.
pub(crate) fn region_cells(grid: &TerrainGrid, region: Rect) -> Option<(UVec2, UVec2)> {
    let lo = ((region.min - grid.origin) / grid.cell_m - 0.5).ceil();
    let hi = ((region.max - grid.origin) / grid.cell_m - 0.5).floor();
    let size = grid.size.as_ivec2();
    let first = lo.as_ivec2().max(IVec2::ZERO);
    let last = hi.as_ivec2().min(size - 1);
    (first.cmple(last).all()).then(|| (first.as_uvec2(), (last + 1).as_uvec2()))
}

/// Whether a snapshot was read from the live terrain's grid.
fn same_grid(a: &TerrainGrid, b: &TerrainGrid) -> bool {
    a.size == b.size
        && (a.cell_m - b.cell_m).abs() <= 1e-3 * b.cell_m
        && (a.origin - b.origin).abs().max_element() <= 1e-3 * b.cell_m
}

/// A restored region, cut out for the render world: `rows` rows of `width` cells from `(x0, z0)`.
#[derive(Clone, Debug)]
pub(crate) struct RestoreGpu {
    pub x0: u32,
    pub z0: u32,
    pub width: u32,
    pub rows: u32,
    pub bedrock: Arc<[f32]>,
    pub sediment: Arc<[f32]>,
}

impl RestoreGpu {
    /// Cut the region out of the snapshot; `None` when no cell centre lies in it.
    fn new(restore: &RestoreTerrain) -> Option<Self> {
        let snap = &restore.snapshot;
        let (first, past) = region_cells(&snap.grid, restore.region_m)?;
        let width = past.x - first.x;
        let rows = past.y - first.y;
        let mut bedrock = Vec::with_capacity((width * rows) as usize);
        let mut sediment = Vec::with_capacity((width * rows) as usize);
        for z in first.y..past.y {
            let row = (z * snap.grid.size.x + first.x) as usize;
            for i in row..row + width as usize {
                bedrock.push(snap.surface[i] - snap.sediment[i]);
                sediment.push(snap.sediment[i]);
            }
        }
        Some(Self {
            x0: first.x,
            z0: first.y,
            width,
            rows,
            bedrock: bedrock.into(),
            sediment: sediment.into(),
        })
    }

    /// Write the rows into the live bedrock and sediment buffers (`size_x` cells per row).
    pub(crate) fn write(
        &self,
        queue: &bevy::render::renderer::RenderQueue,
        bedrock: &bevy::render::render_resource::Buffer,
        sediment: &bevy::render::render_resource::Buffer,
        size_x: u32,
    ) {
        let w = self.width as usize;
        for r in 0..self.rows as usize {
            let offset = (u64::from(self.z0) + r as u64) * u64::from(size_x) + u64::from(self.x0);
            let rows = r * w..(r + 1) * w;
            queue.write_buffer(
                bedrock,
                offset * 4,
                bytemuck::cast_slice(&self.bedrock[rows.clone()]),
            );
            queue.write_buffer(
                sediment,
                offset * 4,
                bytemuck::cast_slice(&self.sediment[rows]),
            );
        }
    }
}

/// One edit waiting for the render world.
enum Edit {
    Stroke(SculptTerrain),
    Restore(RestoreGpu),
}

/// Edits waiting for the render world, in the order they were sent.
#[derive(Resource, Default)]
pub(crate) struct SculptQueue(VecDeque<Edit>);

/// Queue this frame's edits and hand the next one to the render world once it has run the last.
///
/// The request is a plain field of the extracted [`TerrainGpu`] with an epoch; the render world
/// answers through `stroked` and `restored` only (`AGENTS.md`, pipelined rendering). Nothing is
/// sent before the terrain's first generation, so an early edit is not lost under the generator.
pub(crate) fn apply_sculpts(
    mut strokes: MessageReader<SculptTerrain>,
    mut restores: MessageReader<RestoreTerrain>,
    mut queue: ResMut<SculptQueue>,
    gpu: Option<ResMut<TerrainGpu>>,
) {
    // Within one frame every stroke is queued before every restore; across frames the order is kept.
    for stroke in strokes.read() {
        match stroke.validate() {
            Ok(()) => queue.0.push_back(Edit::Stroke(*stroke)),
            Err(error) => warn!("a SculptTerrain stroke is dropped: {error}"),
        }
    }
    let live = gpu.as_ref().map(|g| g.state.grid);
    for restore in restores.read() {
        if let Err(error) = restore.validate() {
            warn!("a RestoreTerrain is dropped: {error}");
        } else if live.is_some_and(|grid| !same_grid(&restore.snapshot.grid, &grid)) {
            warn!(
                "a RestoreTerrain is dropped: its snapshot was not read from the live terrain's grid"
            );
        } else if let Some(cut) = RestoreGpu::new(restore) {
            queue.0.push_back(Edit::Restore(cut));
        }
    }
    let Some(mut gpu) = gpu else {
        return;
    };
    if !gpu.generated.load(Ordering::Relaxed)
        || gpu.stroked.load(Ordering::Relaxed) < gpu.stroke_epoch
        || gpu.restored.load(Ordering::Relaxed) < gpu.restore_epoch
    {
        return;
    }
    match queue.0.pop_front() {
        Some(Edit::Stroke(stroke)) => {
            gpu.stroke = Some(SculptParamsGpu::new(&stroke, &gpu.state.grid));
            gpu.stroke_epoch += 1;
            gpu.changes += 1;
        }
        Some(Edit::Restore(cut)) => {
            gpu.restore = Some(cut);
            gpu.restore_epoch += 1;
            gpu.changes += 1;
        }
        None => {}
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
    fn only_the_stroke_kinds_that_add_a_delta_have_an_inverse() {
        let at = |kind| SculptTerrain::new(Vec2::new(3.0, 4.0), 12.0, kind).with_hardness(0.6);
        let raise = at(SculptKind::Raise { amount_m: 2.5 });
        let undo = raise.inverse().expect("a raise is undone by a lower");
        assert_eq!(
            undo,
            at(SculptKind::Lower { amount_m: 2.5 }).with_hardness(0.6)
        );
        assert_eq!(undo.inverse(), Some(raise));
        assert!(at(SculptKind::Smooth { blend: 1.0 }).inverse().is_none());
        assert!(
            at(SculptKind::Flatten {
                height_m: 1.0,
                blend: 1.0
            })
            .inverse()
            .is_none()
        );
        assert!(
            at(SculptKind::Noise {
                amplitude_m: 1.0,
                feature_m: 5.0,
                seed: 1
            })
            .inverse()
            .is_none()
        );
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

    fn grid() -> TerrainGrid {
        TerrainGrid {
            origin: Vec2::new(-4.0, -4.0),
            cell_m: 1.0,
            size: UVec2::splat(8),
            y0: 0.0,
        }
    }

    #[test]
    fn a_region_takes_the_cells_whose_centres_it_contains() {
        // Centres sit at -3.5 .. 3.5: x in [-1, 1] holds centres -0.5 and 0.5, cells 3 and 4.
        let (first, past) = region_cells(&grid(), Rect::new(-1.0, -1.0, 1.0, 1.0)).expect("cells");
        assert_eq!((first, past), (UVec2::splat(3), UVec2::splat(5)));
        // A region past the edge clamps to the grid.
        let (first, past) =
            region_cells(&grid(), Rect::new(-50.0, 2.0, 50.0, 50.0)).expect("cells");
        assert_eq!((first, past), (UVec2::new(0, 6), UVec2::new(8, 8)));
        // A region between two centres, or outside the grid, holds none.
        assert!(region_cells(&grid(), Rect::new(0.1, 0.1, 0.4, 0.4)).is_none());
        assert!(region_cells(&grid(), Rect::new(10.0, 10.0, 20.0, 20.0)).is_none());
    }

    #[test]
    fn a_restore_is_cut_from_the_snapshots_bedrock_and_sediment() {
        let surface: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let snapshot = TerrainSnapshot {
            grid: grid(),
            surface: Arc::from(surface),
            sediment: Arc::from(vec![0.5; 64]),
            version: 0,
        };
        let restore = RestoreTerrain::new(snapshot, Rect::new(-1.0, -1.0, 1.0, 1.0));
        assert!(restore.validate().is_ok());
        let cut = RestoreGpu::new(&restore).expect("a region");
        assert_eq!((cut.x0, cut.z0, cut.width, cut.rows), (3, 3, 2, 2));
        // Cells (3,3) (4,3) (3,4) (4,4) are indices 27, 28, 35, 36.
        assert_eq!(&*cut.bedrock, &[26.5, 27.5, 34.5, 35.5]);
        assert_eq!(&*cut.sediment, &[0.5; 4]);
        let mut bad = restore.clone();
        bad.region_m = Rect::new(1.0, 1.0, 1.0, 2.0);
        assert!(bad.validate().is_err());
    }
}
