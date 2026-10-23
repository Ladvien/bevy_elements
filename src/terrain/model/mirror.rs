//! CPU transliterations of the terrain kernels that the tests compare against: the procedural
//! generator of `generate.wesl` and the layer weights of `terrain.wesl`.

use bevy::math::{IVec2, IVec3, UVec2, Vec2, Vec3};

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

// The cliff's joint sets (`docs/research/joint-sets.md`)
// ---------------------------------------------------------------------------------------------

/// The bedding plane's normal, unnormalised (`common.wesl`).
pub(crate) const ROCK_BED_TILT: Vec3 = Vec3::new(0.32, 0.90, 0.28);
/// A bed's nominal layer thickness, m, log-uniform per bed (`common.wesl`).
pub(crate) const ROCK_LAYER_MIN_M: f32 = 0.35;
pub(crate) const ROCK_LAYER_MAX_M: f32 = 2.2;
/// How far a parting inside a bed wanders, as a fraction of a layer (`common.wesl`).
pub(crate) const ROCK_PARTING_JITTER: f32 = 0.8;
/// The systematic joints' spacing over the layer's thickness, log-uniform per bed
/// (`common.wesl`).
pub(crate) const ROCK_SPACING_RATIO_MIN: f32 = 0.8;
pub(crate) const ROCK_SPACING_RATIO_MAX: f32 = 3.0;
/// How far a joint wanders from even spacing, as a fraction of the spacing (`common.wesl`).
pub(crate) const ROCK_JOINT_JITTER: f32 = 0.8;
/// The systematic joints' strike (`common.wesl`).
pub(crate) const ROCK_JOINT_STRIKE: Vec3 = Vec3::new(0.8, 0.0, 0.6);
/// How far a layer's systematic joints swing from the strike, rad (`common.wesl`).
pub(crate) const ROCK_JOINT_SWING: f32 = 0.2;
/// The cross joints' spacing over the systematic joints' (`common.wesl`).
pub(crate) const ROCK_CROSS_RATIO: f32 = 1.5;
/// The chance an interval holds cross joints, narrow to wide (`common.wesl`).
pub(crate) const ROCK_CROSS_NARROW: f32 = 0.85;
pub(crate) const ROCK_CROSS_WIDE: f32 = 0.3;
/// A joint's trace half-width, m (`common.wesl`).
pub(crate) const ROCK_TRACE_MIN_M: f32 = 0.012;
pub(crate) const ROCK_TRACE_MAX_M: f32 = 0.030;
/// A parting's trace against a joint's (`common.wesl`).
pub(crate) const ROCK_PARTING_WIDEN: f32 = 1.5;
/// The Weibull's shape and ceiling for the trace's width (`common.wesl`).
pub(crate) const ROCK_BLOCK_WEIBULL: f32 = 1.6;
pub(crate) const ROCK_BLOCK_WEIBULL_MAX: f32 = 2.0;
/// The joints' roughness: its scale and its amplitude, m (`common.wesl`).
pub(crate) const ROCK_ROUGH_M: f32 = 0.6;
pub(crate) const ROCK_ROUGH_AMP_M: f32 = 0.03;

/// A cell of a one-dimensional jittered partition (`common.wesl`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Partition {
    pub index: f32,
    pub low: f32,
    pub high: f32,
}

/// The boundary `k` of a jittered partition (`partition_boundary` in `common.wesl`).
fn partition_boundary(k: f32, seed: IVec2, jitter: f32, first: f32, last: f32) -> f32 {
    if k <= first || k >= last {
        return k;
    }
    k + jitter * (crate::gpu::noise_mirror::hash31(IVec3::new(k as i32, seed.x, seed.y)) - 0.5)
}

/// The cell of a jittered partition that holds `x` (`jittered_partition` in `common.wesl`).
pub(crate) fn jittered_partition(x: f32, seed: IVec2, jitter: f32, first: f32, last: f32) -> Partition {
    let i = x.floor();
    let b0 = partition_boundary(i, seed, jitter, first, last);
    if x < b0 {
        let low = partition_boundary(i - 1.0, seed, jitter, first, last);
        return Partition { index: i - 1.0, low, high: b0 };
    }
    let b1 = partition_boundary(i + 1.0, seed, jitter, first, last);
    if x >= b1 {
        let high = partition_boundary(i + 2.0, seed, jitter, first, last);
        return Partition { index: i + 1.0, low: b1, high };
    }
    Partition { index: i, low: b0, high: b1 }
}

