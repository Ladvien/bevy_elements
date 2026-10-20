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
    /// The coarse scale's own value, in [-1, 1]: the field at 1.25 m that the turf's grain is
    /// made of. The palette uses it as the *clump* of a tone, so a patch's edge is broken and
    /// the near field has colour variation in it without another noise evaluation
    /// (`ground-colour.md` Model 3).
    coarse: f32,
    /// The fine scale's own value, in [-1, 1]: the field at nine centimetres.
    fine: f32,
}

// ---------------------------------------------------------------------------------------------
// The ground's colour (`docs/research/ground-colour.md`)
// ---------------------------------------------------------------------------------------------

/// The palette of ground tones, as multipliers on the turf's albedo: fresh grass, the yellow of
/// a dry patch, the straw of a bleached one, and the earth of a worn place.
///
/// A meadow is not one green. Papavasiliou 2015 §3.3 is the model: each terrain type has a
/// reference colour, and a point's score for that type is the inverse distance of the
/// reference colour to the point's base map colour; the scores are the weights of a palette
/// blend. The tones here are the *variation* of one material rather than four materials, so the
/// palette is mixed in the material's own hue and its mean is the material's own colour.
const TURF_FRESH: vec3<f32> = vec3<f32>(1.00, 1.00, 1.00);
const TURF_DRY: vec3<f32> = vec3<f32>(1.35, 1.20, 0.69);
const TURF_STRAW: vec3<f32> = vec3<f32>(1.55, 1.37, 0.82);
const TURF_WORN: vec3<f32> = vec3<f32>(1.13, 0.95, 0.73);

/// The scale of the palette's patch, m: a patch is a clump of grass a couple of metres across.
const TURF_PATCH_M: f32 = 5.0;
/// The scale of the palette's *distribution* field, m: the field decides how much of each tone
/// there is around, over tens of metres, so the variation reaches the far field.
const TURF_FIELD_M: f32 = 38.0;
/// How far the palette moves the ground's colour towards a tone, 0 to 1.
const TURF_STRENGTH: f32 = 0.85;

