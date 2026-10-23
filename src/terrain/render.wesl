// The owned terrain render: CDLOD vertex displacement from the packed `terrain_cells` buffer and
// the procedural layer blend. See docs/research/terrain.md ("Vertex stage", "Fragment stage").
//
// One file for the forward, prepass and deferred pipelines (like bevy_pbr's pbr.wesl): the vertex
// entry point is the vertex shader of all three, so the main pass and the depth prepass displace
// bit-identically. The fragment entry point is the forward shader and the deferred one; the
// depth/normal prepass fragment stays Bevy's.

import bevy_elements::grid::{GridParams, grid_taps};
import bevy_elements::noise::{perlin3_d, fbm3, seed_offset};
import bevy_elements::terrain::common::{TerrainParams, ground_detail, rock_material, scree_scatter, terrain_live, turf_palette};
import bevy_elements::terrain::textures::{LayerMaps, layer_maps, height_blend, DETAIL_GATE};
import bevy_elements::surface_cover::{weather_surface, ripple_weights, ripple_offset, overcast_relief};
import bevy_elements::trails::surface::{TrailHit, trail_fade, trail_march, trail_none};
import bevy_pbr::render::{
    mesh_functions,
    mesh_view_bindings::{view, lights},
    view_transformations::position_world_to_clip,
    pbr_fragment::pbr_input_from_standard_material,
};

@if(PREPASS_PIPELINE)
import bevy_pbr::prepass::io::VertexOutput;
@else
import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};

@if(PREPASS_PIPELINE && DEFERRED_PREPASS)
import bevy_pbr::prepass::io::FragmentOutput;
@if(PREPASS_PIPELINE && DEFERRED_PREPASS)
import bevy_pbr::deferred::functions::deferred_output;

@if(!PREPASS_PIPELINE)
import bevy_pbr::render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing};

struct TerrainMaterialParams {
    terrain: TerrainParams,
    // Per level: x morph start, y range (m).
    lod: array<vec4<f32>, 16>,
    // x: coarsest level.
    top: vec4<f32>,
    // xy: cloud shadow map origin (metre-frame XZ), z: its extent, w: the lights' mean transmittance.
    shadow: vec4<f32>,
    // xyz: unit direction toward the light the map was made for. w: 1 when a map is bound.
    light: vec4<f32>,
    // xyz: the water's absorption (1/m). w: its refractive index (0 without water rendering).
    absorption: vec4<f32>,
    // x: 1 when the cover maps are bound. y: rain intensity min(1, R_liquid / 20). z: puddle start
    // depth (m). w: full-puddle depth (m).
    cover: vec4<f32>,
    // Phase of each of the four ripple layers, 0–1.
    ripple: vec4<f32>,
    // x: texel edge (m), y: texels a side, z: 1 when the trail field is bound, w: the parallax
    // march's bound on |h| (m).
    trail: vec4<f32>,
    // x: the beam's share of the light on a horizontal surface. yz: the window's centre,
    // metre-frame XZ.
    trail_light: vec4<f32>,
    // The ground's detail textures (`GroundTextures`): xyz one tile's size on the ground (m) for
    // the sand, the soil and the grass, all zero without a texture set; w the share of the
    // procedural grain kept under them (1 without a set).
    ground: vec4<f32>,
    // World units to the metre (`ElementsUnits`): the shader divides the positions Bevy gives it
    // by this and multiplies the ones it hands back.
    units: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> material: TerrainMaterialParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<storage, read> terrain_cells: array<vec4<f32>>;
// (snow depth m, wetness 0–1, snow water equivalent m, —) per cell; a one-cell placeholder while
// `material.cover.x` is 0.
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var<storage, read> cover_cells: array<vec4<f32>>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var cloud_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var cloud_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var trail_field: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var trail_sampler: sampler;
// The ground's detail textures: two array layers per ground layer (`textures.wesl`).
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var ground_maps: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var ground_maps_sampler: sampler;

struct TerrainVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
}

// Height above the terrain's origin (m) at world XZ, from the four taps around it.
fn terrain_height(xz: vec2<f32>) -> f32 {
    let t = grid_taps(material.terrain.grid, xz);
    let a = mix(terrain_cells[t.i00].x, terrain_cells[t.i10].x, t.w.x);
    let b = mix(terrain_cells[t.i01].x, terrain_cells[t.i11].x, t.w.x);
    return mix(a, b, t.w.y);
}

