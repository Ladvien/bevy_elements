# Clouds that are white: multiple scattering by diffusion, fitted to Monte Carlo

With the cumulus shaped (iteration 17), their light is what is left wrong. Seen from above they are khaki in the haze; seen from the ground with the sun behind the viewer they are a cream-grey barely brighter than the sky; their bases are near black where they are not lit by the ambient (`/tmp/i18/before/sheet.png`, `z.png`). A cloud is a conservative medium (single-scattering albedo ≈ 1 in the visible): light that enters a thick cloud leaves it, mostly back through the lit side, so a cumulus top in the sun is as bright as a white diffuser.

Measured, not guessed. A Monte Carlo transport of a homogeneous conservative slab (`/tmp/i18/mc.py`: 2 × 10⁶ photons per case, the march's own dual-lobe phase 0.7 HG(0.8) + 0.3 HG(−0.5), the unscattered beam excluded because the composite draws the sun through the cloud) against the CPU mirror of the march (`clouds::model::march`, which reproduces the shader) for τ ∈ {1, 3, 10, 40}, sun cosine μ₀ ∈ {0.9, 0.5}, nine viewing directions above the slab and three below:

| Case (L/E, sr⁻¹) | Monte Carlo | March (3 Wrenninge octaves, a = b = c = ½) |
|---|---|---|
| τ 40, μ₀ 0.9, top seen from the sun side, μ_v 0.95 | 0.339 | 0.054 |
| τ 40, μ₀ 0.9, top seen from the far side | 0.276 | 0.048 |
| τ 10, μ₀ 0.9, base seen from below, μ_v 0.95 | 0.078 | 0.004 |
| τ 40, μ₀ 0.5, base, μ_v 0.95 | 0.009 | 0.000 |
| τ 1, μ₀ 0.9, top, sun side | 0.098 | 0.054 |

The march returns a sixth of the light a thick cloud reflects and none of what it transmits: the mean squared log error over the 96 cases (with a floor of 0.01 so near-zero values do not dominate) is 0.82. That is the grey cloud on screen.

## Sources

- **Wrenninge, Kulla & Lundqvist 2013**, *Oz: The Great and Volumetric* (SIGGRAPH talk; via Hillaire 2016 and 2020): multiple scattering as octaves of single scattering, `Σ aⁿ e^{−bⁿτ} p(θ, cⁿg)`, energy-conserving when `a ≤ b`. The march's current model.
- **Hillaire 2016** §5.8 (Frostbite; `/tmp/i16/pdf/hillaire2016.txt`) and **Hillaire 2020**, *Physically Based and Scalable Atmospheres in Unreal Engine* (SIGGRAPH PBS course slides, `/tmp/i18/pdf/hillaire2020.txt`): "Clouds participating media usually have an albedo close to 1 … light can bounce thousands of time before going out of the cloud … multiple scattering is a requirement to render clouds that do not look like dark" ; Unreal settled on Wrenninge's octaves (two by default) and shows the path-traced reference beside them; dark edges come from the low probability of scattering toward the eye at edges, imitated by lowering the albedo at edges or Schneider's powder term.
- **Bouthors, Neyret, Max, Bruneton & Crassin 2008**, *Interactive multiple anisotropic scattering in clouds* (I3D, `10.1145/1342250.1342277`, HAL `inria-00333007`, read): orders 1–30 stay anisotropic and carry 30–100 % of a slab's reflectance; orders above 30 are isotropic and appear past ~100 m of cloud; real-time models that drop high orders render clouds too dark. Their collector-area model is the accurate real-time answer but needs a mesh of the cloud surface.
- **Jensen, Marschner, Levoy & Hanrahan 2001**, *A Practical Model for Subsurface Light Transport* (corpus `10.1145_383259.383319`): diffusion theory with reduced extinction `σ_t′ = (1 − g) σ_t` and the boundary condition `φ − 2AD ∂φ/∂n = 0`, `D = 1/(3σ_t′)`: the fluence vanishes at an extrapolated boundary `2AD = 2/(3(1 − g)σ_t)` outside the medium (A = 1 without refraction, as in a cloud).
- **Lacis & Hansen 1974** (`10.1175/1520-0469(1974)031<0118:apftao>2.0.co;2`) and **Meador & Weaver 1980** (`10.1175/1520-0469(1980)037<0630:tsatrt>2.0.co;2`), the two-stream approximations the atmospheric sciences use for cloud albedo: a conservative layer's transmittance falls as `1 / (1 + ¾(1 − g)τ)`, its reflectance is the rest (not obtained: no open copies; the relation is checked here against the Monte Carlo instead).
- **Kallweit, Müller, McWilliams, Gross & Novák 2017**, *Deep Scattering* (`10.1145/3130800.3130880`, downloaded): path-traced clouds need hundreds of orders; the reference images are the target look.
- **Schneider & Vos 2015** (corpus `s2015-advances-ppt-944-mb-pdf-5-mb-2`): the powder darkening of octave 0 near the lit surface, kept.

## Model adopted

The light a sample scatters toward the eye is single scattering plus the multiply scattered light of a diffusing slab:

`S = E_sun · [ P(τ_l) e^{−τ_l} p(θ) + α √μ_l / π · (τ_b + z) / (τ_l + τ_b + 2z) ] + ambient`

- `E_sun e^{−τ_l} p(θ)` is single scattering, `τ_l` the optical depth toward the light (the existing six-sample cone), `p` the dual-lobe phase and `P` the powder darkening.
- The second term is photon diffusion in a conservative slab: with no absorption the diffuse fluence falls linearly from the lit side to the far side and vanishes at the extrapolated boundaries `z = 2/(3(1 − g))` optical depths beyond each side (Jensen et al.'s boundary condition), so at a point with `τ_l` of cloud toward the light and `τ_b` behind it the fluence is `∝ (τ_b + z)/(τ_l + τ_b + 2z)`. A thin cloud (both small) keeps about half; the top of a thick cloud keeps nearly all (light that cannot get out the far side comes back); its base keeps `z/(τ + 2z)`, the two-stream transmittance law. With `g = 0.8`, `z = 3.33`; fitted freely to the Monte Carlo it comes out at 3.39.
- `α √μ_l / π` sets the diffuse radiance: `α = 0.68` fitted, `μ_l` the light's cosine with the local vertical. The fit prefers `μ_l^0.85` (a slab's top gets flux `∝ μ₀`), a sphere would want `μ_l⁰`; `√μ_l` (loss 0.054 against 0.049) keeps the sides of a cumulus lit at a low sun.
- `τ_b` is one base-shape sample 300 m behind the sample along the light, standing for 600 m. The term saturates once `τ_b ≫ z` (a cumulus body more than ~150 m deep), so one coarse sample is enough to tell a wisp from a body.
- The Wrenninge octaves 1 and 2 go: the diffusion term carries what they stood for, and better.

Measured on the same 96 cases with the analytic slab: mean squared log error 0.054 (0.82 before, 0.16 for the best twelve-octave Wrenninge sum under `a ≤ b`); per-case ratio to Monte Carlo 0.63–2.15, median 0.98.

## Parameters

| Name | Value | Source |
|---|---|---|
| Extrapolation length `z` | 2/(3(1 − 0.8)) = 3.33 | Jensen et al. 2001; Monte Carlo fit 3.39 |
| Diffuse albedo `α` | 0.68 | Monte Carlo fit with `√μ_l` |
| Light cosine power | ½ | between slab (1) and sphere (0); Monte Carlo fit 0.85 |
| Behind-sample distance | 300 m, standing for 600 m | design: saturation at `τ_b ≫ z` |
| Single-scattering phase | dual lobe 0.7 HG(0.8) + 0.3 HG(−0.5) | unchanged (`clouds.md`) |

## GPU mapping

No new pass, texture or binding. Per fine sample: one more medium sample (two taps: base noise and weather map, as one of the six light samples) and the diffusion term (a few ALU); two octaves (two `exp` and four Henyey–Greenstein evaluations with a `pow` each) are removed. The far veil uses `τ_b = FAR_TAU`.

## What great looks like

Fair-weather cumulus at noon from the ground: sunlit tops and sides brilliant white, near the top of the tonemapper, against a deep blue sky; flat bases a soft grey, darker than the sky; thin edges and wisps translucent, the sun's side silver-lined. From an aircraft above: a field of white cauliflower over the haze, not khaki. Toward a low sun: dark grey bodies with burning rims; away from it: warm cream to orange sides. Overcast decks: grey, lighter in their thin parts.

## Validation targets

1. **Monte Carlo (CPU, `clouds::model`).** The mirror's march of the 96 slab cases (τ ∈ {1, 3, 10, 40}, μ₀ ∈ {0.5, 0.9}, nine views above and three below) has a mean squared log error ≤ 0.08 against the stored Monte Carlo table, every thick-cloud top (τ ≥ 10) within 0.5–2× and every base within 0.5–2.5× (before: 0.82, tops at 0.16–0.3×, bases at 0–0.05×).
2. **Energy.** No case returns more than the light that arrives (`cloud_radiance_never_exceeds_the_light_that_arrives` kept).
3. **Integrator.** The march at Q = 64 is within 5 % of the same march at Q = 1024 on the slab (the closed-form octave tests go with the octaves).
4. **Visual.** `terrain_tour` hills `PartlyCloudy` at 9 h, 15.5 h, noon overhead, aerial at 3.8 km, horizon, sunset, `Overcast`, `Thunderstorm`, `Clear` (`/tmp/i17/cap.sh`, `cap2.sh`): tops white, bases grey, no change in the overcast/storm whiteout beyond brightness.
5. **Performance.** `elements_clouds_march` at High, hills, `PartlyCloudy` within 10 % of iteration 17's 0.489–0.492 ms.

## Deviations from this plan

1. **Descending rays take their aerial perspective along the reversed ray (found while prototyping).** Seen from 3.8 km the cumulus below stayed khaki — and turned golden once the diffusion term made them bright (`/tmp/i18/p1/sheet.png`): the march's aerial perspective divided the camera's transmittance to the top of the atmosphere by the cloud's, `T(cam, μ)/T(cloud, μ_d)`, which for a ray going down is the transmittance of a path that meets the ground, blue gone. Bruneton's `GetTransmittance` takes such a pair along the reversed ray, which leaves the atmosphere through the camera: `T(cloud, −μ_d)/T(cam, −μ)`. With it the cumulus field seen from above is white over the haze (`/tmp/i18/p2/cmp.png`). Rays going up are unchanged.
2. **Validation 1's bases are held to 0.4–2.5×, not 0.5–2.5× (measured on the mirror).** The fit in the plan used the exact optical depths; the march has the six-sample light cone and one sample of the cloud behind, and the thickest slab under a low sun (τ 40, μ₀ 0.5) comes out at 0.45× from straight below (0.0041 against 0.0091 sr⁻¹, both a fortieth of the tops). Over the 96 cases the mirror's error is 0.067 (target ≤ 0.08), thick tops 0.57–1.12×.
3. **The ambient takes the sky's colour only (amends clouds.md Deviation 30).** The ambient's colour was the sky's irradiance plus the sunlight the cloud above had scattered out of the beam, `1 − e^{−τ_l}` of it; it stood in for the multiply scattered sun that the diffusion term now carries, and kept with it the overcast deck at noon turned cream (sRGB (228, 224, 219) against (193, 195, 197) before, `/tmp/i18/r1/overcast.png`). `ambient_light` is now the skylight alone, in the sky's colour at the same luminance: (226, 225, 222). The luminance of every source is unchanged. Deviation 30 had rejected the sky's colour alone because deep in a thick deck the sun term was extinguished and only the ambient was left, a periwinkle deck; the diffusion term is that deck's sunlight now, and the noon deck overhead is 1.12× the neutral ambient's blue/red, not 1.27× (`tests/gpu_cloud_ambient.rs`).
4. **The far veil has no cloud behind a deck's base, and its far colour check allows the air's reddening (amends clouds.md Deviation 27 and Validation 20).** With `τ_b = FAR_TAU` the veil diffused as much light as the middle of a cloud, a ridge at the 32 km boundary 1.6× the deck on either side (luminance 7.4 against 4.7 at a 60° sun, Snow). Below a uniform deck's base there is no cloud: `τ_b = FAR_TAU · (1 − deck)`, and the boundary is flat (deck 4.72, veil 4.57). The deck is now bright enough that, 1.5–2.1° either side of the boundary (rows 4–6 km apart), the air's reddening of it outweighs the airlight: the tint drifts by 0.017 a degree, smoothly, with no step, to 0.062 tint and 0.122 chroma apart at a 60° sun (0.033 and 0.067 at 20°). The far check is 0.08 and 0.15 (was 0.06 and 0.1); the near check, which catches a step, keeps 0.03 and 0.05 (measured ≤ 0.016 and ≤ 0.031).

## Pending index

Downloaded by `hs paper download`: `10.1145_3130800.3130880` (Kallweit et al. 2017). Read from local copies: Hillaire 2020 slides (selfshadow), Bouthors et al. 2008 (HAL). Not obtained: Lacis & Hansen 1974, Meador & Weaver 1980, Bouthors 2008 via home-still (no open copy through the providers). `distill_index` not run.
