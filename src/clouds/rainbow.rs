//! The rainbow: Airy theory for a Marshall–Palmer shower, tabulated against the scattering angle
//! for the sky composite (`docs/research/rainbow.md`).
//!
//! The table is `p(θ, a₀)`, the phase function of a shower of median drop radius `a₀` per unit of
//! its optical depth, in sr⁻¹ and per linear-sRGB channel: a thin curtain of depth `τ` lit by an
//! illuminance `E` has the radiance `E · p · τ`. The composite reads it at the angle between the
//! sun's direction and the pixel's ray.

use bevy::light::light_consts::lux;
use bevy::prelude::*;

use crate::celestial::SunState;
use crate::climate::Weather;
use crate::precipitation::model::{rain_extinction, snow_extinction};

const PI: f64 = std::f64::consts::PI;

/// First scattering angle of the table, degrees: past the secondary bow's outer supernumeraries.
pub(crate) const THETA_MIN_DEG: f64 = 110.0;
/// Last scattering angle of the table, degrees.
pub(crate) const THETA_MAX_DEG: f64 = 170.0;
/// Angles in the table.
pub(crate) const ANGLES: usize = 512;
/// Median drop radii in the table, log-spaced from [`RADIUS_MIN_MM`] to [`RADIUS_MAX_MM`].
pub(crate) const RADII: usize = 16;
/// Smallest median radius of the table, mm.
pub(crate) const RADIUS_MIN_MM: f64 = 0.02;
/// Largest median radius of the table, mm.
pub(crate) const RADIUS_MAX_MM: f64 = 1.2;

/// Drop sizes integrated per row, log-spaced between these radii, mm.
const SIZES: usize = 32;
const SIZE_MIN_MM: f64 = 0.01;
const SIZE_MAX_MM: f64 = 3.5;
/// Wavelengths integrated, nm: 400 to 700 in 15 nm steps.
const WAVELENGTHS: usize = 21;
/// The sun's disc blurs every fringe finer than its 0.53°: the table's rows are convolved along
/// the angle with the disc's chord profile, `sqrt(1 − (k / 2.3)²)` for `k` texels from the centre
/// (the disc's radius, 0.265°, is 2.3 texels).
const DISC_KERNEL: [f64; 5] = [0.486, 0.892, 1.0, 0.892, 0.486];
/// Marshall–Palmer: `Λ D₀ = 3.67` for the median volume diameter `D₀`.
const MP_LAMBDA_D0: f64 = 3.67;

/// Refractive index of water at a wavelength in nm, fitted to Sadeghi et al. 2012's 1.3314 at
/// 700 nm and 1.3445 at 400 nm.
pub(crate) fn water_index(lambda_nm: f64) -> f64 {
    1.32505 + 3112.0 / (lambda_nm * lambda_nm)
}

/// The Airy function `Ai(x)`: its Maclaurin series where that is well conditioned, the
/// asymptotic forms beyond.
pub(crate) fn airy_ai(x: f64) -> f64 {
    const C1: f64 = 0.355_028_053_887_817_2;
    const C2: f64 = 0.258_819_403_792_806_8;
    if x > 5.0 {
        let zeta = 2.0 / 3.0 * x.powf(1.5);
        return (-zeta).exp() / (2.0 * PI.sqrt() * x.powf(0.25)) * (1.0 - 5.0 / (72.0 * zeta));
    }
    if x < -8.0 {
        let z = -x;
        let zeta = 2.0 / 3.0 * z.powf(1.5);
        let phase = zeta + PI / 4.0;
        return (phase.sin() - 5.0 / (72.0 * zeta) * phase.cos()) / (PI.sqrt() * z.powf(0.25));
    }
    let x3 = x * x * x;
    let (mut f, mut g) = (1.0, x);
    let (mut tf, mut tg) = (1.0, x);
    for k in 0..60 {
        let k = f64::from(k);
        tf *= x3 / ((3.0 * k + 2.0) * (3.0 * k + 3.0));
        tg *= x3 / ((3.0 * k + 3.0) * (3.0 * k + 4.0));
        f += tf;
        g += tg;
        if tf.abs() < 1e-18 && tg.abs() < 1e-18 {
            break;
        }
    }
    C1 * f - C2 * g
}

