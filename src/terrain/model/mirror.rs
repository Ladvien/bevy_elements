//! CPU transliterations of the terrain kernels that the tests compare against: the procedural
//! generator of `generate.wesl` and the layer weights of `terrain.wesl`.

use bevy::math::{UVec2, Vec2, Vec3};

use super::{F_HALF, GAIN, LACUNARITY, OCTAVE_ROTATION, R_LOW, R_SPAN, octave_count, seed_mix};
use crate::gpu::noise_mirror::perlin3_d;
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

/// The slope angle of a surface at a cell by central differences over ±1 cell (one-sided at the
/// edge), degrees.
pub(crate) fn slope_deg(surface: &[f32], size: UVec2, cell_m: f32, i: u32, j: u32) -> f32 {
    let at = |x: u32, z: u32| surface[(z * size.x + x) as usize];
    let (xl, xh) = (i.saturating_sub(1), (i + 1).min(size.x - 1));
    let (zl, zh) = (j.saturating_sub(1), (j + 1).min(size.y - 1));
    let gx = (at(xh, j) - at(xl, j)) / ((xh - xl) as f32 * cell_m);
    let gz = (at(i, zh) - at(i, zl)) / ((zh - zl) as f32 * cell_m);
    (gx * gx + gz * gz).sqrt().atan().to_degrees()
}

/// Percentile `q` (0–1) of `values` (sorted in place).
pub(crate) fn percentile(values: &mut [f32], q: f32) -> f32 {
    values.sort_by(f32::total_cmp);
    values[((values.len() - 1) as f32 * q).round() as usize]
}

/// `smoothstep(a, b, x)` of WGSL.
pub(crate) fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Andersson 2007 §5.2.3, Listing 2: the Overlay blend of `detail` onto `mask` at full opacity.
pub(crate) fn overlay(mask: f32, detail: f32) -> f32 {
    if mask < 0.5 {
        2.0 * mask * detail
    } else {
        1.0 - 2.0 * (1.0 - mask) * (1.0 - detail)
    }
}

/// Feature size of the coarse ground detail's base octave, m (`terrain/common.wesl`).
pub(crate) const DETAIL_COARSE_M: f32 = 1.25;
/// Feature size of the fine ground detail's base octave, m.
pub(crate) const DETAIL_FINE_M: f32 = 0.125;