// The surface gradient (∂h/∂x, ∂h/∂z) at world XZ: a bilinear blend of the stored central
// differences, smooth across cell edges.
fn terrain_gradient(xz: vec2<f32>) -> vec2<f32> {
    let t = grid_taps(material.terrain.grid, xz);
    let g00 = unpack2x16float(bitcast<u32>(terrain_cells[t.i00].w));
    let g10 = unpack2x16float(bitcast<u32>(terrain_cells[t.i10].w));
    let g01 = unpack2x16float(bitcast<u32>(terrain_cells[t.i01].w));
    let g11 = unpack2x16float(bitcast<u32>(terrain_cells[t.i11].w));
    return mix(mix(g00, g10, t.w.x), mix(g01, g11, t.w.x), t.w.y);
}

// Snow depth (m) at world XZ from the cover pack; 0 without the maps.
fn snow_depth(xz: vec2<f32>) -> f32 {
    if material.cover.x < 0.5 {
        return 0.0;
    }
    let t = grid_taps(material.terrain.grid, xz);
    let a = mix(cover_cells[t.i00].x, cover_cells[t.i10].x, t.w.x);
    let b = mix(cover_cells[t.i01].x, cover_cells[t.i11].x, t.w.x);
    return mix(a, b, t.w.y);
}

fn morph_k(distance: f32, lod: u32) -> f32 {
    if f32(lod) >= material.top.x {
        return 0.0;
    }
    let range = material.lod[lod];
    return clamp((distance - range.x) / (range.y - range.x), 0.0, 1.0);
}

// Metre-frame position of a patch vertex with the CDLOD morph applied (Strugar's morphVertex): the
// instance matrix's translation is the patch origin in world units (x, terrain y, z), divided by
// `units`; `MeshTag` is the level.
fn terrain_vertex_position(local: vec3<f32>, instance_index: u32) -> vec4<f32> {
    let m = mesh_functions::get_world_from_local(instance_index);
    let lod = mesh_functions::get_tag(instance_index);
    let origin = vec3<f32>(m[3].x, m[3].y, m[3].z) / material.units;
    let gi = round(local.xz * 32.0);
    let step = material.terrain.grid.cell * f32(1u << lod);
    var wxz = origin.xz + gi * step;
    let d = distance(view.lod_view_world_position / material.units, vec3<f32>(wxz.x, origin.y + terrain_height(wxz), wxz.y));
    let odd = gi - 2.0 * floor(gi * 0.5);
    wxz = wxz - odd * step * morph_k(d, lod);
    // The snow lies on the surface: the same depth in the prepass, so the depths agree.
    return vec4<f32>(wxz.x, origin.y + terrain_height(wxz) + snow_depth(wxz), wxz.y, 1.0);
}

@vertex
fn vertex(v: TerrainVertex) -> VertexOutput {
    var out: VertexOutput;
    let metres = terrain_vertex_position(v.position, v.instance_index);
    let world = vec4<f32>(metres.xyz * material.units, 1.0);
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);
    @if(!PREPASS_PIPELINE || NORMAL_PREPASS_OR_DEFERRED_PREPASS) {
        let g = terrain_gradient(metres.xz);
        out.world_normal = normalize(vec3<f32>(-g.x, 1.0, -g.y));
    }
    @if(PREPASS_PIPELINE && MOTION_VECTOR_PREPASS)
    out.previous_world_position = world;
    @if(PREPASS_PIPELINE && UNCLIPPED_DEPTH_ORTHO_EMULATION) {
        out.unclipped_depth = out.position.z;
        out.position.z = min(out.position.z, 1.0);
    }
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = v.instance_index;
    return out;
}

// Linear albedo of the four layers: the plan's sRGB colours (Deviation 4).
//
// The rock is the plan's colour with a little more blue in it: (0.1005, 0.0890, 0.0783) is a
// *warm* grey, and the noon sun is warm too, so a face lit by it renders pink (measured: the
// face at 2 m read at RGB 155, 147, 138 — warmer than the sky that lit it, and pale). A stone
// face under a blue sky is grey with the blue in it.
const ROCK: vec3<f32> = vec3<f32>(0.076, 0.071, 0.066);
const SOIL: vec3<f32> = vec3<f32>(0.0732, 0.0397, 0.0196);
const SAND: vec3<f32> = vec3<f32>(0.5382, 0.3931, 0.1960);
const GRASS: vec3<f32> = vec3<f32>(0.0397, 0.0835, 0.0134);

// Andersson 2007 §5.2.3, Listing 2: the Overlay blend of a detail value onto a computed mask, at
// full opacity. It keeps 0 and 1, and 0.5 is the identity, so the detail breaks up a transition
// without moving where a layer is fully on or off.
fn overlay(mask: f32, detail: f32) -> f32 {
    return select(1.0 - 2.0 * (1.0 - mask) * (1.0 - detail), 2.0 * mask * detail, mask < 0.5);
}