/// A joint's trace, anti-aliased by its coverage of a pixel (`rock_trace` in `common.wesl`).
pub(crate) fn rock_trace(distance_m: f32, half_width_m: f32, footprint_m: f32) -> f32 {
    let soft = half_width_m.max(2.0 * footprint_m);
    let coverage = half_width_m / soft.max(1e-6);
    (1.0 - smoothstep(0.0, soft, distance_m)) * coverage
}

/// The bed coordinate of a world point with no warp (`rock_material` in `common.wesl`).
pub(crate) fn rock_bed_coordinate(p: Vec3) -> f32 {
    p.dot(ROCK_BED_TILT) / ROCK_BED_M
}

/// The joints at a world point (`rock_blocks` in `common.wesl`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct RockJoints {
    /// The distances to the nearest parting, systematic joint and cross joint, m (the cross
    /// joint's is huge where the interval holds none).
    pub to_parting_m: f32,
    pub to_a_m: f32,
    pub to_b_m: f32,
    /// The traces' half-width in this block, m.
    pub half_width_m: f32,
    /// The layer's thickness, the systematic joints' spacing and this interval's width, m.
    pub thickness_m: f32,
    pub spacing_a_m: f32,
    pub width_a_m: f32,
    /// Whether this interval holds cross joints.
    pub crossed: bool,
    /// The bed, the layer in it, the systematic interval and the cross interval (0 where the
    /// interval holds none): the block's identity.
    pub bed: i32,
    pub layer: i32,
    pub index_a: i32,
    pub index_b: i32,
}

/// The normals of a layer's systematic and cross joints (`rock_blocks` in `common.wesl`): the
/// set's strike in the bedding plane, swung by the layer's own small draw.
pub(crate) fn rock_layer_normals(bed: i32, layer: i32) -> (Vec3, Vec3) {
    let normal = ROCK_BED_TILT.normalize();
    let layer_seed = bed.wrapping_mul(64).wrapping_add(layer);
    let strike = (ROCK_JOINT_STRIKE - normal * ROCK_JOINT_STRIKE.dot(normal)).normalize();
    let swing = (crate::gpu::noise_mirror::hash31(IVec3::new(layer_seed, 29, 0)) - 0.5)
        * ROCK_JOINT_SWING;
    let normal_a = (strike + normal.cross(strike) * swing).normalize();
    (normal_a, normal.cross(normal_a))
}

