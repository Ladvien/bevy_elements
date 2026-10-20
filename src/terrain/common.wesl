// bevy_elements::terrain::common — the terrain grid's placement for shaders that read the packed
// `terrain_cells` buffer (wind solid mask, precipitation ground, ground extension, terrain and
// water materials).
//
//     import bevy_elements::grid::grid_taps;
//     import bevy_elements::terrain::common::{TerrainParams, terrain_live};
//
// `terrain_cells` is `array<vec4<f32>>`, one entry per cell, row-major: x surface height
// (bedrock + sediment, m above `misc.x`), y sediment depth (m), z water depth above the surface
// (m), w `bitcast<f32>(pack2x16float(∂h/∂x, ∂h/∂z))`. Sample it with `grid_taps(params.grid, xz)`
// and four loads; the world height is `misc.x + surface`.

import bevy_elements::grid::GridParams;
import bevy_elements::noise::{perlin2_d, perlin3_d, seed_offset};

struct TerrainParams {
    grid: GridParams,
    // x: world height of a stored height of 0 (m). y: readback downsample factor.
    // z: 1 when a terrain is bound, 0 when the binding is a one-cell placeholder.
    misc: vec4<f32>,
}

fn terrain_live(params: TerrainParams) -> bool {
    return params.misc.z > 0.5;
}

// ---------------------------------------------------------------------------------------------
// Ground detail (`docs/research/ground-detail.md`)
// ---------------------------------------------------------------------------------------------

/// Feature size of the coarse detail's base octave, m.
const DETAIL_COARSE_M: f32 = 1.25;
/// Feature size of the fine detail's base octave, m: the near-field grain.
const DETAIL_FINE_M: f32 = 0.09;
/// Amplitude of the coarse detail as a fraction of the albedo.
const DETAIL_COARSE_AMP: f32 = 0.20;
/// Amplitude of the fine detail as a fraction of the albedo.
const DETAIL_FINE_AMP: f32 = 0.85;
/// Height amplitude of each scale's base octave, m: the bump gradient is the gradient of a
/// height field of this amplitude, so the coarse scale reproduces the old
/// `0.04 × 0.8 = 0.032` per unit of the noise's derivative (`ground-detail.md` Parameters).
const DETAIL_COARSE_HEIGHT_M: f32 = 0.045;
const DETAIL_FINE_HEIGHT_M: f32 = 0.018;
/// Octaves of each scale.
const DETAIL_COARSE_OCTAVES: u32 = 4u;
const DETAIL_FINE_OCTAVES: u32 = 3u;

struct GroundDetail {
    /// Multiplier on the albedo, around 1.
    albedo: f32,
    /// The detail's gradient, already scaled by the bump of each octave that survived.
    grad: vec3<f32>,
    /// 1 where any octave survived the footprint test, 0 in the distance.
    present: f32,
}

/// Weight of an octave of feature size `feature_m` in a pixel whose footprint is `footprint_m`
/// wide: 1 while the octave is resolved (at least two footprints per feature), falling to 0 over
/// the band where it is not. Lagae et al. 2010 §5.2.1, frequency clamping: a band the pixel
/// cannot represent is cancelled rather than aliased (`ground-detail.md` Model 2).
fn detail_keep(feature_m: f32, footprint_m: f32) -> f32 {
    return 1.0 - smoothstep(0.5, 1.0, footprint_m / feature_m);
}

/// One detail scale's contribution: the weighted octave sum, the weights' total, its bump
/// gradient, and 1 where any octave survived the footprint test.
struct ScaleDetail {
    sum: f32,
    norm: f32,
    grad: vec3<f32>,
    present: f32,
}

