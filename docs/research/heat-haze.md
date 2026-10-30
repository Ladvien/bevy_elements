# Heat haze

A flame drawn as a billboard reads as a picture of a fire until the air above it shimmers: the background behind a campfire, a torch, a burning car or an exhaust wobbles, and that wobble is what tells the eye the thing is hot. bevy_lights draws flames and owns no post-process; the ownership map gives atmospheric optics to this crate, and issue #50 asks for an optional effect that takes a list of world-space emitters as plain data and warps the scene behind them. This note models the hot plume above an emitter from fire-engineering correlations, turns its refractive-index field into a ray deflection the way background-oriented schlieren measures it, and draws it as a screen-space refraction pass that touches only the pixels the plumes cover.

## Sources

- **Raffel 2015**, *Background-oriented schlieren (BOS) techniques*, Exp. Fluids 56:60, `10.1007/s00348-015-1927-5` (home-still stem `10.1007_s00348-015-1927-5`, downloaded this pass; scribe was busy, so read through `pdftotext`). §2.1: a ray crossing a refractive-index field is deflected by `ε_y = (1/n₀) ∫ ∂n/∂y dz` (Eq. 2); the image of a background a distance `Z_D` behind the disturbance moves by `Δy = f · Z_D/(Z_D + Z_A − f) · ε_y` (Eq. 3), so the shift on the sensor is the deflection times the share of the distance that lies *behind* the disturbance; Eq. 10: the Gladstone–Dale law `n = Gρ + 1` for a gas. This is the whole screen-space model: a deflection angle from the gradient of the integrated index, and a parallax factor from the depth buffer.
- **Shabbir and George 1994**, *Experiments on a round turbulent buoyant plume*, J. Fluid Mech. 275 (NASA TM 105955), `10.1017/s0022112094002260` (stem `10.1017_s0022112094002260`, downloaded this pass, read through `pdftotext`). Self-similar point-source plume, Table II: mean buoyancy `g β ΔT = 9.4 F₀^{2/3} z^{−5/3} exp(−68 η²)` and velocity `W = 3.4 F₀^{1/3} z^{−1/3} exp(−58 η²)`, `η = r/z` (Eqs. 20, 21); temperature half-width `0.101 z`, velocity half-width `0.1075 z`; centreline temperature fluctuation intensity `θ′/ΔT = 0.40`, velocity `0.33`. Used for the plume's width and the strength of its turbulence.
- **McCaffrey 1979**, *Purely buoyant diffusion flames: some experimental results*, NBSIR 79-1910, `10.6028/NBS.IR.79-1910` (NIST, open: `https://nvlpubs.nist.gov/nistpubs/Legacy/IR/nbsir79-1910.pdf`, read this pass). The centreline of a fire in three regimes of `z* = z / Q^{2/5}` (m/kW^{2/5}), with the summary table on p. 13: continuous flame (`z* < 0.08`, `V = 6.8 z*^{1/2} Q^{1/5}`, `η = 1/2`), intermittent (`0.08–0.2`, `V = 1.9 Q^{1/5}`, `η = 0`) and plume (`> 0.2`, `V = 1.1 z*^{−1/3} Q^{1/5}`, `η = −1/3`), each with the buoyancy relation `V / √(2g z ΔT/T₀) = C = 0.9`, so `ΔT/T₀ = (κ/(0.9√(2g)))² z*^{2η−1}`: 853 K in the flame, `66.6/z*` K in the intermittent zone, `22.3 z*^{−5/3}` K in the plume (for `T₀` = 293 K), continuous at both joins. "Above this zone the flame is intermittent in … a regular frequency of a few Hz" (p. 12). The temperature profile is narrower than the velocity profile by a factor of 1.2 (p. 14).
- **Delichatsios 2007**, *A correlation for the flame height in "group" fires*, Fire Sci. Technol. 26(1), `10.3210/fst.26.1` (stem `10.3210_fst.26.1`, in the corpus). Eq. 4 quotes Heskestad's single-fire flame height `L/D = 3.7 Q*^{2/5} − 1.02`, `Q* = Q / (ρ∞ c_p T∞ √(gD) D²)`.
- **Xia and Zhang 2018**, *A vortex-dynamical scaling theory for flickering buoyant diffusion flames*, J. Fluid Mech. 855, `10.1017/jfm.2018.707` (stem `10.1017_jfm.2018.707`, in the corpus). The puffing frequency of a pool fire, `f = 0.48 √(g/D)`, "equivalent to the scaling relation `f = 1.5 √(1/D)`, given by Cetegen and Ahmed": the frequency at which toroidal vortices shed from the fire and carry the hot gas up in pulses.
- **Edlén 1966**, *The refractive index of air*, Metrologia 2(2):71, `10.1088/0026-1394/2/2/002` (paywalled, not downloaded). The standard-air dispersion formula `(n − 1)·10⁸ = 8342.13 + 2406030/(130 − σ²) + 15997/(38.9 − σ²)`, `σ = 1/λ` µm⁻¹, gives `n − 1 = 2.778 × 10⁻⁴` at 550 nm, 15 °C, 101 325 Pa, so `G = (n − 1)/ρ = 2.27 × 10⁻⁴` m³/kg. Dispersion across the visible is 1.5 % of `n − 1`, so the haze is drawn without colour fringes.
- **Akenine-Möller et al. 2018**, *Real-Time Rendering* 4th ed. (corpus `book_real_time_rendering_4th_akenine`): Fig. 9.10 and §9.4 (a gradual change in index of refraction bends a ray continuously: "mirages and heat distortion"); §12.1 (heat shimmer as an image-processing pass that uses the framebuffer as a texture, after Nguyen's Vulcan demo); §14.5.2, the refraction leak: a perturbed sample may land on a surface *in front of* the refracting volume and pull its colour in, so "the scene depth map could be used to compare … against the perturbed scene sample's depth. If the center pixel is farther away, the offset sample is closer; it is then ignored and replaced by the regular scene sample as if there was no refraction" (after Courrèges' study of DOOM 2016's refraction).
- **Nguyen 2004**, *Fire in the "Vulcan" demo*, GPU Gems ch. 6 (corpus, GPU Gems 1): screen-space 2D distortion that shifts texture coordinates by a GPU-generated perturbation was the shipped way to make flames move; RTR4 cites it for heat shimmer.
- **Evans 2011**, LittleBigPlanet 2's fire and thrusters, SIGGRAPH *Advances* (corpus `s2011-advances-pdf-slides`, pp. 72–77): fire particles splat their "hotness" into four depth layers and "post FX uses the 'hotness' for heat haze effect strength". The haze strength comes from the fire's heat, and the depth layering keeps a foreground object from being warped by a fire behind it; here both come analytically from the plume model and the depth prepass.
- **Luebke et al. 2003**, *Level of Detail for 3D Graphics* (corpus `book_level_of_detail_3d_graphics_luebke`), pp. 278 and 284: the eye resolves about one minute of arc, but discriminates the misalignment of two edges to "2 to 5 seconds of arc (vernier acuity)". Used to size the gain (Deviation 1).
- **Kim, Thürey, James and Gross 2008**, *Wavelet turbulence* (corpus `eth-cgl-sim_anim-Kim08`): the Kolmogorov inertial range, eddy velocity `∝ ℓ^{1/3}`, which a passive scalar (temperature) shares; used for the noise octaves' amplitudes.

## Model adopted

**1. The emitter is a fire of a heat release rate and a base radius.** `HeatSource { power_kw, radius_m }` on an entity with a `Transform`; the plume rises from the entity's position. The issue asked for "position, radius, strength"; the strength is the physical one, the heat release rate `Q`, because every quantity below scales from it (a candle 0.08 kW, a torch 1–5 kW, a campfire 20–100 kW, a bonfire 0.5 MW, a burning car 2–5 MW). The same component also answers a host's own questions: `flame_height_m` (Heskestad through Delichatsios Eq. 4) and `puffing_hz` (`1.5/√D`), so a flame drawn by another crate can be sized and flickered to agree with its haze.

**2. The plume's temperature follows McCaffrey's three regimes**, scaled to the weather's air temperature `T∞` (`ΔT/T∞` is what the correlation gives): `ΔT(z) = T∞ (κ/(0.9√(2g)))² z*^{2η−1}` with `z* = z/Q^{2/5}`, and the centreline rise speed `V(z) = κ z*^{η} Q^{1/5}`. The temperature profile across the plume is Gaussian with a 1/e radius `σ(z) = √(R² + (0.121 z)²)`: the fire's own radius at the base, and Shabbir and George's `exp(−68 η²)` (1/e radius `z/√68 = 0.121 z`) far above it.

**3. The index of refraction follows the density.** At constant pressure `ρ = ρ∞ T∞/T`, so `n − 1 = (n∞ − 1) T∞/T` and the plume's deficit on its axis is `δn₀(z) = (n∞ − 1) ΔT/(T∞ + ΔT)` with `n∞ − 1 = 2.778 × 10⁻⁴ · 288.15/T∞`. A 50 kW campfire: `δn₀` ≈ 2.1 × 10⁻⁴ in the flame, 1.2 × 10⁻⁴ at 1.2 m, 5.6 × 10⁻⁵ at 2.3 m.

**4. The deflection is the gradient of the index integrated along the ray** (Raffel Eq. 2). A ray passing the axis at impact parameter `p` sees, for the Gaussian profile, the projected deficit `Φ(p) = √π σ δn₀ e^{−p²/σ²}`, and `ε = ∂Φ/∂p = −2√π δn₀ (p/σ) e^{−p²/σ²}`, at most `1.52 δn₀` at `p = σ/√2` whatever the plume's size. A ray that crosses the axis at an angle `α` from it travels `1/sin α` further through the plume: `Φ` takes that factor, clamped at `α ≥ 20°` (a view straight down a plume crosses its whole height, which the single-crossing model does not describe). The *mean* field is a weak diverging lens; what the eye reads as haze is the **turbulent** part.

**5. The turbulence.** Temperature fluctuates about the mean with an rms of `0.40 ΔT` (Shabbir and George), in eddies whose size grows with the plume, `ℓ = σ/2`. A ray crossing a path of `2σ` through `2σ/ℓ` independent eddies each contributing `δn′ℓ` accumulates an rms projected deficit `Φ′ = δn′ √(2σℓ)`, and its gradient (the deflection) has an rms of order `δn′ √(2σ/ℓ) = 2 δn′ = 0.8 δn₀`: the shimmer is as strong as the lens. The projected fluctuation is drawn as gradient noise in the plume's frame — across the plume `p/ℓ`, up the plume, and in time — with two octaves; in the inertial range a scalar's eddies scale as `ℓ^{1/3}`, so the deflection from eddies of size `ℓ` scales as `δn_ℓ √(L/ℓ) ∝ ℓ^{−1/6}`: the second octave (`ℓ/2.5`) is drawn at `2.5^{1/6} = 1.16` times the first. Smaller eddies than a pixel are not drawn: each octave fades out as its eddy shrinks from three pixels to one, which is the issue's "fading with distance".

**6. The hot gas rises and pulses.** The noise moves up the plume at the local rise speed `V(z)`, through two layers half a period apart that each restart every period (the flow-map crossfade) so a speed that varies with height never stretches the pattern. The fire sheds a vortex ring at the puffing frequency `f = 1.5/√D`; a ring carries a pulse of hot gas up with the flow, so the envelope is multiplied by `1 + a sin(2π f (t − t_rise(z)))`, `t_rise(z) = ∫ dz/V`, a function of the gas's age that therefore rides up with it. The pulses fade with height (`a = 0.5 e^{−z/(3L)}`).

**7. The wind bends the plume.** In a cross wind `U` (the crate's wind, `log_wind_speed` at the plume's mid height) a parcel that has risen to `z` has drifted `U t_rise(z)` downwind; the axis is the straight chord from the base to the height `z_ref` where `ΔT` falls to 30 K, tilted at most 75° from the vertical.

**8. The shift on screen is the deflection times the background's share of the distance** (Raffel Eq. 3): a background point `D` behind the plume, seen from `Z` in front of it, moves by `ε D/(Z + D)` in angle under a perspective camera, and by `ε D` in world units under an orthographic one. The sky is infinitely far (`ε`); a wall right behind the fire barely moves; the ground where the fire stands does not move at all, so the haze never floats off its fire. Pixels whose own surface is in front of the plume are not shifted, and a shift that would land on a surface in front of the plume is refused (RTR4 §14.5.2), so a post in front of a fire is never smeared into the haze.

**9. The gain.** A campfire's deflections are 1–3 × 10⁻⁴ rad: 20–60″, four to twelve times the eye's vernier threshold of 5″, which is why the shimmer above a real fire is plain to see. A 1080p screen with a 60° field shows 3.7′ (220″) per pixel, and a bilinear resample of a high-contrast edge shows a quarter pixel (55″) at best; drawn at its physical size the shimmer would be ten times below what the screen can show. The deflection is therefore drawn at `gain = 10` times its physical value (Deviation 1), the one knob `HeatHazeSettings` carries; `0` turns the pass off.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| Flame regime | `z* < 0.08` m/kW^{2/5}: `κ = 6.8`, `η = 1/2` | McCaffrey p. 13 |
| Intermittent regime | `0.08 ≤ z* < 0.2`: `κ = 1.9`, `η = 0` | McCaffrey p. 13 |
| Plume regime | `z* ≥ 0.2`: `κ = 1.1`, `η = −1/3` | McCaffrey p. 13 |
| Buoyancy constant `C` | 0.9 | McCaffrey p. 13 |
| Plume temperature 1/e radius | `0.121 z` (`exp(−68 η²)`) | Shabbir and George Table II |
| Temperature fluctuation | `θ′ = 0.40 ΔT` | Shabbir and George Table II |
| Eddy size | `ℓ = σ/2`, second octave `ℓ/2.5` at 1.16× | Model 5 |
| Air refractivity | `n∞ − 1 = 2.778 × 10⁻⁴ · 288.15/T∞` | Edlén 1966 at 550 nm |
| Flame height | `L = D (3.7 Q*^{2/5} − 1.02)`, ≥ 0 | Delichatsios Eq. 4 (Heskestad) |
| Puffing frequency | `f = 1.5/√D` Hz | Xia and Zhang (Cetegen and Ahmed) |
| Puff amplitude | `0.5 e^{−z/(3L)}` | Deviation 4 |
| Plume top | where `ΔT = 10 K`: `z = 1.6 Q^{2/5}` m at 15 °C, tapered over its last 30 % | Model 2, Deviation 5 |
| Wind tilt | chord to `ΔT = 30 K`, at most 75° | Model 7, Deviation 3 |
| Path factor | `1/sin α`, `α ≥ 20°` | Model 4 |
| Gain | 10 | Deviation 1 |
| Largest shift | 48 px | GPU mapping |
| Sources drawn | the nearest 16 to the camera | GPU mapping |

Plume tops at 15 °C: candle 0.6 m, torch (3 kW) 2.4 m, campfire (50 kW) 7.6 m, bonfire (0.5 MW) 19 m, burning car (3 MW) 39 m.

## GPU mapping

- **Main world** (`Update`, `ElementsSystems::Simulation`): every `HeatSource` with a positive power and radius is converted to the metre frame, the nearest 16 to the camera are kept, and each is reduced to a few vectors — the base, the unit axis (tilted by the wind), the plume's top, the constants of its three regimes, its puffing frequency and its flame height — for a uniform array in an extracted resource, with the weather's air temperature, the gain and a wrapping clock.
- **Render world, per view** (`Core3d`, `EarlyPostProcess`, after the precipitation draw, so rain and every transparent are behind it; before bloom and tonemapping, so the bloom of a flame shimmers with it): the plumes' bounding boxes are projected through the view; the union of their screen rectangles, padded by the largest shift, is the only region the pass touches. If it is empty, nothing runs. Otherwise that region of the HDR main texture is copied into a scratch texture of the view's size (`TextureCache`), and a fullscreen triangle scissored to the unpadded rectangle reads the scratch texture and writes the main texture.
- **The fragment** reconstructs its ray from the view uniform and its distance from the depth prepass (the crate's cameras already have one: the clouds and the precipitation read it), and for each plume finds the closest approach of the ray to the plume's axis. A plume whose closest approach is further than `3σ` from the axis, below its base or above its top, or behind the pixel's surface, adds nothing. Otherwise its mean lens and its two octaves of noise give a deflection perpendicular to the ray, which is turned into the screen position of the displaced background point and summed. The sum is clamped to 48 px. A pixel that no plume touches is copied with `textureLoad` (bit-exact); a displaced one is sampled bilinearly, unless the sample's own depth is in front of the nearest plume it crosses, when it keeps its own colour.
- **Cost.** Zero when no plume is on screen; otherwise a copy and a pass over the plumes' rectangle only, with up to 16 ray–line tests per pixel and four gradient-noise evaluations per plume a pixel is inside. Timed as `elements_heat_haze`.
- **WebGPU**: no storage textures, one uniform array, one sampled colour texture and the depth prepass texture.

## What great looks like

- The air above a fire **boils**: a fine, fast, upward-crawling ripple that is strongest just above the flames and loosens and slows as it rises, its eddies growing with height, gone by a few flame heights for a campfire and towering above a burning car.
- It **breathes** with the fire: a soft pulse at the fire's own puffing rate (two a second for a campfire, five for a torch), riding up the plume.
- It is **anchored**: the ground under a fire is still; the haze is only visible against what is behind it, strongest against the far landscape and the sky, never smearing a post, a person or a rock that stands in front of the fire.
- It **leans with the wind** and is subtle at a distance: a far fire's plume is a faint quiver, not a sparkle.
- No seams at the edge of the pass's rectangle, no shift outside a plume, no frozen or repeating pattern, no visible tiling in a sequence, no flicker on the pixels it does not touch.
- Cost that a game can leave on: nothing off screen, a fraction of a millisecond for a campfire filling a third of a 1080p frame.

## Validation targets

1. **Physics on the CPU**: McCaffrey's centreline at the regime joins is continuous to the rounding of his constants (852 K against `66.6/0.08` = 832 K, and `66.6/0.2` = 333 K against `22.3·0.2^{−5/3}` = 326 K: within 3 %); `flame_height_m` for 50 kW on a 0.6 m base is 0.5–0.6 m; `puffing_hz` is 1.94 Hz for a 0.6 m base; `δn₀` for 838 K excess at 15 °C is 2.07 × 10⁻⁴; the largest mean deflection of a Gaussian plume is `1.52 δn₀`.
2. **No plume, no change**: with no `HeatSource`, and with one behind the camera, the rendered frame equals the frame of a gain-0 run bit for bit (the pass did not run or copied exactly).
3. **Outside the plume, no change**: with a fire in view, every pixel outside the plume's projected rectangle, and every pixel of the ground at the fire's base, is bit-identical to the gain-0 frame.
4. **The plume shifts what is behind it**: against a high-contrast background, the pixels above the fire differ from the gain-0 frame, by a mean shift of 1–4 px within a flame height above a 50 kW fire seen from 10 m at 1080p-equivalent pixel size, and the differences fade above the plume's top.
5. **It moves**: two frames 1/10 s apart differ in the plume region, and the differences travel upward between frames (a positive vertical lag in the cross-correlation of their difference images).
6. **Occlusion**: a box standing between the camera and the fire keeps its own pixels unchanged (to the pixel), and its silhouette is not pulled into the haze.
7. **Close-up at 1×**: the haze above a campfire from 6 m at eye height, against hills and sky at noon — a visible boiling ripple with no seams, no blocky noise, no clamped edges.
8. **Far**: the same fire from 150 m — a faint quiver, no sparkle (the octaves have faded).
9. **Wind**: in a 6 m/s wind the plume leans downwind.
10. **Cost**: `elements_heat_haze` for one campfire filling a third of the frame at 1920 × 1080 is under 0.15 ms on the reference GPU; with the fire off screen the span does not appear.

## Deviations from this plan

1. **Gain 10 over the physical deflection.** Physical deflections are a tenth of a 1080p pixel; the ratio of what a screen can show (a quarter pixel at 3.7′, 55″) to what the eye can see (vernier acuity 5″, Luebke et al. p. 284) is 11. Drawn physically, no fire would visibly shimmer on any screen; drawn at 10×, a campfire shifts its background by 1–3 px, the size games draw it (Nguyen's and LittleBigPlanet's haze is a few pixels). The default is confirmed by capture (Outcome).
2. **McCaffrey's correlations outside their range.** They were measured on 14–57 kW methane fires on a 0.3 m burner; they are used here from a candle to a burning car, scaling with `Q^{2/5}`, which is how fire plumes scale (the same `z/Q^{2/5}` collapses Heskestad's data from kilowatts to megawatts). A candle's flame is laminar and taller in this model than in life (0.16 m against about 4 cm), which matters only for its pulse fading.
3. **The bent plume is a straight tilted line**, its tilt from the kinematics of a parcel that rises at `V(z)` and drifts at the wind's speed. A real bent-over plume entrains faster and rises less; the cap of 75° keeps a gale from laying it flat along the ground.
4. **The puff amplitude** (0.5, fading over three flame heights) is chosen by eye: McCaffrey reports a "regular frequency of a few Hz" and intermittency, not an amplitude.
5. **The plume is cut at 10 K**, where the physical deflection is under 1.4 × 10⁻⁵ rad (a seventh of a pixel at the gain), with the envelope tapered to zero over the top 30 % so no edge shows.
6. **The flame zone's width** is the fire's radius blended with the plume's growth (`√(R² + (0.121 z)²)`), not McCaffrey's measured flame-zone profiles, which flatten near the axis.

## Pending index

- `10.1007_s00348-015-1927-5` and `10.1017_s0022112094002260`: downloaded into home-still this pass; not converted (scribe at 192.168.1.110 unavailable, bmb's slots busy) and therefore not indexed. Read through `pdftotext` on big.
- McCaffrey 1979 (NBSIR 79-1910): read from NIST's PDF; not in the library (no DOI resolver hit through `paper_download`).
