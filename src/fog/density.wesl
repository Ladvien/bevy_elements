// Ground-fog density for Bevy's FogVolume: extinction coefficient in m⁻¹ in `.r` of a
// 128×32×128 rgba16float texture that spans the volume's unit cube. See docs/research/fog.md.
//
//   σ(p) = σ_V · exp(−h/H) · (0.6 + 0.4 · f(p − w t)),  σ_V = 3.912 / visibility

import bevy_elements::noise::fbm3;

struct FogParams {
    // xyz: world position of the volume's minimum corner, m. w: 1 / max |fbm| (noise → [0, 1]).
    origin: vec4<f32>,
    // xyz: volume size, m. w: σ_V, m⁻¹.
    extent: vec4<f32>,
    // x: scale height H (m), y: ground height (m), zw: wind offset w·t (world XZ, m).
    shape: vec4<f32>,
    // xyz: texels per axis. w: unused.
    size: vec4<u32>,
}

@group(0) @binding(0) var<uniform> params: FogParams;
@group(0) @binding(1) var density: texture_storage_3d<rgba16float, write>;

// fBm wavelength, m.
const WAVELENGTH_M: f32 = 80.0;

@compute @workgroup_size(4, 4, 4)
fn fog_density(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id >= params.size.xyz) {
        return;
    }
    let uvw = (vec3<f32>(id) + 0.5) / vec3<f32>(params.size.xyz);
    let p = params.origin.xyz + uvw * params.extent.xyz;
    let h = max(p.y - params.shape.y, 0.0);
    let q = (p - vec3<f32>(params.shape.z, 0.0, params.shape.w)) / WAVELENGTH_M;
    let f = clamp(0.5 + 0.5 * fbm3(q, 2u) * params.origin.w, 0.0, 1.0);
    let sigma = params.extent.w * exp(-h / params.shape.x) * (0.6 + 0.4 * f);
    textureStore(density, vec3<i32>(id), vec4<f32>(sigma, 0.0, 0.0, 0.0));
}