// The four layer weights (rock, sand, soil, grass) for a slope angle (degrees), a sediment depth
// (m) and a detail value in [0, 1] (0.5: none). They sum to 1 by construction; `model/mirror.rs`
// is the CPU copy (terrain.md Deviation 19).
//
// The rock and the grass meet at one boundary, and it is *one* field: the rock's mask is the
// slope's ramp with the detail overlaid, and the grass takes what is left of it. The two must
// not be computed separately — the rock's mask overlaid with the detail and the grass's mask
// overlaid with the *same* detail meet at two different places, and the ground shows a hard
// green line along the rock (measured on `mountains` from 700 m: the grass's edge was a
// two-pixel step, and the layer weights on either side of it were 0.05 and 0.95).
fn layer_weights(slope_deg: f32, sediment: f32, detail: f32) -> vec4<f32> {
    // The rock's ramp starts at 30°, not 35°: the ground's turf is the crate's 2D noise in the
    // world XZ plane, and the turf's own gate closes at 37° (see `turf_gate`), so the rock has
    // to be there before the turf goes — otherwise the turf shows on a slope steep enough to
    // weave it (at 44° the turf was at 44 % and the rock at 53 %, so 21 % of the 2D turf was
    // still on the face: the weave).
    let rock = overlay(smoothstep(30.0, 38.0, slope_deg), detail);
    let sand = overlay(smoothstep(0.3, 0.7, sediment), detail) * (1.0 - smoothstep(26.0, 30.0, slope_deg)) * (1.0 - rock);
    // The soil and the grass share what the rock and the sand left, and they divide it at the
    // soil band — which the detail also moves, so the two boundaries are the same field.
    //
    // The soil's band *tapers* as the rock's ramp opens. A slope band that runs up to the
    // rock's edge puts the soil at its full weight right where the rock begins, and the two
    // meet in a hard step: a warm brown rim round every grass patch against the stone (the
    // rim measured 1415 orange pixels at RGB 148, 121, 100 in one frame, and it read as a
    // cartoon outline at 2×). The taper is scree: the soil thins as the rock rises.
    let band = overlay(smoothstep(16.0, 26.0, slope_deg), detail) * (1.0 - smoothstep(22.0, 29.0, slope_deg));
    let rest = (1.0 - rock) * (1.0 - sand);
    return vec4<f32>(rock, sand, rest * band, rest * (1.0 - band));
}

/// The rock's sky occlusion: how much of the sky a face with normal component `n_y` sees.
/// A horizontal surface sees the whole dome; a steep face sees a strip of it, and a face in a
/// valley sees barely any.
///
/// The curve is the crate's own `overcast_relief`'s dome term, `1 − 0.30 (1 − n_y)`, measured
/// on the sky probe's irradiance map (a side 0.70 and an underside 0.40 of a top,
/// `sky-light.md` Deviation 9): the sky's irradiance on a surface of that normal. A face at 45°
/// sees 0.65 of what the ground does, and at 60° half.
fn rock_sky_occlusion(n_y: f32) -> f32 {
    return 1.0 - 0.30 * max(1.0 - n_y, 0.0);
}

/// The ground's turf gate: how much of the ground's own grain a surface wears, by its
/// upness. It closes at 37° (n.y = 0.80) and is fully open at 23° (n.y = 0.92).
///
/// The gate is the *turf's* gate, and it is tight on purpose. The turf is the crate's 2D noise
/// in the world XZ plane, which is the *projection* of the surface onto that plane: on a slope
/// the plane's lattice is stretched along the slope and shows through as a weave at a fixed
/// world direction — 43.5° at 2 m and 44.0° at 6 m on a 45° face (`rock-material.md` Model 1).
/// The turf must therefore be gone before the surface is steep enough to weave it, and the
/// rock's material — the 3D noise at the 3D point — takes over there.
fn turf_gate(n_y: f32) -> f32 {
    return smoothstep(0.80, 0.92, n_y);
}

/// The scale of the ground's large-scale variation, m: the patches of longer and shorter grass,
/// bare earth and drier ground.
const LAYER_VARIATION_M: f32 = 45.0;
/// How far the variation moves the layer weights, 0 to 1. At 1.0 the plains became a lurid
/// green-and-brown patchwork (a football pitch, not ground); at 0.25 the patches are there and
/// the ground is still a meadow.
const LAYER_VARIATION_DEPTH: f32 = 0.25;
/// The scale of the soil band's patchiness, m: the bare earth on a slope comes in patches.
const SOIL_PATCH_M: f32 = 11.0;
/// How much of the slope band's soil the patchiness removes, 0 to 1.
const SOIL_PATCH_DEPTH: f32 = 0.93;

