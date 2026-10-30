//! The lattice hash and the gradient noise of `src/noise.wesl` on the CPU (f32, wrapping u32
//! arithmetic). The tests compare kernels against it, and the terrain's CPU surface generator
//! (`ProceduralTerrain::sample_surface`) evaluates it.

use bevy::math::{IVec3, UVec3, Vec3, Vec4};

/// The lattice period of the hash, in cells (`noise.wesl`'s `NOISE_PERIOD`).
pub(crate) const NOISE_PERIOD: i32 = 4096;

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

pub(crate) fn hash33(p: IVec3) -> Vec3 {
    let w = p & IVec3::splat(NOISE_PERIOD - 1);
    let h: UVec3 = pcg3d(w.as_uvec3()) >> 8u32;
    h.as_vec3() * (1.0 / 16_777_216.0)
}

/// Hash of an integer lattice point to one float in [0, 1), the mirror of `hash31` in
/// `noise.wesl`.
#[cfg(test)]
pub(crate) fn hash31(p: IVec3) -> f32 {
    hash33(p).x
}

pub(crate) fn lattice_gradient(cell: IVec3) -> Vec3 {
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
