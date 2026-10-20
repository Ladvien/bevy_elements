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
import bevy_elements::noise::{perlin3_d, seed_offset};

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
///
/// The detail is the crate's **3D** noise now (see `detail_scale`), and the 3D noise's rms is
/// 0.2268 where the 2D one's was 0.3035 — measured in the crate's own mirror — so the
/// amplitudes carry the factor 1.338 that keeps the ground's grain at the level iteration 28
/// fitted it to. Without it the near field's standard deviation fell from 6.11 to 4.37 DN.
const DETAIL_COARSE_AMP: f32 = 0.268;
/// Amplitude of the fine detail as a fraction of the albedo.
const DETAIL_FINE_AMP: f32 = 1.138;
/// Height amplitude of each scale's base octave, m: the bump gradient is the gradient of a
/// height field of this amplitude, so the coarse scale reproduces the old
/// `0.04 × 0.8 = 0.032` per unit of the noise's derivative (`ground-detail.md` Parameters).
const DETAIL_COARSE_HEIGHT_M: f32 = 0.060;
const DETAIL_FINE_HEIGHT_M: f32 = 0.024;
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
/// The octaves are the crate's **3D** gradient noise, evaluated at the 3D surface point. This is
/// the fix for the weave, and the two earlier attempts are the two ways to get it wrong:
///
/// - A 2D noise in the **world XZ plane** is the *projection* of the surface onto the plane, so
///   on a slope the plane's lattice is stretched along the slope and the lattice shows through
///   as a weave at a fixed world direction: 43.5 degrees at 2 m and 44.0 degrees at 6 m on a 45
///   degree face (`rock-material.md` Model 1). That was iteration 28's fix for the streak below.
/// - A per-pixel **surface frame** — projecting into the frame of the surface's own normal —
///   is isotropic in principle, but the normal comes from the terrain's 2 m cell gradient and so
///   changes from pixel to pixel: the frame rotates, the noise's lattice swims in it, and the
///   result is a herringbone moiré. Measured on the capture: a fine diagonal hatch over the
///   whole face.
/// - The 3D noise at the 3D point has **no projection at all**, so it is the same field seen
///   from every direction and stays put on the surface. Lagae et al. 2010 §5's warning is
///   about *slicing* a 3D noise onto a 2D surface (taking `z = const`), not about evaluating
///   the 3D function at the surface's own point; the crate's 3D noise has 3D lattice gradients
///   and so does not have the compressed-axis defect that made the slice streak
///   (`ground-detail.md` Deviation 1).
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

/// The rock's fracture network: a ridged sum of the crate's 3D noise, `1 - |n|`, which is near
/// zero on a *line* where the noise crosses zero and large away from it. The lines are the
/// joints along which rock breaks; a Worley cell network bounds cells instead and reads as a
/// giraffe's hide (`rock-material.md`, Model 2).
///
/// The octaves are band-limited by their own feature size, like the ground's detail (Lagae et
/// al. 2010 section 5.2.1). The result is 0 on a joint and 1 in the rock between them.
fn rock_ridge(p: vec3<f32>, base_m: f32, octaves: u32, footprint_m: f32, seed: u32) -> f32 {
    var sum = 0.0;
    var norm = 0.0;
    var feature = base_m;
    var weight = 0.5;
    var q = p / base_m;
    for (var o = 0u; o < octaves; o += 1u) {
        let keep = detail_keep(feature, footprint_m);
        if keep > 0.0 {
            let off = seed_offset(seed + o);
            let n = perlin3_d(q + off).x;
            // The ridge: 1 where the noise is zero, 0 where it is at its extreme. Squaring the
            // absolute value sharpens the line without moving it.
            let r = 1.0 - abs(n) * abs(n);
            sum += weight * keep * r;
        }
        norm += weight;
        weight *= 0.5;
        feature *= 0.5;
        q *= 2.0;
    }
    return sum / max(norm, 1e-6);
}

// ---------------------------------------------------------------------------------------------
// Rock's own material (`docs/research/rock-material.md`)
// ---------------------------------------------------------------------------------------------

