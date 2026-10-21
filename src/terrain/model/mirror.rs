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

/// The turf's palette (`terrain/common.wesl`): the four tones, as multipliers on the turf's
/// own colour. The fresh tone is the identity.
pub(crate) const TURF_FRESH: [f32; 3] = [1.00, 1.00, 1.00];
pub(crate) const TURF_DRY: [f32; 3] = [1.35, 1.20, 0.69];
pub(crate) const TURF_STRAW: [f32; 3] = [1.55, 1.37, 0.82];
pub(crate) const TURF_WORN: [f32; 3] = [1.13, 0.95, 0.73];
/// How far the palette moves the ground's colour towards a tone, 0 to 1.
pub(crate) const TURF_STRENGTH: f32 = 0.85;

/// The turf's palette at the given fields (`terrain/common.wesl`): the *shares* of the four
/// tones, which sum to one by construction, and the mixture they make.
pub(crate) fn turf_palette(field: f32, clump: f32) -> ([f32; 3], [f32; 4]) {
    let p = 2.0f32;
    let mut dry = (field.max(0.0) * (0.5 + clump).max(0.0) * 1.6).powf(p).min(1.0);
    let mut straw = ((field - 0.45).max(0.0) * 3.0 * (0.5 + clump).max(0.0) * 1.6)
        .powf(p)
        .min(1.0);
    let mut worn = ((-field - 0.55).max(0.0) * 3.0 * (0.5 - clump).max(0.0) * 1.6)
        .powf(p)
        .min(1.0);
    let others = dry + straw + worn;
    if others > 1.0 {
        let k = 1.0 / others;
        dry *= k;
        straw *= k;
        worn *= k;
    }
    // The fresh share is what is left, and the subtraction can leave it a hair below zero
    // (−2.2e−8 at field 0.5, clump 0.75), which is a negative weight.
    let fresh = (1.0 - dry - straw - worn).max(0.0);
    let shares = [fresh, dry, straw, worn];
    let mut mixed = [0.0f32; 3];
    for c in 0..3 {
        mixed[c] = fresh * TURF_FRESH[c]
            + dry * TURF_DRY[c]
            + straw * TURF_STRAW[c]
            + worn * TURF_WORN[c];
        mixed[c] = 1.0 + (mixed[c] - 1.0) * TURF_STRENGTH;
    }
    (mixed, shares)
}

// The scree (`docs/research/scree.md`)
// ---------------------------------------------------------------------------------------------

/// The cell network's cell, m (`common.wesl`): the *spacing* of the stones in it.
pub(crate) const SCREE_CELL_M: f32 = 0.45;
/// The spread of the stones' sizes, as the range of the per-cell factor (`common.wesl`).
pub(crate) const SCREE_SIZE_MIN: f32 = 0.35;
pub(crate) const SCREE_SIZE_MAX: f32 = 2.20;
/// How much of the cell the largest stone fills (`common.wesl`).
pub(crate) const SCREE_FILL: f32 = 0.72;
/// The fine network's cell, as a fraction of the coarse one (`common.wesl`).
pub(crate) const SCREE_FINE_SCALE: f32 = 0.38;

/// The scree's band: the slope angles between which the stones lie (`render.wesl`). The scree
/// is at the rock's *foot* — from the soil's own band up to the rock's ramp (which opens at
/// 30°) — and the band is *wide*, because a scree is an apron and a narrow band would trace
/// the hillside's break as a contour line.
pub(crate) const SCREE_BAND_DEG: [f32; 4] = [16.0, 26.0, 31.0, 40.0];

/// The scree's weight at a slope angle: 1 at the rock's foot, 0 on the meadow and 0 on the
/// cliff (`render.wesl`).
pub(crate) fn scree_weight(slope_deg: f32) -> f32 {
    smoothstep(SCREE_BAND_DEG[0], SCREE_BAND_DEG[1], slope_deg)
        * (1.0 - smoothstep(SCREE_BAND_DEG[2], SCREE_BAND_DEG[3], slope_deg))
}

/// One scale's stones: how much of the point the stone covers (`common.wesl`).
pub(crate) fn scree_scale_coverage(xz: Vec2, cell_m: f32, size_min: f32, size_max: f32) -> f32 {
    let cell = crate::gpu::noise_mirror::cell2(xz / cell_m);
    let size = size_min + (size_max - size_min) * cell.factor.powi(3);
    let radius = 0.5 * size * SCREE_FILL;
    1.0 - smoothstep(radius * 0.80, radius, cell.distance)
}

