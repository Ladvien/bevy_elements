//! The hot plume above a fire, from fire-engineering correlations: McCaffrey's three regimes for
//! the centreline's excess temperature and rise speed, Shabbir and George's plume width, Heskestad's
//! flame height, the puffing frequency, and the refractive index the temperature leaves in the air
//! (`docs/research/heat-haze.md`, Model 1–7). `haze.wesl` evaluates the same formulas per pixel.

/// Gravity, m/s².
const G: f32 = 9.81;
/// End of McCaffrey's continuous flame regime, `z* = z / Q^{2/5}` in m/kW^{2/5}.
pub(crate) const FLAME_END: f32 = 0.08;
/// End of McCaffrey's intermittent regime, m/kW^{2/5}.
pub(crate) const INTERMITTENT_END: f32 = 0.2;
/// McCaffrey's velocity constants κ: flame m^{1/2}/s, intermittent m/(s·kW^{1/5}), plume
/// m^{4/3}/(s·kW^{1/3}) (NBSIR 79-1910, p. 13).
const KAPPA: [f32; 3] = [6.8, 1.9, 1.1];
/// McCaffrey's buoyancy constant `V / √(2 g z ΔT/T∞)`.
const BUOYANCY: f32 = 0.9;
/// The 1/e radius of the plume's temperature profile over the height above its source,
/// `1/√68` (Shabbir and George 1994, `exp(−68 η²)`).
pub(crate) const PLUME_WIDTH: f32 = 0.121;
/// The air's refractivity `n − 1` at 550 nm, 15 °C and 101 325 Pa (Edlén 1966).
const REFRACTIVITY_15C: f32 = 2.778e-4;
/// 15 °C in kelvin.
const KELVIN_15C: f32 = 288.15;
/// `ρ∞ c_p T∞`, kJ/m³, for Heskestad's `Q*`: at constant pressure `ρ∞ T∞ = p / R_air`, so the
/// air's temperature cancels (`101 325 Pa / 287.05 J/(kg·K) · 1.005 kJ/(kg·K)`).
const RHO_CP_T: f32 = 101_325.0 / 287.05 * 1.005;
/// Kelvin at 0 °C.
pub(crate) const KELVIN: f32 = 273.15;

/// McCaffrey's regime of a height `z_star` (m/kW^{2/5}): 0 flame, 1 intermittent, 2 plume.
fn regime(z_star: f32) -> usize {
    if z_star < FLAME_END {
        0
    } else if z_star < INTERMITTENT_END {
        1
    } else {
        2
    }
}

/// `ΔT/T∞` per regime at `z* = 1`: `(κ / (C √(2g)))²` (the flame's is its constant value).
fn excess_coefficient(regime: usize) -> f32 {
    let k = KAPPA[regime] / (BUOYANCY * (2.0 * G).sqrt());
    k * k
}

/// The centreline's mean excess temperature over the air's, as a fraction of the air's absolute
/// temperature, at `z_star = z / Q^{2/5}` (m/kW^{2/5}): `(κ/(C√(2g)))² z*^{2η−1}`, with
/// `η = 1/2, 0, −1/3` in the flame, intermittent and plume regimes.
pub(crate) fn excess_ratio(z_star: f32) -> f32 {
    let z = z_star.max(1e-6);
    let r = regime(z);
    let c = excess_coefficient(r);
    match r {
        0 => c,
        1 => c / z,
        _ => c * z.powf(-5.0 / 3.0),
    }
}

/// The centreline's mean rise speed, m/s, at `z_star` above a fire with `Q^{1/5}` = `q_fifth`
/// (kW^{1/5}): `κ z*^η Q^{1/5}`.
pub(crate) fn rise_speed(z_star: f32, q_fifth: f32) -> f32 {
    let z = z_star.max(1e-6);
    let r = regime(z);
    let eta = [0.5, 0.0, -1.0 / 3.0][r];
    KAPPA[r] * z.powf(eta) * q_fifth
}

/// How long the gas at the centreline takes to rise from the fire to `z_star`, s: `∫ dz / V`,
/// `Q^{1/5} τ(z*)` with `τ` closed-form in each regime.
pub(crate) fn rise_time(z_star: f32, q_fifth: f32) -> f32 {
    let z = z_star.max(0.0);
    let flame = |z: f32| 2.0 * z.sqrt() / KAPPA[0];
    let tau = if z < FLAME_END {
        flame(z)
    } else if z < INTERMITTENT_END {
        flame(FLAME_END) + (z - FLAME_END) / KAPPA[1]
    } else {
        flame(FLAME_END)
            + (INTERMITTENT_END - FLAME_END) / KAPPA[1]
            + 0.75 * (z.powf(4.0 / 3.0) - INTERMITTENT_END.powf(4.0 / 3.0)) / KAPPA[2]
    };
    tau * q_fifth
}