/// Spacing of the beds, m of world height. A bed is a *thick* band of rock, and a cliff shows a
/// few of them, not a hatch: at 3.5 m a 20 m face shows six beds, at 12 m it shows under two.
const ROCK_BED_M: f32 = 12.0;
/// How far the beds dip and undulate, m of height.
const ROCK_BED_WARP_M: f32 = 3.5;
/// The scale of the dip's undulation, m.
const ROCK_BED_WARP_SCALE_M: f32 = 60.0;
/// The second rock tone, relative to the first: a *darker*, browner bed. Bedded rock alternates
/// light and dark bands, and the dark one is darker rather than lighter: the crate's rock
/// albedo is already 0.10 linear, and a brighter second tone only washed the face further.
const ROCK_SECOND_TONE: vec3<f32> = vec3<f32>(0.72, 0.66, 0.60);
/// How far the beds' tone is mixed in, 0 to 1.
const ROCK_BED_MIX: f32 = 0.55;
/// The scale of the fracture network, m: the feature size of its coarsest octave.
const ROCK_FRACTURE_M: f32 = 1.6;
/// The fracture's width, as a fraction of the ridge's range.
const ROCK_FRACTURE_WIDTH: f32 = 0.16;
/// How dark the fractures are, as a multiplier on the rock's albedo.
const ROCK_FRACTURE_DARKEN: f32 = 0.58;
/// The rock's own grain amplitude, as a fraction of the rock's albedo.
const ROCK_GRAIN_AMP: f32 = 0.55;
/// The rock's grain scales, m.
const ROCK_GRAIN_COARSE_M: f32 = 1.1;
const ROCK_GRAIN_FINE_M: f32 = 0.12;

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

/// The rock's material at the world point `p`, for the surface's normal `n` and a pixel footprint
/// of `footprint_m` metres.
///
/// The **beds** are bands of the world height, warped by a low-frequency field so they dip and
/// undulate like real bedding instead of ruling a stripe pattern; a second, warmer tone is mixed
/// in by them, so the cliff has two rock colours. The **fractures** are the walls of a Worley
/// cell network — the joints along which rock breaks — and they are the one feature of a cliff a
/// player reads as solid at a distance. The **grain** is the ground's own band-limited octaves
/// with rock's amplitude and rock's own seed, so the rock's surface does not carry the grass's
/// tufts, and it is sampled in the surface's frame so it is round on a slope.
fn rock_material(p: vec3<f32>, footprint_m: f32) -> RockMaterial {
    // The beds: the world height, warped. The warp is a 3D noise, so it is one field seen from
    // every angle and the beds stay in the same place on the rock.
    let warp = perlin3_d(p / ROCK_BED_WARP_SCALE_M + seed_offset(31u)).x;
    let bed = (p.y + warp * ROCK_BED_WARP_M) / ROCK_BED_M;
    // A bed's own value: a triangle wave, so each bed has a distinct top and bottom rather
    // than the bands fading into one another.
    let band = abs(fract(bed) - 0.5) * 2.0;
    let bed_mask = smoothstep(0.25, 0.75, band);
    // The fractures: **lines**, and a Worley cell network cannot make them — its walls bound
    // cells, so a face reads as a giraffe's hide (measured: the cell pattern dominated the
    // face at 2 m, and a strong domain warp turned it into coral). A fracture is a *ridge* of
    // the rock's own field, and a ridged noise makes ridges: `1 - |n|` is near zero on a line
    // where the noise crosses zero, and the line is thin because the noise is smooth across
    // it. Two octaves of it give a network of joints of two scales, and a third, finer octave
    // gives the hairline cracks between them.
    let ridged = rock_ridge(p, ROCK_FRACTURE_M, 3u, footprint_m, 71u);
    let fracture_keep = detail_keep(ROCK_FRACTURE_M, footprint_m);
    let crack = (1.0 - smoothstep(ROCK_FRACTURE_WIDTH, ROCK_FRACTURE_WIDTH * 3.0, ridged)) * fracture_keep;
    // The grain: the ground's own octaves with rock's amplitude and seed.
    let coarse = detail_scale(p, ROCK_GRAIN_COARSE_M, 2u, footprint_m, 51u);
    let fine = detail_scale(p, ROCK_GRAIN_FINE_M, 2u, footprint_m, 61u);
    let coarse_norm = max(coarse.norm, 1e-6);
    let fine_norm = max(fine.norm, 1e-6);
    let grain_value = 0.5 * (coarse.sum / coarse_norm + fine.sum / fine_norm);
    let present = max(coarse.present, fine.present);
    let world_grad = 0.5 * (coarse.grad / coarse_norm + fine.grad / fine_norm);
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