/// One detail scale: `octaves` Perlin octaves from `base_m` down by halving (persistence 0.5, as
/// `fbm3` in `noise.wesl`), each band-limited by its own feature size.
///
/// The octaves are the crate's **2D** gradient noise in the world XZ plane, and the gradient is
/// carried back to the world as a surface gradient. This is the ground's *turf*, and the turf
/// is on the near-horizontal surfaces, where the plane's projection is the surface.
///
/// It must not reach a slope. A 2D noise in the world XZ plane is the *projection* of the
/// surface onto that plane, so on a slope the plane's lattice is stretched along the slope and
/// the lattice shows through as a weave at a fixed world direction: 43.5 degrees at 2 m and
/// 44.0 degrees at 6 m on a 45 degree face (`rock-material.md` Model 1). The upness gate in
/// `render.wesl` keeps the turf off the faces, and the rock's own material — the 3D noise at the
/// 3D point, which has no projection at all — takes over there.
fn detail_scale(
    p: vec3<f32>,
    base_m: f32,
    octaves: u32,
    footprint_m: f32,
    seed: u32,
) -> ScaleDetail {
    var out = ScaleDetail(0.0, 0.0, vec3<f32>(0.0), 0.0);
    var feature = base_m;
    var weight = 0.5;
    var q = p.xz / base_m;
    for (var o = 0u; o < octaves; o += 1u) {
        let keep = detail_keep(feature, footprint_m);
        if keep > 0.0 {
            let off = seed_offset(seed + o);
            let n = perlin2_d(q + off.xz);
            out.sum += weight * keep * n.x;
            // The gradient is in the plane; the bump tilts the surface by it.
            out.grad += weight * keep * vec3<f32>(n.y, 0.0, n.z) / feature;
            out.present = max(out.present, keep);
        }
        out.norm += weight;
        weight *= 0.5;
        feature *= 0.5;
        q *= 2.0;
    }
    return out;
}

/// One detail scale in the crate's **3D** noise, evaluated at the 3D surface point: the same
/// octave sum as `detail_scale`, but with no projection, so it is the same field seen from every
/// direction and it stays put on the surface.
///
/// This is what the rock's material is made of. A per-pixel *surface frame* was tried and
/// rejected: the normal comes from the terrain's 2 m cell gradient, so the frame rotates from
/// pixel to pixel, the noise's lattice swims in it, and the result is a herringbone moiré
/// (measured on the capture: a fine diagonal hatch over the whole face). Lagae et al. 2010 §5's
/// warning is about *slicing* a 3D noise onto a 2D surface (taking `z = const`), not about
/// evaluating the 3D function at the surface's own point.
fn detail_scale_3d(
    p: vec3<f32>,
    base_m: f32,
    octaves: u32,
    footprint_m: f32,
    seed: u32,
) -> ScaleDetail {
    var out = ScaleDetail(0.0, 0.0, vec3<f32>(0.0), 0.0);
    var feature = base_m;
    var weight = 0.5;
    var q = p / base_m;
    for (var o = 0u; o < octaves; o += 1u) {
        let keep = detail_keep(feature, footprint_m);
        if keep > 0.0 {
            let off = seed_offset(seed + o);
            let n = perlin3_d(q + off);
            out.sum += weight * keep * n.x;
            out.grad += weight * keep * n.yzw / feature;
            out.present = max(out.present, keep);
        }
        out.norm += weight;
        weight *= 0.5;
        feature *= 0.5;
        q *= 2.0;
    }
    return out;
}

// ---------------------------------------------------------------------------------------------
// Rock's own material (`docs/research/rock-material.md`)
// ---------------------------------------------------------------------------------------------