/// The ground's two low-frequency fields: the *large* one, which moves the soil and the grass
/// weights against each other, and the *fine* one, which breaks the slope band's soil into
/// patches.
///
/// The large field is what gives a plain its patches of bare earth: the layer masks vary only
/// with slope and sediment, so on a plain — where the grass mask is 1 everywhere — the whole
/// view is one colour; Andersson's Overlay detail "does not affect areas where the base
/// procedural mask is 0.0 or 1.0" (2007 section 5.2.3), so it cannot break that up. This
/// field does: it moves the weights themselves (`rock-material.md` Model 4).
///
/// The fine field breaks the *slope* band's soil into patches. A slope band on a mountain is a
/// contour line: the soil drew a continuous red-brown ribbon round every rock, and the whole
/// massif read as a topographic map (measured: the red-brown pixels were 2.59 % of the frame at
/// 700 m and ran unbroken along every face; with the patches they are 1.26 %).
struct GroundFields {
    /// The large field, in [-1, 1].
    large: f32,
    /// The patchiness of the slope band's soil, 0 to 1.
    scatter: f32,
}

/// The ground's two low-frequency fields at `p`.
///
/// The fine field is only evaluated where there is soil to break up: on a plain and on a
/// cliff the soil's weight is zero, and a 3D noise evaluation is the crate's most expensive
/// shader operation (the mountains' opaque pass is 0.136 ms on the baseline build).
fn ground_fields(p: vec3<f32>, soil_weight: f32) -> GroundFields {
    // The field is normalised by the noise's own rms (0.2268, measured in the crate's mirror)
    // so that `LAYER_VARIATION_DEPTH` is the *fraction* of the grass that a patch turns to
    // soil, and not a fraction of a small number. Without the normalisation the shift was
    // 0.08 of the weights, and the soil's and the grass's albedos differ by so little in linear
    // terms that the patches measured 0.5 DN — invisible (the plains' 60 px block standard
    // deviation was 2.25 DN before and 2.21 after).
    let large = perlin3_d(p / LAYER_VARIATION_M + seed_offset(71u)).x / 0.2268;
    if soil_weight < 0.001 {
        return GroundFields(large, 1.0);
    }
    // The fine field: the same noise at a quarter of the scale, with its own offset.
    let fine = perlin3_d(p / SOIL_PATCH_M + seed_offset(83u)).x / 0.2268;
    return GroundFields(
        large,
        1.0 - SOIL_PATCH_DEPTH * smoothstep(-1.1, 0.7, fine),
    );
}

/// The ground's large-scale variation of the layer weights: the *large* field takes soil from
/// the grass, so a plain gets patches instead of one colour. The field is a *contrast*
/// modulation: the weights are moved against each other and then renormalised, so the sum is
/// still 1 and the ground's mean colour is unchanged.
fn layer_variation(field: f32, w: vec4<f32>) -> vec4<f32> {
    // Only the soil and the grass move against each other: the rock and the sand are where the
    // terrain's shape put them, and a patch of "more rock" on a plain is not a thing.
    //
    // The shift is *taken from the grass and given to the soil*, so it works where the soil's
    // weight is zero — which is everywhere on a plain. Moving the two weights against each
    // other instead (each by the other's weight) does nothing when one of them is zero.
    let shift = clamp(field, -1.0, 1.0) * LAYER_VARIATION_DEPTH * w.w;
    var v = w;
    // The shift is taken from the grass and given to the soil, and both ends are clamped: a
    // field more negative than the soil's weight would take the soil below zero (measured:
    // −0.054 on a plain), and a negative weight is a negative albedo.
    let taken = clamp(shift, -v.z, v.w);
    v.z = v.z + taken;
    v.w = v.w - taken;
    let total = v.x + v.y + v.z + v.w;
    return v / max(total, 1e-6);
}

/// The raw cloud deck transmittance over the ground at `p` (1.0 outside the map or without one):
/// the deck the sky's light comes through, so the shade ramp's signal (`snow-cover.md`
/// Deviation 15).
fn cloud_deck(p: vec3<f32>) -> f32 {
    let l = material.light.xyz;
    var t = 1.0;
    if material.light.w > 0.5 && l.y > 0.1 {
        let uv = (p.xz - l.xz * (p.y / l.y) - material.shadow.xy) / material.shadow.z;
        if all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)) {
            t = textureSampleLevel(cloud_map, cloud_sampler, uv, 0.0).r;
        }
    }
    return t;
}

// Cloud shadow scale ρ = (0.25 + 0.75 T_px) / (0.25 + 0.75 T̄) at a world position, `T̄` the
// mean transmittance the lights carry; the same formula the weathered materials use
// (`surface_cover.wesl::cloud_shadow_ratio`; broken-cloud-light.md, Model 2).
fn cloud_shadow_ratio(p: vec3<f32>) -> f32 {
    let here = 0.25 + 0.75 * cloud_deck(p);
    let lights = max(0.25 + 0.75 * material.shadow.w, 0.01);
    return clamp(here / lights, 0.0, 4.0);
}