/// The scree's coverage at a point: the coarse network's stone, and the fine network's stone
/// where the coarse one does not cover (`common.wesl`).
pub(crate) fn scree_coverage(xz: Vec2) -> f32 {
    let coarse = scree_scale_coverage(xz, SCREE_CELL_M, SCREE_SIZE_MIN, SCREE_SIZE_MAX);
    let fine = scree_scale_coverage(
        xz,
        SCREE_CELL_M * SCREE_FINE_SCALE,
        SCREE_SIZE_MIN * 0.8,
        SCREE_SIZE_MAX * 0.8,
    );
    coarse.max(fine * (1.0 - coarse))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scree is a *scatter*, not a band: its coverage must be **discrete** — the ground
    /// under the scree is stones and turf between them, not a uniform wash.
    ///
    /// The check is the research note's first validation target: the coverage along a cut
    /// across the scree must have *peaks* — a histogram with a second mode — and not a
    /// monotone gradient. A band's weight is a smooth ramp, so its coverage is smooth too.
    #[test]
    fn the_scree_is_discrete_and_not_a_wash() {
        // A cut across the scree, 40 cm apart, 4 m long.
        let mut coverage = Vec::new();
        for i in 0..200 {
            coverage.push(scree_coverage(Vec2::new(i as f32 * 0.04, 3.0)));
        }
        // The stones: how many times the coverage crosses the middle of its range on its way
        // up. A smooth wash crosses once or twice; a scatter of stones crosses many.
        let mut crossings = 0;
        for w in coverage.windows(2) {
            let a = w[0] > 0.5;
            let b = w[1] > 0.5;
            if a != b {
                crossings += 1;
            }
        }
        assert!(
            crossings >= 6,
            "the scree is a scatter, not a wash: {crossings} crossings in 8 m"
        );
        // And there is turf between the stones: the coverage is below one everywhere, and
        // below the middle over a good part of the cut.
        assert!(
            coverage.iter().all(|c| *c <= 1.0),
            "the coverage never exceeds one"
        );
        let below_half = coverage.iter().filter(|c| **c < 0.5).count();
        assert!(
            below_half * 2 > coverage.len(),
            "the turf shows between the stones: {below_half} of {} below half",
            coverage.len()
        );
    }

    /// The stones have *many* sizes: the note's second validation target, from Verdian et al.
    /// 2021 — a talus runs from 2 mm to 575 mm, and the median moves by more than 10-fold
    /// between sites.
    ///
    /// The measure is the *area* each stone takes: a scatter of one size repeated reads as a
    /// cobble pavement, and a scatter of many sizes reads as scree. The stones' *positions*
    /// are checked too, because a network whose points all landed in one corner of their cells
    /// would be a grid of stones, not a scatter.
    #[test]
    fn the_stones_have_many_sizes() {
        // The coarse network's stones, over a 20 m square, and the area each one covers.
        let mut areas = Vec::new();
        let mut offsets = Vec::new();
        for j in 0..200 {
            for i in 0..200 {
                let p = Vec2::new(i as f32 * 0.1, j as f32 * 0.1);
                let cell = crate::gpu::noise_mirror::cell2(p / SCREE_CELL_M);
                let size = SCREE_SIZE_MIN + (SCREE_SIZE_MAX - SCREE_SIZE_MIN) * cell.factor.powi(3);
                let radius = 0.5 * size * SCREE_FILL;
                areas.push(radius * radius);
                // The point's offset inside its own cell, in [0, 1).
                offsets.push(cell.point.fract());
            }
        }
        areas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p10 = areas[areas.len() / 10];
        let p90 = areas[areas.len() * 9 / 10];
        // The areas span at least 4:1 between the tenth and the ninetieth percentile.
        assert!(
            p90 > p10 * 4.0,
            "the stones' areas span only {:.1}:1", p90 / p10
        );
        // The stones are spread inside their cells: the mean offset is near the middle, and
        // the offsets are not all the same point.
        let mean = offsets.iter().fold(Vec2::ZERO, |a, b| a + *b) / offsets.len() as f32;
        assert!(
            mean.x > 0.35 && mean.x < 0.65 && mean.y > 0.35 && mean.y < 0.65,
            "the stones sit in the middle of their cells on average: {mean:?}"
        );
        let spread = offsets
            .iter()
            .fold(Vec2::ZERO, |a, b| a + (*b - mean).abs())
            / offsets.len() as f32;
        assert!(
            spread.x > 0.2 && spread.y > 0.2,
            "the stones are spread inside their cells: {spread:?}"
        );
    }

    /// The scree is at the rock's **foot**: on the meadow and on the cliff there are no
    /// stones, and the band's weight peaks where the soil's band dies and the rock's ramp
    /// begins (the crate's own layer weights: the soil is gone by 30° and the rock is full at
    /// 38°).
    #[test]
    fn the_scree_lies_at_the_rocks_foot() {
        assert_eq!(scree_weight(0.0), 0.0, "the meadow has no stones");
        assert_eq!(scree_weight(15.0), 0.0, "the meadow has no stones");
        assert_eq!(scree_weight(45.0), 0.0, "the cliff has no stones");
        // The band's weight is strongest just below the rock's ramp.
        let peak = (14..42)
            .map(|d| (d as f32, scree_weight(d as f32)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(d, _)| d)
            .unwrap_or(0.0);
        assert!(
            (22.0..=33.0).contains(&peak),
            "the scree peaks at {peak} degrees, at the rock's foot"
        );
        // And it is on the ground the soil has left and the rock has not taken: at 28° the
        // crate's own layer weights are mostly grass and soil.
        let w = layer_weights(28.0, 0.0, 0.5);
        assert!(w[0] < 0.1, "the rock has not begun at 28 degrees: {:?}", w);
        // The band is an *apron*, not a contour line: it is at least six degrees wide at
        // half its strength, or it traces the hillside's break as a line.
        let wide = (14..42)
            .filter(|d| scree_weight(*d as f32) > 0.5)
            .count();
        assert!(wide >= 6, "the scree's band is {wide} degrees wide, too narrow");
    }

    /// The turf's palette: the four tones' shares must sum to one by construction (so the
    /// mixture is a convex combination and the ground's mean cannot drift), every share must
    /// be in range, and the mixture must stay inside the tones' convex hull — the fresh
    /// grass is the turf's own colour, so a place with no dry tone must be exactly the
    /// identity.
    ///
    /// Three other normalisations were tried and each failed: by the unweighted mean of the
    /// tones the ground's mean brightness drifted 3.2 DN, by the mixture's own colour the hue
    /// was cancelled entirely (the G/R ratio was 1.0 at every field), and by a per-channel
    /// mean the hue was cancelled again. The shares are the fourth way and the only one that
    /// keeps both the hue and the mean.
    #[test]
    fn the_turf_palette_shares_sum_to_one_and_keep_the_mean_and_the_hue() {
        let mut hues = Vec::new();
        for field in (-30..=30).map(|k| k as f32 / 20.0) {
            for clump in (-20..=20).map(|k| k as f32 / 20.0) {
                let (mixed, shares) = turf_palette(field, clump);
                assert!(
                    (shares.iter().sum::<f32>() - 1.0).abs() < 1e-5,
                    "the shares do not sum to one at {field}/{clump}: {shares:?}"
                );
                assert!(
                    shares.iter().all(|s| (0.0..=1.0).contains(s)),
                    "a share is out of range at {field}/{clump}: {shares:?}"
                );
                assert!(
                    mixed.iter().all(|c| (0.5..=2.0).contains(c)),
                    "the mixture left the tones' range at {field}/{clump}: {mixed:?}"
                );
                // The hue must survive: the G/R ratio has to *move* with the field, or the
                // palette is a brightness change and not a colour change.
                hues.push(mixed[1] / mixed[0]);
            }
        }
        let min = hues.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = hues.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        // The ratio's *maximum* is 1.0 — the fresh tone is the identity, so a place with no
        // dry tone is exactly the turf's own hue — and its minimum is the straw's.
        assert!(
            min < 0.95 && max <= 1.0 + 1e-5,
            "the palette's hue does not move: the G/R ratio spans {min} to {max}"
        );
        // A place with no dry tone is exactly the turf's own colour.
        let (mixed, _) = turf_palette(0.0, 0.0);
        for c in mixed {
            assert!((c - 1.0).abs() < 1e-5, "the fresh tone is not the identity: {mixed:?}");
        }
    }

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
