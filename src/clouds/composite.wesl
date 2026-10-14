// Composite the resolved cloud history over the scene: `scene · T + L` with the blend
// `src.rgb + dst.rgb · src.a`. A pixel whose scene surface is nearer than the cloud entry of its
// ray shows no cloud. See docs/research/clouds.md ("Aerial perspective and composite").

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;
import bevy_elements::clouds::common::{
    CloudParams, ground_before_layer, ray_dir, scene_distance, shell_range,
};

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> params: CloudParams;
@group(0) @binding(2) var history: texture_2d<f32>;
@group(0) @binding(3) var depth_tex: texture_depth_2d;

/// Transmittances below this are an opaque cloud; above the second the sky is really in view.
const OPAQUE_BELOW: f32 = 0.02;
const SKY_ABOVE: f32 = 0.2;

@fragment
fn cloud_composite(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Debug mode 4: the composite adds nothing (the tests render the same frame without it).
    if params.misc.y > 3.5 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let pixel = vec2<i32>(in.position.xy);
    let depth = textureLoad(depth_tex, pixel, 0);
    if depth > 0.0 {
        let cam = view.world_position;
        let dir = ray_dir(in.uv, view.world_from_clip, cam);
        let big_r = params.camera.w;
        let shell = shell_range(big_r + cam.y, dir.y, params.layer.x, params.layer.y, big_r);
        if scene_distance(in.uv, depth, view.world_from_clip, cam) < shell.x {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
    }
    var cloud = textureLoad(history, pixel, 0);
    if depth <= 0.0 {
        // The sky: Bevy's atmosphere is lit by the sun light, which carries only the beam
        // `T̄` under cloud, but the sky above the cloud is not dimmed by the cloud below it.
        // The march stops at a transmittance of 0.01, so the gain ramps in above that: an
        // opaque cloud must not amplify the sky behind it. A ray that meets the planet before
        // the layer (past the edge of a finite ground) ends beneath the deck, which the beam
        // lights correctly: no gain there (clouds.md Deviation 28).
        let cam = view.world_position;
        let dir = ray_dir(in.uv, view.world_from_clip, cam);
        let big_r = params.camera.w;
        let o_r = big_r + cam.y;
        let shell = shell_range(o_r, dir.y, params.layer.x, params.layer.y, big_r);
        if ground_before_layer(o_r, dir.y, shell.x, big_r) < 0.0 {
            cloud.a *= mix(1.0, params.sky.x, smoothstep(OPAQUE_BELOW, SKY_ABOVE, cloud.a));
        }
    }
    return cloud;
}