/// What a ray's geometry gives for one order of bow, one index and one polarisation-averaged
/// Fresnel weight.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Caustic {
    /// The geometric bow's scattering angle, radians.
    pub theta_r: f64,
    /// `cos i₀` of Descartes' ray.
    pub cos_i0: f64,
    /// `sin i₀ cos i₀ ε`, the ray's share of the cross-section's area element.
    pub weight: f64,
    /// `D''(i₀)`, rad⁻¹.
    pub curvature: f64,
}

/// Deviation of a ray of incidence `i` after `n` internal reflections, radians.
pub(crate) fn deviation(i: f64, n: u32, eta: f64) -> f64 {
    let r = (i.sin() / eta).asin();
    2.0 * i - 2.0 * (f64::from(n) + 1.0) * r + f64::from(n) * PI
}

/// Descartes' ray for `n` internal reflections (1 primary, 2 secondary) at index `eta`.
pub(crate) fn caustic(n: u32, eta: f64) -> Caustic {
    let nf = f64::from(n);
    let cos_i0 = ((eta * eta - 1.0) / (nf * (nf + 2.0))).sqrt();
    let i0 = cos_i0.acos();
    let sin_r = i0.sin() / eta;
    let r0 = sin_r.asin();
    let cos_r = r0.cos();
    let d = deviation(i0, n, eta);
    let theta_r = if n == 2 { 2.0 * PI - d } else { d };
    let curvature = -2.0 * (nf + 1.0) * (-eta * i0.sin() * cos_r + cos_i0 * cos_i0 * sin_r / cos_r)
        / (eta * eta * cos_r * cos_r);
    let (rs, rp) = fresnel_reflectance(cos_i0, cos_r, eta);
    let eps =
        0.5 * ((1.0 - rs).powi(2) * rs.powi(n as i32) + (1.0 - rp).powi(2) * rp.powi(n as i32));
    Caustic {
        theta_r,
        cos_i0,
        weight: i0.sin() * cos_i0 * eps,
        curvature,
    }
}

/// Fresnel reflectances `(R_s, R_p)` of a dielectric interface for an incident cosine `ci`,
/// refracted cosine `cr` and relative index `eta`.
fn fresnel_reflectance(ci: f64, cr: f64, eta: f64) -> (f64, f64) {
    let rs = ((ci - eta * cr) / (ci + eta * cr)).powi(2);
    let rp = ((eta * ci - cr) / (eta * ci + cr)).powi(2);
    (rs, rp)
}

/// Airy's differential cross-section of one drop for one order of bow at one wavelength: what
/// does not depend on the angle, hoisted out of the table's loop.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Section {
    theta_r: f64,
    secondary: bool,
    /// `C`, rad⁻¹: `z = δ · C`.
    scale: f64,
    /// `4π a² sin i₀ cos i₀ ε √C / √(2 D'')`, mm²: the cross-section's amplitude before `1 / sin θ`.
    amplitude: f64,
}

impl Section {
    pub(crate) fn new(c: &Caustic, n: u32, lambda_nm: f64, a_mm: f64) -> Self {
        let ka = 2.0 * PI * (a_mm * 1.0e6) / lambda_nm;
        let scale = (ka * c.cos_i0).powf(2.0 / 3.0) * (2.0 / c.curvature).powf(1.0 / 3.0);
        Self {
            theta_r: c.theta_r,
            secondary: n == 2,
            scale,
            amplitude: 4.0 * PI * a_mm * a_mm * c.weight * scale.sqrt()
                / (2.0 * c.curvature).sqrt(),
        }
    }

    /// `dσ/dΩ`, mm²/sr, at the scattering angle `theta` (radians).
    pub(crate) fn at(&self, theta: f64) -> f64 {
        // The lit side is `θ > θ_R` for the primary and `θ < θ_R` for the secondary.
        let delta = if self.secondary {
            self.theta_r - theta
        } else {
            theta - self.theta_r
        };
        let z = -delta * self.scale;
        // Deep in the shadow the Airy function is zero to f32; far on the lit side the quadratic
        // expansion of D(i) no longer holds: the glow fades out over 8° to 22°.
        if z > 12.0 || delta.to_degrees() > 22.0 {
            return 0.0;
        }
        let ai = airy_ai(z);
        let fade = 1.0 - smooth(8.0, 22.0, delta.max(0.0).to_degrees());
        self.amplitude / theta.sin().max(0.05) * ai * ai * fade
    }
}

