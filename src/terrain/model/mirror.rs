//! CPU transliterations of the terrain kernels that the tests compare against: the procedural
//! generator of `generate.wesl` and the layer weights of `terrain.wesl`.

use bevy::math::{IVec3, UVec2, Vec2, Vec3};

use super::{F_HALF, GAIN, LACUNARITY, OCTAVE_ROTATION, R_LOW, R_SPAN, octave_count, seed_mix};
use crate::gpu::noise_mirror::perlin3_d;
use crate::terrain::ProceduralTerrain;

/// Both noise sums of a sample point: `(f, r)` of the research note.
pub(crate) fn noise_sums(local_xz: Vec2, feature_m: f32, octaves: u32, seed: u64) -> (f32, f32) {
    let p = local_xz / feature_m;
    let mix = seed_mix(seed);
    let (mut f_sum, mut r_sum, mut norm, mut gain) = (0.0, 0.0, 0.0, 1.0);
    let mut weight = 1.0_f32;
    for k in 0..octaves {
        let (s, c) = (OCTAVE_ROTATION * k as f32).sin_cos();
        let q = Vec2::new(c * p.x - s * p.y, s * p.x + c * p.y) * LACUNARITY.powi(k as i32);
        let v = ((mix + k * 1031) % 4096) as f32;
        let n = perlin3_d(Vec3::new(q.x + v, q.y + v, 0.5 + k as f32)).x;
        let ridge = (1.0 - n.abs()).max(0.0);
        let s_k = weight * ridge * ridge;
        f_sum += gain * n;
        r_sum += gain * s_k;
        norm += gain;
        weight = (2.0 * s_k).clamp(0.0, 1.0);
        gain *= GAIN;
    }
    (f_sum / norm, r_sum / norm)
}

/// The unit terrain value `(1 − q) u_f + q u_r` of the two sums.
pub(crate) fn unit_height(f: f32, r: f32, ridged: f32) -> f32 {
    let u_f = (0.5 + f / (2.0 * F_HALF)).clamp(0.0, 1.0);
    let u_r = ((r - R_LOW) / R_SPAN).clamp(0.0, 1.0);
    (1.0 - ridged) * u_f + ridged * u_r
}

/// The surface height at a point `local_xz` metres from the terrain's minimum corner, relative
/// to the terrain transform: the base height plus the relief times the unit value.
pub(crate) fn surface_height(p: &ProceduralTerrain, octaves: u32, local_xz: Vec2) -> f32 {
    let (f, r) = noise_sums(local_xz, p.feature_size_m, octaves, p.seed);
    p.base_height_m + p.relief_m * unit_height(f, r, p.ridged.clamp(0.0, 1.0))
}

/// The generated surface at every cell of a `size` grid of `cell_m` cells, row-major.
pub(crate) fn generate_surface(p: &ProceduralTerrain, size: UVec2, cell_m: f32) -> Vec<f32> {
    let octaves = octave_count(p.octaves, p.feature_size_m, cell_m);
    let mut out = Vec::with_capacity((size.x * size.y) as usize);
    for j in 0..size.y {
        for i in 0..size.x {
            let local = (Vec2::new(i as f32, j as f32) + 0.5) * cell_m;
            out.push(surface_height(p, octaves, local));
        }
    }
    out
}

/// The slope angle of a surface at a cell by central differences over ±1 cell (one-sided at the
/// edge), degrees.
pub(crate) fn slope_deg(surface: &[f32], size: UVec2, cell_m: f32, i: u32, j: u32) -> f32 {
    let at = |x: u32, z: u32| surface[(z * size.x + x) as usize];
    let (xl, xh) = (i.saturating_sub(1), (i + 1).min(size.x - 1));
    let (zl, zh) = (j.saturating_sub(1), (j + 1).min(size.y - 1));
    let gx = (at(xh, j) - at(xl, j)) / ((xh - xl) as f32 * cell_m);
    let gz = (at(i, zh) - at(i, zl)) / ((zh - zl) as f32 * cell_m);
    (gx * gx + gz * gz).sqrt().atan().to_degrees()
}

/// Percentile `q` (0–1) of `values` (sorted in place).
pub(crate) fn percentile(values: &mut [f32], q: f32) -> f32 {
    values.sort_by(f32::total_cmp);
    values[((values.len() - 1) as f32 * q).round() as usize]
}

/// `smoothstep(a, b, x)` of WGSL.
pub(crate) fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Andersson 2007 §5.2.3, Listing 2: the Overlay blend of `detail` onto `mask` at full opacity.
pub(crate) fn overlay(mask: f32, detail: f32) -> f32 {
    if mask < 0.5 {
        2.0 * mask * detail
    } else {
        1.0 - 2.0 * (1.0 - mask) * (1.0 - detail)
    }
}

/// Feature size of the coarse ground detail's base octave, m (`terrain/common.wesl`).
pub(crate) const DETAIL_COARSE_M: f32 = 1.25;
/// Feature size of the fine ground detail's base octave, m.
pub(crate) const DETAIL_FINE_M: f32 = 0.125;

