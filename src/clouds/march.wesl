// The cloud ray march, one texel per k×k block of pixels (k = 4 or 2), one pixel of each block
// per frame. Output: premultiplied, exposed radiance (rgb) and transmittance (a). See
// docs/research/clouds.md ("Ray march", "Lighting at a fine sample", "Aerial perspective").

import bevy_render::view::View;
import bevy_elements::noise::hash31;
import bevy_elements::clouds::common::{
    CloudParams, Medium, MAX_MARCH_M, PI, altitude, cloud_medium, coarse_shape, light_at,
    phase_octave, ray_dir, sample_transmittance, scene_distance, shell_range, slot_offset,
};

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> params: CloudParams;
@group(0) @binding(2) var base_tex: texture_3d<f32>;
@group(0) @binding(3) var detail_tex: texture_3d<f32>;
@group(0) @binding(4) var weather_tex: texture_2d<f32>;
@group(0) @binding(5) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(6) var aerial_lut: texture_3d<f32>;
@group(0) @binding(7) var depth_tex: texture_depth_2d;
@group(0) @binding(8) var smp_repeat: sampler;
@group(0) @binding(9) var smp_clamp: sampler;
@group(0) @binding(10) var march_out: texture_storage_2d<rgba16float, write>;

/// Consecutive empty fine samples after which the march returns to coarse steps (Nubis).
const EMPTY_LIMIT: u32 = 10u;
/// Transmittance below which the march stops (Högfeldt, Schneider's alpha → 1).
const MIN_TRANSMITTANCE: f32 = 0.01;
/// Tangent of the light-sample cone half angle.
const CONE_TAN: f32 = 0.1;
/// Bottom weight of the ambient gradient (ground bounce).
const AMBIENT_BOTTOM: f32 = 0.2;
/// Distance at which cloud extinction starts to fade toward the march limit, m.
const FADE_START_M: f32 = 16000.0;

fn medium_at(h: f32, xz: vec2<f32>, detail: bool) -> Medium {
    return cloud_medium(h, xz, detail, params, base_tex, detail_tex, weather_tex, smp_repeat);
}

/// Optical depth toward the light from the point `off` (offset from the camera), m⁻¹·m: six
/// samples in a cone, base shape only. Segment lengths 30, 60, 120, 240, 480 m and a far sample
/// at 1800 m covering 1740 m; times (1 + g) per sample. Each sample sits at a fixed golden-angle
/// azimuth around the light direction (Schneider's fixed cone kernel): the lighting is a smooth
/// function of position, so pixels do not carry independent noise.
fn light_optical_depth(off: vec3<f32>, l: vec3<f32>, cam: vec3<f32>) -> f32 {
    var dist = array<f32, 6>(15.0, 60.0, 150.0, 330.0, 690.0, 1800.0);
    var len = array<f32, 6>(30.0, 60.0, 120.0, 240.0, 480.0, 1740.0);
    let helper = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(l.y) > 0.99);
    let e1 = normalize(cross(l, helper));
    let e2 = cross(l, e1);
    var tau = 0.0;
    for (var j = 0u; j < 6u; j += 1u) {
        let phi = 2.39996323 * f32(j + 1u);
        let perp = cos(phi) * e1 + sin(phi) * e2;
        let o = off + l * dist[j] + perp * (CONE_TAN * dist[j]);
        let h = altitude(o.xz, cam.y + o.y, params.camera.w);
        let m = medium_at(h, cam.xz + o.xz, false);
        tau += (1.0 + m.g) * m.sigma * len[j];
    }
    return tau;
}

