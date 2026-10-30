# Rainbows

The crate draws rain, a sun that breaks through cloud, and a sky that follows both, and it has no rainbow: the commonest sign of "the storm is ending" in any shipped game with weather (Horizon, Red Dead 2, Forza Horizon, Microsoft Flight Simulator all draw one) is missing. This note adds one in the sky composite, from the same weather map that places the rain and the same sun the sky is lit by, so a bow appears where a shower stands opposite the sun and nowhere else.

## Sources

- Sadeghi, Munoz, Laven, Jarosz, Seron, Gutierrez, Jensen 2012, *Physically-based simulation of rainbows*, ACM TOG 31(1), `10.1145/2077341.2077344` (home-still stem `10.1145_2077341.2077344`, downloaded and converted this pass; read pages 4 and 6). §3: the primary bow for red (η = 1.3314) is at θ = 137.7° and for violet (η = 1.3445, 400 nm) at 139.6°; the secondary bow at 129.5° (red) to 126.1° (violet), colours reversed, Alexander's dark band between; Airy's peak lies at about 138.9°, not at the geometric angle; supernumerary arcs are interference; drops under about 20 µm give white fog bows because the maxima overlap; the rainbow is a 1D function of the scattering angle θ between the incident and outgoing directions.
- Airy 1838 and van de Hulst 1957, *Light Scattering by Small Particles*, §13 (cited by the paper): the uniform approximation of the rainbow caustic, `I ∝ Ai²(−z)`. Re-derived below.
- Lee 1998, *Mie theory, Airy theory, and the natural rainbow*, Appl. Opt. 37(9), `10.1364/ao.37.001506` (search hit, not read): Airy theory is accurate for drops above about 0.2 mm and Mie is needed below.
- Gedzelman 1982, *Rainbow brightness*, Appl. Opt. 21(17), `10.1364/ao.21.003032` (search hit, not read): bow brightness against drop size and the sun's share of the shower.
- Marshall and Palmer 1948 (`docs/research/precipitation.md`): the drop-size law the crate's rain already uses, `N(D) = N₀ e^{−ΛD}`, `Λ = 4.1 R^{−0.21}` mm⁻¹, giving a median volume diameter `D₀ = 3.67 / Λ = 0.9 R^{0.21}` mm.
- Wyman, Sloan, Shirley 2013, *Simple analytic approximations to the CIE XYZ colour matching functions*, JCGT 2(2) (multi-lobe Gaussian fit; recalled, not read).
- The crate's own: `docs/research/clouds.md` (weather map, composite, sky gain), `docs/research/precipitation.md` (rain extinction `2.12×10⁻⁴ R^0.68` m⁻¹), `docs/research/twilight.md` (the sun's colour at low altitude).
- Corpus search: `distill_search` for "rainbow … Airy … drop size distribution" returned no rainbow document (only Belcour's iridescence and Tatarchuk's rain-drop notes), so the library held none before this pass.

## Model adopted

**Geometry.** A ray enters a spherical drop at incidence `i`, refracts to `r` (`sin i = η sin r`), reflects `N` times inside (N = 1 primary, 2 secondary) and leaves. Its deviation is `D_N(i) = 2i − 2(N+1) r + Nπ`, and the scattering angle is `θ = D` for N = 1 and `θ = 2π − D` for N = 2. `D'(i) = 2 − 2(N+1) cos i / (η cos r)` vanishes at Descartes' ray, `cos² i₀ = (η² − 1) / (N(N + 2))`, where `D` has its minimum `D_min = θ_R` (the geometric bow). Lit side is `D > D_min`: `θ > θ_R` for the primary (inside the bow, toward the antisolar point) and `θ < θ_R` for the secondary (outside it). The curvature is `D''(i) = −2(N+1) · [−η sin i cos r + cos² i sin r / cos r] / (η² cos² r)`, and `dr/di = cos i / (η cos r)`.

