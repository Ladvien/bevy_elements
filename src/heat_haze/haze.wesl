// Heat haze: each pixel's ray is tested against every plume's axis. A plume it passes, in front
// of the pixel's own surface, bends it by the gradient of the plume's projected index deficit: the
// mean lens (a Gaussian profile) and two octaves of turbulence moving up the plume. The background
// point the bent ray reaches is drawn instead: a shift of `ε D / (Z + D)` for a background `D`
// behind the plume (Raffel 2015, Eq. 3), so the ground the fire stands on does not move. A shift
// that would land on a surface in front of the plume is refused (RTR4 §14.5.2). See
// docs/research/heat-haze.md; `src/heat_haze/model.rs` has the same formulas on the CPU.

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;
import bevy_elements::noise::{hash31, perlin3_d};

struct HazeSource {
    base: vec4<f32>,
    axis: vec4<f32>,
    fire: vec4<f32>,
    look: vec4<f32>,
}

struct HazeParams {
    globals: vec4<f32>,
    optics: vec4<f32>,
    sources: array<HazeSource, 16>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> params: HazeParams;
@group(0) @binding(2) var scene: texture_2d<f32>;
@group(0) @binding(3) var depth_tex: texture_depth_2d;
@group(0) @binding(4) var smp: sampler;

const SQRT_PI: f32 = 1.7724539;
const TWO_PI: f32 = 6.2831853;
/// McCaffrey's regimes in `z* = z / Q^{2/5}` and `ΔT/T∞ = (κ/(0.9√(2g)))² z*^{2η−1}`.
const FLAME_END: f32 = 0.08;
const INTERMITTENT_END: f32 = 0.2;
const FLAME_EXCESS: f32 = 2.909464;
const INTERMITTENT_EXCESS: f32 = 0.2271367;
const PLUME_EXCESS: f32 = 0.0761336;
/// Rise time `τ(z*)` at the regime joins: `2√0.08 / 6.8` and that plus `0.12 / 1.9`; `0.2^{4/3}`.
const RISE_FLAME_END: f32 = 0.0831890;
const RISE_INTERMITTENT_END: f32 = 0.1463469;
const INTERMITTENT_END_4_3: f32 = 0.1169607;
/// The plume's temperature 1/e radius over height (Shabbir and George 1994).
const PLUME_WIDTH: f32 = 0.121;
/// Mean temperature fluctuation over the mean excess (Shabbir and George 1994).
const FLUCTUATION: f32 = 0.40;
/// Rms of one component of `perlin3_d`'s gradient per lattice unit (`model::NOISE_GRADIENT_RMS`).
const NOISE_GRADIENT_RMS: f32 = 1.0;
/// The second octave's eddies are 2.5 times smaller and bend 2.5^{1/6} times more (Kolmogorov);
/// together the two octaves have the rms of the single-scale estimate: `1/√(1 + 1.165²)`.
const OCTAVE_RATIO: f32 = 2.5;
const OCTAVE_GAIN: f32 = 1.165;
const OCTAVE_NORM: f32 = 0.6512;
/// The path through the plume grows as `1/sin α`, held at `α ≥ 20°`.
const MIN_SIN: f32 = 0.342;
/// The sky's distance, and how far behind the plume an orthographic view's sky stands, m.
const NO_SURFACE: f32 = 1e30;
const ORTHO_SKY_M: f32 = 1000.0;

/// `ΔT/T∞` on the plume's axis.
fn excess_ratio(zs: f32) -> f32 {
    if zs < FLAME_END {
        return FLAME_EXCESS;
    }
    if zs < INTERMITTENT_END {
        return INTERMITTENT_EXCESS / zs;
    }
    return PLUME_EXCESS * pow(zs, -1.6666667);
}

/// The centreline's rise speed, m/s.
fn rise_speed(zs: f32, q_fifth: f32) -> f32 {
    if zs < FLAME_END {
        return 6.8 * sqrt(max(zs, 1e-4)) * q_fifth;
    }
    if zs < INTERMITTENT_END {
        return 1.9 * q_fifth;
    }
    return 1.1 * pow(zs, -0.33333334) * q_fifth;
}

/// How long the gas took to rise from the fire to `zs`, s.
fn rise_time(zs: f32, q_fifth: f32) -> f32 {
    var rise: f32;
    if zs < FLAME_END {
        rise = 2.0 * sqrt(max(zs, 0.0)) / 6.8;
    } else if zs < INTERMITTENT_END {
        rise = RISE_FLAME_END + (zs - FLAME_END) / 1.9;
    } else {
        rise = RISE_INTERMITTENT_END + 0.75 * (pow(zs, 1.3333334) - INTERMITTENT_END_4_3) / 1.1;
    }
    return rise * q_fifth;
}

/// The plume's turbulence at `(p, s)`: one crossfade layer, both octaves, as a deflection across
/// (`x`) and up (`y`) the plume per unit of rms; eddies of size `scale`, smaller than `footprint`
/// fade out.
fn turbulence(
    p: f32,
    s: f32,
    rise: f32,
    phase: f32,
    period: f32,
    cycle: i32,
    layer: i32,
    seed: i32,
    scale: f32,
    footprint: f32,
) -> vec2<f32> {
    let lifted = s - rise * phase * period;
    let jitter = vec3<f32>(hash31(vec3<i32>(cycle, layer, seed)), hash31(vec3<i32>(seed, cycle, layer)), hash31(vec3<i32>(layer, seed, cycle))) * 512.0;
    var sum = vec2<f32>(0.0);
    let fine = scale / OCTAVE_RATIO;
    let coarse_n = perlin3_d(vec3<f32>(p / scale, lifted / scale, 0.0) + jitter);
    sum += coarse_n.yz * smoothstep(1.0, 3.0, scale / footprint);
    let fine_n = perlin3_d(vec3<f32>(p / fine, lifted / fine, 17.0) + jitter);
    sum += fine_n.yz * (OCTAVE_GAIN * smoothstep(1.0, 3.0, fine / footprint));
    return sum;
}

@fragment
fn heat_haze(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pix = vec2<i32>(floor(in.position.xy));
    let clock = params.globals.x;
    let units = params.globals.y;
    let count = u32(params.globals.z);
    let limit = params.globals.w;
    let index_scale = params.optics.x;
    let vp = view.viewport;
    let uv = (in.position.xy - vp.xy) / vp.zw;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let ortho = view.clip_from_view[3].w == 1.0;

    // The pixel's ray in metres: from the eye, or along the view from the near plane.
    let near_h = view.world_from_clip * vec4<f32>(ndc, 1.0, 1.0);
    let near_m = near_h.xyz / near_h.w / units;
    var origin = view.world_position / units;
    var dir: vec3<f32>;
    if ortho {
        let mid_h = view.world_from_clip * vec4<f32>(ndc, 0.5, 1.0);
        dir = normalize(mid_h.xyz / mid_h.w / units - near_m);
        origin = near_m;
    } else {
        dir = normalize(near_m - origin);
    }
    let depth = textureLoad(depth_tex, pix, 0);
    var surface = NO_SURFACE;
    if depth > 0.0 {
        let hit = view.world_from_clip * vec4<f32>(ndc, depth, 1.0);
        surface = dot(hit.xyz / hit.w / units - origin, dir);
    }
    // A pixel's size: radians (perspective) or metres (orthographic).
    let pixel = 2.0 / (view.clip_from_view[1][1] * vp.w);

    var bend = vec3<f32>(0.0);
    var shove = vec3<f32>(0.0);
    var nearest = NO_SURFACE;
    for (var i = 0u; i < count; i += 1u) {
        let src = params.sources[i];
        let ax = src.axis.xyz;
        let from_base = origin - src.base.xyz;
        let b = dot(dir, ax);
        let d = dot(dir, from_base);
        let e = dot(ax, from_base);
        let denom = max(1.0 - b * b, 1e-4);
        // The closest approach of the ray (at `t`) to the axis (at height `s`).
        let t = (b * e - d) / denom;
        let s = (e - b * d) / denom;
        let top = src.base.w;
        if s <= 0.0 || s >= top {
            continue;
        }
        let radius = src.axis.w;
        let sigma = sqrt(radius * radius + PLUME_WIDTH * PLUME_WIDTH * s * s);
        if t < -sigma || surface < t - sigma {
            continue;
        }
        let gap = (origin + dir * t) - (src.base.xyz + ax * s);
        let side = cross(dir, ax);
        let across = side / max(length(side), 1e-6);
        let p = dot(gap, across);
        if abs(p) > 3.0 * sigma {
            continue;
        }
        let tc = max(t, 0.0);
        let q_two_fifths = src.fire.x;
        let q_fifth = src.fire.y;
        let zs = s / q_two_fifths;
        let ratio = excess_ratio(zs);
        let deficit = index_scale * ratio / (1.0 + ratio) * (1.0 - smoothstep(0.7 * top, top, s));
        let path = 1.0 / max(sqrt(denom), MIN_SIN);
        let pulse = 1.0 + 0.5 * exp(-s / src.fire.w) * sin(TWO_PI * src.fire.z * (clock - rise_time(zs, q_fifth)));
        var footprint = pixel / units;
        if !ortho {
            footprint = pixel * max(tc, 1e-3);
        }
        let q = p / sigma;
        let gauss = exp(-q * q);

        // The mean lens: the hot core diverges the light passing it.
        var across_eps = 2.0 * SQRT_PI * deficit * q * gauss * path * pulse * smoothstep(0.5, 2.0, sigma / footprint);

        // The turbulence: two crossfading layers, each restarting with a fresh pattern every period.
        let period = src.look.x;
        let seed = i32(src.look.y);
        let scale = 0.5 * sigma;
        let rise = rise_speed(zs, q_fifth);
        let phase_a = fract(clock / period);
        let phase_b = fract(clock / period + 0.5);
        let weight_a = 1.0 - abs(2.0 * phase_a - 1.0);
        let weight_b = 1.0 - weight_a;
        let cycle_a = i32(floor(clock / period));
        let cycle_b = i32(floor(clock / period + 0.5));
        let wobble = turbulence(p, s, rise, phase_a, period, cycle_a, 0, seed, scale, footprint) * weight_a
            + turbulence(p, s, rise, phase_b, period, cycle_b, 1, seed, scale, footprint) * weight_b;
        let rms = FLUCTUATION * deficit * 2.0 * sqrt(path) * pulse * gauss * OCTAVE_NORM / NOISE_GRADIENT_RMS
            / sqrt(weight_a * weight_a + weight_b * weight_b);
        across_eps += rms * wobble.x;
        let up_raw = ax - dir * b;
        let up = up_raw / max(length(up_raw), 1e-6);
        let eps = across * across_eps + up * (rms * wobble.y);

        if surface >= NO_SURFACE {
            if ortho {
                shove += eps * ORTHO_SKY_M;
            } else {
                bend += eps;
            }
        } else {
            shove += eps * max(surface - tc, 0.0);
        }
        nearest = min(nearest, tc);
    }

    if nearest >= NO_SURFACE {
        return textureLoad(scene, pix, 0);
    }
    // Where the bent ray shows the background point.
    var clip: vec4<f32>;
    if surface >= NO_SURFACE && !ortho {
        clip = view.clip_from_world * vec4<f32>(dir + bend, 0.0);
    } else {
        var far = surface;
        if surface >= NO_SURFACE {
            far = nearest + ORTHO_SKY_M;
        }
        clip = view.clip_from_world * vec4<f32>((origin + dir * far + shove) * units, 1.0);
    }
    let shifted_ndc = clip.xy / clip.w;
    let shifted_uv = vec2<f32>(shifted_ndc.x * 0.5 + 0.5, 0.5 - shifted_ndc.y * 0.5);
    var delta = (shifted_uv - uv) * vp.zw;
    let len = length(delta);
    if len > limit {
        delta *= limit / len;
    }
    if len < 1e-3 {
        return textureLoad(scene, pix, 0);
    }
    let size = vec2<f32>(textureDimensions(scene));
    let at = in.position.xy + delta;
    let at_pix = clamp(vec2<i32>(floor(at)), vec2<i32>(0), vec2<i32>(size) - vec2<i32>(1));
    // A sample on a surface in front of the plume would pull that surface into the haze.
    let plume_clip = view.clip_from_world * vec4<f32>((origin + dir * nearest) * units, 1.0);
    if textureLoad(depth_tex, at_pix, 0) > plume_clip.z / plume_clip.w {
        return textureLoad(scene, pix, 0);
    }
    return textureSampleLevel(scene, smp, at / size, 0.0);
}
