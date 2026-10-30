// Composite the resolved cloud history over the scene: `scene · T + L` with the blend
// `src.rgb + dst.rgb · src.a`. A pixel whose scene surface is nearer than the cloud entry of its
// ray shows no cloud. See docs/research/clouds.md ("Aerial perspective and composite").

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;
import bevy_elements::clouds::common::{
    CloudParams, ground_before_layer, is_orthographic, ray_meets_sea, scene_distance, shell_range,
    sky_through_clouds, view_ray,
};

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> params: CloudParams;
@group(0) @binding(2) var history: texture_2d<f32>;
@group(0) @binding(3) var depth_tex: texture_depth_2d;
@group(0) @binding(4) var weather_tex: texture_2d<f32>;
@group(0) @binding(5) var bow_tex: texture_2d<f32>;
@group(0) @binding(6) var smp_repeat: sampler;
@group(0) @binding(7) var smp_clamp: sampler;

/// The scattering angles the phase-function table spans: cosines of 110° and 170°.
const BOW_COS_NEAR: f32 = -0.34202;
const BOW_COS_FAR: f32 = -0.98481;
/// The table's angles, radians (110° and 60° more).
const BOW_THETA_MIN: f32 = 1.91986;
const BOW_THETA_SPAN: f32 = 1.04720;
/// Rain's extinction, m⁻¹, at `R` mm/h is `2.12e-4 R^0.68` (docs/research/precipitation.md); the
/// weather map's green channel is `R / 25`.
const RAIN_EXTINCTION: f32 = 2.12e-4;
/// The curtain's taps: horizontal distances `1500 · 2^j` m (1.5, 3, 6 and 12 km), each standing for
/// the length `x` (the first) or `0.75 x` (the rest) of the ray's ground track.

/// The light a shower sends back along a sky ray, `E · p(θ) · (1 − e^{−τ})`, display-referred: the
/// phase function of the table at the angle to the sun, and the curtain's optical depth from the
/// weather map where the ray is still under the cloud base.
fn rainbow_light(cam: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    let c = dot(dir, params.light.xyz);
    // A bow is a pattern of angles: an orthographic view has one direction for every pixel.
    if params.bow.w < 0.0 || is_orthographic(view.clip_from_view) || c > BOW_COS_NEAR || c < BOW_COS_FAR || dir.y < -0.02 {
        return vec3<f32>(0.0);
    }
    let theta = acos(c);
    let table = textureSampleLevel(
        bow_tex,
        smp_clamp,
        vec2<f32>((theta - BOW_THETA_MIN) / BOW_THETA_SPAN, params.bow.w),
        0.0,
    ).rgb;
    let flat_len = max(length(dir.xz), 1.0e-4);
    let tan_e = max(dir.y, 0.0) / flat_len;
    let heading = dir.xz / flat_len;
    var tau = 0.0;
    for (var j = 0; j < 4; j += 1) {
        let x = 1500.0 * exp2(f32(j));
        let stand = select(0.75 * x, x, j == 0);
        // The shower stands from the ground to the cloud base.
        let height = cam.y + x * tan_e;
        let reach = 1.0 - smoothstep(0.7 * params.layer.x, params.layer.x, height);
        let xz = cam.xz + heading * x;
        let g = textureSampleLevel(weather_tex, smp_repeat, (xz - params.weather.xy) / params.weather.z, 0.0).g;
        let k = RAIN_EXTINCTION * pow(g * 25.0, 0.68);
        tau += k * stand / flat_len * reach;
    }
    return params.bow.xyz * table * (1.0 - exp(-tau)) * view.exposure;
}

@fragment
fn cloud_composite(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Debug mode 4: the composite adds nothing (the tests render the same frame without it).
    if params.misc.y > 3.5 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let pixel = vec2<i32>(in.position.xy);
    let depth = textureLoad(depth_tex, pixel, 0);
    let ray = view_ray(
        in.uv,
        view.world_from_clip,
        view.world_position,
        is_orthographic(view.clip_from_view),
        params.camera.xyz,
        params.units.x,
    );
    let cam = ray.origin;
    let dir = ray.dir;
    if depth > 0.0 {
        let big_r = params.camera.w;
        let shell = shell_range(big_r + cam.y, dir.y, params.layer.x, params.layer.y, big_r);
        if scene_distance(in.uv, depth, view.world_from_clip, ray, params.units.x) < shell.x {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
    }
    // The still sea draws the pixels whose ray meets it; the slab below is for what lies past it.
    if depth <= 0.0 && ray_meets_sea(cam, dir, params) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    var cloud = textureLoad(history, pixel, 0);
    if depth <= 0.0 {
        // The sky: the cloud over Bevy's atmosphere, with the sky gain and the weather's air
        // (`sky_through_clouds`; the sky probe composes its cube the same way). A ray that meets
        // the planet before the layer (past the edge of a finite ground) ends beneath the deck,
        // which the beam lights correctly: no gain there (clouds.md Deviation 28).
        let big_r = params.camera.w;
        let o_r = big_r + cam.y;
        let shell = shell_range(o_r, dir.y, params.layer.x, params.layer.y, big_r);
        if ground_before_layer(o_r, dir.y, shell.x, big_r) < 0.0 {
            // The shower is nearer than the cloud and in front of it: the cloud's transmittance
            // does not dim it, and the air between dims it with the cloud.
            cloud = vec4<f32>(cloud.rgb + rainbow_light(cam, dir), cloud.a);
            cloud = sky_through_clouds(cloud, shell, params, params.haze.xyz);
        }
    }
    return cloud;
}