@compute @workgroup_size(8, 8, 1)
fn cloud_march(@builtin(global_invocation_id) id: vec3<u32>) {
    let low = params.sizes.zw;
    if any(id.xy >= low) {
        return;
    }
    let k = params.counts.z;
    let size = params.sizes.xy;
    let frame = params.counts.x;
    let px = min(id.xy * k + slot_offset(frame, k), size - vec2<u32>(1u));
    let uv = (vec2<f32>(px) + 0.5) / vec2<f32>(size);
    let cam = view.world_position;
    let dir = ray_dir(uv, view.world_from_clip, cam);
    let big_r = params.camera.w;
    let o_r = big_r + cam.y;
    let h_b = params.layer.x;
    let h_t = params.layer.y;

    let shell = shell_range(o_r, dir.y, h_b, h_t, big_r);
    let t0 = shell.x;
    let t1 = min(shell.y, MAX_MARCH_M);
    if t1 <= t0 {
        textureStore(march_out, vec2<i32>(id.xy), vec4<f32>(0.0, 0.0, 0.0, 1.0));
        return;
    }

    // A block whose pixels all show surfaces nearer than the cloud entry shows no cloud.
    var hidden = true;
    for (var j = 0u; j < k && hidden; j += 1u) {
        for (var i = 0u; i < k && hidden; i += 1u) {
            let q = min(id.xy * k + vec2<u32>(i, j), size - vec2<u32>(1u));
            let quv = (vec2<f32>(q) + 0.5) / vec2<f32>(size);
            let d = textureLoad(depth_tex, vec2<i32>(q), 0);
            let qdir = ray_dir(quv, view.world_from_clip, cam);
            let q_shell = shell_range(o_r, qdir.y, h_b, h_t, big_r);
            if scene_distance(quv, d, view.world_from_clip, cam) >= q_shell.x {
                hidden = false;
            }
        }
    }
    if hidden && params.misc.y < 0.5 {
        textureStore(march_out, vec2<i32>(id.xy), vec4<f32>(0.0, 0.0, 0.0, 1.0));
        return;
    }

    let l = params.light.xyz;
    if params.misc.y > 1.5 {
        // Debug: the light at the entry point, for the sun-colour test.
        let o = dir * t0;
        let h = altitude(o.xz, cam.y + o.y, big_r);
        let mu = (dot(o.xz, l.xz) + (big_r + cam.y + o.y) * l.y) / (big_r + h);
        let e = light_at(transmittance_lut, smp_clamp, h, mu, params) / max(params.light_e.xyz, vec3<f32>(1e-6));
        textureStore(march_out, vec2<i32>(id.xy), vec4<f32>(e, 1.0));
        return;
    }

    let steps = f32(params.counts.y);
    let dt = max(clamp((t1 - t0) / steps, 25.0, 400.0), (t1 - t0) / (1.5 * steps));
    let jitter = hash31(vec3<i32>(i32(px.x), i32(px.y), i32(frame)));
    let cos_t = dot(dir, l);
    let powder_weight = clamp(0.5 - 0.5 * cos_t, 0.0, 1.0);
    let ambient = params.light_e.w;

    var radiance = vec3<f32>(0.0);
    var transmittance = 1.0;
    var depth_sum = 0.0;
    var depth_weight = 0.0;
    var t = t0 + dt * jitter;
    var coarse = true;
    var empty = 0u;
    var resume = t;
    let max_iterations = params.counts.y * 3u;
    for (var it = 0u; it < max_iterations; it += 1u) {
        if t >= t1 || transmittance < MIN_TRANSMITTANCE {
            break;
        }
        let off = dir * t;
        let h = altitude(off.xz, cam.y + off.y, big_r);
        let xz = cam.xz + off.xz;
        if coarse {
            let shape = coarse_shape(h, xz, params, base_tex, weather_tex, smp_repeat);
            if shape > 0.0 {
                coarse = false;
                empty = 0u;
                t = max(t - 2.0 * dt, resume);
            } else {
                t += 2.0 * dt;
            }
            continue;
        }
        var m = medium_at(h, xz, true);
        // Distant cloud thins out with distance instead of ending in a hard wall at the march
        // limit (Deviation 11 of the research note).
        m.sigma *= 1.0 - smoothstep(FADE_START_M, MAX_MARCH_M, t);
        if m.sigma <= 0.0 {
            empty += 1u;
            if empty >= EMPTY_LIMIT {
                coarse = true;
                resume = t + dt;
            }
            t += dt;
            continue;
        }
        empty = 0u;
        // Each sample stands for the segment of length dt it starts (the jitter shifts the
        // sample inside its segment, not the segment).
        let seg = clamp(t1 - (t - dt * jitter), 0.0, dt);
        let step_t = exp(-m.sigma * seg);

        let mu_s = (dot(off.xz, l.xz) + (big_r + cam.y + off.y) * l.y) / (big_r + h);
        let sun = light_at(transmittance_lut, smp_clamp, h, mu_s, params);
        let tau_l = light_optical_depth(off, l, cam);
        let powder = mix(1.0, 1.0 - exp(-2.0 * tau_l), powder_weight);
        // Three octaves of multiple scattering: weight a^n, light-path extinction b^n, phase g c^n.
        var scatter = sun * (
            powder * exp(-tau_l) * phase_octave(cos_t, 1.0)
            + 0.5 * exp(-0.5 * tau_l) * phase_octave(cos_t, 0.5)
            + 0.25 * exp(-0.25 * tau_l) * phase_octave(cos_t, 0.25)
        );
        let f = (h - h_b) / (h_t - h_b);
        scatter += vec3<f32>(ambient * (AMBIENT_BOTTOM + (1.0 - AMBIENT_BOTTOM) * f));
        if params.flash.w > 0.0 {
            let d = length(cam + off - params.flash.xyz);
            let source = params.flash.w / (4.0 * PI * max(d * d, 2500.0))
                * exp(-0.5 * m.sigma * min(d, 150.0));
            scatter += vec3<f32>(source);
        }
        radiance += transmittance * scatter * (1.0 - step_t);
        depth_sum += transmittance * t;
        depth_weight += transmittance;
        transmittance *= step_t;
        t += dt;
    }

    if depth_weight <= 0.0 {
        textureStore(march_out, vec2<i32>(id.xy), vec4<f32>(0.0, 0.0, 0.0, 1.0));
        return;
    }

    // Aerial perspective at the mean cloud depth, then the fade into haze.
    let depth = depth_sum / depth_weight;
    let fade = smoothstep(20000.0, 32000.0, depth);
    let slices = params.atmo.z;
    let range = params.atmo.y;
    let slice = clamp(depth / range - 0.5 / slices, 0.0, 1.0);
    let aerial = textureSampleLevel(aerial_lut, smp_clamp, vec3<f32>(uv, slice), 0.0);
    let in_scatter = exp(aerial.rgb) * clamp(depth / (range / slices), 0.0, 1.0);
    let r_d = sqrt(max(o_r * o_r + 2.0 * o_r * depth * dir.y + depth * depth, 1.0));
    let mu_d = (o_r * dir.y + depth) / r_d;
    let t_cam = sample_transmittance(transmittance_lut, smp_clamp, o_r, dir.y, params);
    let t_far = sample_transmittance(transmittance_lut, smp_clamp, r_d, mu_d, params);
    let t_ap = min(t_cam / max(t_far, vec3<f32>(1e-6)), vec3<f32>(1.0));

    let rgb = (1.0 - fade) * (radiance * t_ap + in_scatter * (1.0 - transmittance));
    let alpha = 1.0 - (1.0 - fade) * (1.0 - transmittance);
    textureStore(march_out, vec2<i32>(id.xy), vec4<f32>(rgb * view.exposure, alpha));
}