/// Spacing of the beds, m of world height. A bed is a *thick* band of rock, and a cliff shows a
/// few of them, not a hatch: at 3.5 m a 20 m face shows six beds, at 12 m it shows under two.
/// At 4.5 m the face a player stands at — four metres of it — shows one or two bands, which is
/// what reads as bedding; at 12 m it showed none and the rock was a wash.
const ROCK_BED_M: f32 = 4.5;
/// How far the beds dip and undulate, m of height.
const ROCK_BED_WARP_M: f32 = 3.5;
/// The bedding plane's tilt: the direction the beds run and how steeply they rise. A bed is a
/// surface inside the rock, and it is *tilted* — the world's height is not the rock's bedding,
/// and a massif banded by its own contours is a topographic map, not a mountain.
const ROCK_BED_TILT: vec3<f32> = vec3<f32>(0.32, 0.90, 0.28);
/// The second rock tone, relative to the first: a *darker*, browner bed. Bedded rock alternates
/// light and dark bands, and the dark one is darker rather than lighter: the crate's rock
/// albedo is already 0.10 linear, and a brighter second tone only washed the face further.
const ROCK_SECOND_TONE: vec3<f32> = vec3<f32>(0.86, 0.78, 0.70);
/// How far the beds' tone is mixed in, 0 to 1. The tone itself is 0.72, so at 0.85 the beds
/// span a 20 % albedo drop — enough to read as bands on a lit face.
const ROCK_BED_MIX: f32 = 0.34;
/// The rock's own grain scales, m: the coarse one is the field the fractures are ridges of.
const ROCK_GRAIN_COARSE_M: f32 = 2.6;
const ROCK_GRAIN_FINE_M: f32 = 0.12;
/// The fracture's width, as a fraction of the ridge's range.
const ROCK_FRACTURE_WIDTH: f32 = 0.22;
/// How dark the fractures are, as a multiplier on the rock's albedo. A crack is a shadow: the
/// face beside it sees less sky.
const ROCK_FRACTURE_DARKEN: f32 = 0.34;
/// The rock's own grain amplitude, as a fraction of the rock's albedo.
const ROCK_GRAIN_AMP: f32 = 0.75;

/// Rock's own material: the colour the rock's texture gives it, the grain's bump gradient, and
/// whether the grain survives the pixel's footprint. A cliff face is bedded, jointed and
/// two-toned; without this it is one flat colour, which is what the crate had
/// (`rock-material.md` Model 2).
struct RockMaterial {
    /// A multiplier on the rock's albedo: the beds' tone, the fractures' darkening, and the
    /// grain's modulation.
    tint: vec3<f32>,
    /// The grain's bump gradient, in the world.
    grad: vec3<f32>,
    /// 1 where the rock's own detail survives the pixel's footprint.
    present: f32,
}

