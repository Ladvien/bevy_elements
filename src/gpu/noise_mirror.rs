//! A CPU transliteration of `src/noise.wesl` (f32, wrapping u32 arithmetic), used by tests to
//! check the analytic derivative and to measure the normalisation constants the kernels use.

use bevy::math::{IVec3, UVec3, Vec3, Vec4};

const NOISE_PERIOD: i32 = 4096;

fn pcg3d(v_in: UVec3) -> UVec3 {
    let mul = |a: u32, b: u32| a.wrapping_mul(b);
    let mut v = UVec3::new(
        mul(v_in.x, 1_664_525).wrapping_add(1_013_904_223),
        mul(v_in.y, 1_664_525).wrapping_add(1_013_904_223),
        mul(v_in.z, 1_664_525).wrapping_add(1_013_904_223),
    );
    v.x = v.x.wrapping_add(mul(v.y, v.z));
    v.y = v.y.wrapping_add(mul(v.z, v.x));
    v.z = v.z.wrapping_add(mul(v.x, v.y));
    v ^= v >> 16;
    v.x = v.x.wrapping_add(mul(v.y, v.z));
    v.y = v.y.wrapping_add(mul(v.z, v.x));
    v.z = v.z.wrapping_add(mul(v.x, v.y));
    v
}

fn hash33(p: IVec3) -> Vec3 {
    let w = p & IVec3::splat(NOISE_PERIOD - 1);
    let h: UVec3 = pcg3d(w.as_uvec3()) >> 8u32;
    h.as_vec3() * (1.0 / 16_777_216.0)
}

fn lattice_gradient(cell: IVec3) -> Vec3 {
    (hash33(cell) * 2.0 - 1.0 + Vec3::splat(1e-6)).normalize()
}

/// Mirror of `perlin3_d`: (value, ∂/∂x, ∂/∂y, ∂/∂z).
pub(crate) fn perlin3_d(p: Vec3) -> Vec4 {
    let i = p.floor().as_ivec3();
    let f = p - p.floor();
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
    let g = |o: IVec3| lattice_gradient(i + o);
    let (ga, gb, gc, gd) = (
        g(IVec3::ZERO),
        g(IVec3::X),
        g(IVec3::Y),
        g(IVec3::new(1, 1, 0)),
    );
    let (ge, gf, gg, gh) = (
        g(IVec3::Z),
        g(IVec3::new(1, 0, 1)),
        g(IVec3::new(0, 1, 1)),
        g(IVec3::ONE),
    );
    let va = ga.dot(f);
    let vb = gb.dot(f - Vec3::X);
    let vc = gc.dot(f - Vec3::Y);
    let vd = gd.dot(f - Vec3::new(1.0, 1.0, 0.0));
    let ve = ge.dot(f - Vec3::Z);
    let vf = gf.dot(f - Vec3::new(1.0, 0.0, 1.0));
    let vg = gg.dot(f - Vec3::new(0.0, 1.0, 1.0));
    let vh = gh.dot(f - Vec3::ONE);
    let (k1, k2, k3) = (vb - va, vc - va, ve - va);
    let (k4, k5, k6) = (va - vb - vc + vd, va - vc - ve + vg, va - vb - ve + vf);
    let k7 = -va + vb + vc - vd + ve - vf - vg + vh;
    let value = va
        + u.x * k1
        + u.y * k2
        + u.z * k3
        + u.x * u.y * k4
        + u.y * u.z * k5
        + u.z * u.x * k6
        + u.x * u.y * u.z * k7;
    let yzx = Vec3::new(u.y, u.z, u.x);
    let zxy = Vec3::new(u.z, u.x, u.y);
    let grad = ga
        + u.x * (gb - ga)
        + u.y * (gc - ga)
        + u.z * (ge - ga)
        + u.x * u.y * (ga - gb - gc + gd)
        + u.y * u.z * (ga - gc - ge + gg)
        + u.z * u.x * (ga - gb - ge + gf)
        + u.x * u.y * u.z * (-ga + gb + gc - gd + ge - gf - gg + gh)
        + du * (Vec3::new(k1, k2, k3)
            + yzx * Vec3::new(k4, k5, k6)
            + zxy * Vec3::new(k6, k4, k5)
            + yzx * zxy * k7);
    Vec4::new(value, grad.x, grad.y, grad.z) * 1.1547
}

pub(crate) fn seed_offset(k: u32) -> Vec3 {
    Vec3::new(
        (k * 1031 % 4096) as f32,
        (k * 2053 % 4096) as f32,
        (k * 3079 % 4096) as f32,
    )
}

/// Mirror of `fbm3`.
pub(crate) fn fbm3(p: Vec3, octaves: u32) -> f32 {
    let (mut sum, mut amplitude, mut norm, mut q) = (0.0, 0.5, 0.0, p);
    for o in 0..octaves {
        sum += amplitude * perlin3_d(q + seed_offset(o + 1)).x;
        norm += amplitude;
        amplitude *= 0.5;
        q *= 2.0;
    }
    sum / f32::max(norm, 1e-6)
}

/// Mirror of `curl3`.
pub(crate) fn curl3(p: Vec3) -> Vec3 {
    let a = perlin3_d(p + seed_offset(11));
    let b = perlin3_d(p + seed_offset(12));
    let c = perlin3_d(p + seed_offset(13));
    Vec3::new(c.z - b.w, a.w - c.y, b.y - a.z)
}

/// A deterministic spread of sample points over `[0, extent)³`.
pub(crate) fn samples(n: usize, extent: f32) -> impl Iterator<Item = Vec3> {
    (0..n).map(move |k| {
        let h = hash33(IVec3::new(k as i32, 7, 1_234));
        let j = hash33(IVec3::new(k as i32, 99, 4_321));
        Vec3::new(h.x, h.y, j.z) * extent
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_analytic_derivative_matches_finite_differences() {
        let h = 1e-3;
        let mut worst: f32 = 0.0;
        for p in samples(2_000, 4.0) {
            let d = perlin3_d(p);
            for (axis, analytic) in [(Vec3::X, d.y), (Vec3::Y, d.z), (Vec3::Z, d.w)] {
                let fd = (perlin3_d(p + axis * h).x - perlin3_d(p - axis * h).x) / (2.0 * h);
                worst = worst.max((fd - analytic).abs());
            }
        }
        assert!(worst < 2e-3, "worst derivative error {worst}");
    }

    #[test]
    fn the_lattice_is_periodic() {
        let p = Vec3::new(3.3, 7.7, 11.1);
        let q = p + Vec3::new(4096.0, -4096.0, 8192.0);
        assert!((perlin3_d(p) - perlin3_d(q)).abs().max_element() < 2e-3);
    }

    #[test]
    fn the_kernel_normalisation_constants_match_the_noise() {
        let n = 100_000;
        let rms = (samples(n, 256.0)
            .map(|p| curl3(p).length_squared())
            .sum::<f32>()
            / n as f32)
            .sqrt();
        let fbm_max = samples(n, 256.0)
            .map(|p| fbm3(p, 2).abs())
            .fold(0.0_f32, f32::max);
        println!("curl rms {rms}, 2-octave fbm max |n| {fbm_max}");
        assert!(
            (rms * crate::wind::CURL_NORMALISE - 1.0).abs() < 0.02,
            "curl rms {rms}"
        );
        assert!(
            (fbm_max - crate::fog::FBM2_MAX).abs() < 0.02,
            "fbm max {fbm_max}"
        );
    }
}