fn smooth(lo: f64, hi: f64, x: f64) -> f64 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The CIE 1931 colour matching functions by Wyman, Sloan and Shirley 2013 (multi-lobe fit).
fn cie_xyz(lambda_nm: f64) -> [f64; 3] {
    let g = |l: f64, mu: f64, s1: f64, s2: f64| {
        let t = (l - mu) / if l < mu { s1 } else { s2 };
        (-0.5 * t * t).exp()
    };
    let x = 1.056 * g(lambda_nm, 599.8, 37.9, 31.0) + 0.362 * g(lambda_nm, 442.0, 16.0, 26.7)
        - 0.065 * g(lambda_nm, 501.1, 20.4, 26.2);
    let y = 0.821 * g(lambda_nm, 568.8, 46.9, 40.5) + 0.286 * g(lambda_nm, 530.9, 16.3, 31.1);
    let z = 1.217 * g(lambda_nm, 437.0, 11.8, 36.0) + 0.681 * g(lambda_nm, 459.0, 26.0, 13.8);
    [x, y, z]
}

/// Linear sRGB of an equal-energy-normalised spectrum sample at `lambda_nm`.
fn spectral_rgb(lambda_nm: f64) -> [f64; 3] {
    let [x, y, z] = cie_xyz(lambda_nm);
    [
        3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
        0.0557 * x - 0.2040 * y + 1.0570 * z,
    ]
}

/// The per-channel gain that makes an equal-energy spectrum white.
fn white_gain() -> [f64; 3] {
    let mut sum = [0.0; 3];
    for j in 0..WAVELENGTHS {
        let c = spectral_rgb(lambda_of(j));
        for k in 0..3 {
            sum[k] += c[k];
        }
    }
    [1.0 / sum[0], 1.0 / sum[1], 1.0 / sum[2]]
}

fn lambda_of(j: usize) -> f64 {
    400.0 + 15.0 * j as f64
}

/// The median radius of a Marshall–Palmer shower of `rate_mm_h`, mm: `D₀ = 0.9 R^0.21` mm.
pub(crate) fn median_radius_mm(rate_mm_h: f32) -> f32 {
    0.45 * rate_mm_h.max(1.0e-3).powf(0.21)
}

/// The table row's texture coordinate (texel centre) for a median radius in mm.
pub(crate) fn row_coordinate(radius_mm: f32) -> f32 {
    let t = ((f64::from(radius_mm) / RADIUS_MIN_MM).ln() / (RADIUS_MAX_MM / RADIUS_MIN_MM).ln())
        .clamp(0.0, 1.0);
    ((t * (RADII - 1) as f64 + 0.5) / RADII as f64) as f32
}

/// The phase-function table: `ANGLES` columns by `RADII` rows of `[r, g, b]` in sr⁻¹ per unit
/// optical depth, row-major.
pub(crate) fn build_table() -> Vec<[f32; 3]> {
    let gain = white_gain();
    // Each wavelength's colour with the white gain folded in.
    let colours: [[f64; 3]; WAVELENGTHS] = std::array::from_fn(|j| {
        let m = spectral_rgb(lambda_of(j));
        [m[0] * gain[0], m[1] * gain[1], m[2] * gain[2]]
    });
    let sizes: Vec<f64> = (0..SIZES)
        .map(|s| SIZE_MIN_MM * (SIZE_MAX_MM / SIZE_MIN_MM).powf(s as f64 / (SIZES - 1) as f64))
        .collect();
    // Per drop size and angle, the cross-section in each channel, over both orders, the
    // wavelengths.
    let casts: [[Caustic; 2]; WAVELENGTHS] = std::array::from_fn(|j| {
        let eta = water_index(lambda_of(j));
        [caustic(1, eta), caustic(2, eta)]
    });
    let mut q = vec![[0.0f64; 3]; SIZES * ANGLES];
    for (s, &a) in sizes.iter().enumerate() {
        let sections: [[Section; 2]; WAVELENGTHS] = std::array::from_fn(|j| {
            [
                Section::new(&casts[j][0], 1, lambda_of(j), a),
                Section::new(&casts[j][1], 2, lambda_of(j), a),
            ]
        });
        for col in 0..ANGLES {
            let theta = (THETA_MIN_DEG
                + (THETA_MAX_DEG - THETA_MIN_DEG) * (col as f64 + 0.5) / ANGLES as f64)
                .to_radians();
            let mut rgb = [0.0; 3];
            for (j, pair) in sections.iter().enumerate() {
                let sigma = pair[0].at(theta) + pair[1].at(theta);
                for k in 0..3 {
                    rgb[k] += sigma * colours[j][k];
                }
            }
            q[s * ANGLES + col] = rgb;
        }
    }
    let mut table = Vec::with_capacity(RADII * ANGLES);
    for row in 0..RADII {
        let a0 =
            RADIUS_MIN_MM * (RADIUS_MAX_MM / RADIUS_MIN_MM).powf(row as f64 / (RADII - 1) as f64);
        // Weights `a² n(a) da` with `da = a d ln a`, over the extinction `2π a²`.
        let w: Vec<f64> = sizes
            .iter()
            .map(|&a| a * a * a * (-MP_LAMBDA_D0 * a / a0).exp())
            .collect();
        let extinction: f64 = sizes
            .iter()
            .zip(&w)
            .map(|(&a, &w)| w * 2.0 * PI * a * a)
            .sum();
        for col in 0..ANGLES {
            let mut rgb = [0.0; 3];
            for (s, &w) in w.iter().enumerate() {
                for k in 0..3 {
                    rgb[k] += w * q[s * ANGLES + col][k];
                }
            }
            table.push(rgb.map(|v| (v.max(0.0) / extinction) as f32));
        }
    }
    // The sun's disc, along each row.
    let norm: f64 = DISC_KERNEL.iter().sum();
    let mut blurred = table.clone();
    for row in 0..RADII {
        for col in 0..ANGLES {
            let mut sum = [0.0f64; 3];
            for (k, w) in DISC_KERNEL.iter().enumerate() {
                let at = (col + k).saturating_sub(2).min(ANGLES - 1);
                for c in 0..3 {
                    sum[c] += w * f64::from(table[row * ANGLES + at][c]);
                }
            }
            blurred[row * ANGLES + col] = sum.map(|v| (v / norm) as f32);
        }
    }
    blurred
}