/// The rock's material at the world point `p`, for a pixel footprint of `footprint_m` metres.
///
/// The **beds** are bands of the world height, warped by a low-frequency field so they dip and
/// undulate like real bedding instead of ruling a stripe pattern; a second, darker tone is mixed
/// in by them, so the cliff has two rock colours. The **fractures** are *lines* — the joints
/// along which rock breaks — and they are the one feature of a cliff a player reads as "solid" at
/// a distance. The **grain** is the rock's own band-limited octaves, so the rock's surface does
/// not carry the grass's tufts.
///
/// The grain's octaves and the fractures' octaves are **one** set of noise evaluations: a
/// fracture is a ridge of the same field the grain is made of, so the ridged transform is
/// applied to the noise the grain already has. That is half the noise the two would cost
/// separately, and the opaque pass is the crate's hot path (measured: the mountains' opaque
/// pass went from 0.145 ms to 0.309 ms with the two sums separate).
fn rock_material(p: vec3<f32>, warp: f32, footprint_m: f32) -> RockMaterial {
    // The beds: the rock's *own* bedding plane, not the world's height. A bed is a surface
    // inside the rock, and it is usually tilted — and if the beds followed the world height
    // they would be the terrain's own contours, which is a topographic map: the massif came
    // out banded with rings round every hill (measured: the mountain from 700 m was a set of
    // concentric contour bands, and no rock is bedded in the shape of the hill above it).
    // The bed's coordinate is a *tilted* height, so the beds cut across the terrain at an
    // angle, and the warp tilts them further.
    let tilted = dot(p, ROCK_BED_TILT);
    let bed = (tilted + warp * ROCK_BED_WARP_M) / ROCK_BED_M;
    // A bed's own value: a triangle wave, so each bed has a distinct top and bottom rather
    // than the bands fading into one another. The mask is sharp: a bed is a *band* of rock
    // with an edge, not a gradient.
    let band = abs(fract(bed) - 0.5) * 2.0;
    let bed_mask = smoothstep(0.30, 0.52, band);
    // The rock's own field: two octaves, one coarse and one fine, band-limited by their own
    // feature sizes. The fractures are the ridges of the coarse one.
    let coarse = detail_scale_3d(p, ROCK_GRAIN_COARSE_M, 1u, footprint_m, 51u);
    let fine = detail_scale_3d(p, ROCK_GRAIN_FINE_M, 1u, footprint_m, 61u);
    let coarse_norm = max(coarse.norm, 1e-6);
    let fine_norm = max(fine.norm, 1e-6);
    let grain_value = 0.5 * (coarse.sum / coarse_norm + fine.sum / fine_norm);
    let present = max(coarse.present, fine.present);
    let world_grad = 0.5 * (coarse.grad / coarse_norm + fine.grad / fine_norm);
    // The fractures: the ridge of the coarse octave, which is near zero on a line where the
    // noise crosses zero. A Worley cell network cannot make them — it bounds *cells*, so a face
    // reads as a giraffe's hide (measured: the cell pattern dominated the face at 2 m, and a
    // strong domain warp turned it into coral).
    // The fractures are band-limited by *their own* width, not by the cell they live in: a
    // crack is a line a fraction of a cell wide, so it aliases long before the cell does.
    // Band-limited by the cell, the cracks survived as a fine corduroy over the whole massif
    // at 700 m (measured: the mountain read as fabric, not stone).
    let fracture_keep = detail_keep(ROCK_GRAIN_COARSE_M * ROCK_FRACTURE_WIDTH, footprint_m);
    let ridged = 1.0 - abs(coarse.sum / coarse_norm) * abs(coarse.sum / coarse_norm);
    let crack = (1.0 - smoothstep(ROCK_FRACTURE_WIDTH, ROCK_FRACTURE_WIDTH * 3.0, ridged)) * fracture_keep;
    // The colour: the beds' second tone, the fractures' darkening, and the grain's modulation.
    let tint = mix(vec3<f32>(1.0), ROCK_SECOND_TONE, bed_mask * ROCK_BED_MIX);
    let dark = mix(1.0, ROCK_FRACTURE_DARKEN, crack);
    let grain_tint = 1.0 + ROCK_GRAIN_AMP * grain_value;
    return RockMaterial(
        tint * dark * grain_tint,
        world_grad * (ROCK_GRAIN_AMP * 0.02),
        present,
    );
}

/// The ground's detail at `p` (metres) for a view whose footprint is `footprint_m` metres per
/// pixel: two octave sums, each band-limited by its own scale, whose albedo modulation and bump
/// gradient replace the one fixed 0.8 m⁻¹ band faded by a distance constant. `albedo` is a
/// multiplier around 1; `grad` is the surface gradient the bump adds.
///
/// The octaves are the crate's 3D noise at the 3D point, so the detail is the same field on
/// every surface and does not weave the cliff faces (`rock-material.md` Model 1).
fn ground_detail(p: vec3<f32>, footprint_m: f32) -> GroundDetail {
    let coarse = detail_scale(p, DETAIL_COARSE_M, DETAIL_COARSE_OCTAVES, footprint_m, 1u);
    let fine = detail_scale(p, DETAIL_FINE_M, DETAIL_FINE_OCTAVES, footprint_m, 21u);
    let coarse_value = coarse.sum / max(coarse.norm, 1e-6);
    let fine_value = fine.sum / max(fine.norm, 1e-6);
    let world_grad = DETAIL_COARSE_HEIGHT_M * coarse.grad + DETAIL_FINE_HEIGHT_M * fine.grad;
    return GroundDetail(
        1.0 + DETAIL_COARSE_AMP * coarse_value + DETAIL_FINE_AMP * fine_value,
        world_grad,
        max(coarse.present, fine.present),
    );
}