/// The ground's colour: a multiplier on the turf's albedo.
///
/// The palette is chosen per *patch*, not per pixel: at 2 m a pixel is a centimetre and a straw
/// blade is a decimetre, so a per-pixel choice of a tone is a dithered speckle. The patch is
/// selected by a field, and the boundary between two tones is soft, so the tones' grain crosses
/// it (Lefèbvre & Neyret 2003's dithering of a low-resolution areas map, done with a smooth
/// boundary instead of a dither so the transition is not a speckle).
///
/// The result is normalised so that its mean is 1: a game's lighting is balanced around its
/// ground's colour, so the change must be a *variation* and not a shift (the measured hue's
/// spread over a near-ground patch was 0.010 on the plains before this note).
///
/// `base` is the ground's own colour, and its *chroma* decides how far the tone goes: a place
/// where the ground is already the soil's brown does not also get the straw's yellow, and a
/// shadow does not turn the grass into straw (Papavasiliou's weight vector, which drops the
/// lightness for exactly that reason).
fn turf_palette(p: vec3<f32>, base: vec3<f32>, field: f32, coarse: f32, fine: f32) -> vec3<f32> {
    // The distribution field — how much of each tone there is around — is the ground's own
    // large-scale field, which the caller already has: the palette is given it, so the
    // palette's only new evaluation is the *selection* field below. (The two extra 3D noise
    // evaluations the palette first carried cost the mountains' opaque pass 0.132 ms -> 0.148
    // ms, and the ground's field and the palette's were the same field at 45 m and 38 m.)
    //
    // The clump: which tone this piece of ground wears. A field at five metres picks it, and
    // the ground's own *detail* — a field at a metre, which the turf already evaluates for its
    // grain — is the clump's finer octave, so the near field has variation in it without
    // another noise evaluation.
    let patch_field = perlin3_d(p / TURF_PATCH_M + seed_offset(97u)).x / 0.2268;
    // The clump is the patch's field at five metres *plus* the turf's own fine field. The fine
    // field is at nine centimetres, which is far too fine to break a five-metre edge on its
    // own, but it is what makes the edge ragged rather than a smooth curve: a dry patch's
    // edge is broken, and the mixture of the two gives that.
    let clump = clamp(patch_field + coarse * 0.55 + fine * 0.45, -1.0, 1.0);
    // The score of each tone: how much of it belongs here. The field is the *distribution* (the
    // amount of the tone around) and the clump the *selection* (this piece of ground), and the
    // score is their product, so a tone is common where the field is high and present where the
    // clump is.
    //
    // The scores are raised to the power `p`, which is Papavasiliou's exponent: "using lower
    // values for p, broader transitional regions between terrain types can be achieved while
    // larger values create sharper, more defined transitions". At p = 1 the dry grass faded in
    // over the whole field and the meadow read as a haze; at p = 2 the patches have an edge.
    //
    // The scores are *shares* of the turf's own colour: the three dry tones take what they need
    // and the fresh grass has what is left, so the four sum to 1 and the palette is a convex
    // combination of them. A convex combination cannot drift the ground's mean: the mixture is
    // between the tones and the tones bracket the fresh one. (Normalising the mixture instead
    // was tried three ways and each failed: by the unweighted mean of the tones the ground's
    // mean brightness drifted 3.2 DN, by the mixture's own colour the hue was cancelled
    // entirely, and by a per-channel mean the hue was cancelled again.)
    // The scores are squared — Papavasiliou's exponent p = 2. A square is a multiply, not a
    // `pow`: three `pow` calls per pixel cost the mountains' opaque pass 0.137 ms -> 0.252 ms,
    // and a `pow` is a transcendental.
    //
    // The exponent is a *literal* everywhere it is written as a `pow` elsewhere: `pow(x, p)`
    // with `p` in a `let` makes naga emit a broken shader *without an error*, and the ground
    // renders as a dark navy (the same symptom as an unimported name; see AGENTS.md).
    let d = max(field, 0.0) * max(0.5 + clump, 0.0) * 1.6;
    let st = max(field - 0.45, 0.0) * 3.0 * max(0.5 + clump, 0.0) * 1.6;
    let w = max(-field - 0.55, 0.0) * 3.0 * max(0.5 - clump, 0.0) * 1.6;
    var s_dry = min(d * d, 1.0);
    var s_straw = min(st * st, 1.0);
    var s_worn = min(w * w, 1.0);
    let others = s_dry + s_straw + s_worn;
    if others > 1.0 {
        let k = 1.0 / others;
        s_dry = s_dry * k;
        s_straw = s_straw * k;
        s_worn = s_worn * k;
    }
    // The fresh share is what is left, and the subtraction can leave it a hair below zero
    // (−2.2e−8 at field 0.5, clump 0.75), which is a negative weight.
    let s_fresh = max(1.0 - s_dry - s_straw - s_worn, 0.0);
    // The palette, mixed by the shares.
    let mixed = s_fresh * TURF_FRESH + s_dry * TURF_DRY + s_straw * TURF_STRAW + s_worn * TURF_WORN;
    var tint = mix(vec3<f32>(1.0), mixed, TURF_STRENGTH);
    // The base colour's own chroma decides how far the tone goes.
    // The base colour's own chroma decides how far the tone goes. The chroma is measured
    // *relative* to the colour's own brightness: a dark albedo has a small absolute chroma by
    // construction (the crate's grass is (0.0397, 0.0835, 0.0134), whose chroma is 0.065), and
    // an absolute measure would leave the palette at 39 % of its strength on the ground and
    // nowhere near it on a bright one.
    let lum = dot(base, vec3<f32>(0.2126, 0.7152, 0.0722));
    let chroma = length(base - vec3<f32>(lum)) / max(lum, 1e-4);
    let carry = clamp(chroma * 1.6, 0.0, 1.0);
    return mix(vec3<f32>(1.0), tint, carry);
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
const ROCK_SECOND_TONE: vec3<f32> = vec3<f32>(1.06, 0.93, 0.76);
/// How far the beds' tone is mixed in, 0 to 1. The tone itself is 0.72, so at 0.85 the beds
/// span a 20 % albedo drop — enough to read as bands on a lit face.
const ROCK_BED_MIX: f32 = 0.34;
/// The rock's own grain scales, m: the coarse one is the field the fractures are ridges of.
const ROCK_GRAIN_COARSE_M: f32 = 2.6;
const ROCK_GRAIN_FINE_M: f32 = 0.12;
/// The fracture's width, as a fraction of the ridge's range.
const ROCK_FRACTURE_WIDTH: f32 = 0.30;
/// How dark the fractures are, as a multiplier on the rock's albedo. A crack is a shadow: the
/// face beside it sees less sky.
const ROCK_FRACTURE_DARKEN: f32 = 0.26;
/// The rock's own grain amplitude, as a fraction of the rock's albedo.
const ROCK_GRAIN_AMP: f32 = 0.60;

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
    // The relief's gradient is the *coarse* octave's alone. The fine octave's gradient is
    // anisotropic on a slope: a 3D noise's gradient has a vertical component, and the surface
    // compresses it, so the fine relief was a directional weave (measured: the near band of a
    // rock face had a structure-tensor coherence of 0.71 at 29-39 degrees, against the flat
    // ground's 0.36). The fine octave stays in the albedo, where a direction is invisible.
    let world_grad = coarse.grad / coarse_norm;
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
    // The rock's relief. A face with no relief is a painted colour: the light does no work on
    // it, and it reads as fog or as concrete. The grain's own gradient gives the surface its
    // small relief, and the fractures give it the *step* a crack has: the ridge's gradient
    // turns the crack from a drawn line into a groove the sun rakes.
    let relief = world_grad * (ROCK_GRAIN_AMP * 0.20);
    return RockMaterial(
        tint * dark * grain_tint,
        relief,
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
        coarse_value,
        fine_value,
    );
}