/// `detail_keep` of `terrain/common.wesl`: the weight of an octave of feature size `feature_m`
/// in a pixel whose footprint is `footprint_m` wide. 1 while the octave is resolved (at least
/// two footprints per feature), falling to 0 over the band where it is not (Lagae et al. 2010
/// §5.2.1, frequency clamping; `docs/research/ground-detail.md` Model 2).
pub(crate) fn detail_keep(feature_m: f32, footprint_m: f32) -> f32 {
    let t = ((footprint_m / feature_m - 0.5) / 0.5).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

/// The four layer weights `(rock, sand, soil, grass)` of `render.wesl` for a slope angle
/// (degrees), a sediment depth (m) and a detail value in [0, 1] (0.5: none).
pub(crate) fn layer_weights(slope_deg: f32, sediment_m: f32, detail: f32) -> [f32; 4] {
    let rock = overlay(smoothstep(30.0, 38.0, slope_deg), detail);
    let sand = overlay(smoothstep(0.3, 0.7, sediment_m), detail)
        * (1.0 - smoothstep(26.0, 30.0, slope_deg))
        * (1.0 - rock);
    let band = overlay(smoothstep(16.0, 26.0, slope_deg), detail) * (1.0 - smoothstep(22.0, 29.0, slope_deg));
    let rest = (1.0 - rock) * (1.0 - sand);
    [rock, sand, rest * band, rest * (1.0 - band)]
}

/// The scale of the ground's large-scale variation, m (`render.wesl`).
pub(crate) const LAYER_VARIATION_M: f32 = 45.0;
/// How far the variation moves the layer weights (`render.wesl`).
pub(crate) const LAYER_VARIATION_DEPTH: f32 = 0.25;
/// The rms of the crate's 3D noise, measured in the crate's own mirror: the field is
/// normalised by it so that `LAYER_VARIATION_DEPTH` is the fraction of the grass a patch turns
/// to soil.
pub(crate) const NOISE_RMS: f32 = 0.2268;

/// The ground's large-scale variation of the layer weights (`render.wesl`): a low-frequency
/// field that takes soil from the grass, so a plain — where the soil's weight is zero — gets
/// patches instead of one colour. The field is the `ground_fields` function's large one.
pub(crate) fn layer_variation(field: f32, w: [f32; 4]) -> [f32; 4] {
    let field = field.clamp(-1.0, 1.0);
    let shift = field * LAYER_VARIATION_DEPTH * w[3];
    let mut v = w;
    // The shift is taken from the grass and given to the soil, and both ends are clamped: a
    // field more negative than the soil's weight would take the soil below zero (measured:
    // −0.054 on a plain), and a negative weight is a negative albedo.
    let taken = shift.clamp(-v[2], v[3]);
    v[2] += taken;
    v[3] -= taken;
    let total = v.iter().sum::<f32>();
    v.map(|x| x / total.max(1e-6))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An octave is kept whole while the pixel resolves it, cancelled once it cannot, and the
    /// transition is monotone and smooth; the two scales overlap so no band of the ground's
    /// detail is left uncovered (`ground-detail.md` Validation 1-3).
    #[test]
    fn the_detail_band_limit_keeps_resolved_octaves_and_cancels_the_rest() {
        assert_eq!(detail_keep(DETAIL_COARSE_M, 0.0), 1.0);
        assert_eq!(detail_keep(DETAIL_COARSE_M, DETAIL_COARSE_M / 2.0), 1.0);
        assert_eq!(detail_keep(DETAIL_COARSE_M, DETAIL_COARSE_M), 0.0);
        assert_eq!(detail_keep(DETAIL_COARSE_M, 10.0), 0.0);
        // Monotone in the footprint, and continuous (no popping as the camera pulls back).
        let mut last = 1.0;
        for k in 0..=400 {
            let f = f64::from(k) / 400.0 * f64::from(DETAIL_COARSE_M);
            let w = detail_keep(DETAIL_COARSE_M, f as f32);
            assert!(w <= last + 1e-6, "not monotone at {f}");
            assert!((last - w) < 0.02, "a step of {} at {f}", last - w);
            last = w;
        }
        // The overlap: an octave held whole (half its feature) while the next coarser is gone.
        // The fine scale is 10× the coarse one's frequency, so there is always a band of
        // footprints between them where one has faded and the other has not yet.
        let half_fine = DETAIL_FINE_M * 0.5;
        let half_coarse = DETAIL_COARSE_M * 0.5;
        assert!(detail_keep(DETAIL_FINE_M, half_fine) > 0.99);
        assert_eq!(detail_keep(DETAIL_COARSE_M, half_coarse), 1.0);
        assert!(half_fine < half_coarse);
    }

    /// FNV-1a over the bytes of `values`.
    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    /// The statistics of one preset at 1024² cells of 2 m: (spread of `h − h_base` between the
    /// 0.5 and 99.5 percentiles, slope p50, p95, share above 40° in percent).
    fn stats(p: &ProceduralTerrain) -> (f32, f32, f32, f32) {
        let size = UVec2::splat(1024);
        let surface = generate_surface(p, size, 2.0);
        let mut heights: Vec<f32> = surface.iter().map(|h| h - p.base_height_m).collect();
        let spread = percentile(&mut heights, 0.995) - percentile(&mut heights, 0.005);
        let mut slopes: Vec<f32> = (0..size.y)
            .flat_map(|j| (0..size.x).map(move |i| (i, j)))
            .map(|(i, j)| slope_deg(&surface, size, 2.0, i, j))
            .collect();
        let steep = slopes.iter().filter(|s| **s > 40.0).count() as f32 / slopes.len() as f32;
        (
            spread,
            percentile(&mut slopes, 0.5),
            percentile(&mut slopes, 0.95),
            steep * 100.0,
        )
    }

    /// Validation 1: the two noise sums on this crate's noise land on the constants the unit
    /// maps assume. The percentiles are the stationary ones (pooled over seeds): one seed's
    /// 2048 m tile of a 600 m feature has only 3.4 base-octave cells, so its own percentiles
    /// scatter by ±0.08 (seed 42: f −0.347 / 0.256).
    #[test]
    fn the_noise_sums_fill_their_unit_ranges() {
        let (mut f, mut r) = (Vec::new(), Vec::new());
        for seed in 0..32u64 {
            for j in 0..96 {
                for i in 0..96 {
                    let p = Vec2::new(i as f32, j as f32) * 0.31 + Vec2::splat(0.1);
                    let (a, b) = noise_sums(p, 1.0, 8, seed * 7 + 1);
                    f.push(a);
                    r.push(b);
                }
            }
        }
        let f_lo = percentile(&mut f, 0.005);
        let f_hi = percentile(&mut f, 0.995);
        let r_lo = percentile(&mut r, 0.005);
        let r_hi = percentile(&mut r, 0.995);
        println!("f {f_lo:.3} {f_hi:.3}  r {r_lo:.3} {r_hi:.3}");
        assert!(
            (f_lo + F_HALF).abs() < 0.03 && (f_hi - F_HALF).abs() < 0.03,
            "f {f_lo} {f_hi}"
        );
        assert!(
            (r_lo - R_LOW).abs() < 0.03 && (r_hi - (R_LOW + R_SPAN)).abs() < 0.03,
            "r {r_lo} {r_hi}"
        );
    }

    /// Validation 1: per preset, the percentile spread of the heights, the slope percentiles and
    /// the share of rock-steep ground, re-blessed once on this crate's noise (Deviation 11; the
    /// note's numpy port, a 2D gradient noise, is about 30 % steeper).
    #[test]
    fn the_presets_have_the_measured_statistics() {
        // (preset, spread m, slope p50, slope p95, share above 40° in %), seed 42, 2 m cells.
        let table = [
            (ProceduralTerrain::hills(42), 91.8, 12.4, 25.5, 0.0),
            (ProceduralTerrain::mountains(42), 549.5, 41.1, 61.9, 52.6),
            (ProceduralTerrain::dunes(42), 39.9, 9.1, 18.6, 0.0),
            (ProceduralTerrain::plains(42), 13.7, 1.4, 2.9, 0.0),
        ];
        let mut misses = Vec::new();
        for (preset, spread, p50, p95, steep) in table {
            let (s, m, h, e) = stats(&preset);
            println!(
                "R {} F {}: spread {s:.1} (want {spread}) slope p50 {m:.1} ({p50}) p95 {h:.1} ({p95}) >40° {e:.2} % ({steep})",
                preset.relief_m, preset.feature_size_m
            );
            if (s / spread - 1.0).abs() >= 0.10
                || (m / p50 - 1.0).abs() >= 0.15
                || (h / p95 - 1.0).abs() >= 0.15
                || (e - steep).abs() >= 3.0
            {
                misses.push(format!("R {} F {}", preset.relief_m, preset.feature_size_m));
            }
        }
        assert!(misses.is_empty(), "{misses:?}");
    }

    /// Validation 2: the generator is frozen on the CPU mirror.
    #[test]
    fn the_terrain_is_frozen() {
        let surface = generate_surface(&ProceduralTerrain::hills(42), UVec2::splat(256), 8.0);
        let mm: Vec<i32> = surface
            .iter()
            .map(|h| (h * 1000.0).round() as i32)
            .collect();
        let hash = fnv1a(bytemuck::cast_slice(&mm));
        println!("the_terrain_is_frozen: {hash:#018x}");
        assert_eq!(
            hash, GOLDEN_HILLS_256,
            "the generator changed: re-bless once, deliberately"
        );
    }

    const GOLDEN_HILLS_256: u64 = 0x6f04_892c_64bc_b5eb;

    /// Validation 7: the layer weights sum to one for any detail, pick the plan's layers without
    /// detail, and the detail never moves a layer that is fully on or off.
    #[test]
    fn the_layer_weights_sum_to_one_and_pick_the_right_layer() {
        for a in (0..90).step_by(1) {
            for s in [0.0, 0.2, 0.4, 0.5, 0.6, 0.8, 1.0, 4.0] {
                for d in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let w = layer_weights(a as f32, s, d);
                    assert!(
                        (w.iter().sum::<f32>() - 1.0).abs() < 1e-6,
                        "{a}° {s} m detail {d}: {w:?}"
                    );
                    assert!(w.iter().all(|x| (0.0..=1.0).contains(x)));
                }
            }
        }
        assert!(layer_weights(45.0, 0.0, 0.5)[0] >= 0.99);
        assert!(layer_weights(20.0, 1.0, 0.5)[1] >= 0.99);
        assert_eq!(layer_weights(10.0, 0.0, 0.5)[3], 1.0);
        // The soil band is 20° to 30°, tapering from 26° as the rock's ramp opens, and it must
        // sit *below* the rock's ramp (30° to 38°): where the two overlap, a mix of a warm
        // brown and a cool grey is a pink patch, and the ground showed them on the hills
        // (measured: 0.32 % of the frame).
        let soil = layer_weights(21.0, 0.0, 0.5)[2];
        assert!((0.3..0.7).contains(&soil), "soil at 21°: {soil}");
        assert!(layer_weights(31.0, 0.0, 0.5)[2] < 0.4, "the soil must be gone by 31°");
        // The taper: the soil thins as the rock rises, so the grass meets the stone without a
        // warm brown rim (measured: 1415 orange pixels at RGB 148, 121, 100 in one frame, and
        // none after).
        assert!(layer_weights(28.0, 0.0, 0.5)[2] < 0.35, "the soil must taper into the rock");
        // Fully on or off stays put under any detail: rock at 50°, grass at 10°.
        for d in [0.0, 0.3, 0.7, 1.0] {
            assert_eq!(layer_weights(50.0, 0.0, d)[0], 1.0);
            assert_eq!(layer_weights(10.0, 0.0, d)[3], 1.0);
        }
        // Inside a transition the detail spans the whole range. The rock's ramp is 30° to 38°,
        // so 34° is its middle: there the detail takes the rock from 0 to 1.
        assert_eq!(layer_weights(34.0, 0.0, 0.0)[0], 0.0);
        assert_eq!(layer_weights(34.0, 0.0, 1.0)[0], 1.0);
        // The rock's ramp starts at 30°, so a slope is already half rock at 34°: the ground's
        // turf is the crate's 2D noise in the world XZ plane, and it must be gone before the
        // surface is steep enough to weave it (`rock-material.md` Model 1).
        assert!(layer_weights(34.0, 0.0, 0.5)[0] > 0.3, "the rock must be there by 34°");
        // The ramp is narrow on purpose: a wide one has the rock's *partial* weight mixing
        // with the soil's, and a warm brown mixed with a cool grey is a pink patch (measured:
        // 9.67 % of the mountain's frame was pink before, 0.01 % after).
        assert!(layer_weights(31.0, 0.0, 0.5)[0] < 0.2, "the rock must not creep below 30°");
    }

    /// The ground's large-scale variation must reach a **plain**, where the soil's weight is
    /// zero and the grass's is one. Moving the two weights against each other (each by the
    /// other's weight) does nothing there, and the plain stayed one colour; the shift has to
    /// be taken from the grass and given to the soil. The weights must still sum to one and
    /// stay in range, or the ground's mean colour drifts (`rock-material.md` Model 4).
    #[test]
    fn the_layer_variation_reaches_a_plain_and_keeps_the_weights_summing_to_one() {
        let plain = layer_weights(4.0, 0.0, 0.5);
        assert_eq!(plain[2], 0.0, "a plain has no soil to start with");
        assert_eq!(plain[3], 1.0);
        // Walk a line across the field: the soil must appear, the grass must give it up, and
        // the pair must still sum to one.
        let mut max_soil = 0.0f32;
        let mut min_soil = 1.0f32;
        for k in 0..200 {
            let p = Vec3::new(k as f32 * 3.0, 0.0, 0.0);
            let field = crate::gpu::noise_mirror::perlin3_d(
                p / LAYER_VARIATION_M + crate::gpu::noise_mirror::seed_offset(71),
            )
            .x
                / NOISE_RMS;
            let w = layer_variation(field, plain);
            assert!(
                (w.iter().sum::<f32>() - 1.0).abs() < 1e-5,
                "the weights do not sum to one at {p:?}: {w:?}"
            );
            assert!(w.iter().all(|x| (0.0..=1.0).contains(x)), "{w:?}");
            max_soil = max_soil.max(w[2]);
            min_soil = min_soil.min(w[2]);
        }
        assert!(
            max_soil > 0.05,
            "the variation never reaches a plain (the most soil was {max_soil})"
        );
        assert!(
            min_soil < 0.02,
            "the variation never gives the soil back (the least soil was {min_soil})"
        );
        // The rock and the sand are where the terrain's shape put them, and the variation must
        // not move them.
        let steep = layer_weights(50.0, 0.0, 0.5);
        let field = crate::gpu::noise_mirror::perlin3_d(
            Vec3::new(30.0, 0.0, 0.0) / LAYER_VARIATION_M + crate::gpu::noise_mirror::seed_offset(71),
        )
        .x
            / NOISE_RMS;
        let w = layer_variation(field, steep);
        assert_eq!(w[0], 1.0, "the variation moved the rock");
    }

    /// Validation 5: on the plane `h = 0.5 x` the central-difference slope is atan(0.5) at every
    /// cell, edges included, and a flat field has slope 0.
    #[test]
    fn the_slope_of_a_plane_is_exact_up_to_the_edge() {
        let size = UVec2::new(16, 8);
        let plane: Vec<f32> = (0..size.y)
            .flat_map(|_| (0..size.x).map(|i| 0.5 * (i as f32 + 0.5) * 2.0))
            .collect();
        for j in 0..size.y {
            for i in 0..size.x {
                let a = slope_deg(&plane, size, 2.0, i, j);
                assert!((a - 26.565).abs() < 0.01, "cell {i},{j}: {a}");
            }
        }
        assert_eq!(slope_deg(&vec![3.0; 128], size, 2.0, 0, 0), 0.0);
    }
}