/// `detail_keep` of `terrain/common.wesl`: the weight of an octave of feature size `feature_m`
/// in a pixel whose footprint is `footprint_m` wide. 1 while the octave is resolved (at least
/// two footprints per feature), falling to 0 over the band where it is not (Lagae et al. 2010
/// §5.2.1, frequency clamping; `docs/research/ground-detail.md` Model 2).
pub(crate) fn detail_keep(feature_m: f32, footprint_m: f32) -> f32 {
    let t = ((footprint_m / feature_m - 0.5) / 0.5).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

/// The four layer weights `(rock, sand, soil, grass)` of `render.wesl` for a slope angle
/// (degrees), a sediment depth (m) and a detail value in [0, 1] (0.5: none).
pub(crate) fn layer_weights(slope_deg: f32, sediment_m: f32, detail: f32) -> [f32; 4] {
    let rock = overlay(smoothstep(30.0, 38.0, slope_deg), detail);
    let sand = overlay(smoothstep(0.3, 0.7, sediment_m), detail)
        * (1.0 - smoothstep(26.0, 30.0, slope_deg))
        * (1.0 - rock);
    let band = overlay(smoothstep(16.0, 26.0, slope_deg), detail) * (1.0 - smoothstep(22.0, 29.0, slope_deg));
    let rest = (1.0 - rock) * (1.0 - sand);
    [rock, sand, rest * band, rest * (1.0 - band)]
}

/// The scale of the ground's large-scale variation, m (`render.wesl`).
pub(crate) const LAYER_VARIATION_M: f32 = 45.0;
/// How far the variation moves the layer weights (`render.wesl`).
pub(crate) const LAYER_VARIATION_DEPTH: f32 = 0.25;
/// The rms of the crate's 3D noise, measured in the crate's own mirror: the field is
/// normalised by it so that `LAYER_VARIATION_DEPTH` is the fraction of the grass a patch turns
/// to soil.
pub(crate) const NOISE_RMS: f32 = 0.2268;

/// The ground's large-scale variation of the layer weights (`render.wesl`): a low-frequency
/// field that takes soil from the grass, so a plain — where the soil's weight is zero — gets
/// patches instead of one colour. The field is the `ground_fields` function's large one.
pub(crate) fn layer_variation(field: f32, w: [f32; 4]) -> [f32; 4] {
    let field = field.clamp(-1.0, 1.0);
    let shift = field * LAYER_VARIATION_DEPTH * w[3];
    let mut v = w;
    // The shift is taken from the grass and given to the soil, and both ends are clamped: a
    // field more negative than the soil's weight would take the soil below zero (measured:
    // −0.054 on a plain), and a negative weight is a negative albedo.
    let taken = shift.clamp(-v[2], v[3]);
    v[2] += taken;
    v[3] -= taken;
    let total = v.iter().sum::<f32>();
    v.map(|x| x / total.max(1e-6))
}

/// The turf's palette (`terrain/common.wesl`): the four tones, as multipliers on the turf's
/// own colour. The fresh tone is the identity.
pub(crate) const TURF_FRESH: [f32; 3] = [1.00, 1.00, 1.00];
pub(crate) const TURF_DRY: [f32; 3] = [1.35, 1.20, 0.69];
pub(crate) const TURF_STRAW: [f32; 3] = [1.55, 1.37, 0.82];
pub(crate) const TURF_WORN: [f32; 3] = [1.13, 0.95, 0.73];
/// How far the palette moves the ground's colour towards a tone, 0 to 1.
pub(crate) const TURF_STRENGTH: f32 = 0.85;

/// The turf's palette at the given fields (`terrain/common.wesl`): the *shares* of the four
/// tones, which sum to one by construction, and the mixture they make.
pub(crate) fn turf_palette(field: f32, clump: f32) -> ([f32; 3], [f32; 4]) {
    let p = 2.0f32;
    let mut dry = (field.max(0.0) * (0.5 + clump).max(0.0) * 1.6).powf(p).min(1.0);
    let mut straw = ((field - 0.45).max(0.0) * 3.0 * (0.5 + clump).max(0.0) * 1.6)
        .powf(p)
        .min(1.0);
    let mut worn = ((-field - 0.55).max(0.0) * 3.0 * (0.5 - clump).max(0.0) * 1.6)
        .powf(p)
        .min(1.0);
    let others = dry + straw + worn;
    if others > 1.0 {
        let k = 1.0 / others;
        dry *= k;
        straw *= k;
        worn *= k;
    }
    // The fresh share is what is left, and the subtraction can leave it a hair below zero
    // (−2.2e−8 at field 0.5, clump 0.75), which is a negative weight.
    let fresh = (1.0 - dry - straw - worn).max(0.0);
    let shares = [fresh, dry, straw, worn];
    let mut mixed = [0.0f32; 3];
    for c in 0..3 {
        mixed[c] = fresh * TURF_FRESH[c]
            + dry * TURF_DRY[c]
            + straw * TURF_STRAW[c]
            + worn * TURF_WORN[c];
        mixed[c] = 1.0 + (mixed[c] - 1.0) * TURF_STRENGTH;
    }
    (mixed, shares)
}

// The cliff's blocks (`docs/research/rock-blocks.md`)
// ---------------------------------------------------------------------------------------------

/// The block network's cell, m (`common.wesl`): the scale of the pieces the rock breaks into.
pub(crate) const ROCK_BLOCK_M: f32 = 0.95;
/// How far the blocks stretch along the bedding, as a factor (`common.wesl`).
pub(crate) const ROCK_BLOCK_TABULAR: f32 = 2.6;
/// The bedding plane's normal, unnormalised (`common.wesl`).
pub(crate) const ROCK_BED_TILT: Vec3 = Vec3::new(0.32, 0.90, 0.28);
/// A joint's trace half-width, m (`common.wesl`): the weathered mouth of an aperture "mainly
/// less than a millimetre" (Palmström 2015), drawn per block between the two.
pub(crate) const ROCK_TRACE_MIN_M: f32 = 0.012;
pub(crate) const ROCK_TRACE_MAX_M: f32 = 0.030;
/// The open blocks' size, in cells (`common.wesl`): the cells whose feature points share a
/// lattice cell this many cells across are one block.
pub(crate) const ROCK_BLOCK_GROUP: f32 = 2.6;
/// A tight joint's trace, as a share of an open one's (`common.wesl`).
pub(crate) const ROCK_TIGHT_INK: f32 = 0.35;
/// The **second joint set's** normal: a cross joint, roughly perpendicular to the bedding and
/// tilted (`common.wesl`).
pub(crate) const ROCK_JOINT2_NORMAL: Vec3 = Vec3::new(0.71, 0.46, -0.53);
/// The second joint set's **spacing range**, m (`common.wesl`).
pub(crate) const ROCK_JOINT2_MIN_M: f32 = 0.55;
pub(crate) const ROCK_JOINT2_MAX_M: f32 = 2.60;
/// The **formation's** cell, m: the scale at which the second set's spacing is constant
/// (`common.wesl`).
pub(crate) const ROCK_FORMATION_M: f32 = 46.0;
/// How far the second set's planes **wander**, as a fraction of their own spacing: planar,
/// 2 % of a segment (`common.wesl`).
pub(crate) const ROCK_JOINT2_WANDER: f32 = 0.04;
/// The share of the blocks the second set splits (`common.wesl`).
pub(crate) const ROCK_JOINT2_SHARE: f32 = 0.5;
/// The warp that bends the cells, m (`common.wesl`): the warp's vector is the crate's own 3D
/// noise's *gradient*, whose mean magnitude is 0.94, so the amplitude is also the
/// displacement's scale; 0.08 m over the warp's 3.8 m keeps the borders planar (about 2 %).
pub(crate) const ROCK_BLOCK_WARP_M: f32 = 0.08;
/// How many cells the warp's noise varies over (`common.wesl`).
pub(crate) const ROCK_BLOCK_WARP_CELLS: f32 = 4.0;
/// The Weibull's shape for the joint's width, the distribution the fracture spacing follows
/// (Verdian et al. 2021). A shape of 1 is the exponential and 2 is the Rayleigh.
pub(crate) const ROCK_BLOCK_WEIBULL: f32 = 1.6;
/// The Weibull's draw's ceiling: the 95th percentile, and not the unbounded tail. The
/// fracture spacing's own distribution is *undersampled* in its lower tail, so the tail is
/// bounded in the data and it is bounded here.
pub(crate) const ROCK_BLOCK_WEIBULL_MAX: f32 = 2.0;

/// The block network's coordinates for a world point, stretched along the bedding plane's
/// normal so the cells are thin across the beds and long along them: slabs (`common.wesl`).
///
/// The coordinates are **warped** before the lookup, by the crate's own 3D noise's *gradient*,
/// so the cells' boundaries wander: a cell network's cells are convex polygons of nearly the
/// same area, and a face of them reads as a mosaic — or, with the blocks' step, as *shingles*
/// (`docs/research/rock-fragments.md`).
pub(crate) fn rock_block_coords(p: Vec3) -> Vec3 {
    let normal = ROCK_BED_TILT.normalize();
    let along = p + normal * p.dot(normal) * (ROCK_BLOCK_TABULAR - 1.0);
    let warp = crate::gpu::noise_mirror::perlin3_d(p / (ROCK_BLOCK_M * ROCK_BLOCK_WARP_CELLS));
    let gradient = Vec3::new(warp.y, warp.z, warp.w);
    (along + gradient * ROCK_BLOCK_WARP_M) / ROCK_BLOCK_M
}

/// The rock's **palette**: the colours a cliff is made of (`docs/research/rock-palette.md`).
///
/// The tones are multipliers on the rock's own albedo, and their **equal-weight mean is
/// exactly (1, 1, 1)**, so the rock's mean colour and mean brightness cannot move.
pub(crate) const ROCK_TONES: [Vec3; 4] = [
    Vec3::new(0.848, 1.015, 1.309),
    Vec3::new(1.223, 1.083, 0.832),
    Vec3::new(1.214, 1.270, 1.256),
    Vec3::new(0.715, 0.632, 0.603),
];
/// The width of a split in the palette, as a fraction of the draw's range (`common.wesl`).
pub(crate) const ROCK_PALETTE_EDGE: f32 = 0.12;
/// How far the region's field biases the bed's selection (`common.wesl`).
pub(crate) const BED_REGION_BIAS: f32 = 0.30;
/// How far the palette is mixed in, 0 to 1 (`common.wesl`).
pub(crate) const ROCK_PALETTE_STRENGTH: f32 = 0.90;

/// The rock's **palette** at the *bed's* scale (`docs/research/bed-colour.md`).
///
/// The same four rocks as the fragments' palette, and the same normalisation: the
/// equal-weight mean is exactly (1, 1, 1), so the rock's mean colour cannot drift.
pub(crate) const BED_TONES: [Vec3; 4] = ROCK_TONES;
/// The width of a split in the palette, as a fraction of the draw's range (`common.wesl`).
pub(crate) const BED_PALETTE_EDGE: f32 = ROCK_PALETTE_EDGE;
/// How far the palette is mixed in, 0 to 1 (`common.wesl`).
pub(crate) const BED_PALETTE_STRENGTH: f32 = ROCK_PALETTE_STRENGTH;

/// A bed's own **colour** at a bed index: the palette's mixture for that index, and its own
/// brightness.
///
/// The **selection** is the bed's index's own hash and the **distribution** a second,
/// independent hash of the same index. The **region** is a slow field that biases both, so a
/// whole massif leans towards one rock and the massifs separate into warm and cool.
pub(crate) fn bed_palette(index: f32, region: f32) -> Vec3 {
    let i = index.floor() as i32;
    let h = crate::gpu::noise_mirror::hash31(IVec3::new(i, 7, 0));
    let h2 = crate::gpu::noise_mirror::hash31(IVec3::new(i, 11, 0));
    let sel = smoothstep(
        0.5 - BED_PALETTE_EDGE,
        0.5 + BED_PALETTE_EDGE,
        (h + region * BED_REGION_BIAS).clamp(0.0, 1.0),
    );
    let reg = smoothstep(
        0.5 - BED_PALETTE_EDGE,
        0.5 + BED_PALETTE_EDGE,
        (h2 + region * BED_REGION_BIAS).clamp(0.0, 1.0),
    );
    let shares = [
        (1.0 - sel) * (1.0 - reg),
        (1.0 - sel) * reg,
        sel * (1.0 - reg),
        sel * reg,
    ];
    let total: f32 = shares.iter().sum();
    let mut mixed = Vec3::ZERO;
    for (share, tone) in shares.iter().zip(BED_TONES.iter()) {
        mixed += *tone * (*share / total);
    }
    Vec3::ONE.lerp(mixed, BED_PALETTE_STRENGTH)
}

/// The **second joint set's** spacing at a world point, m: a *formation's* own draw.
///
/// A formation's cross-joint spacing is a property of the formation, so the spacing is
/// **constant within** it and changes from one formation to the next (`docs/research/
/// block-sizes.md`).
pub(crate) fn rock_joint2_spacing(p: Vec3) -> f32 {
    let formation = (p / ROCK_FORMATION_M).floor();
    let h = crate::gpu::noise_mirror::hash31(formation.as_ivec3());
    let weibull = (-(1.0 - h).max(1e-6).ln())
        .powf(1.0 / ROCK_BLOCK_WEIBULL)
        .min(ROCK_BLOCK_WEIBULL_MAX);
    ROCK_JOINT2_MIN_M + (ROCK_JOINT2_MAX_M - ROCK_JOINT2_MIN_M) * weibull / ROCK_BLOCK_WEIBULL_MAX
}

/// A joint's **trace** at `distance_m` from it, anti-aliased by its coverage of a pixel whose
/// footprint is `footprint_m` (`rock_trace` in `common.wesl`).
pub(crate) fn rock_trace(distance_m: f32, half_width_m: f32, footprint_m: f32) -> f32 {
    let pixel = 0.5 * footprint_m;
    let soft = half_width_m.max(pixel);
    let coverage = (half_width_m / pixel.max(1e-6)).min(1.0);
    (1.0 - smoothstep(0.0, soft, distance_m)) * coverage
}

/// The joints at a world point (`rock_block_scale` in `common.wesl`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct RockJoints {
    /// The distance to the first set's border, m: the bisector of the two nearest points.
    pub to_border_m: f32,
    /// The distance to the second set's nearest plane, m.
    pub to_plane_m: f32,
    /// Whether the point's block is split by the second set.
    pub split: bool,
    /// The traces' half-width in this block, m.
    pub half_width_m: f32,
    /// The lattice cell that owns the piece's feature point: the piece's identity.
    pub block: IVec3,
    /// The open block the piece belongs to: the group of cells parted from the next by open
    /// joints.
    pub group: IVec3,
    /// Whether the nearest border is an open joint (between two blocks) or a tight one.
    pub open: bool,
    /// The second set's coordinate, in spacings: its integers are the planes.
    pub phase: f32,
    /// The second set's spacing here, m.
    pub spacing_m: f32,
}