// The light's way down to the bed through `depth` metres of water, per channel (water-rendering.md,
// "The bed under the water"): `exp(-σ d / μ')` with `μ'` the sine of the sun's elevation after
// refraction at the surface. The view leg back up is the water's own absorb pass.
fn bed_transmittance(depth: f32) -> vec3<f32> {
    let n = material.absorption.w;
    if n <= 1.0 || depth <= 0.0 || lights.n_directional_lights == 0u {
        return vec3<f32>(1.0);
    }
    // The brightest directional light is the sun (the moon is a second light).
    var brightest = 0.0;
    var up = 0.0;
    for (var i = 0u; i < lights.n_directional_lights; i += 1u) {
        let c = lights.directional_lights[i].color;
        let luminance = c.r + c.g + c.b;
        if luminance > brightest {
            brightest = luminance;
            up = lights.directional_lights[i].direction_to_light.y;
        }
    }
    if brightest <= 0.0 {
        return vec3<f32>(1.0);
    }
    let mu = clamp(up, 0.05, 1.0);
    let mu_r = sqrt(max(1.0 - (1.0 - mu * mu) / (n * n), 0.01));
    return exp(-material.absorption.xyz * depth / mu_r);
}

// Compacted snow's albedo relative to fresh snow on a trench floor ([INFERENCE], trails.md
// Deviation 1).
const COMPACTED_SNOW_TINT: vec3<f32> = vec3<f32>(0.86, 0.91, 0.97);

// The direction toward the brightest directional light (the sun, else the moon); straight up
// without one.
fn brightest_light() -> vec3<f32> {
    var brightest = 0.0;
    var l = vec3<f32>(0.0, 1.0, 0.0);
    for (var i = 0u; i < lights.n_directional_lights; i += 1u) {
        let c = lights.directional_lights[i].color;
        let luminance = c.r + c.g + c.b;
        if luminance > brightest {
            brightest = luminance;
            l = lights.directional_lights[i].direction_to_light;
        }
    }
    return l;
}

// The ground layers with their detail textures: the layers' weights after the height blend,
// their colours times their albedo detail, and the relief's slope, roughness and occlusion,
// each weighted by its layer's share (so they fade where the rock takes over).
struct GroundTextured {
    // Sand, soil, grass.
    weights: vec3<f32>,
    albedo: vec3<f32>,
    slope: vec2<f32>,
    rough: f32,
    occlusion: f32,
}

// The ground layers' maps at `xz` (metre frame) for the ground weights `w` (sand, soil, grass),
// with the point's screen derivatives `dx`, `dy`. A layer below the gate is not sampled.
fn ground_textured(xz: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, w: vec3<f32>) -> GroundTextured {
    let tile = material.ground.xyz;
    let neutral = LayerMaps(vec3<f32>(1.0), 0.5, vec2<f32>(0.0), 1.0, 1.0);
    var sand = neutral;
    var soil = neutral;
    var grass = neutral;
    if w.x > DETAIL_GATE {
        sand = layer_maps(ground_maps, ground_maps_sampler, 0u, xz / tile.x, dx / tile.x, dy / tile.x);
    }
    if w.y > DETAIL_GATE {
        soil = layer_maps(ground_maps, ground_maps_sampler, 1u, xz / tile.y, dx / tile.y, dy / tile.y);
    }
    if w.z > DETAIL_GATE {
        grass = layer_maps(ground_maps, ground_maps_sampler, 2u, xz / tile.z, dx / tile.z, dy / tile.z);
    }
    let b = height_blend(w, vec3<f32>(sand.height, soil.height, grass.height));
    let rest = 1.0 - (b.x + b.y + b.z);
    var out: GroundTextured;
    out.weights = b;
    out.albedo = b.x * SAND * sand.albedo + b.y * SOIL * soil.albedo + b.z * GRASS * grass.albedo;
    out.slope = b.x * sand.slope + b.y * soil.slope + b.z * grass.slope;
    out.rough = rest + b.x * sand.rough + b.y * soil.rough + b.z * grass.rough;
    out.occlusion = rest + b.x * sand.occlusion + b.y * soil.occlusion + b.z * grass.occlusion;
    return out;
}

// The trail field at the fragment `p` on ground of slope `g` (∂y/∂x, ∂y/∂z): a flat no-op
// outside the window or without trails.
fn trail_at(p: vec3<f32>, g: vec2<f32>) -> TrailHit {
    let eye = view.world_position / material.units;
    let fade = trail_fade(p.xz, material.trail_light.yz, material.trail);
    if fade <= 0.0 {
        return trail_none(p.xz);
    }
    return trail_march(trail_field, trail_sampler, material.trail, p, eye, g, brightest_light(), fade);
}

