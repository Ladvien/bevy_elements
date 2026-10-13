// GroundExtension: a consumer's mesh shaded as the terrain's ground. Water standing on the ground
// darkens it (the wet-shading ramp of docs/research/wetness.md, driven by the water depth until
// M4's wetness map) and cloud shadows fall on it, both by world XZ. Vertex stage and geometry stay
// the consumer's. See docs/research/terrain.md ("GroundExtension").

import bevy_elements::grid::{GridParams, grid_taps};
import bevy_elements::terrain::common::{TerrainParams, terrain_live};
import bevy_pbr::render::{
    mesh_view_bindings::lights,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
};

struct GroundParams {
    terrain: TerrainParams,
    // xy: cloud shadow map origin (world XZ), z: its extent, w: camera transmittance.
    shadow: vec4<f32>,
    // xyz: unit direction toward the light the map was made for. w: 1 when a map is bound.
    light: vec4<f32>,
    // xyz: the water's absorption (1/m). w: its refractive index (0 without water rendering).
    absorption: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> ground: GroundParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<storage, read> terrain_cells: array<vec4<f32>>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var cloud_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var cloud_sampler: sampler;

// Water depth above the ground (m) at world XZ; 0 without a terrain.
fn water_depth(xz: vec2<f32>) -> f32 {
    if !terrain_live(ground.terrain) {
        return 0.0;
    }
    let t = grid_taps(ground.terrain.grid, xz);
    if !t.inside {
        return 0.0;
    }
    let a = mix(terrain_cells[t.i00].z, terrain_cells[t.i10].z, t.w.x);
    let b = mix(terrain_cells[t.i01].z, terrain_cells[t.i11].z, t.w.x);
    return mix(a, b, t.w.y);
}

// The sun's way down to the ground through `depth` metres of water, per channel: `exp(-σ d / μ')`
// (docs/research/water-rendering.md, "The bed under the water").
fn bed_transmittance(depth: f32) -> vec3<f32> {
    let n = ground.absorption.w;
    if n <= 1.0 || depth <= 0.0 || lights.n_directional_lights == 0u {
        return vec3<f32>(1.0);
    }
    let mu = clamp(lights.directional_lights[0].direction_to_light.y, 0.05, 1.0);
    let mu_r = sqrt(max(1.0 - (1.0 - mu * mu) / (n * n), 0.01));
    return exp(-ground.absorption.xyz * depth / mu_r);
}

fn cloud_shadow_ratio(p: vec3<f32>) -> f32 {
    let l = ground.light.xyz;
    var t = 1.0;
    if ground.light.w > 0.5 && l.y > 0.1 {
        let uv = (p.xz - l.xz * (p.y / l.y) - ground.shadow.xy) / ground.shadow.z;
        if all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)) {
            t = textureSampleLevel(cloud_map, cloud_sampler, uv, 0.0).r;
        }
    }
    let here = 0.25 + 0.75 * t;
    let camera = max(0.25 + 0.75 * ground.shadow.w, 0.01);
    return here / camera;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let depth = water_depth(in.world_position.xz);
    let wet = smoothstep(0.0, 0.02, depth);
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * (1.0 - 0.5 * wet), pbr_input.material.base_color.a);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = vec4<f32>(out.color.rgb * cloud_shadow_ratio(in.world_position.xyz) * bed_transmittance(depth), out.color.a);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
