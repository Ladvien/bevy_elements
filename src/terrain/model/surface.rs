//! The procedural generator of `generate.wesl` on the CPU: the surface a [`ProceduralTerrain`]
//! draws, evaluated without a GPU. The terrain tests compare the kernel against it, and
//! [`ProceduralTerrain::sample_surface`] exposes it.

use bevy::math::{UVec2, Vec2, Vec3};

use super::{F_HALF, GAIN, LACUNARITY, OCTAVE_ROTATION, R_LOW, R_SPAN, octave_count, seed_mix};
use crate::gpu::perlin::perlin3_d;
use crate::terrain::ProceduralTerrain;

/// Both noise sums of a sample point: `(f, r)` of the research note.
pub(crate) fn noise_sums(local_xz: Vec2, feature_m: f32, octaves: u32, seed: u64) -> (f32, f32) {
    let p = local_xz / feature_m;
    let mix = seed_mix(seed);
    let (mut f_sum, mut r_sum, mut norm, mut gain) = (0.0, 0.0, 0.0, 1.0);
    let mut weight = 1.0_f32;
    for k in 0..octaves {
        let (s, c) = (OCTAVE_ROTATION * k as f32).sin_cos();
        let q = Vec2::new(c * p.x - s * p.y, s * p.x + c * p.y) * LACUNARITY.powi(k as i32);
        let v = ((mix + k * 1031) % 4096) as f32;
        let n = perlin3_d(Vec3::new(q.x + v, q.y + v, 0.5 + k as f32)).x;
        let ridge = (1.0 - n.abs()).max(0.0);
        let s_k = weight * ridge * ridge;
        f_sum += gain * n;
        r_sum += gain * s_k;
        norm += gain;
        weight = (2.0 * s_k).clamp(0.0, 1.0);
        gain *= GAIN;
    }
    (f_sum / norm, r_sum / norm)
}

/// The unit terrain value `(1 − q) u_f + q u_r` of the two sums.
pub(crate) fn unit_height(f: f32, r: f32, ridged: f32) -> f32 {
    let u_f = (0.5 + f / (2.0 * F_HALF)).clamp(0.0, 1.0);
    let u_r = ((r - R_LOW) / R_SPAN).clamp(0.0, 1.0);
    (1.0 - ridged) * u_f + ridged * u_r
}

/// The surface height at a point `local_xz` metres from the terrain's minimum corner, relative
/// to the terrain transform: the base height plus the relief times the unit value.
pub(crate) fn surface_height(p: &ProceduralTerrain, octaves: u32, local_xz: Vec2) -> f32 {
    let (f, r) = noise_sums(local_xz, p.feature_size_m, octaves, p.seed);
    p.base_height_m + p.relief_m * unit_height(f, r, p.ridged.clamp(0.0, 1.0))
}

/// The generated surface at every cell of a `size` grid of `cell_m` cells, row-major.
pub(crate) fn generate_surface(p: &ProceduralTerrain, size: UVec2, cell_m: f32) -> Vec<f32> {
    let octaves = octave_count(p.octaves, p.feature_size_m, cell_m);
    let mut out = Vec::with_capacity((size.x * size.y) as usize);
    for j in 0..size.y {
        for i in 0..size.x {
            let local = (Vec2::new(i as f32, j as f32) + 0.5) * cell_m;
            out.push(surface_height(p, octaves, local));
        }
    }
    out
}