/// The open block of the cell nearest to `q` and whether its border is open (`common.wesl`):
/// the cells whose feature points share a lattice cell `ROCK_BLOCK_GROUP` cells across.
fn rock_group(q: Vec3, cell: &crate::gpu::noise_mirror::Cell3) -> (IVec3, IVec3, bool) {
    let base = q.floor();
    let own = base + (cell.point - Vec3::splat(0.5)).round();
    let other = base + (cell.second - Vec3::splat(0.5)).round();
    let group = (own / ROCK_BLOCK_GROUP).floor();
    let open = group != (other / ROCK_BLOCK_GROUP).floor();
    (own.as_ivec3(), group.as_ivec3(), open)
}

/// The joints at `p` (`common.wesl`).
pub(crate) fn rock_joints(p: Vec3) -> RockJoints {
    let q = rock_block_coords(p);
    let cell = crate::gpu::noise_mirror::cell3(q);
    let (block, group, open) = rock_group(q, &cell);
    let normal = ROCK_BED_TILT.normalize();
    let span = cell.point - cell.second;
    let away = span / span.length().max(1e-6);
    let through = away + normal * away.dot(normal) * (ROCK_BLOCK_TABULAR - 1.0);
    let stretch = through.length();
    let weibull = (-(1.0 - cell.factor).max(1e-6).ln())
        .powf(1.0 / ROCK_BLOCK_WEIBULL)
        .min(ROCK_BLOCK_WEIBULL_MAX);
    let spacing = rock_joint2_spacing(p);
    let wander = crate::gpu::noise_mirror::perlin3_d(p / (ROCK_BLOCK_M * ROCK_BLOCK_WARP_CELLS)).x
        * spacing
        * ROCK_JOINT2_WANDER;
    let phase = (p.dot(ROCK_JOINT2_NORMAL.normalize()) + wander) / spacing;
    let split_draw = crate::gpu::noise_mirror::hash31(group + IVec3::new(29, 31, 37));
    RockJoints {
        to_border_m: cell.border.max(0.0) * ROCK_BLOCK_M / stretch,
        to_plane_m: (phase - phase.round()).abs() * spacing,
        split: split_draw < ROCK_JOINT2_SHARE,
        half_width_m: ROCK_TRACE_MIN_M
            + (ROCK_TRACE_MAX_M - ROCK_TRACE_MIN_M) * weibull / ROCK_BLOCK_WEIBULL_MAX,
        block,
        group,
        open,
        phase,
        spacing_m: spacing,
    }
}

/// The joints' trace at a world point for a pixel footprint, 0 to 1 (`common.wesl`).
pub(crate) fn rock_joint_at(p: Vec3, footprint_m: f32) -> f32 {
    let j = rock_joints(p);
    let ink = if j.open { 1.0 } else { ROCK_TIGHT_INK };
    let trace1 = rock_trace(j.to_border_m, j.half_width_m, footprint_m) * ink;
    let trace2 = if j.split {
        rock_trace(j.to_plane_m, j.half_width_m, footprint_m)
    } else {
        0.0
    };
    trace1.max(trace2)
}

/// The joints' trace at a world point, seen up close (a footprint of a millimetre).
pub(crate) fn rock_joint(p: Vec3) -> f32 {
    rock_joint_at(p, 0.001)
}

// The beds (`docs/research/bedding.md`)
// ---------------------------------------------------------------------------------------------

/// The nominal spacing between beds, m (`common.wesl`).
pub(crate) const ROCK_BED_M: f32 = 4.5;
/// The bed lattice's warp, in bed units (`common.wesl`).
pub(crate) const BED_WARP_AMP: f32 = 1.0;

/// The bed's structure at the coordinate `bed`: which bed it is, where its bottom is and
/// where its top is (`common.wesl`).
///
/// The bed whose bottom is at or below the coordinate: the lattice is uniform, so the
/// candidate is the coordinate's own floor, and the walk goes down while the bed above still
/// covers the coordinate.
/// The bed lattice's warp, in bed units: how far a boundary wanders from the uniform
/// lattice (`common.wesl`).
pub(crate) fn bed_warp(index: f32) -> f32 {
    let cell = IVec3::new(index as i32, 3, 0);
    let h = crate::gpu::noise_mirror::hash31(cell);
    BED_WARP_AMP
        * (0.60 * (index * 0.21).sin()
            + 0.28 * (index * 0.53 + 1.3).sin()
            + 0.12 * (index * 1.17 + 2.6).sin())
        + 0.10 * (h - 0.5)
}

/// The boundary between the bed whose index is `i` and the next one, in bed units.
pub(crate) fn bed_boundary(index: f32) -> f32 {
    index + bed_warp(index)
}

pub(crate) fn bed_structure(bed: f32) -> (f32, f32, f32) {
    // The bed whose span covers the coordinate. The lattice's boundaries are at
    // `index * mean + warp(index)`, and the bed is the one whose boundary is at or below the
    // coordinate.
    //
    // The estimate is the coordinate's own position in the lattice, and the warp is a bounded
    // displacement, so a couple of steps of the fixed point
    // `i = (bed - warp(i)) / mean` bring it to the right bed. The iteration must be *stable*:
    // taking the floor at the end of it would flip between two beds at a boundary, so the
    // candidate is taken as the *largest* bed whose bottom is at or below the coordinate.
    let mut index = bed;
    for _ in 0..4 {
        index = bed - bed_warp(index);
    }
    // The candidate bed, and the one below it: the coordinate is in one of them, and the one
    // whose bottom is at or below the coordinate and whose top is above it is the bed.
    let mut bed_index = index.floor();
    let mut bottom = bed_boundary(bed_index);
    if bottom > bed {
        // The estimate overshot into the bed above: step down.
        bed_index -= 1.0;
        bottom = bed_boundary(bed_index);
    }
    let top = bed_boundary(bed_index + 1.0);
    (bed_index, bottom, top)
}

