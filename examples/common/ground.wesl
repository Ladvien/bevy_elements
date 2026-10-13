// The example ground: StandardMaterial lighting scaled by the cloud shadow map, as the research
// note's consumer mapping prescribes (docs/research/clouds.md, "Cloud shadow map"):
//
//     uv = (P.xz − l.xz · P.y / l.y − origin) / extent,   T = map(uv).r  (1 outside the map)
//
// The sun light already carries the factor 0.25 + 0.75 · T̄ for the camera; the pixel is scaled by
// the ratio of its own factor to the camera's.

import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
};

struct CloudShadowParams {
    // xy: map origin (world XZ), z: map extent, w: camera transmittance.
    map: vec4<f32>,
    // xyz: unit direction toward the light.
    light: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> cloud: CloudShadowParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var cloud_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var cloud_sampler: sampler;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    let l = cloud.light.xyz;
    var t = 1.0;
    if l.y > 0.1 {
        let p = in.world_position.xyz;
        let uv = (p.xz - l.xz * (p.y / l.y) - cloud.map.xy) / cloud.map.z;
        if all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)) {
            t = textureSampleLevel(cloud_map, cloud_sampler, uv, 0.0).r;
        }
    }
    let here = 0.25 + 0.75 * t;
    let camera = max(0.25 + 0.75 * cloud.map.w, 0.01);
    out.color = vec4<f32>(out.color.rgb * (here / camera), out.color.a);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