/// The table as `Rgba16Float` texel bytes (`ANGLES` × `RADII`, alpha 1).
pub(crate) fn table_bytes() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(ANGLES * RADII * 8);
    for texel in build_table() {
        for c in [texel[0], texel[1], texel[2], 1.0] {
            bytes.extend_from_slice(&half_bits(c).to_le_bytes());
        }
    }
    bytes
}

/// `f32` to IEEE half, rounding to nearest even; overflow saturates to the largest half.
fn half_bits(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let man = bits & 0x007f_ffff;
    if exp >= 31 {
        return sign | 0x7bff;
    }
    if exp <= 0 {
        if exp < -10 {
            return sign;
        }
        let m = (man | 0x0080_0000) >> (1 - exp);
        return sign | ((m + 0x0fff + ((m >> 13) & 1)) >> 13) as u16;
    }
    let rounded = (man + 0x0fff + ((man >> 13) & 1)) >> 13;
    sign | (((exp as u32) << 10) + rounded) as u16
}

/// Optical depth of Bevy's `Atmosphere::earth` at the zenith per channel (Hillaire's Table 1 on
/// the effective scale heights of `ScatteringMedium::earth`, `model::bevy_optical_transmittance`):
/// the sun's colour at the camera is `exp(−τ · air mass)`.
const ZENITH_OPTICAL_DEPTH: [f32; 3] = [0.096, 0.218, 0.451];

/// Kasten and Young's relative air mass (1989) of a sun at `altitude_deg`.
fn air_mass(altitude_deg: f32) -> f32 {
    let h = altitude_deg.max(0.0);
    1.0 / (h.to_radians().sin() + 0.505_72 * (h + 6.079_95).powf(-1.636_4))
}

/// The fraction of the sun's light that reaches the ground per channel, for a sun at
/// `altitude_deg` (clear air).
pub(crate) fn sun_transmittance(altitude_deg: f32) -> Vec3 {
    let am = air_mass(altitude_deg);
    Vec3::from(ZENITH_OPTICAL_DEPTH.map(|tau| (-tau * am).exp()))
}

/// The rate the weather map's green channel stands for at 1, mm/h
/// ([`CloudLayer::precipitation_from_weather`](super::CloudLayer::precipitation_from_weather)).
const MAP_FULL_RATE_MM_H: f32 = 25.0;

/// The share of the weather's precipitation that falls as rain, 0–1. Only liquid drops make a
/// bow; a snowflake is neither liquid nor round (`docs/research/rainbow.md`, "Only the liquid
/// share makes a bow").
fn rain_share(weather: &Weather) -> f32 {
    1.0 - weather.snow_fraction()
}

