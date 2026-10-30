// Composite the resolved cloud history over the scene: `scene · T + L` with the blend
// `src.rgb + dst.rgb · src.a`. A pixel whose scene surface is nearer than the cloud entry of its
// ray shows no cloud. See docs/research/clouds.md ("Aerial perspective and composite").

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;
import bevy_elements::clouds::common::{CloudParams, ray_dir, scene_distance, shell_range};

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> params: CloudParams;
@group(0) @binding(2) var history: texture_2d<f32>;
@group(0) @binding(3) var depth_tex: texture_depth_2d;

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
    return textureLoad(history, pixel, 0);
}
