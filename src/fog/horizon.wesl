// The sky's own colour at the horizon: Bevy's sky-view LUT sampled along a band of directions just
// above the horizon, averaged over azimuth. `DistanceFog`'s colour is written from that mean, so a
// fogged surface, the far field and the space past a finite terrain converge on the sky actually
// behind them instead of on a fixed hue (`docs/research/horizon-colour.md`).
//
// The LUT is the exact function a sky pixel takes: `bevy_pbr-0.20.0/src/atmosphere/render_sky.wesl`
// samples it with `sample_sky_view_lut` for every sky pixel on the lookup path, and
// `sky_view_lut.wesl` fills it by raymarching Bevy's own medium. The value below is not a model of
// the sky; it is the sky.
//
// Wenzel 2006 §6.5 does the same and reports the simplification this takes: "all samples taken
// during the sky texture update for directions resembling the horizon are additionally averaged",
// and "using all of the horizon samples to produce a better matching fog color seems tempting but
// it was found that the difference to the average of all horizon samples is barely noticeable and
// doesn't justify the additional overhead". One mean over the horizon's directions is enough.
//
// One workgroup of `AZIMUTHS` threads, one azimuth each. The mean is the CPU's: 32 `vec4` is 512 B
// of readback and saves a workgroup reduction the shader would otherwise carry.
//
// The azimuth is measured in atmosphere space, whose reference differs from world's by a rotation
// about the local vertical. It does not matter: the set spans the whole circle, and the mean of a
// full turn is invariant to where the turn starts.

struct HorizonParams {
    // x: planet `inner_radius`, m. yz: the LUT's texels per side. w: unused.
    planet: vec4<f32>,
    // xy: the LUT's texels per side. z: the band's elevation above the true horizon, rad.
    // w: the camera's radius, m (`inner_radius` + altitude).
    view: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: HorizonParams;
@group(0) @binding(1) var sky_view_lut: texture_2d<f32>;
@group(0) @binding(2) var sky_lut_sampler: sampler;
@group(0) @binding(3) var<storage, read_write> samples: array<vec4<f32>>;

/// Azimuths averaged.
const AZIMUTHS: u32 = 32u;
const PI: f32 = 3.14159265;
const HALF_PI: f32 = 1.57079633;
const FRAC_2_PI: f32 = 0.15915494309;

/// `unit_to_sub_uvs` of `bevy_pbr-0.20.0/src/atmosphere/functions.wesl:57`, which keeps a sample
/// off a texel's edge: the LUT is sampled exactly as Bevy samples it.
fn unit_to_sub_uvs(val: vec2<f32>, resolution: vec2<f32>) -> vec2<f32> {
    return (val + 0.5 / resolution) * (resolution / (resolution + 1.0));
}

/// `sky_view_lut_r_mu_azimuth_to_uv` of the same file (`:78`): the horizon-detail mapping that
/// concentrates the LUT's rows at the horizon, so a band just above it is resolved.
fn sky_lut_uv(r: f32, mu: f32, azimuth: f32, ri: f32, size: vec2<f32>) -> vec2<f32> {
    let u = azimuth * FRAC_2_PI + 0.5;
    let v_horizon = sqrt(max(r * r - ri * ri, 0.0));
    let cos_beta = clamp(v_horizon / max(r, 1.0), -1.0, 1.0);
    let horizon_zenith = PI - acos(cos_beta);
    let view_zenith = acos(clamp(mu, -1.0, 1.0));
    let l = view_zenith - horizon_zenith;
    let v = 0.5 + 0.5 * sign(l) * sqrt(abs(l) / HALF_PI);
    return unit_to_sub_uvs(vec2<f32>(u, v), size);
}

@compute @workgroup_size(32, 1, 1)
fn fog_horizon(@builtin(local_invocation_index) i: u32) {
    if i >= AZIMUTHS {
        return;
    }
    let r = params.view.w;
    let azimuth = (f32(i) + 0.5) / f32(AZIMUTHS) * 2.0 * PI - PI;
    // The band's zenith cosine. The true horizon's zenith angle is `PI - acos(c)` with
    // `c = sqrt(r² − R²)/r` (Bevy's own mapping), so a direction `band` above it has
    // `mu = cos(that − band) = sin(band)·sqrt(1 − c²) − c·cos(band)`.
    let c = clamp(sqrt(max(r * r - params.planet.x * params.planet.x, 0.0)) / max(r, 1.0), 0.0, 1.0);
    let band = params.view.z;
    let mu = sin(band) * sqrt(max(1.0 - c * c, 0.0)) - c * cos(band);
    let uv = sky_lut_uv(r, mu, azimuth, params.planet.x, params.view.xy);
    samples[i] = vec4<f32>(textureSampleLevel(sky_view_lut, sky_lut_sampler, uv, 0.0).rgb, 1.0);
}