/// The rainbow's uniform (`CloudParams::bow`): the sun's light at the camera and the row of the
/// phase-function table that the rain's median drop picks; `w` is −1 (no bow) while the sun is
/// below the horizon, no rain falls (snow makes none), or the strength is 0.
pub(crate) fn bow_params(sun: &SunState, weather: &Weather, beam: f32, strength: f32) -> Vec4 {
    let rain_mm_h = weather.precipitation_mm_h * rain_share(weather);
    if strength <= 0.0 || sun.altitude_deg <= 0.0 || rain_mm_h <= 0.0 {
        return Vec4::new(0.0, 0.0, 0.0, -1.0);
    }
    let light =
        sun_transmittance(sun.altitude_deg) * (lux::RAW_SUNLIGHT * beam.clamp(0.0, 1.0) * strength);
    light.extend(row_coordinate(median_radius_mm(rain_mm_h)))
}

/// The rainbow's curtain (`CloudParams::shower`), the extinctions where the weather map's green
/// is 1, m⁻¹: x the rain's, y the snow's. The green is the total rate over 25 mm/h, so a texel of
/// green `g` holds `25 g · rain_share` mm/h of rain, extinction `x · g^0.68`, and the rest as
/// snow, extinction `y · g`. The two share the curtain's light by their extinction and only the
/// rain's makes the bow (`docs/research/rainbow.md`, "Snow in the curtain takes its share of the
/// light").
pub(crate) fn shower_params(weather: &Weather) -> Vec4 {
    let rain = rain_share(weather);
    Vec4::new(
        rain_extinction(MAP_FULL_RATE_MM_H * rain),
        snow_extinction(MAP_FULL_RATE_MM_H * (1.0 - rain)),
        0.0,
        0.0,
    )
}

/// Settings of the rainbow ([`crate::CloudPlugin`]).
///
/// Insert it before the plugin to override. A bow needs the sun below 42° of elevation, a
/// shower in the weather map opposite it and the sun reaching that shower
/// (`docs/research/rainbow.md`).
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct RainbowSettings {
    /// The gain on the sun's light on the shower; 0 switches the rainbow off, 1 is the thin-curtain
    /// estimate `E · p · τ` as it stands. Default 5, which makes the primary bow of a 5 mm/h
    /// shower add about a third (32 %) to the sky behind it under a 22° sun (`docs/research/rainbow.md`,
    /// Deviation 1).
    pub strength: f32,
}