**Dispersion.** `η(λ) = 1.32505 + 3112 / λ²` (λ in nm), fitted to the paper's two indices (1.3314 at 700 nm, 1.3445 at 400 nm); it gives 1.3353 at 550 nm (the tabulated 1.3338 is within 0.2 %).

**Airy.** Near `i₀` write `u = i − i₀`, `δ = D − D_min`. The phase of the outgoing wave at angle `θ` has `dΦ/du = k a cos i₀ (δ − ½ D'' u²)`, so the amplitude is the Airy integral `∫ exp(i(−t³/3 + z t)) dt = 2π Ai(z)` with
`z = δ · C`, `C = (k a cos i₀)^{2/3} (2 / D'')^{1/3}`, `k = 2π/λ`, `a` the drop radius.
Matching the lit side to the two-root geometric-optics sum (`dσ/dΩ = a² sin i cos i ε / (sin θ |D'|)`, with `Ai²(−z) → 1/(2π√z)` on average) fixes the scale:
`dσ/dΩ = K · Ai²(−z)`, `K = 4π a² · (sin i₀ cos i₀ ε / sin θ) · √C / √(2 D'')`,
with the Fresnel weight of the ray `ε = ½ Σ_{s,p} (1 − R)² R^N` (R the reflectance at `i`; by reciprocity the internal one at `r` is the same). Check against the paper: for a = 0.1 mm and 550 nm, `C = 60.6 rad⁻¹` and the first maximum (`z = 1.0188`) lies 0.96° above the geometric angle, which puts the peak at about 138.8° (the paper's Airy peak is "approximately 138.9°"); for a = 0.5 mm the offset is 0.33° and the supernumerary spacing about 0.6°.

**Drops.** Rain follows Marshall–Palmer, `n(a) ∝ exp(−3.67 a / a₀)` with median radius `a₀ = 0.45 R^{0.21}` mm (R in mm/h). The phase function of a shower is the extinction-weighted mean, `p(θ) = Σ w(a) dσ/dΩ(a, θ) / Σ w(a) 2π a²` with `w = a² n(a) da`, in sr⁻¹; so `L = E_sun · p(θ) · τ` is the radiance of a thin curtain of optical depth `τ` lit by the sun's illuminance `E_sun`. Small drops spread and overlap the colours, so a drizzle (small `a₀`) gives the pale, wide, nearly white bow of the paper's §3.2 and a heavy shower a narrow vivid one; the exponential's many small drops also give the bow its soft pale inner glow.

**Only the liquid share makes a bow.** A rainbow is single scattering by water drops, and its angles are those of a sphere: Sadeghi et al. §3 ("Rainbows are created from the interaction between light and a participating medium composed of water drops suspended in the air"; Lorenz–Mie "can only provide an accurate solution in the case of spherical water drops"), with ice crystals named as a separate scattering problem (§2, Yang and Liou's FDTD). A snowflake is neither liquid nor round, so it scatters no bow (ice makes halos, which are another phenomenon). `Weather::precipitation_mm_h` is the liquid-water equivalent of rain and snow together and `Weather::snow_fraction()` is the share falling as snow, so the bow takes the rate `R_rain = R · (1 − snow_fraction())`, the split every other liquid consumer in the crate already makes (`weathered/model.rs`, `cover/mod.rs`, `hydrology/mod.rs`): the gate (no rain, no bow), the drop row (`a₀ = 0.45 R_rain^0.21`) and the curtain's extinction all use it. Sleet at half snow draws the bow of its rain alone, from fewer and smaller drops.

**Colour.** 21 wavelengths from 400 to 700 nm, each through Wyman's CIE fit and the XYZ-to-linear-sRGB matrix; the result is normalised so an equal-energy spectrum of `p = 1` is `(1, 1, 1)`, and negative lobes (the saturated cyan and violet) are clamped to 0. The sun's own colour is applied at draw time (`bevy_optical_transmittance`), so a bow at sunset is red-gold.

**Where it appears.** The scattering angle of a pixel is `θ = acos(dot(dir, toward_sun))`, so the bow is a circle about the antisolar point and its centre sits at elevation `−h` for a sun at elevation `h`: the whole arc is below the horizon for `h > 42°`, which the geometry produces by itself. The curtain's optical depth along the ray comes from the weather map: the ray's height `y` at horizontal distance `d` is `d tan e`, and a shower at `p = camera.xz + dir.xz d` with precipitation `G` (the map's green, the total rate over 25 mm/h) reaches that height while `y < cloud base`; four taps at 1.5, 3, 6 and 12 km each add `k_ext(25 G s) · Δd` (the crate's rain extinction law, `s = 1 − snow_fraction()` the rain share). The sun must reach the shower: the factor is the cloud shadow map's mean beam `T̄` (the same one that restores the sky's dome, `CloudShadows::mean_transmittance`), so under full overcast the bow is zero and in a break between showers it is full.

## Parameters

| Parameter | Default | Source |
| --- | --- | --- |
| wavelengths | 21, 400–700 nm | Wyman 2013 |
| η(λ) | `1.32505 + 3112/λ²` | Sadeghi 2012 §3.1 |
| drop law | Marshall–Palmer, `a₀ = 0.45 R^0.21` mm | `precipitation.md` |
| LUT | 512 angles over 110–170°, 16 median radii from 0.02 to 1.2 mm (log) | this note |
| curtain taps | 1.5, 3, 6, 12 km | the weather map's 32 km extent |
| `RainbowSettings::strength` | 5.0 | a gain on `E · p · τ`, measured (Deviation 1) |

## GPU mapping

The bow is drawn by the cloud composite pass, which already runs at full resolution after the transparent pass and already writes `(L, T)` for every sky pixel, so it costs one more branch in that fragment and no pass. The phase function is a CPU-built `Rgba16Float` 2D texture (512 angles × 16 median radii, 128 KiB) built once at start-up, sampled bilinearly; rows are keyed by `log(a₀)`. The composite gains three bindings: the weather map, a repeat sampler (the pipeline already owns one) and the LUT. A pixel outside `θ ∈ [110°, 170°]`, or with the sun below the horizon, or with a zero strength, does one dot product and returns before any texture read. `CloudParams::bow` carries the sun's colour at the camera (lux, times the beam) in `xyz` and the median radius's row in `w`; `CloudParams::shower.x` carries the rain's extinction where the map's green is 1, `k_ext(25 s) = 2.12×10⁻⁴ (25 s)^0.68` m⁻¹ (`precipitation::model::rain_extinction`, the one copy of the law), so a tap's extinction is `shower.x · G^0.68`, exactly `k_ext(25 G s)`, at the cost the old constant had.

## What great looks like

- A full, thin, saturated primary arc with red outside and violet inside, a paler, fainter secondary 8–9° outside it with the colours reversed, and a visibly darker sky between them (Alexander's dark band), as in the paper's figure 2.
- A soft, slightly brighter sky inside the primary; nothing at all where there is no shower, and the arc fading with the shower's edge rather than ending in a hard line.
- Drizzle gives a wide pale bow; a thunderstorm's end a narrow vivid one; fog gives none.
- At sunset the bow is red-gold and rises; it can be seen from the ground only for sun elevations below 42°.
- Faint pale supernumeraries inside the primary in a light shower (drops near 0.3 mm); washed out by a wide size distribution in a heavy one.
- The arc never moves relative to the sky when the camera turns (it is anchored to the antisolar point), and costs nothing measurable.

## Validation targets

1. **Geometry (CPU, test).** The primary's geometric angle is 137.7° ± 0.3° for 700 nm and 139.6° ± 0.3° for 400 nm; the secondary's is 129.5° ± 0.5° and 126.1° ± 0.5°; the Airy peak of a 0.1 mm drop at 550 nm lies 0.9–1.0° above the geometric angle.
2. **Airy (CPU, test).** `Ai(0) = 0.355028`, `Ai(1) = 0.135292`, the first maximum of `Ai²(−z)` at `z = 1.0188`.
3. **Curvature (CPU, test).** The closed-form `D''` equals a finite difference of `D'` within 1 %.
4. **Order of colours (CPU, test).** In the LUT for a 0.5 mm shower the primary's peak red is at a smaller angle than its peak blue, and the secondary's is at a larger angle than its blue; the dark band at 133° is below 5 % of the primary's peak.
5. **Drop size (CPU, test).** The LUT's primary peak width grows and its saturation falls from a 0.5 mm shower to a drizzle of 0.05 mm.
6. **Gating (CPU, test).** The gate is 0 with the sun below the horizon, 0 under full overcast, 0 with no rain, and monotone in each.
7. **1x, GPU.** A sun at 20° elevation, a shower of 5 mm/h on a broken sky: a primary arc of luminance at least 1.3× the sky just outside it and a visible hue sequence red to violet; a frame with `strength = 0` is the frame without the feature (byte-identical in the sky pixels).
8. **Close-up, GPU.** The arc crop at 4× shows no banding or stair-stepping on the ring (the LUT is filtered) and no seam at the weather map's edge.
9. **Angles.** Facing away from the sun: the arc. Facing the sun: nothing. Sun at 60°: no arc in the frame. Sun at 8°: a nearly semicircular bow.
10. **Cost.** Sky pixels outside the 110–170° ring and all scene pixels pay one `dot` and one compare.
11. **Snow and sleet (CPU, test; GPU).** At −5 °C with snow possible (`snow_fraction() = 1`) there is no bow: `bow.w = −1`, the shower's extinction is 0, and the primary's ring adds under 2 % of the sky; at 0.5 °C (half the rate as snow) the drop row is that of the liquid half and the ring adds less than rain at the same total rate and more than nothing. Rain at 15 °C is unchanged.

## Deviations from this plan

1. **`strength` 5.0, not 1.** The thin-curtain estimate `L = E p τ` with the Marshall–Palmer table gives a primary bow that adds only 10 % to a blue sky under a 22° sun at 5 mm/h (first capture, strength 0.35 of the sun: 0.056 linear against a sky near 0.5): the bow is there and invisible. Marshall–Palmer over-counts small drops (Gunn–Marshall and every drizzle measurement), which spread the colours and lower the peak, and the real shower that makes a vivid bow is a narrower distribution; a gain of 3 added 24 % at the peak bin and read as delicate (a hostile reading: not a hero), a gain of 5 adds 32 % at the peak bin and reads as a clear double bow with the pale inner glow at 17.5, 18.5 and 19 h (the pass's captures `t-h0..h2`), neither garish at sunset. A gain, not a physical constant: it is a setting, 0 switches the feature off.
2. **The sun's colour** is `exp(−τ₀ · air mass)` with `τ₀ = (0.096, 0.218, 0.451)` (Hillaire's coefficients on the effective scale heights of Bevy's medium), checked against `model::bevy_optical_transmittance` within 15 % for 8°–70°, not `ZENITH_TRANSMITTANCE`, which is a measured sky colour (0.904, 0.640, 0.353) and turned a 22° sun to orange.
3. **The sun's disc** (0.53°) is applied as a 5-tap chord kernel along the angle of the finished table, not per tap in the integral: eight times cheaper and the same blur.
4. **The bow sees the liquid rate, not the total** (issue #51, iteration 103). The first version gated, sized and thickened the curtain with `Weather::precipitation_mm_h`, which counts snow as its water equivalent, so a full double bow stood over falling snow opposite a low winter sun, and sleet took the drop size and optical depth of its total rate. The physics is in Model adopted ("Only the liquid share makes a bow"); the composite's `RAIN_EXTINCTION` constant moved to the CPU as `CloudParams::shower`.

## Pending index

`10.1145_2077341.2077344` converted (12 pages) and now indexed (`distill_search` returns its §2–§3 chunks, 2026-10-30). Read in full from `markdown_read` pages 4 and 6, and pages 1, 3 and 10 through search hits; the rest (validation figures, the drop-shape model) was not needed.
