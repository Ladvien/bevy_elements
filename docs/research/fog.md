# Fog — research note

## Sources

- Wronski, B. (2014). Volumetric fog: unified compute shader based solution to atmospheric scattering. SIGGRAPH 2014, *Advances in Real-Time Rendering in Games, Part II*. `https://advances.realtimerendering.com/s2014/wronski/bwronski_volumetric_fog_siggraph2014.pdf` (read; pages 3–67 text). Used: Beer–Lambert law (p. 9), Henyey–Greenstein phase (p. 12), exponential vertical density attenuation and one animated Perlin octave (p. 37: "We tried using multiple octaves, but … difference was rather subtle for added cost"), 3D textures written by compute and marched in depth slices (pp. 22–24, 40–41), 160×90×64 frustum volume (p. 25), 5-pass cost split (~1.1 ms total, density+lighting 0.43 ms, pp. 47–48).
- Hillaire, S. (2015). Physically based and unified volumetric rendering in Frostbite. SIGGRAPH 2015, *Advances in Real-Time Rendering in Games*. ACM DOI `10.1145/2776880.2787702`; slides read at `https://www.slideshare.net/slideshow/physically-based-and-unified-volumetric-rendering-in-frostbite/51840934` (slides 1–28): single-scattering equation (slide 10), participating-medium definition $\sigma_a, \sigma_s, \sigma_t = \sigma_s + \sigma_a$, albedo $\rho = \sigma_s/\sigma_t$, phase $g$ (slide 14), "decouple material from lighting" (slide 7), V-buffer RGBA16F per froxel (slide 15), energy-conserving froxel integration $\int_0^D e^{-\sigma_t x} S\,dx = (S - S\,e^{-\sigma_t D})/\sigma_t$ (slide 28), temporal jitter with 5 % blend (slide 22).
- Koschmieder, H. (1924). Theorie der horizontalen Sichtweite. *Beitr. Phys. freien Atm.* 12:33–53 (law, used via Bevy's documentation; not opened): $V = \ln(1/\varepsilon)/\sigma$, with $\varepsilon = 0.02$ giving $V = 3.912/\sigma$ ($\ln 50 = 3.9120$); Bevy 0.20 uses the revised 5 % threshold (`bevy_pbr-0.20.0/src/fog.rs:437`, 2.9957/V). WMO's meteorological optical range is defined at 5 % transmittance (Bevy's doc links `https://www.biral.com/wp-content/uploads/2015/02/Introduction_to_visibility-v2-2.pdf`; not opened).
- Bevy 0.20 facts: `docs/research/bevy-0.20-render.md` ("Fog and lighting"): `FogVolume` (`bevy_light-0.20.0/src/volumetric.rs:78-125`), density sampling (`bevy_pbr-0.20.0/src/volumetric_fog/volumetric_fog.wesl:262-276`, `:356-368`), `DistanceFog`/`FogFalloff` (`bevy_pbr-0.20.0/src/fog.rs:56-481`). Additionally read for this note: `fog.rs:417-430` (per-channel `Atmospheric` construction) and `volumetric_fog.wesl:279`, `:434` (extinction uses `density * (absorption + scattering)`).

## Model adopted

**Extinction and visibility.** Beer–Lambert: transmittance along a path, $T = \exp\!\big(-\int_0^s \sigma_t(\mathbf x_t)\,dt\big)$, $\sigma_t$ in m⁻¹ (Wronski p. 9, Hillaire slide 10). Koschmieder with 2 % contrast:

$$ \sigma_V = \frac{\ln 50}{V} = \frac{3.912}{V},\qquad V = \text{Weather.visibility\_m},\ \sigma_V\ [\mathrm{m^{-1}}] $$

so the optical depth over the visibility distance is $\sigma_V V = 3.912$ and $T = 0.02$. Phase function: Henyey–Greenstein with asymmetry $g$ (Wronski p. 12, Hillaire slide 14); Bevy's `FogVolume::scattering_asymmetry` (0.5 by default in code) is kept.

**Ground fog density** (camera-following `FogVolume`, 800 × 120 × 800 m, centred at camera XZ snapped to whole texels and at ground height):

$$ \sigma(\mathbf p) = \sigma_V\ e^{-h/H}\ \big(0.6 + 0.4\,f(\mathbf p - \mathbf w\,t)\big),\qquad H = \begin{cases}25\ \mathrm m & \text{Fog preset}\\ 60\ \mathrm m & \text{otherwise}\end{cases} $$

with $h$ = height above ground [m], $\mathbf w$ the 10 m wind vector [m/s], $t$ [s], and $f \in [0,1]$ a 2-octave value/Perlin fBm of wavelength 80 m (octave count and wavelength are implementation choices; Wronski used one octave). The factor $0.6 + 0.4 f$ lies in [0.6, 1.0]; with $E[f] = 0.5$ the mean factor is 0.8, so the mean optical depth over distance $V$ at $h = 0$ is $0.8 \times 3.912 = 3.13$, and the factor is exactly 1.0 where $f = 1$. Vertical optical depth from the ground to infinity is $\sigma_V H$ (Fog, $V$ = 300 m: 0.3260; $V$ = 20 000 m, $H$ = 60 m: 0.0117); truncated at the volume top (120 m): $\sigma_V H(1 - e^{-120/H})$ = 0.3233 (Fog) or 0.0101 ($V$ = 20 000 m). The exponential height profile is Wronski's "vertical media density attenuation" (p. 37).

**Mapping to Bevy `FogVolume`.** The shader computes `extinction = density_factor * texture.r * (absorption + scattering)` per step (`volumetric_fog.wesl:262-279`). To make the texture value *equal* the extinction coefficient in m⁻¹, set `absorption = 1 - ω`, `scattering = ω`, so `absorption + scattering = 1`, and `density_factor = 1` (the plan's value). $\omega = \sigma_s/\sigma_t$ is the single-scattering albedo (Hillaire slide 14); the default $\omega$ is a design value, 0.95 [INFERENCE, no cited measurement]; fog droplets and dust are nearly non-absorbing at visible wavelengths. `fog_color` is the lit-colour tint: white for fog, lerped toward the dust albedo $(0.76, 0.60, 0.42)$ by `Weather.dust`. The texture is `rgba16float`, `.r` = density, `.gba` = 0, unit-cube UVW = local position + 0.5, `ImageSampler::linear()` with Repeat addressing; Bevy reads only `.r` and returns density 0 outside the unit cube.

**Dust.** $\sigma_V$ already uses `Weather.visibility_m` (DustStorm preset: 800 m), so extinction from dust is already in the density; the "dust term" the plan adds is the albedo/colour change above ([INFERENCE]: adding a second dust density would double-count the preset's 800 m visibility). The height scale stays 60 m. The dust albedo $(0.76, 0.60, 0.42)$ is the plan's colour (tan loess/sand; no external source).

**Distance fog** (camera `DistanceFog`): Bevy's `FogFalloff::Atmospheric { extinction, inscattering }` computes, per channel, transmittance $\exp(-d\,\mathrm{ext}_c)$ and in-scattered light with $\mathrm{insc}_c$ (fog.rs:284-300). `from_visibility_contrast_colors(V, ε, ext_color, insc_color)` builds (fog.rs:417-430)

$$ \mathrm{ext}_c = (1 - c^{ext}_c)^{e}\ \frac{-\ln\varepsilon}{V},\qquad \mathrm{insc}_c = (c^{insc}_c)^{e}\ \frac{-\ln\varepsilon}{V}\quad (e = 2.71828) $$

— the colour channels are raised to Euler's number, they do not scale linearly. With the plan's extinction colour $(0.35, 0.40, 0.45)$ the per-channel factor $(1-c)^e$ is $(0.3101, 0.2494, 0.1969)$, i.e. the transmittance at distance $V$ is $\exp(-0.3101\cdot 2.9957) = 0.395$ (R), 0.474 (G), 0.554 (B) with Bevy's 5 % constant, or 0.297 / 0.377 / 0.463 with ε = 0.02. The distance fog would therefore *not* reach the 2 % (or 5 %) contrast at the stated visibility. Corrected model, recorded in Deviations: extinction per channel $\mathrm{ext}_c = \sigma_V\ (1.243,\ 1.000,\ 0.789)$ — the same hue ratios as the plan's colour, normalised so that green (the photopic channel that defines visibility) equals $\sigma_V$ — so $T_G(V) = 0.02$ exactly, red extinguishes faster and blue slower (distance shifts toward blue, matching the plan's colour intent); inscattering keeps Bevy's mapping, $(c^{insc})^{e}\sigma_V$ with $c^{insc} = (0.75, 0.80, 0.85)\cdot f_{sun}\cdot \mathbf{tint}$ where $f_{sun} = \max(\sin\mathrm{alt}, 0.02)$, $\mathbf{tint} = \mathrm{lerp}\big((1,1,1), (1.0, 0.55, 0.3),\ \mathrm{clamp}(1 - \mathrm{alt}/10°, 0, 1)\big)$ for altitude < 10°, and for dust `lerp(c_insc, (0.76, 0.60, 0.42), dust)`.

**Combined with the ground-fog volume.** Both fogs apply: at ground level along a horizontal ray the effective extinction is $\sigma_V\,(\text{distance}) + \sigma(\mathbf p)$, so with the Fog preset the effective visibility near the ground is roughly half of `Weather.visibility_m` (mean volume factor 0.8 → total ≈ 1.8 × $\sigma_V$ ⇒ effective visibility ≈ 0.56 V). The plan accepts this (distance fog is the far field, the volume is the near field with height structure); the optical-depth check below is therefore defined per component.

**Why a compute-written 3D texture, not froxels.** Bevy 0.20 still has `FogVolume::density_texture` (no replacement needed, `bevy-0.20-render.md`), so the plan's own froxel pass (Wronski 160×90×64) is not built. Wronski's decoupling idea (density written by compute into a volume texture, marched separately) is what the plan's `fog.wgsl` kernel and Bevy's `volumetric_fog` pass together implement, with world-aligned instead of frustum-aligned layout. World-aligned has the disadvantage Wronski notes (marching needs many samples, p. 25) but Bevy's fog already marches with its own `VolumetricFog::step_count` (quality 32–96).

## Parameters

| name | symbol | default | unit | source |
|---|---|---|---|---|
| Contrast threshold | $\varepsilon$ | 0.02 | 1 | Koschmieder (3.912 = ln 50); plan 1.4 states σ = 3.912/visibility. Bevy's own default is 0.05 (fog.rs:437) |
| Extinction from visibility | $\sigma_V$ | $3.912/V$ | m⁻¹ | Koschmieder; plan 1.4. Examples: V = 300 m → 0.013040; 800 m → 0.004890; 20 000 m → 1.956e-4; 40 000 m → 9.78e-5 |
| Scale height, Fog preset | $H_{fog}$ | 25 | m | plan 1.4; exponential profile: Wronski 2014 p. 37 |
| Scale height, other | $H$ | 60 | m | plan 1.4 |
| Density modulation | — | $0.6 + 0.4 f$, $f\in[0,1]$ | 1 | plan 1.4 |
| fBm octaves / wavelength | — | 2 / 80 m | — / m | implementation choice (Wronski p. 37 used one octave; plan says "fbm") |
| fBm advection | — | $\mathbf p - \mathbf w\,t$ | m | plan 1.4 |
| Volume extent | — | 800 × 120 × 800 | m | plan 1.4 |
| Density texture | — | 128 × 32 × 128, `rgba16float` (`.r` used) | texels | plan 1.4; texel 6.25 × 3.75 × 6.25 m (computed) |
| Density factor | `density_factor` | 1 | 1 | plan 1.4; extinction = `density_factor * tex.r * (absorption + scattering)` (volumetric_fog.wesl:279) |
| Absorption / scattering | `absorption`, `scattering` | $1-\omega$, $\omega$ (sum 1) | 1 | Hillaire 2015 slide 14 (definitions); $\omega$ = 0.95 is a design choice |
| Single-scattering albedo | $\omega$ | 0.95 (fog), 0.9 at dust = 1 (lerp) | 1 | design value; definition from Hillaire 2015 slide 14 |
| Scattering asymmetry | $g$ | 0.5 | 1 | Bevy 0.20 default (`volumetric.rs:~118`); HG: Wronski 2014 p. 12 |
| Dust albedo / fog colour | — | (0.76, 0.60, 0.42) | linear RGB | plan 1.4 |
| Distance-fog extinction hue (plan) | — | (0.35, 0.40, 0.45) | colour | plan 1.4; used only as ratios, see Deviations |
| Distance-fog extinction coefficients (adopted) | $\mathrm{ext}$ | $\sigma_V(1.243, 1.000, 0.789)$ | m⁻¹ | derived from plan hue via Bevy fog.rs:417-430 |
| Distance-fog inscattering colour | $c^{insc}$ | (0.75, 0.80, 0.85) × $f_{sun}$ × tint | colour | plan 1.4 |
| Sun factor | $f_{sun}$ | $\max(\sin\mathrm{alt}, 0.02)$ | 1 | plan 1.4 |
| Dusk tint colour / onset | — | (1.0, 0.55, 0.3) / altitude < 10° | colour / deg | plan 1.4 |
| `VolumetricFog.step_count` | — | 32 / 48 / 64 / 96 (Low … Ultra) | steps | plan Shared architecture (ElementsQuality); Bevy default 64 |
| Density workgroup | — | 4 × 4 × 4 | threads | plan Shared architecture |
| Temporal blend (not used) | — | 5 % | 1 | Hillaire 2015 slide 22 (reference only; Bevy's `VolumetricFog::jitter` is left at 0) |

## GPU mapping

One compute kernel `fog.wgsl` writes the 3D density texture each frame: `@workgroup_size(4, 4, 4)` (64 invocations; limit 256), dispatch 32 × 8 × 32 = 8192 workgroups for 128 × 32 × 128 = 524 288 texels. Bindings: 1 write-only storage texture (`rgba16float`, `texture_storage_3d<rgba16float, write>`), 1 uniform buffer (`FogParams`: world origin snapped to texel multiples, σ_V, H, wind×t offset, dust, extent, precipitation extinction term in M2), 0 storage buffers. This is far below the 4 storage textures per stage and 8 storage buffers per stage limits; the texture is `Rgba16Float` write-only, a legal WebGPU storage format (`bevy-0.20-render.md` limits and format tables). The texture is created with `TextureDimension::D3`, `STORAGE_BINDING | TEXTURE_BINDING`, linear sampler, `RenderAssetUsages::RENDER_WORLD`. Bevy's fog pass reads the same texture through the `GpuImage` view as a filterable 3D float texture (binding 2 of group 1 in the fog shader). Memory: 128·32·128·8 B = 4 MiB. The fog volume's `Transform` is `translation = (cx, ground_y + 60, cz)`, `scale = (800, 120, 800)` so UVW $= (x - c_x)/800 + 0.5$, $y$-up. Camera-following with centre snapped to the 6.25 m horizontal texel (and the density origin the same snapped value) keeps the fBm world-anchored so the field does not swim.

Half-float range check: σ_V at 40 000 m visibility is 9.78e-5, at height 120 m × exp(−2) = 1.3e-5 (a subnormal below 6.1e-5, absolute spacing 6e-8, relative error ≤ 0.5 %); at 300 m it is 0.0130 (relative error ≤ 4.9e-4). Both are acceptable for fog. The plan's cost budget (wind + fog compute ≤ 0.3 ms at High) is reference-checked against Wronski 2014 p. 47 (density+lighting 0.43 ms on 2013 console GPU at 160×90×64 = 921 600 texels, 1.76× this volume, with lighting included; this kernel computes density only).

## Validation targets

1. **Optical depth over visibility distance (CPU model of the kernel, then GPU readback):** with the fBm term forced to 1 (factor $0.6 + 0.4\cdot 1 = 1.0$) and $h = 0$, $\int_0^{V}\sigma\,dx = 3.912$ for $V$ = 300 m (±0.5 %: 3.912 ± 0.020), $V$ = 120 m and $V$ = 400 m (inside the volume's ±400 m); transmittance $e^{-3.912} = 0.0200$ (±0.0002). Integrate with trapezoid over the texture's trilinearly sampled values. Source: Koschmieder; arithmetic here. With $f = 0$ (minimum) the integral is $0.6 \times 3.912 = 2.347$; mean over unseeded fBm ≈ 0.8 × 3.912 = 3.13 (±15 % over random noise offsets).
2. **Vertical optical depth:** for Fog preset ($V$ = 300 m, $H$ = 25 m, factor 1) $\int_0^{120\,\mathrm m}\sigma\,dh = 0.3233$ (±1 %); untruncated σ_V H = 0.3260. For $V$ = 20 000 m, $H$ = 60 m: 0.01015 (±2 %, half-float quantisation). Source: formula above.
3. **Half-float encoding:** the kernel's texel value read back equals the analytic σ within 5e-4 relative for $\sigma \ge 6.1\times10^{-5}$. Source: `rgba16float` precision (10-bit mantissa).
4. **Distance fog (CPU unit test of the `DistanceFog` builder):** with $V$ = 300 m the green-channel transmittance at $d = V$ is $\exp(-\mathrm{ext}_G V) = 0.0200 \pm 0.0002$; red 0.0077 ($= 0.02^{1.243}$), blue 0.0457 ($= 0.02^{0.789}$); `extinction.y == 3.912/300 = 0.013040` (±1e-6). The plan's unmodified `from_visibility_colors` would give $T_G(V) = 0.474$ with Bevy's 5 % constant (0.377 with ε = 0.02) — recorded as the reason for the deviation. Source: Bevy fog.rs:417-430, Koschmieder.
5. **Colour rule:** inscattering at altitude 90° equals $((0.75, 0.80, 0.85)\cdot1.0)^{e}\,\sigma_V$ = $(0.4575, 0.5452, 0.6429)\,\sigma_V$; at altitude 0° the factor is 0.02 and the tint is (1.0, 0.55, 0.3); at altitude 5° the tint is `lerp(white, (1, 0.55, 0.3), 0.5)` = (1.0, 0.775, 0.65). Dust = 1 yields exactly (0.76, 0.60, 0.42) before the exponent. Source: plan 1.4.
6. **Bevy binding check (headless asset test):** the density image is `D3`, 128 × 32 × 128, `Rgba16Float`, has `STORAGE_BINDING | TEXTURE_BINDING`, and `FogVolume.density_factor == 1.0` and `absorption + scattering == 1.0` within 1e-6. Source: `bevy-0.20-render.md`, volumetric_fog.wesl:279.
7. **Visual (plan M1):** `weather_tour` screenshots of Fog and Clear show visibly different fog density and sun dimming; Fog preset distant objects at 300 m reach ≈ 2 % contrast at the visibility distance from the distance-fog term alone, and the ground fog thickens toward the ground (25 m scale height) with density structure moving with the wind; at dawn the Fog preset under a low sun shows warm in-scattering. Source: plan Verification M1, Visual.
8. **Cost (plan M1 bench):** wind + fog compute ≤ 0.3 ms total at High, 1920 × 1080; the fog kernel's share is recorded in the README Performance table. Source: plan Verification M1.

## Deviations from this plan

1. **`DistanceFog` is built with an explicit `FogFalloff::Atmospheric`, not `FogFalloff::from_visibility_colors`.** The plan calls `from_visibility_colors(visibility, extinction, inscattering)` with extinction colour (0.35, 0.40, 0.45). Bevy raises each channel to $e$ after subtracting from 1 and uses a 5 % contrast constant, so the resulting transmittance at the visibility distance is 0.395 / 0.474 / 0.554 (R/G/B), not 0.05 or 0.02 (`bevy_pbr-0.20.0/src/fog.rs:417-430`, `:437`; arithmetic in this note). The adopted extinction is $\sigma_V\,(1.243, 1.000, 0.789)$ with $\sigma_V = 3.912/V$, which keeps the plan's hue ordering (blue extinguished least) and the plan's 3.912 constant, and makes the volume and the distance fog use one visibility definition. Inscattering colours and the sun factor are unchanged.
2. **The sun/sky factor multiplies the inscattering colour only, not the extinction colour.** The plan multiplies both. Extinction (visibility) is a property of the medium, independent of illumination (Hillaire 2015 slide 7, "decouple material from lighting"; Koschmieder's law has no light-source term), so scaling it by $\max(\sin\mathrm{alt}, 0.02)$ would make the visibility at dusk 50 × shorter than `Weather.visibility_m`.
3. **Fog brightness comes from the scene's luminance times the camera's exposure, not from $\max(\sin\mathrm{alt}, 0.02)$; in-scattering coefficients equal the extinction coefficients.** Bevy multiplies lit surfaces by `view.exposure` (`bevy_pbr-0.20.0/src/render/pbr_functions.wesl:850`) but adds the `DistanceFog` colour after exposure, unscaled (`render/fog.wesl:64-77`), so the fog colour is a post-exposure value. The plan's sun factor assumes a fixed exposure: under this crate's auto exposure, which lifts the night by ~28 EV, it would leave night fog almost black against a moonlit scene. Adopted: `DistanceFog.color = hue × (E/π) × exposure`, with $E$ the same scene illuminance the auto exposure meters (sun + 15 % sky + moon + floor, cloud-dimmed) and `exposure = 2^{-EV_{100}}/1.2` from the camera's `Exposure`; `hue` keeps the plan's (0.75, 0.80, 0.85), dusk tint and dust tint. With in-scattering = extinction per channel, a fully extinguished pixel takes exactly the fog colour (single-scattering radiance $L(1 - T)$ for homogeneous fog), so validation 5 checks `fog_hue` (tint rules) rather than the $(c)^{e}\sigma_V$ in-scattering coefficients.

## Pending index

`distill_index` skipped (distill/Qdrant down) for:

- `wronski-2014-volumetric-fog` (SIGGRAPH 2014 course; URL above, no DOI)
- `10.1145_2776880.2787702` (Hillaire 2015; slides only, not downloaded/converted)
- `koschmieder-1924` (not obtainable by DOI; used via Bevy docs)