// The scree (`docs/research/scree.md`)
// ---------------------------------------------------------------------------------------------

/// The cell network's cell, m (`common.wesl`): the *spacing* of the stones in it.
pub(crate) const SCREE_CELL_M: f32 = 0.45;
/// The spread of the stones' sizes, as the range of the per-cell factor (`common.wesl`).
pub(crate) const SCREE_SIZE_MIN: f32 = 0.35;
pub(crate) const SCREE_SIZE_MAX: f32 = 2.20;
/// How much of the cell the largest stone fills (`common.wesl`).
pub(crate) const SCREE_FILL: f32 = 0.72;
/// The fine network's cell, as a fraction of the coarse one (`common.wesl`).
pub(crate) const SCREE_FINE_SCALE: f32 = 0.38;

/// The scree's band: the slope angles between which the stones lie (`render.wesl`). The scree
/// is at the rock's *foot* — from the soil's own band up to the rock's ramp (which opens at
/// 30°) — and the band is *wide*, because a scree is an apron and a narrow band would trace
/// the hillside's break as a contour line.
pub(crate) const SCREE_BAND_DEG: [f32; 4] = [16.0, 26.0, 31.0, 40.0];

/// The scree's weight at a slope angle: 1 at the rock's foot, 0 on the meadow and 0 on the
/// cliff (`render.wesl`).
pub(crate) fn scree_weight(slope_deg: f32) -> f32 {
    smoothstep(SCREE_BAND_DEG[0], SCREE_BAND_DEG[1], slope_deg)
        * (1.0 - smoothstep(SCREE_BAND_DEG[2], SCREE_BAND_DEG[3], slope_deg))
}

/// One scale's stones: how much of the point the stone covers (`common.wesl`).
pub(crate) fn scree_scale_coverage(xz: Vec2, cell_m: f32, size_min: f32, size_max: f32) -> f32 {
    let cell = crate::gpu::noise_mirror::cell2(xz / cell_m);
    let size = size_min + (size_max - size_min) * cell.factor.powi(3);
    let radius = 0.5 * size * SCREE_FILL;
    1.0 - smoothstep(radius * 0.80, radius, cell.distance)
}