/// The joints at `p`, in the bed coordinate `bed` (`rock_blocks` in `common.wesl`).
pub(crate) fn rock_joints(p: Vec3, bed: f32) -> RockJoints {
    let hash31 = crate::gpu::noise_mirror::hash31;
    let rough = crate::gpu::noise_mirror::perlin3_d(
        p / ROCK_ROUGH_M + crate::gpu::noise_mirror::seed_offset(139),
    );
    let (bed_index, bottom, top) = bed_structure(bed);
    let bed_id = bed_index as i32;
    let bed_m = (top - bottom) * ROCK_BED_M;
    let style = hash31(IVec3::new(bed_id, 17, 0));
    let nominal = (ROCK_LAYER_MIN_M.ln()
        + (ROCK_LAYER_MAX_M.ln() - ROCK_LAYER_MIN_M.ln()) * style)
        .exp();
    let layers = (bed_m / nominal).round().max(1.0);
    let layer_m = bed_m / layers;
    let u_flat = ((bed - bottom) / (top - bottom)).clamp(0.0, 0.99999) * layers;
    let inner = u_flat.min(layers - u_flat).clamp(0.0, 1.0);
    let u = u_flat + rough.w * ROCK_ROUGH_AMP_M / layer_m * inner;
    let layer = jittered_partition(u, IVec2::new(bed_id, 19), ROCK_PARTING_JITTER, 0.0, layers);
    let thickness = (layer.high - layer.low) * layer_m;
    let to_parting = ((u - layer.low) * layer_m).min((layer.high - u) * layer_m);
    let layer_id = layer.index as i32;
    let layer_seed = bed_id.wrapping_mul(64).wrapping_add(layer_id);
    let draw_ratio = hash31(IVec3::new(bed_id, 23, 0));
    let ratio = (ROCK_SPACING_RATIO_MIN.ln()
        + (ROCK_SPACING_RATIO_MAX.ln() - ROCK_SPACING_RATIO_MIN.ln()) * draw_ratio)
        .exp();
    let spacing_a = ratio * thickness;
    let (normal_a, normal_b) = rock_layer_normals(bed_id, layer_id);
    let a = (p.dot(normal_a) + rough.y * ROCK_ROUGH_AMP_M) / spacing_a
        + 17.0 * hash31(IVec3::new(layer_seed, 31, 0));
    let cell_a = jittered_partition(a, IVec2::new(layer_seed, 37), ROCK_JOINT_JITTER, -1e9, 1e9);
    let to_a = ((a - cell_a.low) * spacing_a).min((cell_a.high - a) * spacing_a);
    let index_a = cell_a.index as i32;
    let width_a = (cell_a.high - cell_a.low) * spacing_a;
    let chance = ROCK_CROSS_NARROW
        + (ROCK_CROSS_WIDE - ROCK_CROSS_NARROW) * smoothstep(1.0, 3.0, width_a / thickness);
    let crossed = hash31(IVec3::new(index_a, layer_seed, 41)) < chance;
    let (to_b, index_b) = if crossed {
        let spacing_b = ROCK_CROSS_RATIO * spacing_a;
        let b = (p.dot(normal_b) + rough.z * ROCK_ROUGH_AMP_M) / spacing_b
            + 17.0 * hash31(IVec3::new(index_a, layer_seed, 43));
        let interval_seed = layer_seed.wrapping_mul(4099).wrapping_add(index_a);
        let cell_b =
            jittered_partition(b, IVec2::new(interval_seed, 47), ROCK_JOINT_JITTER, -1e9, 1e9);
        let to_b = ((b - cell_b.low) * spacing_b).min((cell_b.high - b) * spacing_b);
        (to_b, cell_b.index as i32 + 1)
    } else {
        (1e9, 0)
    };
    let block_x = index_a.wrapping_mul(131).wrapping_add(index_b);
    let draw2 = crate::gpu::noise_mirror::hash33(IVec3::new(block_x, layer_seed, 59));
    let weibull = (-(1.0 - draw2.x).max(1e-6).ln())
        .powf(1.0 / ROCK_BLOCK_WEIBULL)
        .min(ROCK_BLOCK_WEIBULL_MAX);
    RockJoints {
        to_parting_m: to_parting,
        to_a_m: to_a,
        to_b_m: to_b,
        half_width_m: (ROCK_TRACE_MIN_M
            + (ROCK_TRACE_MAX_M - ROCK_TRACE_MIN_M) * weibull / ROCK_BLOCK_WEIBULL_MAX)
            * (0.85 + 0.6 * rough.x).clamp(0.4, 1.3),
        thickness_m: thickness,
        spacing_a_m: spacing_a,
        width_a_m: width_a,
        crossed,
        bed: bed_id,
        layer: layer_id,
        index_a,
        index_b,
    }
}