impl Default for RainbowSettings {
    fn default() -> Self {
        Self { strength: 5.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(rad: f64) -> f64 {
        rad.to_degrees()
    }

    #[test]
    fn airy_matches_its_tabulated_values() {
        assert!((airy_ai(0.0) - 0.355_028).abs() < 1e-6);
        assert!((airy_ai(1.0) - 0.135_292).abs() < 1e-6);
        assert!((airy_ai(-1.0) - 0.535_561).abs() < 1e-6);
        // Where the series hands over to the asymptotic forms the function is continuous.
        for edge in [5.0, -8.0] {
            let (a, b) = (airy_ai(edge - 1e-9), airy_ai(edge + 1e-9));
            assert!((a - b).abs() < 2e-3 * a.abs().max(1e-6), "{edge}: {a} {b}");
        }
        // First maximum of Ai²(−z).
        let peak = (0..4000)
            .map(|k| 0.5 + k as f64 * 0.001)
            .max_by(|a, b| airy_ai(-a).powi(2).total_cmp(&airy_ai(-b).powi(2)))
            .unwrap_or(0.0);
        assert!((peak - 1.0188).abs() < 2e-3, "{peak}");
    }

    #[test]
    fn the_geometric_bows_are_where_sadeghi_puts_them() {
        let red = caustic(1, 1.3314);
        let violet = caustic(1, 1.3445);
        assert!(
            (deg(red.theta_r) - 137.7).abs() < 0.3,
            "{}",
            deg(red.theta_r)
        );
        assert!(
            (deg(violet.theta_r) - 139.6).abs() < 0.3,
            "{}",
            deg(violet.theta_r)
        );
        let red2 = caustic(2, 1.3314);
        let violet2 = caustic(2, 1.3445);
        assert!(
            (deg(red2.theta_r) - 129.5).abs() < 0.5,
            "{}",
            deg(red2.theta_r)
        );
        assert!(
            (deg(violet2.theta_r) - 126.1).abs() < 0.5,
            "{}",
            deg(violet2.theta_r)
        );
    }

    #[test]
    fn the_curvature_is_the_second_derivative_of_the_deviation() {
        for (n, eta) in [(1, 1.3314), (2, 1.3445), (1, 1.34)] {
            let c = caustic(n, eta);
            let i0 = c.cos_i0.acos();
            let h = 1e-4;
            let second = (deviation(i0 + h, n, eta) - 2.0 * deviation(i0, n, eta)
                + deviation(i0 - h, n, eta))
                / (h * h);
            assert!(
                (second - c.curvature).abs() < 0.01 * c.curvature.abs(),
                "order {n}: {second} against {}",
                c.curvature
            );
            // Descartes' ray is the minimum of the deviation.
            let d = |i: f64| deviation(i, n, eta);
            assert!(d(i0) < d(i0 + 0.01) && d(i0) < d(i0 - 0.01));
        }
    }

    #[test]
    fn a_large_drops_first_fringe_lies_where_airy_puts_it() {
        let eta = water_index(550.0);
        let c = caustic(1, eta);
        // The cross-section's peak over the angles of a 0.1 mm drop.
        let best = (0..6000)
            .map(|k| c.theta_r + (k as f64 - 1000.0) * 1e-5)
            .max_by(|a, b| {
                Section::new(&c, 1, 550.0, 0.1)
                    .at(*a)
                    .total_cmp(&Section::new(&c, 1, 550.0, 0.1).at(*b))
            })
            .unwrap_or(0.0);
        let offset = deg(best - c.theta_r);
        assert!((0.85..1.05).contains(&offset), "{offset}");
        // 0.5 mm: closer to the geometric angle by (a ratio)^(2/3).
        let best5 = (0..6000)
            .map(|k| c.theta_r + (k as f64 - 1000.0) * 1e-5)
            .max_by(|a, b| {
                Section::new(&c, 1, 550.0, 0.5)
                    .at(*a)
                    .total_cmp(&Section::new(&c, 1, 550.0, 0.5).at(*b))
            })
            .unwrap_or(0.0);
        let ratio = offset / deg(best5 - c.theta_r);
        assert!((ratio - 5.0f64.powf(2.0 / 3.0)).abs() < 0.15, "{ratio}");
    }

    fn row_at(table: &[[f32; 3]], radius_mm: f64) -> &[[f32; 3]] {
        let t = (radius_mm / RADIUS_MIN_MM).ln() / (RADIUS_MAX_MM / RADIUS_MIN_MM).ln();
        let row = (t * (RADII - 1) as f64).round() as usize;
        &table[row * ANGLES..(row + 1) * ANGLES]
    }

    fn col_of(theta_deg: f64) -> usize {
        ((theta_deg - THETA_MIN_DEG) / (THETA_MAX_DEG - THETA_MIN_DEG) * ANGLES as f64) as usize
    }

    fn peak_angle(row: &[[f32; 3]], channel: usize, from: f64, to: f64) -> f64 {
        let best = (col_of(from)..col_of(to))
            .max_by(|a, b| row[*a][channel].total_cmp(&row[*b][channel]))
            .unwrap_or(0);
        THETA_MIN_DEG + (best as f64 + 0.5) * (THETA_MAX_DEG - THETA_MIN_DEG) / ANGLES as f64
    }

    /// One table for the tests: building it is the cost of a start-up.
    fn table() -> &'static Vec<[f32; 3]> {
        static TABLE: std::sync::LazyLock<Vec<[f32; 3]>> = std::sync::LazyLock::new(build_table);
        &TABLE
    }

    #[test]
    fn a_shower_has_two_bows_with_opposite_colour_orders_and_a_dark_band() {
        let table = table();
        let row = row_at(table, 0.45 * 5.0f64.powf(0.21));
        // Primary: red outside (larger angle is *inside*: red is at the smaller angle).
        let red = peak_angle(row, 0, 134.0, 142.0);
        let blue = peak_angle(row, 2, 134.0, 142.0);
        assert!(red < blue, "primary: red {red}, blue {blue}");
        // Secondary: the order reverses.
        let red2 = peak_angle(row, 0, 122.0, 132.0);
        let blue2 = peak_angle(row, 2, 122.0, 132.0);
        assert!(red2 > blue2, "secondary: red {red2}, blue {blue2}");
        let primary = row[col_of(134.0)..col_of(142.0)]
            .iter()
            .map(|t| t[0] + t[1] + t[2])
            .fold(0.0, f32::max);
        let secondary = row[col_of(122.0)..col_of(132.0)]
            .iter()
            .map(|t| t[0] + t[1] + t[2])
            .fold(0.0, f32::max);
        let band = row[col_of(132.7)..col_of(133.3)]
            .iter()
            .map(|t| t[0] + t[1] + t[2])
            .fold(0.0, f32::max);
        assert!(secondary < primary, "{secondary} against {primary}");
        assert!(
            secondary > 0.05 * primary,
            "the secondary vanishes: {secondary} / {primary}"
        );
        assert!(
            band < 0.05 * primary,
            "Alexander's band is {} of the primary",
            band / primary
        );
    }

    #[test]
    fn small_drops_make_a_wider_and_paler_bow() {
        let table = table();
        let width_and_saturation = |radius: f64| {
            let row = row_at(table, radius);
            // The red band alone: the spread of the colours across 2° is not the bow's own width.
            let lum = |t: &[f32; 3]| t[0];
            let peak = row[col_of(134.0)..col_of(142.0)]
                .iter()
                .map(lum)
                .fold(0.0, f32::max);
            // The run of columns around the peak that stays above half of it.
            let top = (col_of(134.0)..col_of(142.0))
                .max_by(|a, b| lum(&row[*a]).total_cmp(&lum(&row[*b])))
                .unwrap_or(0);
            let mut wide = 1;
            let mut k = top;
            while k > 0 && lum(&row[k - 1]) > 0.5 * peak {
                k -= 1;
                wide += 1;
            }
            k = top;
            while k + 1 < ANGLES && lum(&row[k + 1]) > 0.5 * peak {
                k += 1;
                wide += 1;
            }
            // Saturation at the brightest texel.
            let t = row[col_of(134.0)..col_of(142.0)]
                .iter()
                .max_by(|a, b| lum(a).total_cmp(&lum(b)))
                .copied()
                .unwrap_or([0.0; 3]);
            let mx = t[0].max(t[1]).max(t[2]);
            let mn = t[0].min(t[1]).min(t[2]);
            (wide, (mx - mn) / mx.max(1e-9))
        };
        let (wide_big, sat_big) = width_and_saturation(1.0);
        let (wide_small, sat_small) = width_and_saturation(0.05);
        assert!(
            wide_small > wide_big,
            "{wide_small} against {wide_big} (saturation {sat_small}, {sat_big})"
        );
        assert!(sat_small < sat_big, "{sat_small} against {sat_big}");
    }

    #[test]
    fn the_table_is_finite_and_its_rows_are_found() {
        let table = table();
        assert_eq!(table.len(), ANGLES * RADII);
        assert!(table.iter().flatten().all(|v| v.is_finite() && *v >= 0.0));
        assert!((row_coordinate(RADIUS_MIN_MM as f32) - 0.5 / RADII as f32).abs() < 1e-6);
        assert!(
            (row_coordinate(RADIUS_MAX_MM as f32) - (RADII as f32 - 0.5) / RADII as f32).abs()
                < 1e-6
        );
        // A drizzle's median is smaller than a thunderstorm's.
        assert!(median_radius_mm(0.5) < median_radius_mm(25.0));
    }

    #[test]
    fn the_suns_colour_follows_the_atmospheres_own_transmittance() {
        for altitude in [8.0f32, 15.0, 25.0, 40.0, 70.0] {
            let own = sun_transmittance(altitude);
            let oracle = super::super::model::bevy_optical_transmittance(
                0.0,
                f64::from(altitude).to_radians().sin(),
            );
            for c in 0..3 {
                let rel = (f64::from(own[c]) - oracle[c]).abs() / oracle[c];
                assert!(
                    rel < 0.15,
                    "{altitude} deg, channel {c}: {} against {}",
                    own[c],
                    oracle[c]
                );
            }
        }
    }

    #[test]
    fn half_floats_round_trip() {
        for v in [0.0f32, 1.0, 0.5, 3.25, 0.000_1, 120.0, 1.0e-6] {
            let bits = half_bits(v);
            let sign = 1.0;
            let exp = ((bits >> 10) & 0x1f) as i32;
            let man = (bits & 0x3ff) as f32;
            let back = if exp == 0 {
                sign * man * 2.0f32.powi(-24)
            } else {
                sign * (1.0 + man / 1024.0) * 2.0f32.powi(exp - 15)
            };
            assert!((back - v).abs() <= v * 1.0e-3 + 1.0e-7, "{v} -> {back}");
        }
    }

    fn shower(rate_mm_h: f32, temperature_c: f32) -> Weather {
        Weather {
            precipitation_mm_h: rate_mm_h,
            temperature_c,
            snow_possible: 1.0,
            ..Weather::default()
        }
    }

    fn low_sun() -> SunState {
        SunState {
            altitude_deg: 20.0,
            ..SunState::default()
        }
    }

    #[test]
    fn falling_snow_draws_no_bow() {
        let snow = shower(5.0, -5.0);
        assert_eq!(snow.snow_fraction(), 1.0);
        assert_eq!(
            bow_params(&low_sun(), &snow, 1.0, 5.0).w,
            -1.0,
            "snow makes no rainbow"
        );
        assert_eq!(
            shower_params(&snow).x,
            0.0,
            "snow puts no drops in the curtain"
        );
        let rain = shower(5.0, 15.0);
        assert!(
            bow_params(&low_sun(), &rain, 1.0, 5.0).w >= 0.0,
            "rain still does"
        );
        // Rain alone: the full rate's extinction and no snow in the curtain.
        assert_eq!(
            shower_params(&rain),
            Vec4::new(rain_extinction(25.0), 0.0, 0.0, 0.0)
        );
    }

    /// The rain's share of a weather-map texel's extinction at green `g` (`composite.wesl`).
    fn rain_part(weather: &Weather, g: f32) -> f32 {
        let s = shower_params(weather);
        let rain = s.x * g.powf(0.68);
        rain / (rain + s.y * g)
    }

    #[test]
    fn sleet_draws_the_bow_of_its_rain_through_its_snow() {
        // At 0.5 °C half of 5 mm/h falls as snow: the drops are those of 2.5 mm/h of rain.
        let sleet = shower(5.0, 0.5);
        assert_eq!(sleet.snow_fraction(), 0.5);
        let sun = low_sun();
        assert_eq!(
            bow_params(&sun, &sleet, 1.0, 5.0),
            bow_params(&sun, &shower(2.5, 15.0), 1.0, 5.0)
        );
        // A weather-map texel of green g stands for 25 g mm/h in all, half rain and half snow.
        for g in [0.05f32, 0.2, 1.0] {
            let s = shower_params(&sleet);
            let rain = s.x * g.powf(0.68);
            let want = rain_extinction(25.0 * g * 0.5);
            assert!(
                (rain - want).abs() <= 1e-5 * want,
                "rain at green {g}: {rain} against {want}"
            );
            let snow = s.y * g;
            let want = snow_extinction(25.0 * g * 0.5);
            assert!(
                (snow - want).abs() <= 1e-5 * want,
                "snow at green {g}: {snow} against {want}"
            );
        }
        // The snow, some thirty times as opaque per mm/h, takes nearly all the curtain's light.
        let part = rain_part(&sleet, 0.2);
        assert!((part - 0.031).abs() < 0.002, "{part}");
    }

    #[test]
    fn the_bow_fades_through_the_sleet_band_without_a_step() {
        // From 2 °C (all rain) to −1 °C (all snow) the rain's share of the curtain falls
        // continuously and monotonically to 0, so the last rain draws almost nothing when the
        // gate shuts the bow. The fall is steep at first (the first 3 % of snow halves it), so
        // the sweep steps a thousandth of a degree.
        let mut previous = rain_part(&shower(5.0, 2.0), 0.2);
        assert_eq!(previous, 1.0);
        let mut last_open = 1.0;
        for k in 1..=3000 {
            let t = 2.0 - 3.0 * k as f32 / 3000.0;
            let weather = shower(5.0, t);
            let open = bow_params(&low_sun(), &weather, 1.0, 5.0).w >= 0.0;
            let part = if open { rain_part(&weather, 0.2) } else { 0.0 };
            assert!(part <= previous + 1e-6, "{t} °C: {part} after {previous}");
            assert!(
                previous - part < 0.02,
                "a step at {t} °C: {previous} to {part}"
            );
            if open {
                last_open = part;
            }
            previous = part;
        }
        assert_eq!(previous, 0.0);
        assert!(last_open < 0.01, "the gate shuts a bow of {last_open}");
    }
}
