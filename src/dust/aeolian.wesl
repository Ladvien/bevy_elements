// Aeolian transport of the loose sand layer (docs/research/dust.md "Model adopted").
//
//   aeolian_capacity  per cell, the flux the surface wind carries at saturation: the terrain-
//                     modified wind (Venturi speed-up over the 50 m ring mean, shadow behind
//                     upwind rises) through k u² (u − u_t)+
//   aeolian_flux      one upwind sweep of the relaxation dq/ds = (q_s − q) / L_sat, limited by the
//                     sand the cell holds and receives
//   aeolian_move      per cell, the bed change by the flux divergence (donor-cell face fluxes)
//
// All bindings are read_write in every entry point so the three share one layout: the kernels read
// other cells only from buffers they do not write (capacity writes `cap`, flux writes `q_out`,
// move writes only its own `sediment`), so every dispatch is race-free. The flux ping-pongs:
// `aeolian_flux` reads `q_in` and writes `q_out`, `aeolian_move` reads `q_out`.

import bevy_elements::grid::GridParams;
import bevy_elements::dust::surface_wind::{
    RING_M, SHADOW_MARCH_M, venturi_speedup, shadow_factor, flux_capacity,
};

struct AeolianParams {
    grid: GridParams,
    // xy: unit wind direction (x, z). z: the 10 m wind, m/s. w: 1 when the boundary is closed.
    wind: vec4<f32>,
    // x: flux coefficient k, m²/s per (m/s)³. y: transport threshold u_t, m/s.
    law: vec4<f32>,
    // x: 1 − exp(−Δ / L_sat). y: the bed's time step, s.
    step: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: AeolianParams;
@group(0) @binding(1) var<storage, read_write> bedrock: array<f32>;
@group(0) @binding(2) var<storage, read_write> sediment: array<f32>;
@group(0) @binding(3) var<storage, read_write> cap: array<f32>;
@group(0) @binding(4) var<storage, read_write> q_in: array<f32>;
@group(0) @binding(5) var<storage, read_write> q_out: array<f32>;

fn in_grid(c: vec2<i32>) -> bool {
    let n = vec2<i32>(params.grid.size);
    return c.x >= 0 && c.y >= 0 && c.x < n.x && c.y < n.y;
}

fn cell_index(c: vec2<i32>) -> u32 {
    return u32(c.y) * params.grid.size.x + u32(c.x);
}

/// The surface height at a cell, the edge cell's for a cell off the grid.
fn surface_at(c: vec2<i32>) -> f32 {
    let m = vec2<i32>(params.grid.size) - vec2<i32>(1);
    let k = cell_index(clamp(c, vec2<i32>(0), m));
    return bedrock[k] + sediment[k];
}

/// Whether the face toward a downwind neighbour lets sand through: always inside the grid, and
/// over the edge when the boundary is open.
fn face_open(c: vec2<i32>) -> bool {
    return in_grid(c) || params.wind.w < 0.5;
}

/// The flux sign of the wind per axis: the cell the sand comes from is `c − s`.
fn wind_sign() -> vec2<i32> {
    let u = params.wind.xy;
    return vec2<i32>(select(-1, 1, u.x >= 0.0), select(-1, 1, u.y >= 0.0));
}

@compute @workgroup_size(8, 8, 1)
fn aeolian_capacity(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = params.grid.size;
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let c = vec2<i32>(id.xy);
    let cell = params.grid.cell;
    let z = surface_at(c);
    let dir = params.wind.xy;

    // The steepest rise of the surface upwind, over the march radius, one tap per cell.
    let taps = max(i32(floor(SHADOW_MARCH_M / cell + 1e-3)), 1);
    var slope = -1e9;
    for (var i = 1; i <= taps; i += 1) {
        let offset = vec2<i32>(round(dir * f32(i)));
        slope = max(slope, (surface_at(c - offset) - z) / (f32(i) * cell));
    }
    let shadow = shadow_factor(slope);

    // The height above the mean of eight taps on the ring.
    var ring_dirs = array<vec2<f32>, 8>(
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.70710678, 0.70710678),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(-0.70710678, 0.70710678),
        vec2<f32>(-1.0, 0.0),
        vec2<f32>(-0.70710678, -0.70710678),
        vec2<f32>(0.0, -1.0),
        vec2<f32>(0.70710678, -0.70710678),
    );
    let ring = RING_M / cell;
    var mean = 0.0;
    for (var d = 0u; d < 8u; d += 1u) {
        mean += surface_at(c + vec2<i32>(round(ring_dirs[d] * ring)));
    }
    mean = mean * 0.125;

    let u = params.wind.z * venturi_speedup(z - mean) * shadow;
    cap[cell_index(c)] = flux_capacity(u, params.law.x, params.law.y);
}

@compute @workgroup_size(8, 8, 1)
fn aeolian_flux(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = params.grid.size;
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let c = vec2<i32>(id.xy);
    let k = cell_index(c);
    let s = wind_sign();
    let a = abs(params.wind.xy);
    let from_x = c - vec2<i32>(s.x, 0);
    let from_z = c - vec2<i32>(0, s.y);
    var q_x = 0.0;
    var q_z = 0.0;
    if in_grid(from_x) {
        q_x = q_in[cell_index(from_x)];
    }
    if in_grid(from_z) {
        q_z = q_in[cell_index(from_z)];
    }
    // What arrives, per unit time and width of cell, and the same as one flux.
    let arriving = a.x * q_x + a.y * q_z;
    let q_up = arriving / (a.x + a.y);
    let relaxed = q_up + (cap[k] - q_up) * params.step.x;

    // The cell gives away what it receives and what it holds: `out ≤ arriving + h Δ / Δt`.
    var out_weight = 0.0;
    if face_open(c + vec2<i32>(s.x, 0)) {
        out_weight += a.x;
    }
    if face_open(c + vec2<i32>(0, s.y)) {
        out_weight += a.y;
    }
    var q = relaxed;
    if out_weight > 1e-6 {
        q = min(relaxed, (arriving + sediment[k] * params.grid.cell / params.step.y) / out_weight);
    }
    q_out[k] = max(q, 0.0);
}

@compute @workgroup_size(8, 8, 1)
fn aeolian_move(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = params.grid.size;
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let c = vec2<i32>(id.xy);
    let k = cell_index(c);
    let s = wind_sign();
    let a = abs(params.wind.xy);
    let from_x = c - vec2<i32>(s.x, 0);
    let from_z = c - vec2<i32>(0, s.y);
    var arriving = 0.0;
    if in_grid(from_x) {
        arriving += a.x * q_out[cell_index(from_x)];
    }
    if in_grid(from_z) {
        arriving += a.y * q_out[cell_index(from_z)];
    }
    var leaving = 0.0;
    if face_open(c + vec2<i32>(s.x, 0)) {
        leaving += a.x * q_out[k];
    }
    if face_open(c + vec2<i32>(0, s.y)) {
        leaving += a.y * q_out[k];
    }
    sediment[k] = max(sediment[k] + params.step.y / params.grid.cell * (arriving - leaving), 0.0);
}