/// The scree's coverage at a point: the coarse network's stone, and the fine network's stone
/// where the coarse one does not cover (`common.wesl`).
pub(crate) fn scree_coverage(xz: Vec2) -> f32 {
    let coarse = scree_scale_coverage(xz, SCREE_CELL_M, SCREE_SIZE_MIN, SCREE_SIZE_MAX);
    let fine = scree_scale_coverage(
        xz,
        SCREE_CELL_M * SCREE_FINE_SCALE,
        SCREE_SIZE_MIN * 0.8,
        SCREE_SIZE_MAX * 0.8,
    );
    coarse.max(fine * (1.0 - coarse))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The beds are **bands and not blotches**: a vertical cut across a cliff crosses a bed
    /// every few metres, and the crossings are *bounded* — a smear has no bound.
    ///
    /// The measurement that started this: the old warp displaced the bed coordinate by up to
    /// 0.78 of a bed, and a 4.5 m grid distorted by a 45 m-scale field is a set of blotches.
    #[test]
    fn the_beds_are_bands_and_not_blotches() {
        // A vertical cut through the rock, 40 m of it, in the bed's own coordinate.
        let mut crossings = 0;
        let mut spacings = Vec::new();
        let mut last = None;
        let mut last_y = 0.0_f32;
        for i in 0..1000 {
            let y = i as f32 * 0.04;
            // The coordinate in bed units: the height along the bed plane, in the nominal
            // spacing, which is the lattice's spacing.
            let bed = (y * 0.90) / ROCK_BED_M;
            // The bed's *index*: the cut crosses a bed's edge where the index changes.
            let (index, _, _) = bed_structure(bed);
            if let Some(was) = last
                && was != index
            {
                crossings += 1;
                // The spacing is the distance between two crossings. The first is measured
                // from the cut's start, which is in the middle of a bed, so it is dropped.
                if crossings > 1 {
                    spacings.push(y - last_y);
                }
                last_y = y;
            }
            last = Some(index);
        }
        assert!(
            crossings >= 4,
            "the beds are bands: {crossings} crossings in 40 m"
        );
        // The spacings are *bounded*: a bed's thickness is the gap between two boundaries, and
        // the warp is bounded, so the distance between two edges is bounded too. A smear — the
        // old warp — has no bound: it moves the coordinate by most of a bed, and the edges
        // land anywhere.
        let longest = spacings.iter().copied().fold(0.0_f32, f32::max);
        let shortest = spacings.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            longest < 3.0 * ROCK_BED_M,
            "the beds' spacing is bounded: the longest is {longest} m"
        );
        assert!(
            shortest > 0.2 * ROCK_BED_M,
            "the beds' spacing is bounded: the shortest is {shortest} m"
        );
    }

    /// The beds **stack**: their thicknesses are a draw, not a constant, and the draw is
    /// heavy-tailed, so a few beds are much thicker than the rest.
    ///
    /// Talling 2001: a bed's thickness follows a log-normal distribution, and a succession's
    /// thicknesses are "the sum of a series of log-normal frequency distributions".
    #[test]
    fn the_beds_thicknesses_vary_and_stack() {
        // The thicknesses are the *gaps* between the lattice's boundaries, and they vary
        // because the warp is a smooth field with a bounded derivative.
        let mut thicknesses = Vec::new();
        for i in 0..2000 {
            thicknesses.push(bed_boundary(i as f32 + 1.0) - bed_boundary(i as f32));
        }
        thicknesses.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p10 = thicknesses[thicknesses.len() / 10];
        let p90 = thicknesses[thicknesses.len() * 9 / 10];
        // The thicknesses span at least 1.25:1 between the tenth and the ninetieth
        // percentile, and none is degenerate.
        assert!(
            p90 > p10 * 1.25,
            "the beds' thicknesses span only {:.2}:1", p90 / p10
        );
        assert!(p10 > 0.35, "the beds' thicknesses are bounded: {p10}");
        // The beds *stack*: the boundaries are increasing, so every bed is above the one below
        // it and the succession is a sequence and not a set of overlapping bands.
        let mut increasing = 0;
        for i in 0..2000 {
            if bed_boundary(i as f32 + 1.0) > bed_boundary(i as f32) {
                increasing += 1;
            }
        }
        assert_eq!(increasing, 2000, "the beds stack");
    }

    /// A piece of the rock: its cell, and its slab where the second set splits its block.
    fn piece(j: &RockJoints) -> (IVec3, i32) {
        (j.block, if j.split { j.phase.floor() as i32 } else { 0 })
    }

    /// An open block of the rock: its group, and its slab where the second set splits it.
    fn open_block(j: &RockJoints) -> (IVec3, i32) {
        (j.group, if j.split { j.phase.floor() as i32 } else { 0 })
    }

    /// A cut across a face, 2 mm apart and 30 m long, oblique to the axes and to both sets.
    fn cut() -> Vec<(f32, RockJoints, f32)> {
        let dir = Vec3::new(0.83, 0.31, 0.46).normalize();
        (0..15_000)
            .map(|i| {
                let s = i as f32 * 0.002;
                let p = Vec3::new(3.0, 12.0, 30.0) + dir * s;
                (s, rock_joints(p), rock_joint(p))
            })
            .collect()
    }

    /// The sample indices along a cut where `key` changes.
    fn changes<K: PartialEq>(
        samples: &[(f32, RockJoints, f32)],
        key: impl Fn(&RockJoints) -> K,
    ) -> Vec<usize> {
        (1..samples.len())
            .filter(|&k| key(&samples[k].1) != key(&samples[k - 1].1))
            .collect()
    }

    /// The changes with no trace above `ink` within 1 cm of them, as distances along the cut.
    fn untraced(samples: &[(f32, RockJoints, f32)], at: &[usize], ink: f32) -> Vec<f32> {
        at.iter()
            .filter(|&&k| {
                let window = &samples[k.saturating_sub(5)..(k + 5).min(samples.len())];
                !window.iter().any(|s| s.2 > ink)
            })
            .map(|&k| samples[k].0)
            .collect()
    }

    /// The rock's pieces meet at **thin traces**, and only there (`docs/research/rock-joints.md`).
    ///
    /// A joint's separation is "mainly less than a millimetre" (Palmström 2015), so its trace is
    /// a hairline at the block's scale; the band a fifth of a block wide it replaced read as
    /// chicken wire. A trace is where one piece meets the next: the old joint was the lattice
    /// cell's wall, which drew lines through pieces and left the meeting of two unmarked. And
    /// the joints are **open** between blocks and **tight** inside one: an open joint is a dark
    /// crack, a tight one a hairline, so the face is blocks and not a patio of equal tiles.
    #[test]
    fn the_pieces_meet_at_thin_traces() {
        let samples = cut();
        let dark = samples.iter().filter(|s| s.2 > 0.5).count();
        let share = dark as f32 / samples.len() as f32;
        assert!(
            share < 0.08,
            "the open traces are lines: {:.1} % of the cut is dark",
            share * 100.0
        );
        let pieces = changes(&samples, piece);
        let blocks = changes(&samples, open_block);
        assert!(
            pieces.len() >= 15 && blocks.len() >= 5 && 2 * pieces.len() > 3 * blocks.len(),
            "the cut crosses blocks, and more pieces than blocks: {} pieces, {} blocks",
            pieces.len(),
            blocks.len()
        );
        // Every change of piece is on a hairline at least, and every change of block on a
        // crack.
        let unmarked = untraced(&samples, &pieces, ROCK_TIGHT_INK * 0.9);
        let uncracked = untraced(&samples, &blocks, 0.5);
        // And every trace holds a change: a run of marked samples is where two pieces meet.
        let mut runs = Vec::new();
        let mut start = None;
        for (k, s) in samples.iter().enumerate() {
            match (s.2 > ROCK_TIGHT_INK * 0.9, start) {
                (true, None) => start = Some(k),
                (false, Some(a)) => {
                    runs.push((a, k));
                    start = None;
                }
                _ => {}
            }
        }
        let empty = runs
            .iter()
            .filter(|(a, b)| !pieces.iter().any(|c| *c >= *a && *c <= *b))
            .count();
        println!(
            "{} pieces and {} blocks, {:.1} % dark, unmarked {unmarked:?}, uncracked \
             {uncracked:?}, {empty} of {} traces with no change",
            pieces.len(),
            blocks.len(),
            share * 100.0,
            runs.len()
        );
        assert!(unmarked.is_empty(), "every piece's edge is a trace: {unmarked:?}");
        assert!(uncracked.is_empty(), "every block's edge is a crack: {uncracked:?}");
        assert!(
            empty * 10 <= runs.len(),
            "a trace is where two pieces meet: {empty} of {} traces cross none",
            runs.len()
        );
    }

    /// The pieces are of **many sizes**, and not all the same: the fracture spacing follows a
    /// Weibull whose geometric standard deviation is about 0.31 (Verdian et al. 2021), and a face
    /// of equal cells is a quilt.
    #[test]
    fn the_blocks_are_of_many_sizes() {
        let samples = cut();
        let mut edges = vec![0.0];
        for k in 1..samples.len() {
            if piece(&samples[k].1) != piece(&samples[k - 1].1) {
                edges.push(samples[k].0);
            }
        }
        let mut sizes: Vec<f32> = edges
            .windows(2)
            .skip(1)
            .map(|w| w[1] - w[0])
            .filter(|s| *s > 0.05)
            .collect();
        assert!(sizes.len() >= 12, "the sizes to measure: {}", sizes.len());
        sizes.sort_by(f32::total_cmp);
        let p10 = sizes[sizes.len() / 10];
        let p90 = sizes[sizes.len() * 9 / 10];
        let ratio = p90 / p10.max(1e-6);
        assert!(
            ratio >= 2.0,
            "the blocks are of many sizes: the p90/p10 ratio is {ratio:.2} over {} pieces \
             ({p10:.2} m to {p90:.2} m)",
            sizes.len()
        );
    }

    /// A far trace keeps its **ink** and fades: past the line's own width the pixel is wider
    /// than the line, so the line covers part of it. The integral of the trace across the line
    /// is the same at every footprint, and its peak falls with the coverage; a trace widened to
    /// the pixel at full darkness drew a ribbon at every distance.
    #[test]
    fn a_far_trace_keeps_its_ink_and_fades() {
        let half_width = 0.02;
        let ink = |footprint: f32| {
            (0..200_000)
                .map(|i| rock_trace(i as f32 * 1e-5, half_width, footprint) * 1e-5)
                .sum::<f32>()
        };
        let near = ink(0.001);
        for footprint in [0.01, 0.04, 0.1, 0.5, 1.0] {
            let far = ink(footprint);
            assert!(
                (far / near - 1.0).abs() < 0.02,
                "the trace's ink at a {footprint} m footprint: {far} against {near}"
            );
        }
        assert_eq!(rock_trace(0.0, half_width, 0.001), 1.0, "a near trace is black");
        let far_peak = rock_trace(0.0, half_width, 8.0 * half_width);
        assert!(
            far_peak <= 0.25 + 1e-6,
            "a trace a quarter of the pixel is a quarter as dark: {far_peak}"
        );
    }

    /// The cross joints are **planar**: along 4 m of a plane its trace leaves the chord by less
    /// than 2 % of the length, Palmström's planar-to-wavy limit (`docs/research/rock-joints.md`).
    /// At iteration 40's wander the planes waved by about a tenth and read as scribble.
    #[test]
    fn the_cross_joints_are_planar() {
        let normal = ROCK_JOINT2_NORMAL.normalize();
        let along = normal.cross(Vec3::Y).normalize();
        let mut worst: f32 = 0.0;
        let mut segments = 0;
        for k in 0..200 {
            let start = Vec3::new(k as f32 * 7.3, 15.0 + (k % 7) as f32 * 3.1, k as f32 * -4.9);
            let first = rock_joints(start);
            let plane = first.phase.round();
            let offsets: Vec<f32> = (0..=40)
                .map(|s| rock_joints(start + along * (s as f32 * 0.1)))
                .take_while(|j| j.spacing_m == first.spacing_m)
                .map(|j| (plane - j.phase) * j.spacing_m)
                .collect();
            if offsets.len() < 41 {
                continue;
            }
            segments += 1;
            let (a, b) = (offsets[0], offsets[40]);
            let deviation = offsets
                .iter()
                .enumerate()
                .map(|(i, o)| (o - (a + (b - a) * i as f32 / 40.0)).abs())
                .fold(0.0, f32::max);
            worst = worst.max(deviation / 4.0);
        }
        println!("{segments} segments, the worst waviness {:.2} %", worst * 100.0);
        assert!(segments >= 100, "the segments to measure: {segments}");
        assert!(
            worst < 0.02,
            "the cross joints are planar: the worst waviness is {:.2} %",
            worst * 100.0
        );
    }

    /// The **second joint set's spacing** varies, and that is what makes the pieces many-sized.
    ///
    /// A block is the intersection of *two* families of planes, so where the second set's
    /// planes are close the pieces are chips and where they are far the pieces are slabs
    /// (`docs/research/block-sizes.md`). The spacing is a *formation's* own draw, so the spread
    /// is *between* formations, and the formations are 46 m apart.
    ///
    /// The measure: the spacing over a run of formations. The spacing's own distribution is a
    /// Weibull (Verdian et al. 2021: the Weibull form "yielded the best fit to the data in
    /// most cases"), so the spread is wide.
    #[test]
    fn the_second_joint_sets_spacing_varies() {
        let mut spacings = Vec::new();
        // The formations are 46 m apart, so the cut crosses many of them.
        for i in 0..400 {
            let p = Vec3::new(i as f32 * 3.1, 18.0 + (i as f32) * 0.91, 0.0);
            spacings.push(rock_joint2_spacing(p));
        }
        let mut sorted = spacings.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p10 = sorted[sorted.len() / 10];
        let p90 = sorted[sorted.len() * 9 / 10];
        let ratio = p90 / p10.max(1e-6);
        assert!(
            ratio >= 2.5,
            "the second set's spacing varies: the p90/p10 ratio is {ratio:.2} over {} \
             formations ({p10:.2} m to {p90:.2} m)",
            spacings.len()
        );
    }

    /// The rock's **palette** does not move the rock's mean colour.
    ///
    /// The rule is the crate's own, from the ground's palette: "the ground is a place and not
    /// a paint colour, while the *mean* colour and the *mean* brightness stay where they were
    /// (a ground that changes colour changes the game's lighting)". A convex combination of
    /// tones that bracket the base cannot drift the mean, and the tones are normalised so
    /// that their equal-weight mean is exactly (1, 1, 1).
    #[test]
    fn the_palette_does_not_move_the_mean() {
        let mut mean = Vec3::ZERO;
        for tone in ROCK_TONES.iter() {
            mean += *tone;
        }
        mean /= 4.0;
        assert!(
            (mean - Vec3::ONE).length() < 1e-3,
            "the palette's equal-weight mean is {mean:?}, not (1, 1, 1)"
        );
    }

    /// The **beds** carry the rock's palette, and the beds of a succession are of *different rock*.
    ///
    /// The measure: the hue of the beds' colours over a run of bed indices. A succession's beds
    /// are a sandstone, a shale, a limestone — different minerals, different colours — and the
    /// crate's bed's tone was a *scalar*, so a distant massif was one grey.
    #[test]
    fn the_beds_are_of_different_rock() {
        // The beds' own hues over a run of indices, and the *region*'s effect on them.
        let mut hues = Vec::new();
        for i in 0..200 {
            let c = bed_palette(i as f32, 0.0);
            let max = c.x.max(c.y).max(c.z);
            let min = c.x.min(c.y).min(c.z);
            let d = max - min;
            if d < 1e-6 {
                continue;
            }
            let h = if max == c.x {
                ((c.y - c.z) / d).rem_euclid(6.0)
            } else if max == c.y {
                (c.z - c.x) / d + 2.0
            } else {
                (c.x - c.y) / d + 4.0
            };
            hues.push(h * 60.0);
        }
        assert!(hues.len() > 150, "the hues to measure: {}", hues.len());
        let mut sorted = hues.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let spread = sorted[sorted.len() - 1] - sorted[0];
        assert!(
            spread > 60.0,
            "the beds are of different rock: the hue's spread is {spread:.1} degrees"
        );
    }

    /// The **region** shifts the beds' colour, so a whole massif is one rock and not a set of
    /// stripes.
    ///
    /// The measure: the *same* bed's colour under two different regions. A range of mountains
    /// is made of *formations*, and a formation is a massif — so the field is at the massif's
    /// scale and the bed's own draw only tints it.
    #[test]
    fn the_region_shifts_the_beds_colour() {
        let mut differ = 0;
        let mut samples = 0;
        for i in 0..200 {
            let a = bed_palette(i as f32, -0.5);
            let b = bed_palette(i as f32, 0.5);
            if (a - b).length() > 0.02 {
                differ += 1;
            }
            samples += 1;
        }
        assert!(
            differ * 100 / samples > 50,
            "the region shifts the beds' colour: {differ} of {samples} beds differ"
        );
    }

    /// The blocks are **tabular**: thin across the bedding and long along it, so a face shows
    /// slabs and not columns (`docs/research/rock-blocks.md` Deviation 10).
    ///
    /// The measure is the blocks crossed per metre along a cut in the bedding's plane and along
    /// its normal. A slab is crossed more often across the beds. An earlier version of this test
    /// compared two directions that both lay in the bedding plane, and it passed while the
    /// lattice made the blocks 2.5 m columns (`docs/research/rock-joints.md` Deviation 5).
    #[test]
    fn the_blocks_are_tabular() {
        let normal = ROCK_BED_TILT.normalize();
        let in_plane = (Vec3::X - normal * normal.x).normalize();
        let starts = [
            Vec3::new(3.0, 12.0, 30.0),
            Vec3::new(-40.0, 25.0, 7.0),
            Vec3::new(18.0, 4.0, -22.0),
        ];
        let crossings = |dir: Vec3| {
            let mut count = 0;
            for start in starts {
                let mut last = rock_joints(start).block;
                for i in 1..10_000 {
                    let block = rock_joints(start + dir * (i as f32 * 0.002)).block;
                    if block != last {
                        count += 1;
                        last = block;
                    }
                }
            }
            count as f32 / 60.0
        };
        let along = crossings(in_plane);
        let across = crossings(normal);
        assert!(
            across > 1.8 * along,
            "the blocks are slabs: {across:.2} blocks per metre across the beds against \
             {along:.2} along them"
        );
    }

    /// The blocks are in the rock's **3D** frame, so they do not weave on a slope.
    ///
    /// The measure: the joint's pattern at two heights on the same vertical line must be
    /// *different* — a 2D network in the ground plane would give the same pattern at both,
    /// which is the weave the crate's rock note warns about.
    #[test]
    fn the_blocks_are_in_the_rocks_own_frame() {
        // The *cell* each point falls in, at two heights on the same vertical line. A 2D
        // network in the ground plane would give the same cell at both heights, which is the
        // weave the crate's rock note warns about; a 3D network gives a different one.
        //
        // The measure is the cell and not the joint: the joint is a *line* a fraction of a
        // cell wide, so two heights are usually both *off* a joint and trivially agree, and
        // the test would pass for a 2D network.
        let mut same = 0;
        for i in 0..200 {
            let x = i as f32 * 0.37;
            let a = rock_block_coords(Vec3::new(x, 10.0, 40.0)).floor();
            let b = rock_block_coords(Vec3::new(x, 14.0, 40.0)).floor();
            if a == b {
                same += 1;
            }
        }
        // A 3D network: the two heights are in different cells, for most of the points. A 2D
        // network in the ground plane would agree on every one.
        assert!(
            same < 100,
            "the blocks are in the rock's 3D frame: {same} of 200 are in the same cell \
             across 4 m of height"
        );
    }

    /// The scree is a *scatter*, not a band: its coverage must be **discrete** — the ground
    /// under the scree is stones and turf between them, not a uniform wash.
    ///
    /// The check is the research note's first validation target: the coverage along a cut
    /// across the scree must have *peaks* — a histogram with a second mode — and not a
    /// monotone gradient. A band's weight is a smooth ramp, so its coverage is smooth too.
    #[test]
    fn the_scree_is_discrete_and_not_a_wash() {
        // A cut across the scree, 40 cm apart, 4 m long.
        let mut coverage = Vec::new();
        for i in 0..200 {
            coverage.push(scree_coverage(Vec2::new(i as f32 * 0.04, 3.0)));
        }
        // The stones: how many times the coverage crosses the middle of its range on its way
        // up. A smooth wash crosses once or twice; a scatter of stones crosses many.
        let mut crossings = 0;
        for w in coverage.windows(2) {
            let a = w[0] > 0.5;
            let b = w[1] > 0.5;
            if a != b {
                crossings += 1;
            }
        }
        assert!(
            crossings >= 6,
            "the scree is a scatter, not a wash: {crossings} crossings in 8 m"
        );
        // And there is turf between the stones: the coverage is below one everywhere, and
        // below the middle over a good part of the cut.
        assert!(
            coverage.iter().all(|c| *c <= 1.0),
            "the coverage never exceeds one"
        );
        let below_half = coverage.iter().filter(|c| **c < 0.5).count();
        assert!(
            below_half * 2 > coverage.len(),
            "the turf shows between the stones: {below_half} of {} below half",
            coverage.len()
        );
    }

    /// The stones have *many* sizes: the note's second validation target, from Verdian et al.
    /// 2021 — a talus runs from 2 mm to 575 mm, and the median moves by more than 10-fold
    /// between sites.
    ///
    /// The measure is the *area* each stone takes: a scatter of one size repeated reads as a
    /// cobble pavement, and a scatter of many sizes reads as scree. The stones' *positions*
    /// are checked too, because a network whose points all landed in one corner of their cells
    /// would be a grid of stones, not a scatter.
    #[test]
    fn the_stones_have_many_sizes() {
        // The coarse network's stones, over a 20 m square, and the area each one covers.
        let mut areas = Vec::new();
        let mut offsets = Vec::new();
        for j in 0..200 {
            for i in 0..200 {
                let p = Vec2::new(i as f32 * 0.1, j as f32 * 0.1);
                let cell = crate::gpu::noise_mirror::cell2(p / SCREE_CELL_M);
                let size = SCREE_SIZE_MIN + (SCREE_SIZE_MAX - SCREE_SIZE_MIN) * cell.factor.powi(3);
                let radius = 0.5 * size * SCREE_FILL;
                areas.push(radius * radius);
                // The point's offset inside its own cell, in [0, 1).
                offsets.push(cell.point.fract());
            }
        }
        areas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p10 = areas[areas.len() / 10];
        let p90 = areas[areas.len() * 9 / 10];
        // The areas span at least 4:1 between the tenth and the ninetieth percentile.
        assert!(
            p90 > p10 * 4.0,
            "the stones' areas span only {:.1}:1", p90 / p10
        );
        // The stones are spread inside their cells: the mean offset is near the middle, and
        // the offsets are not all the same point.
        let mean = offsets.iter().fold(Vec2::ZERO, |a, b| a + *b) / offsets.len() as f32;
        assert!(
            mean.x > 0.35 && mean.x < 0.65 && mean.y > 0.35 && mean.y < 0.65,
            "the stones sit in the middle of their cells on average: {mean:?}"
        );
        let spread = offsets
            .iter()
            .fold(Vec2::ZERO, |a, b| a + (*b - mean).abs())
            / offsets.len() as f32;
        assert!(
            spread.x > 0.2 && spread.y > 0.2,
            "the stones are spread inside their cells: {spread:?}"
        );
    }

    /// The scree is at the rock's **foot**: on the meadow and on the cliff there are no
    /// stones, and the band's weight peaks where the soil's band dies and the rock's ramp
    /// begins (the crate's own layer weights: the soil is gone by 30° and the rock is full at
    /// 38°).
    #[test]
    fn the_scree_lies_at_the_rocks_foot() {
        assert_eq!(scree_weight(0.0), 0.0, "the meadow has no stones");
        assert_eq!(scree_weight(15.0), 0.0, "the meadow has no stones");
        assert_eq!(scree_weight(45.0), 0.0, "the cliff has no stones");
        // The band's weight is strongest just below the rock's ramp.
        let peak = (14..42)
            .map(|d| (d as f32, scree_weight(d as f32)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(d, _)| d)
            .unwrap_or(0.0);
        assert!(
            (22.0..=33.0).contains(&peak),
            "the scree peaks at {peak} degrees, at the rock's foot"
        );
        // And it is on the ground the soil has left and the rock has not taken: at 28° the
        // crate's own layer weights are mostly grass and soil.
        let w = layer_weights(28.0, 0.0, 0.5);
        assert!(w[0] < 0.1, "the rock has not begun at 28 degrees: {:?}", w);
        // The band is an *apron*, not a contour line: it is at least six degrees wide at
        // half its strength, or it traces the hillside's break as a line.
        let wide = (14..42)
            .filter(|d| scree_weight(*d as f32) > 0.5)
            .count();
        assert!(wide >= 6, "the scree's band is {wide} degrees wide, too narrow");
    }

    /// The turf's palette: the four tones' shares must sum to one by construction (so the
    /// mixture is a convex combination and the ground's mean cannot drift), every share must
    /// be in range, and the mixture must stay inside the tones' convex hull — the fresh
    /// grass is the turf's own colour, so a place with no dry tone must be exactly the
    /// identity.
    ///
    /// Three other normalisations were tried and each failed: by the unweighted mean of the
    /// tones the ground's mean brightness drifted 3.2 DN, by the mixture's own colour the hue
    /// was cancelled entirely (the G/R ratio was 1.0 at every field), and by a per-channel
    /// mean the hue was cancelled again. The shares are the fourth way and the only one that
    /// keeps both the hue and the mean.
    #[test]
    fn the_turf_palette_shares_sum_to_one_and_keep_the_mean_and_the_hue() {
        let mut hues = Vec::new();
        for field in (-30..=30).map(|k| k as f32 / 20.0) {
            for clump in (-20..=20).map(|k| k as f32 / 20.0) {
                let (mixed, shares) = turf_palette(field, clump);
                assert!(
                    (shares.iter().sum::<f32>() - 1.0).abs() < 1e-5,
                    "the shares do not sum to one at {field}/{clump}: {shares:?}"
                );
                assert!(
                    shares.iter().all(|s| (0.0..=1.0).contains(s)),
                    "a share is out of range at {field}/{clump}: {shares:?}"
                );
                assert!(
                    mixed.iter().all(|c| (0.5..=2.0).contains(c)),
                    "the mixture left the tones' range at {field}/{clump}: {mixed:?}"
                );
                // The hue must survive: the G/R ratio has to *move* with the field, or the
                // palette is a brightness change and not a colour change.
                hues.push(mixed[1] / mixed[0]);
            }
        }
        let min = hues.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = hues.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        // The ratio's *maximum* is 1.0 — the fresh tone is the identity, so a place with no
        // dry tone is exactly the turf's own hue — and its minimum is the straw's.
        assert!(
            min < 0.95 && max <= 1.0 + 1e-5,
            "the palette's hue does not move: the G/R ratio spans {min} to {max}"
        );
        // A place with no dry tone is exactly the turf's own colour.
        let (mixed, _) = turf_palette(0.0, 0.0);
        for c in mixed {
            assert!((c - 1.0).abs() < 1e-5, "the fresh tone is not the identity: {mixed:?}");
        }
    }

    /// An octave is kept whole while the pixel resolves it, cancelled once it cannot, and the
    /// transition is monotone and smooth; the two scales overlap so no band of the ground's
    /// detail is left uncovered (`ground-detail.md` Validation 1-3).
    #[test]
    fn the_detail_band_limit_keeps_resolved_octaves_and_cancels_the_rest() {
        assert_eq!(detail_keep(DETAIL_COARSE_M, 0.0), 1.0);
        assert_eq!(detail_keep(DETAIL_COARSE_M, DETAIL_COARSE_M / 2.0), 1.0);
        assert_eq!(detail_keep(DETAIL_COARSE_M, DETAIL_COARSE_M), 0.0);
        assert_eq!(detail_keep(DETAIL_COARSE_M, 10.0), 0.0);
        // Monotone in the footprint, and continuous (no popping as the camera pulls back).
        let mut last = 1.0;
        for k in 0..=400 {
            let f = f64::from(k) / 400.0 * f64::from(DETAIL_COARSE_M);
            let w = detail_keep(DETAIL_COARSE_M, f as f32);
            assert!(w <= last + 1e-6, "not monotone at {f}");
            assert!((last - w) < 0.02, "a step of {} at {f}", last - w);
            last = w;
        }
        // The overlap: an octave held whole (half its feature) while the next coarser is gone.
        // The fine scale is 10× the coarse one's frequency, so there is always a band of
        // footprints between them where one has faded and the other has not yet.
        let half_fine = DETAIL_FINE_M * 0.5;
        let half_coarse = DETAIL_COARSE_M * 0.5;
        assert!(detail_keep(DETAIL_FINE_M, half_fine) > 0.99);
        assert_eq!(detail_keep(DETAIL_COARSE_M, half_coarse), 1.0);
        assert!(half_fine < half_coarse);
    }

    /// FNV-1a over the bytes of `values`.
    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    /// The statistics of one preset at 1024² cells of 2 m: (spread of `h − h_base` between the
    /// 0.5 and 99.5 percentiles, slope p50, p95, share above 40° in percent).
    fn stats(p: &ProceduralTerrain) -> (f32, f32, f32, f32) {
        let size = UVec2::splat(1024);
        let surface = generate_surface(p, size, 2.0);
        let mut heights: Vec<f32> = surface.iter().map(|h| h - p.base_height_m).collect();
        let spread = percentile(&mut heights, 0.995) - percentile(&mut heights, 0.005);
        let mut slopes: Vec<f32> = (0..size.y)
            .flat_map(|j| (0..size.x).map(move |i| (i, j)))
            .map(|(i, j)| slope_deg(&surface, size, 2.0, i, j))
            .collect();
        let steep = slopes.iter().filter(|s| **s > 40.0).count() as f32 / slopes.len() as f32;
        (
            spread,
            percentile(&mut slopes, 0.5),
            percentile(&mut slopes, 0.95),
            steep * 100.0,
        )
    }

    /// Validation 1: the two noise sums on this crate's noise land on the constants the unit
    /// maps assume. The percentiles are the stationary ones (pooled over seeds): one seed's
    /// 2048 m tile of a 600 m feature has only 3.4 base-octave cells, so its own percentiles
    /// scatter by ±0.08 (seed 42: f −0.347 / 0.256).
    #[test]
    fn the_noise_sums_fill_their_unit_ranges() {
        let (mut f, mut r) = (Vec::new(), Vec::new());
        for seed in 0..32u64 {
            for j in 0..96 {
                for i in 0..96 {
                    let p = Vec2::new(i as f32, j as f32) * 0.31 + Vec2::splat(0.1);
                    let (a, b) = noise_sums(p, 1.0, 8, seed * 7 + 1);
                    f.push(a);
                    r.push(b);
                }
            }
        }
        let f_lo = percentile(&mut f, 0.005);
        let f_hi = percentile(&mut f, 0.995);
        let r_lo = percentile(&mut r, 0.005);
        let r_hi = percentile(&mut r, 0.995);
        println!("f {f_lo:.3} {f_hi:.3}  r {r_lo:.3} {r_hi:.3}");
        assert!(
            (f_lo + F_HALF).abs() < 0.03 && (f_hi - F_HALF).abs() < 0.03,
            "f {f_lo} {f_hi}"
        );
        assert!(
            (r_lo - R_LOW).abs() < 0.03 && (r_hi - (R_LOW + R_SPAN)).abs() < 0.03,
            "r {r_lo} {r_hi}"
        );
    }

    /// Validation 1: per preset, the percentile spread of the heights, the slope percentiles and
    /// the share of rock-steep ground, re-blessed once on this crate's noise (Deviation 11; the
    /// note's numpy port, a 2D gradient noise, is about 30 % steeper).
    #[test]
    fn the_presets_have_the_measured_statistics() {
        // (preset, spread m, slope p50, slope p95, share above 40° in %), seed 42, 2 m cells.
        let table = [
            (ProceduralTerrain::hills(42), 91.8, 12.4, 25.5, 0.0),
            (ProceduralTerrain::mountains(42), 549.5, 41.1, 61.9, 52.6),
            (ProceduralTerrain::dunes(42), 39.9, 9.1, 18.6, 0.0),
            (ProceduralTerrain::plains(42), 13.7, 1.4, 2.9, 0.0),
        ];
        let mut misses = Vec::new();
        for (preset, spread, p50, p95, steep) in table {
            let (s, m, h, e) = stats(&preset);
            println!(
                "R {} F {}: spread {s:.1} (want {spread}) slope p50 {m:.1} ({p50}) p95 {h:.1} ({p95}) >40° {e:.2} % ({steep})",
                preset.relief_m, preset.feature_size_m
            );
            if (s / spread - 1.0).abs() >= 0.10
                || (m / p50 - 1.0).abs() >= 0.15
                || (h / p95 - 1.0).abs() >= 0.15
                || (e - steep).abs() >= 3.0
            {
                misses.push(format!("R {} F {}", preset.relief_m, preset.feature_size_m));
            }
        }
        assert!(misses.is_empty(), "{misses:?}");
    }

    /// Validation 2: the generator is frozen on the CPU mirror.
    #[test]
    fn the_terrain_is_frozen() {
        let surface = generate_surface(&ProceduralTerrain::hills(42), UVec2::splat(256), 8.0);
        let mm: Vec<i32> = surface
            .iter()
            .map(|h| (h * 1000.0).round() as i32)
            .collect();
        let hash = fnv1a(bytemuck::cast_slice(&mm));
        println!("the_terrain_is_frozen: {hash:#018x}");
        assert_eq!(
            hash, GOLDEN_HILLS_256,
            "the generator changed: re-bless once, deliberately"
        );
    }

    const GOLDEN_HILLS_256: u64 = 0x6f04_892c_64bc_b5eb;

    /// Validation 7: the layer weights sum to one for any detail, pick the plan's layers without
    /// detail, and the detail never moves a layer that is fully on or off.
    #[test]
    fn the_layer_weights_sum_to_one_and_pick_the_right_layer() {
        for a in (0..90).step_by(1) {
            for s in [0.0, 0.2, 0.4, 0.5, 0.6, 0.8, 1.0, 4.0] {
                for d in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let w = layer_weights(a as f32, s, d);
                    assert!(
                        (w.iter().sum::<f32>() - 1.0).abs() < 1e-6,
                        "{a}° {s} m detail {d}: {w:?}"
                    );
                    assert!(w.iter().all(|x| (0.0..=1.0).contains(x)));
                }
            }
        }
        assert!(layer_weights(45.0, 0.0, 0.5)[0] >= 0.99);
        assert!(layer_weights(20.0, 1.0, 0.5)[1] >= 0.99);
        assert_eq!(layer_weights(10.0, 0.0, 0.5)[3], 1.0);
        // The soil band is 20° to 30°, tapering from 26° as the rock's ramp opens, and it must
        // sit *below* the rock's ramp (30° to 38°): where the two overlap, a mix of a warm
        // brown and a cool grey is a pink patch, and the ground showed them on the hills
        // (measured: 0.32 % of the frame).
        let soil = layer_weights(21.0, 0.0, 0.5)[2];
        assert!((0.3..0.7).contains(&soil), "soil at 21°: {soil}");
        assert!(layer_weights(31.0, 0.0, 0.5)[2] < 0.4, "the soil must be gone by 31°");
        // The taper: the soil thins as the rock rises, so the grass meets the stone without a
        // warm brown rim (measured: 1415 orange pixels at RGB 148, 121, 100 in one frame, and
        // none after).
        assert!(layer_weights(28.0, 0.0, 0.5)[2] < 0.35, "the soil must taper into the rock");
        // Fully on or off stays put under any detail: rock at 50°, grass at 10°.
        for d in [0.0, 0.3, 0.7, 1.0] {
            assert_eq!(layer_weights(50.0, 0.0, d)[0], 1.0);
            assert_eq!(layer_weights(10.0, 0.0, d)[3], 1.0);
        }
        // Inside a transition the detail spans the whole range. The rock's ramp is 30° to 38°,
        // so 34° is its middle: there the detail takes the rock from 0 to 1.
        assert_eq!(layer_weights(34.0, 0.0, 0.0)[0], 0.0);
        assert_eq!(layer_weights(34.0, 0.0, 1.0)[0], 1.0);
        // The rock's ramp starts at 30°, so a slope is already half rock at 34°: the ground's
        // turf is the crate's 2D noise in the world XZ plane, and it must be gone before the
        // surface is steep enough to weave it (`rock-material.md` Model 1).
        assert!(layer_weights(34.0, 0.0, 0.5)[0] > 0.3, "the rock must be there by 34°");
        // The ramp is narrow on purpose: a wide one has the rock's *partial* weight mixing
        // with the soil's, and a warm brown mixed with a cool grey is a pink patch (measured:
        // 9.67 % of the mountain's frame was pink before, 0.01 % after).
        assert!(layer_weights(31.0, 0.0, 0.5)[0] < 0.2, "the rock must not creep below 30°");
    }

    /// The ground's large-scale variation must reach a **plain**, where the soil's weight is
    /// zero and the grass's is one. Moving the two weights against each other (each by the
    /// other's weight) does nothing there, and the plain stayed one colour; the shift has to
    /// be taken from the grass and given to the soil. The weights must still sum to one and
    /// stay in range, or the ground's mean colour drifts (`rock-material.md` Model 4).
    #[test]
    fn the_layer_variation_reaches_a_plain_and_keeps_the_weights_summing_to_one() {
        let plain = layer_weights(4.0, 0.0, 0.5);
        assert_eq!(plain[2], 0.0, "a plain has no soil to start with");
        assert_eq!(plain[3], 1.0);
        // Walk a line across the field: the soil must appear, the grass must give it up, and
        // the pair must still sum to one.
        let mut max_soil = 0.0f32;
        let mut min_soil = 1.0f32;
        for k in 0..200 {
            let p = Vec3::new(k as f32 * 3.0, 0.0, 0.0);
            let field = crate::gpu::noise_mirror::perlin3_d(
                p / LAYER_VARIATION_M + crate::gpu::noise_mirror::seed_offset(71),
            )
            .x
                / NOISE_RMS;
            let w = layer_variation(field, plain);
            assert!(
                (w.iter().sum::<f32>() - 1.0).abs() < 1e-5,
                "the weights do not sum to one at {p:?}: {w:?}"
            );
            assert!(w.iter().all(|x| (0.0..=1.0).contains(x)), "{w:?}");
            max_soil = max_soil.max(w[2]);
            min_soil = min_soil.min(w[2]);
        }
        assert!(
            max_soil > 0.05,
            "the variation never reaches a plain (the most soil was {max_soil})"
        );
        assert!(
            min_soil < 0.02,
            "the variation never gives the soil back (the least soil was {min_soil})"
        );
        // The rock and the sand are where the terrain's shape put them, and the variation must
        // not move them.
        let steep = layer_weights(50.0, 0.0, 0.5);
        let field = crate::gpu::noise_mirror::perlin3_d(
            Vec3::new(30.0, 0.0, 0.0) / LAYER_VARIATION_M + crate::gpu::noise_mirror::seed_offset(71),
        )
        .x
            / NOISE_RMS;
        let w = layer_variation(field, steep);
        assert_eq!(w[0], 1.0, "the variation moved the rock");
    }

    /// Validation 5: on the plane `h = 0.5 x` the central-difference slope is atan(0.5) at every
    /// cell, edges included, and a flat field has slope 0.
    #[test]
    fn the_slope_of_a_plane_is_exact_up_to_the_edge() {
        let size = UVec2::new(16, 8);
        let plane: Vec<f32> = (0..size.y)
            .flat_map(|_| (0..size.x).map(|i| 0.5 * (i as f32 + 0.5) * 2.0))
            .collect();
        for j in 0..size.y {
            for i in 0..size.x {
                let a = slope_deg(&plane, size, 2.0, i, j);
                assert!((a - 26.565).abs() < 0.01, "cell {i},{j}: {a}");
            }
        }
        assert_eq!(slope_deg(&vec![3.0; 128], size, 2.0, 0, 0), 0.0);
    }
}