@if(!PREPASS_PIPELINE || DEFERRED_PREPASS)
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = in.world_position.xyz / material.units;

    // Cell fetch: sediment and water bilinear; the normal from the stored gradient.
    let t = grid_taps(material.terrain.grid, p.xz);
    let c00 = terrain_cells[t.i00];
    let c10 = terrain_cells[t.i10];
    let c01 = terrain_cells[t.i01];
    let c11 = terrain_cells[t.i11];
    let sediment = mix(mix(c00.y, c10.y, t.w.x), mix(c01.y, c11.y, t.w.x), t.w.y);
    let water_depth = mix(mix(c00.z, c10.z, t.w.x), mix(c01.z, c11.z, t.w.x), t.w.y);
    let g = mix(
        mix(unpack2x16float(bitcast<u32>(c00.w)), unpack2x16float(bitcast<u32>(c10.w)), t.w.x),
        mix(unpack2x16float(bitcast<u32>(c01.w)), unpack2x16float(bitcast<u32>(c11.w)), t.w.x),
        t.w.y,
    );
    // Trails (docs/research/trails.md): the field's parallax hit is the visible point, and its
    // slope tilts the ground's normal.
    let trail = trail_at(p, g);
    let pv = vec3<f32>(trail.xz.x, p.y + trail.h + dot(g, trail.xz - p.xz), trail.xz.y);
    let depth = max(-trail.h, 0.0);
    let pressed = smoothstep(0.005, 0.04, depth);
    var n = normalize(vec3<f32>(-(g.x + trail.grad.x), 1.0, -(g.y + trail.grad.y)));

    // Slope angle and the mask detail: one fBm octave at 48 m, contrast-scaled into [0, 1] and
    // overlaid on the masks (Andersson 5.2.3), so the transitions are irregular rather than
    // iso-slope lines.
    // The slope is the terrain's: a footprint's wall is not a cliff.
    // One octave, not four: the field only has to *move* the layer boundaries, and every
    // octave is a 3D noise evaluation in the crate's hottest pass. Four octaves took the
    // mountains' opaque pass from 0.136 ms to 0.249 ms; one leaves it at 0.143 ms, and the
    // transitions are just as irregular (the capture is the same either way).
    let alpha = degrees(atan(length(g)));
    let detail = clamp(0.5 + 0.75 * fbm3(p / 48.0, 1u), 0.0, 1.0);
    var w = layer_weights(alpha, sediment, detail);
    // The ground's two low-frequency fields: the large one moves the soil and the grass
    // weights, the fine one breaks the slope band's soil into patches (`ground_fields`). The
    // fine one is only evaluated where there is soil to break up.
    let fields = ground_fields(p, w.z);
    w.z = w.z * fields.scatter;
    w = layer_variation(fields.large, w);

    // The pixel's footprint on the surface, in metres: the detail is band-limited by what the
    // pixel can resolve *on the surface*, not on the ground plane. The derivatives are taken
    // here, in uniform control flow, because the detail textures' gradients are these too.
    let dpx = dpdx(pv);
    let dpy = dpdy(pv);
    let footprint = max(length(dpx), length(dpy));

    // The ground's detail textures (`docs/research/ground-textures.md`): each ground layer's own
    // albedo, relief, roughness and occlusion, hex-tiled, and the layers' heights decide where
    // they meet. Without a texture set (`material.ground.x` is 0) the ground is the procedural
    // one alone, exactly.
    let maps_on = material.ground.x > 0.0 && w.y + w.z + w.w > DETAIL_GATE;
    var maps = GroundTextured(w.yzw, vec3<f32>(0.0), vec2<f32>(0.0), 1.0, 1.0);
    if maps_on {
        maps = ground_textured(pv.xz, dpx.xz, dpy.xz, w.yzw);
        w = vec4<f32>(w.x, maps.weights);
    }
    // A trench through grass or soil shows the soil (sand stays sand; rock takes no imprint).
    let trench = smoothstep(0.01, 0.05, depth) * (w.z + w.w);
    // The layers' own colours without the textures' detail: the turf's palette reads their chroma.
    let plain = mix(w.x * ROCK + w.y * SAND + w.z * SOIL + w.w * GRASS, SOIL, trench);
    var albedo = select(plain, mix(w.x * ROCK + maps.albedo, SOIL, trench), maps_on);
    let rough = mix(0.9, 0.95, w.z) * maps.rough;

    // The ground's detail, and the rock's own material (`docs/research/rock-material.md`).
    // The ground's grain is turf, soil and sand: it belongs on the near-horizontal surfaces,
    // and the turf's gate keeps it off the steep faces. The rock's material is bedded, jointed
    // and two-toned, and it is what a cliff face wears; the two cross-fade over the rock
    // weight, so a face that turns into a slope loses its bedding as it gains its turf.
    //
    // Each is evaluated only where it shows. The ground's grain is invisible on a face (the
    // turf gate and the rock weight both close) and the rock's material is invisible on a
    // meadow, and every octave is a noise evaluation in the crate's hottest pass — the
    // mountains' opaque pass is 0.138 ms on the baseline build.
    let upness = turf_gate(n.y);
    let rockness = w.x;
    let turf = upness * (1.0 - rockness);
    var tint = vec3<f32>(1.0);
    var grad = vec3<f32>(0.0);
    if turf > 0.01 {
        let grain = ground_detail(pv, footprint);
        // Under detail textures the procedural grain's *albedo* keeps only its share
        // (`GroundTextures::grain`): its mottle is the felt the textures replace. Its *relief*
        // stays whole: the turf's 10–50 cm bumps are the ground's micro-topography, which a low
        // sun reveals across the mid field, where the textures' own normals have averaged flat.
        tint = tint * mix(1.0, grain.albedo, turf * material.ground.w);
        grad = grad + grain.grad * turf;
        // The turf's palette: a meadow is not one green (`ground-colour.md`). The palette's
        // *clump* is the detail's own fine field, which the turf has already evaluated for its
        // grain, so the near field has colour variation in it for no extra noise.
        tint = tint * mix(vec3<f32>(1.0), turf_palette(pv, plain, fields.large, grain.coarse, grain.fine), turf);
    }
    // The scree: the stone that has weathered off the rock above lies *below* it, on the band
    // of slope just under the rock's ramp. Without it the grass meets the rock in a single
    // step, and a mountain's foot is never a clean line — it is a scatter of stones thinning
    // into the turf.
    //
    // The band is a *band* and not a ramp: the scree is at the rock's **foot**, so it lies
    // where the soil's band dies (it is gone by 30°) and the rock's ramp begins (it opens at
    // 30°). A mask that rose with the slope would put the stones on the cliff, which is
    // where the rock already is, and one that sat on the soil's band would put them in the
    // middle of the meadow's bare earth.
    //
    // The band is *wide*, from the soil's own band up to the rock's ramp: a scree is an apron
    // of stones, tens of metres deep on a big cliff, and a narrow band traces the hillside's
    // break as a contour line, which is the one thing a scree is not. The band's weight falls
    // towards both ends, so the stones are dense at the cliff's foot and thin out into the
    // turf above and the bare rock below.
    //
    // The scatter is *stones*, not a colour (`docs/research/scree.md`): the cell network's
    // cells are the stones, each with its own size and its own relief, and the turf shows
    // between them.
    let scree = smoothstep(16.0, 26.0, alpha) * (1.0 - smoothstep(31.0, 40.0, alpha)) * (1.0 - rockness);
    if scree > 0.01 {
        // The ground *between* the stones is the scree's own matrix, not the turf: a scree has
        // no grass in it, and a lawn with pebbles on it is a lawn. The matrix is the crate's
        // own SOIL tone, a little greyer: a scree's matrix is the dirt between the stones, and
        // at full SOIL it read as warm bare earth (a dirt road tracing the hillside). It is
        // *darker* than the stones, which is what makes them stand out: light grey rock on
        // dark brown dirt. The *edges* of the band are broken by the same `detail` field the
        // layer boundaries use, so the band does not end on a contour line.
        let edge = overlay(smoothstep(0.5, 1.0, scree), detail);
        let scree_soil = mix(SOIL, ROCK * 1.5, 0.50);
        albedo = mix(albedo, scree_soil, edge * 0.85);
        let stones = scree_scatter(pv.xz, footprint);
        if stones.present > 0.0 {
            // The stone's *occlusion* of the matrix beside it: the contact shadow that makes
            // the stones sit in the dirt instead of on it.
            albedo = albedo * (1.0 - 0.45 * stones.contact * scree);
            // A stone is the *rock*, not a dark patch of whatever it lies on: the stone's own
            // colour *replaces* the ground's. So the stone is mixed into the albedo, not
            // multiplied into the tint.
            let stone = ROCK * stones.tint;
            albedo = mix(albedo, stone, stones.coverage * scree);
            // The stones' relief, so the sun rakes their edges and they read as things.
            grad = grad + vec3<f32>(stones.grad.x, 0.0, stones.grad.y) * scree;
        }
    }
    if rockness > 0.01 {
        let rock = rock_material(pv, fields.large, footprint);
        tint = tint * mix(vec3<f32>(1.0), rock.tint, rockness);
        grad = grad + rock.grad * rockness;
    }
    // A face sees less of the sky than the ground does, and a steep face sees much less. The
    // crate's `overcast_relief` does the same under a cloud deck; here it is the rock's own
    // occlusion, and without it a cliff is lit at 3.4 times its albedo and reads as fog (a face
    // at 6 m rendered at 137 DN with a saturation of 8 — a washed-out grey, not stone).
    tint = tint * mix(1.0, rock_sky_occlusion(n.y), rockness);
    albedo = albedo * tint;
    // The detail textures' relief, already weighted by the ground layers' shares.
    grad = grad + vec3<f32>(maps.slope.x, 0.0, maps.slope.y);
    n = normalize(n - (grad - dot(grad, n) * n));

    // Ground cover: wet darkening and gloss, puddles with ripples while it rains, snow
    // (docs/research/wetness.md, snow-cover.md). The water depth is the hydrology layer's.
    var wet = 0.0;
    var snow_cover = 0.0;
    var puddle = 0.0;
    var ripple = vec2<f32>(0.0);
    if material.cover.x > 0.5 {
        let k00 = cover_cells[t.i00];
        let k10 = cover_cells[t.i10];
        let k01 = cover_cells[t.i01];
        let k11 = cover_cells[t.i11];
        let cover = mix(mix(k00.xy, k10.xy, t.w.x), mix(k01.xy, k11.xy, t.w.x), t.w.y);
        let water = mix(mix(c00.z, c10.z, t.w.x), mix(c01.z, c11.z, t.w.x), t.w.y);
        // A trench pressed to the bottom of the snow shows the ground under it.
        snow_cover = smoothstep(0.0, 0.05, cover.x) * (1.0 - smoothstep(cover.x - 0.02, cover.x, depth));
        wet = cover.y;
        puddle = smoothstep(material.cover.z, material.cover.w, water);
        // Ruts in saturated ground hold water on their floors (not their walls).
        let level = 1.0 - smoothstep(0.3, 0.8, length(trail.grad));
        puddle = max(puddle, smoothstep(0.6, 1.0, wet) * smoothstep(0.01, 0.04, depth) * level * (1.0 - snow_cover));
        if puddle > 0.0 && material.cover.y > 0.0 {
            ripple = ripple_offset(pv.xz, material.ripple, ripple_weights(material.cover.y));
        }
    }
    let surface = weather_surface(
        albedo, rough, 0.0, pbr_input.material.reflectance, n, wet, puddle, snow_cover, ripple,
    );
    albedo = surface.base_color;
    n = surface.normal;
    // Compacted snow on a trench floor: smoother, and bluish where the light reaches it through
    // the snow walls (Barré-Brisebois 2014's transition tint; trails.md Deviation 1).
    let packed = snow_cover * pressed;
    albedo = albedo * mix(vec3<f32>(1.0), COMPACTED_SNOW_TINT, packed);
    let rough_out = mix(surface.roughness, surface.roughness * 0.75, packed);

    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);
    pbr_input.material.perceptual_roughness = rough_out;
    pbr_input.material.metallic = surface.metallic;
    pbr_input.material.reflectance = surface.reflectance;
    pbr_input.world_normal = n;
    pbr_input.N = n;
    pbr_input.world_position = vec4<f32>(pv * material.units, 1.0);
    pbr_input.diffuse_occlusion = pbr_input.diffuse_occlusion * trail.open * maps.occlusion;

    let ratio = vec3<f32>(cloud_shadow_ratio(p)) * bed_transmittance(water_depth);
    // The trail's self-shadow removes the beam's share of the light inside its trenches.
    let trail_shade = 1.0 - material.trail_light.x * (1.0 - trail.lit);
@if(PREPASS_PIPELINE) {
    // Deferred: the lighting runs later, so the shadow scales the albedo instead.
    pbr_input.material.base_color = vec4<f32>(albedo * ratio * trail_shade, 1.0);
    return deferred_output(in, pbr_input);
}
@if(!PREPASS_PIPELINE) {
    var out: FragmentOutput;
    // The overcast dome shades the surface through the environment map (the clouds' sky probe,
    // `docs/research/sky-light.md`); the relief cue adds what the dome lacks, ramped by the deck
    // over this ground (`cloud_deck`, 1 in clear air — exactly no change there).
    out.color = apply_pbr_lighting(pbr_input);
    out.color = vec4<f32>(
        out.color.rgb * overcast_relief(pbr_input.N.y, 1.0 - cloud_deck(p)) * ratio * trail_shade,
        out.color.a,
    );
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
}