/// The joints' trace at `p` for a pixel footprint, 0 to 1 (`rock_blocks` in `common.wesl`).
pub(crate) fn rock_joint_at(p: Vec3, bed: f32, footprint_m: f32) -> f32 {
    let j = rock_joints(p, bed);
    rock_trace(j.to_parting_m, j.half_width_m * ROCK_PARTING_WIDEN, footprint_m)
        .max(rock_trace(j.to_a_m, j.half_width_m, footprint_m))
        .max(rock_trace(j.to_b_m, j.half_width_m, footprint_m))
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
    // The candidate bed, and its neighbours: the coordinate is in one of them, and the one
    // whose bottom is at or below the coordinate and whose top is above it is the bed.
    let mut bed_index = index.floor();
    let mut bottom = bed_boundary(bed_index);
    let mut top = bed_boundary(bed_index + 1.0);
    if bottom > bed {
        // The estimate overshot into the bed above: step down.
        bed_index -= 1.0;
        top = bottom;
        bottom = bed_boundary(bed_index);
    } else if bed >= top {
        // The estimate undershot: the coordinate is past the bed's top, in the bed above.
        // Without this step the bed changed up to a third of a metre past its own boundary.
        bed_index += 1.0;
        bottom = top;
        top = bed_boundary(bed_index + 1.0);
    }
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

    /// The joints at a world point, in its unwarped bed coordinate.
    fn joints(p: Vec3) -> RockJoints {
        rock_joints(p, rock_bed_coordinate(p))
    }

    /// A block's identity: its bed, its layer, its systematic interval and its cross interval.
    fn block(j: &RockJoints) -> (i32, i32, i32, i32) {
        (j.bed, j.layer, j.index_a, j.index_b)
    }

    /// A deterministic spread of points over a few hundred metres of rock.
    fn spread(i: usize) -> Vec3 {
        let i = i as f32;
        Vec3::new(
            (i * 0.731) % 300.0,
            (i * 0.113) % 80.0,
            -((i * 0.417) % 300.0),
        )
    }

    /// The distances along `dir` from `origin`, over `length_m`, at which `key` changes,
    /// counting only changes between two samples that both pass `keep`: a probe beside a rough
    /// parting or joint wanders across it, and the samples on the far side belong to another
    /// layer or interval.
    fn changes_along<K: PartialEq>(
        origin: Vec3,
        dir: Vec3,
        length_m: f32,
        keep: impl Fn(&RockJoints) -> bool,
        key: impl Fn(&RockJoints) -> K,
    ) -> Vec<f32> {
        let steps = (length_m / 0.002) as usize;
        let mut last: Option<K> = None;
        let mut out = Vec::new();
        for s in 0..steps {
            let d = s as f32 * 0.002;
            let j = joints(origin + dir * d);
            if !keep(&j) {
                last = None;
                continue;
            }
            let now = key(&j);
            if last.as_ref().is_some_and(|l| *l != now) {
                out.push(d);
            }
            last = Some(now);
        }
        out
    }

    /// The mean distance from each of `lower`'s positions to the nearest of `upper`'s, as a
    /// share of `spacing_m`: 0 where the two sets of joints line up, a quarter where they are
    /// independent.
    fn mean_offset(lower: &[f32], upper: &[f32], spacing_m: f32) -> Option<f32> {
        if lower.is_empty() || upper.is_empty() {
            return None;
        }
        let total: f32 = lower
            .iter()
            .map(|c| upper.iter().map(|u| (c - u).abs()).fold(f32::MAX, f32::min))
            .map(|d| (d / spacing_m).min(0.5))
            .sum();
        Some(total / lower.len() as f32)
    }

    /// The beds are split into **layers** by partings, the bed's own boundaries among them, and
    /// each bed draws its own layer thickness, so a succession mixes thin flaggy beds and
    /// massive ones (`docs/research/joint-sets.md` Validation 1).
    #[test]
    fn the_partings_split_the_beds_into_layers_of_many_thicknesses() {
        // Up the bed coordinate, a millimetre at a time over 200 m of rock.
        let step = 0.001 / ROCK_BED_M;
        let mut last = rock_joints(Vec3::ZERO, 0.0);
        let mut thicknesses = vec![last.thickness_m];
        let mut misses = Vec::new();
        for i in 1..200_000 {
            let bed = i as f32 * step;
            let j = rock_joints(Vec3::ZERO, bed);
            // A bed boundary: a parting is here, not a jittered one nearby.
            if j.bed != last.bed && j.to_parting_m > 0.003 {
                misses.push((bed * ROCK_BED_M, j.to_parting_m));
            }
            if (j.bed, j.layer) != (last.bed, last.layer) {
                thicknesses.push(j.thickness_m);
            }
            last = j;
        }
        assert!(misses.is_empty(), "every bed boundary is a parting: {misses:?}");
        thicknesses.sort_by(f32::total_cmp);
        let p10 = thicknesses[thicknesses.len() / 10];
        let p90 = thicknesses[thicknesses.len() * 9 / 10];
        println!("{} layers, p10 {p10:.2} m, p90 {p90:.2} m", thicknesses.len());
        assert!(thicknesses.len() > 100, "the layers to measure: {}", thicknesses.len());
        assert!(
            p90 / p10 >= 3.0,
            "the layers are of many thicknesses: p10 {p10:.2} m, p90 {p90:.2} m"
        );
    }

    /// The systematic joints are spaced **in proportion to their layer's thickness**: "spacings
    /// linearly related to the thickness of the fractured layer" (Bai & Pollard 2000), so a
    /// thin layer is cut into small blocks and a thick one into big ones (Validation 2).
    #[test]
    fn the_systematic_joints_scale_with_the_layer() {
        let mut pairs: Vec<(f32, f32)> = (0..4000)
            .map(|i| {
                let j = joints(spread(i));
                (j.thickness_m, j.spacing_a_m)
            })
            .collect();
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let median = |mut v: Vec<f32>| {
            v.sort_by(f32::total_cmp);
            v[v.len() / 2]
        };
        let quarter = pairs.len() / 4;
        let thin = median(pairs[..quarter].iter().map(|p| p.1).collect());
        let thick = median(pairs[pairs.len() - quarter..].iter().map(|p| p.1).collect());
        let ratio = median(pairs.iter().map(|p| p.1 / p.0).collect());
        println!("spacing: thinnest quarter {thin:.2} m, thickest {thick:.2} m; S/T {ratio:.2}");
        assert!(
            thick > 2.0 * thin,
            "the spacing follows the layer: {thin:.2} m in the thinnest layers, {thick:.2} m in \
             the thickest"
        );
        assert!(
            (1.2..=2.0).contains(&ratio),
            "the median spacing over thickness is {ratio:.2}"
        );
    }

    /// The systematic joints **end at the partings**: each layer has its own offset, so the
    /// joints above a parting do not continue the ones below it — the stagger that makes a
    /// bedded cliff a wall of blocks and not a grid (Narr & Suppe 1991, after Bai 2000;
    /// Validation 3).
    #[test]
    fn the_systematic_joints_end_at_the_partings() {
        let normal = ROCK_BED_TILT.normalize();
        let mut offsets = Vec::new();
        for k in 0..30 {
            // Up from a start until the layer changes: a point on a parting.
            let start = spread(k * 97);
            let first = joints(start);
            let mut q = start;
            while (joints(q).bed, joints(q).layer) == (first.bed, first.layer) {
                q += normal * 0.002;
            }
            let below = q - normal * 0.01;
            let above = q + normal * 0.01;
            let (dir, _) = rock_layer_normals(first.bed, first.layer);
            let upper = joints(above);
            let (upper_a, _) = rock_layer_normals(upper.bed, upper.layer);
            // Across the joints, along the lower layer's own normal, 6 m each way.
            let lower_layer = (first.bed, first.layer);
            let upper_layer = (upper.bed, upper.layer);
            let lower = changes_along(
                below - dir * 6.0,
                dir,
                12.0,
                |j| (j.bed, j.layer) == lower_layer,
                |j| j.index_a,
            );
            let upper_cross = changes_along(
                above - dir * 6.0,
                dir,
                12.0,
                |j| (j.bed, j.layer) == upper_layer,
                |j| j.index_a,
            );
            let spacing = upper.spacing_a_m / upper_a.dot(dir).abs().max(0.5);
            if let Some(m) = mean_offset(&lower, &upper_cross, spacing) {
                offsets.push(m);
            }
        }
        let mean = offsets.iter().sum::<f32>() / offsets.len().max(1) as f32;
        println!("{} partings, mean offset across them {mean:.3} of a spacing", offsets.len());
        assert!(offsets.len() >= 20, "the partings to measure: {}", offsets.len());
        assert!(
            mean > 0.15,
            "the joints end at the partings: their mean offset across one is {mean:.3} of a \
             spacing (0 if they continued, 0.25 if independent)"
        );
    }

    /// The cross joints **abut** the systematic joints: they lie between two of them and do not
    /// line up across one — the ladder (Bai 2000) — and a narrow interval holds them more
    /// often than a wide one (Engelder & Gross 1993, after Bai 2000; Validation 4).
    #[test]
    fn the_cross_joints_abut_the_systematic_joints() {
        let (mut narrow, mut narrow_crossed, mut wide, mut wide_crossed) = (0, 0, 0, 0);
        for i in 0..20_000 {
            let j = joints(spread(i));
            let width = j.width_a_m / j.thickness_m;
            if width < 1.2 {
                narrow += 1;
                narrow_crossed += usize::from(j.crossed);
            } else if width > 2.4 {
                wide += 1;
                wide_crossed += usize::from(j.crossed);
            }
        }
        let narrow_share = narrow_crossed as f32 / narrow.max(1) as f32;
        let wide_share = wide_crossed as f32 / wide.max(1) as f32;
        println!("crossed: {narrow_share:.2} of {narrow} narrow, {wide_share:.2} of {wide} wide");
        assert!(narrow > 500 && wide > 500, "the intervals to measure: {narrow}, {wide}");
        assert!(
            narrow_share > wide_share + 0.25,
            "cross joints favour the narrow intervals: {narrow_share:.2} against {wide_share:.2}"
        );
        // Across a systematic joint, the cross joints of the two intervals do not line up.
        let mut offsets = Vec::new();
        for k in 0..400 {
            let start = spread(k * 31);
            let j = joints(start);
            let (normal_a, normal_b) = rock_layer_normals(j.bed, j.layer);
            // To the next systematic joint, then just past it on either side.
            let layer = (j.bed, j.layer);
            let edges =
                changes_along(start, normal_a, 8.0, |j| (j.bed, j.layer) == layer, |j| j.index_a);
            let Some(edge) = edges.first() else { continue };
            let near = start + normal_a * (edge - 0.01);
            let far = start + normal_a * (edge + 0.01);
            let (a, b) = (joints(near), joints(far));
            let same_layer = (a.bed, a.layer) == (b.bed, b.layer);
            if !(a.crossed && b.crossed) || !same_layer || a.index_a == b.index_a {
                continue;
            }
            let left_interval = (a.bed, a.layer, a.index_a);
            let right_interval = (b.bed, b.layer, b.index_a);
            let left = changes_along(
                near - normal_b * 4.0,
                normal_b,
                8.0,
                |j| (j.bed, j.layer, j.index_a) == left_interval,
                |j| j.index_b,
            );
            let right = changes_along(
                far - normal_b * 4.0,
                normal_b,
                8.0,
                |j| (j.bed, j.layer, j.index_a) == right_interval,
                |j| j.index_b,
            );
            if let Some(m) = mean_offset(&left, &right, ROCK_CROSS_RATIO * a.spacing_a_m) {
                offsets.push(m);
            }
        }
        let mean = offsets.iter().sum::<f32>() / offsets.len().max(1) as f32;
        println!("{} systematic joints, cross joints' mean offset {mean:.3}", offsets.len());
        assert!(offsets.len() >= 30, "the systematic joints to measure: {}", offsets.len());
        assert!(
            mean > 0.15,
            "the cross joints do not cross a systematic joint: mean offset {mean:.3} of a spacing"
        );
    }

    /// The blocks meet at **thin traces**, and only there: a joint's separation is "mainly
    /// less than a millimetre" (Palmström 2015), so its trace is a hairline at the block's
    /// scale, and every edge between two blocks is one (Validation 5).
    #[test]
    fn the_blocks_meet_at_thin_traces() {
        let dir = Vec3::new(0.83, 0.31, 0.46).normalize();
        let origin = Vec3::new(3.0, 12.0, 30.0);
        let samples: Vec<(f32, RockJoints, f32)> = (0..15_000)
            .map(|i| {
                let s = i as f32 * 0.002;
                let p = origin + dir * s;
                let bed = rock_bed_coordinate(p);
                (s, rock_joints(p, bed), rock_joint_at(p, bed, 0.001))
            })
            .collect();
        let dark = samples.iter().filter(|s| s.2 > 0.5).count() as f32 / samples.len() as f32;
        let edges: Vec<usize> = (1..samples.len())
            .filter(|&k| block(&samples[k].1) != block(&samples[k - 1].1))
            .collect();
        let untraced: Vec<f32> = edges
            .iter()
            .filter(|&&k| {
                let window = &samples[k.saturating_sub(5)..(k + 5).min(samples.len())];
                !window.iter().any(|s| s.2 > 0.5)
            })
            .map(|&k| samples[k].0)
            .collect();
        let mut runs = Vec::new();
        let mut start = None;
        for (k, s) in samples.iter().enumerate() {
            match (s.2 > 0.5, start) {
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
            .filter(|(a, b)| !edges.iter().any(|e| *e >= *a && *e <= *b))
            .count();
        println!(
            "{} blocks, {:.1} % dark, untraced {untraced:?}, {empty} of {} traces with no edge",
            edges.len(),
            dark * 100.0,
            runs.len()
        );
        assert!(edges.len() >= 15, "the cut crosses blocks: {}", edges.len());
        assert!(dark < 0.08, "the traces are lines: {:.1} % of the cut is dark", dark * 100.0);
        assert!(untraced.is_empty(), "every block's edge is a trace: {untraced:?}");
        assert!(
            empty * 10 <= runs.len(),
            "a trace is where two blocks meet: {empty} of {} cross none",
            runs.len()
        );
        // And the blocks are of many sizes along the cut.
        let mut sizes: Vec<f32> = edges
            .windows(2)
            .map(|w| samples[w[1]].0 - samples[w[0]].0)
            .filter(|s| *s > 0.05)
            .collect();
        sizes.sort_by(f32::total_cmp);
        let p10 = sizes[sizes.len() / 10];
        let p90 = sizes[sizes.len() * 9 / 10];
        assert!(
            p90 / p10 >= 2.5,
            "the blocks are of many sizes: {p10:.2} m to {p90:.2} m along the cut"
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
        assert!(far_peak <= 0.25 + 1e-6, "a trace a quarter of the pixel is a quarter as dark");
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

    /// The **beds** carry a palette too, and the beds of a succession are of *different rock*.
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