/// Height above a fire of `power_kw` at which the centreline's excess temperature falls to
/// `excess_k` in air at `air_k`, m (in the plume regime, which every excess below 320 K reaches).
pub(crate) fn height_at_excess(power_kw: f32, air_k: f32, excess_k: f32) -> f32 {
    let z_star = (excess_coefficient(2) * air_k / excess_k.max(1e-3)).powf(0.6);
    z_star.max(INTERMITTENT_END) * power_kw.max(0.0).powf(0.4)
}

/// The air's refractivity `n − 1` at `air_k`, at sea-level pressure: `ρ ∝ 1/T` (Gladstone–Dale).
pub(crate) fn refractivity(air_k: f32) -> f32 {
    REFRACTIVITY_15C * KELVIN_15C / air_k.max(1.0)
}

/// The plume's index deficit on its axis, `δn₀ = (n∞ − 1) ΔT/(T∞ + ΔT)`, from the excess ratio
/// `ΔT/T∞` and the air's refractivity.
pub(crate) fn index_deficit(excess_ratio: f32, refractivity: f32) -> f32 {
    refractivity * excess_ratio / (1.0 + excess_ratio)
}

/// The 1/e radius of the plume's temperature profile `height_m` above a fire of `radius_m`, m:
/// the fire's own radius at its base, Shabbir and George's `0.121 z` far above it.
pub(crate) fn plume_sigma(radius_m: f32, height_m: f32) -> f32 {
    (radius_m * radius_m + (PLUME_WIDTH * height_m).powi(2)).sqrt()
}

/// The mean deflection, radians, of a ray passing a Gaussian plume of 1/e radius `sigma` at
/// impact parameter `p`, for an index deficit `deficit` on its axis: the derivative of the
/// projected deficit `√π σ δn₀ e^{−p²/σ²}` (Raffel 2015, Eq. 2), positive away from the axis
/// (the hot core is a diverging lens).
pub(crate) fn mean_deflection(p: f32, sigma: f32, deficit: f32) -> f32 {
    let q = p / sigma;
    2.0 * std::f32::consts::PI.sqrt() * deficit * q * (-q * q).exp()
}

/// Heskestad's mean flame height, m, of a fire of `power_kw` on a base of `diameter_m`:
/// `D (3.7 Q*^{2/5} − 1.02)`, `Q* = Q / (ρ∞ c_p T∞ √(gD) D²)`, never negative (Delichatsios 2007,
/// Eq. 4).
pub(crate) fn flame_height(power_kw: f32, diameter_m: f32) -> f32 {
    if power_kw <= 0.0 || diameter_m <= 0.0 {
        return 0.0;
    }
    let q_star = power_kw / (RHO_CP_T * (G * diameter_m).sqrt() * diameter_m * diameter_m);
    (diameter_m * (3.7 * q_star.powf(0.4) - 1.02)).max(0.0)
}

/// The frequency at which a fire on a base of `diameter_m` sheds its vortex rings, Hz:
/// `0.48 √(g/D)`, about `1.5/√D` (Xia and Zhang 2018, after Cetegen and Ahmed).
pub(crate) fn puffing_frequency(diameter_m: f32) -> f32 {
    0.48 * (G / diameter_m.max(1e-3)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, rel: f32) -> bool {
        (a - b).abs() <= rel * b.abs()
    }

    /// Validation target 1: McCaffrey's centreline: 852 K in the flame at 293 K, joining the
    /// intermittent zone and the plume to the rounding of his constants.
    #[test]
    fn the_centreline_temperature_follows_mccaffreys_three_regimes() {
        let t0 = 293.0;
        assert!(close(excess_ratio(0.04) * t0, 852.5, 0.005));
        let flame_join = excess_ratio(FLAME_END - 1e-4) / excess_ratio(FLAME_END + 1e-4);
        let plume_join =
            excess_ratio(INTERMITTENT_END - 1e-4) / excess_ratio(INTERMITTENT_END + 1e-4);
        assert!((flame_join - 1.0).abs() < 0.03, "{flame_join}");
        assert!((plume_join - 1.0).abs() < 0.03, "{plume_join}");
        // The plume's law: 22.3 K at z* = 1, falling as z*^(−5/3).
        assert!(close(excess_ratio(1.0) * t0, 22.3, 0.01));
        assert!(close(excess_ratio(2.0) / excess_ratio(1.0), 2f32.powf(-5.0 / 3.0), 1e-4));
        // Monotone: the plume only cools as it rises.
        let mut last = f32::MAX;
        for k in 1..400 {
            let r = excess_ratio(k as f32 * 0.01);
            assert!(r <= last * 1.03, "z* {}: {r} after {last}", k as f32 * 0.01);
            last = r;
        }
    }

    /// The rise speed is McCaffrey's, and the rise time is its integral.
    #[test]
    fn the_rise_time_integrates_the_rise_speed() {
        let q_fifth = 50f32.powf(0.2);
        assert!(close(rise_speed(0.1, q_fifth), 1.9 * q_fifth, 1e-5));
        assert!(close(rise_speed(1.0, q_fifth), 1.1 * q_fifth, 1e-5));
        let q_two_fifths = 50f32.powf(0.4);
        for z_m in [0.2f32, 0.6, 1.5, 4.0] {
            // dt = dz / V, z = z* Q^{2/5}.
            let n = 20_000;
            let dz = z_m / n as f32;
            let numeric: f32 = (0..n)
                .map(|k| dz / rise_speed((k as f32 + 0.5) * dz / q_two_fifths, q_fifth))
                .sum();
            let closed = rise_time(z_m / q_two_fifths, q_fifth);
            assert!(close(closed, numeric, 0.01), "{z_m} m: {closed} vs {numeric}");
        }
    }

    /// Validation target 1: Heskestad's flame height, the puffing frequency, the refractivity and
    /// the Gaussian lens's largest deflection.
    #[test]
    fn a_campfire_has_the_height_pulse_and_lens_the_literature_gives() {
        let height = flame_height(50.0, 0.6);
        assert!((0.5..=0.6).contains(&height), "{height}");
        // A fire too weak for its base has no flame above it.
        assert_eq!(flame_height(0.5, 3.0), 0.0);
        assert!(close(puffing_frequency(0.6), 1.5 / 0.6f32.sqrt(), 0.02));
        let deficit = index_deficit(838.0 / 288.15, refractivity(288.15));
        assert!(close(deficit, 2.07e-4, 0.01), "{deficit}");
        // Colder air is denser and bends more.
        assert!(refractivity(263.15) > refractivity(288.15));
        let sigma = 0.3;
        let peak = (0..3000)
            .map(|k| mean_deflection(k as f32 * 0.001, sigma, 1.0))
            .fold(0.0f32, f32::max);
        assert!(close(peak, 1.52, 0.01), "{peak}");
        let at = mean_deflection(sigma / 2f32.sqrt(), sigma, 1.0);
        assert!(close(at, peak, 1e-3));
        assert!(mean_deflection(-0.1, sigma, 1.0) < 0.0);
    }

    /// `haze.wesl` normalises the turbulence by the rms of one component of `perlin3_d`'s
    /// gradient per lattice unit (its `NOISE_GRADIENT_RMS`).
    #[test]
    fn the_noise_gradient_rms_is_the_shaders_normalisation() {
        const SHADER_NOISE_GRADIENT_RMS: f32 = 1.0;
        let points: Vec<_> = crate::gpu::noise_mirror::samples(40_000, 512.0).collect();
        let rms = |c: usize| {
            let sum: f32 = points
                .iter()
                .map(|p| crate::gpu::noise_mirror::perlin3_d(*p)[c].powi(2))
                .sum();
            (sum / points.len() as f32).sqrt()
        };
        let (across, up) = (rms(1), rms(2));
        assert!(
            close(across, SHADER_NOISE_GRADIENT_RMS, 0.03)
                && close(up, SHADER_NOISE_GRADIENT_RMS, 0.03),
            "{across} {up}"
        );
    }

    /// The plume's top (10 K) and the width blend: the fire's radius at the base, `0.121 z` far up.
    #[test]
    fn the_plume_widens_from_the_fire_and_ends_where_it_is_ten_kelvin_warm() {
        let top = height_at_excess(50.0, 288.15, 10.0);
        let z_star = top / 50f32.powf(0.4);
        assert!(close(excess_ratio(z_star) * 288.15, 10.0, 1e-3), "{top}");
        assert!((7.0..8.0).contains(&top), "{top}");
        assert!(close(plume_sigma(0.3, 0.0), 0.3, 1e-6));
        assert!(close(plume_sigma(0.3, 50.0), 0.121 * 50.0, 0.01));
    }
}
