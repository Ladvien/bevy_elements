# Ground micro-detail: a surface that reads at a metre

The terrain's *shape* is convincing at every scale the crate renders: CDLOD keeps a crack-free
hill silhouette from 2 m to 4 km, the layer masks put rock on the steep faces and sand in the
low sediment, and the trails press real geometry into the snow. What is missing is the ground's
**surface**: up close it is a flat wash with no grain, no clumping, no structure between the
terrain's 2 m cells and the shader's one noise octave.

Measured on the pre-change build (`/tmp/i28g/before`, `terrain_tour`, noon, `ELEMENTS_AGL=6`):

| Scene | Region | Standard deviation |
|---|---|---|
| `hills` at 6 m | a 200 × 100 px patch of near grass | **1.07 DN** (of 255) |
| `hills` at 6 m | the whole lower frame | 6.9 DN, and that is the hills' shading |
| `hills` at 2 m, grazing | near ground | 12.1 DN (silhouette and fog) |
| `plains` at 2 m | the whole ground | a single green with a slow 0.5 DN drift |
| `dunes` at 6 m | the sand sea | a smooth beige gradient, no grain or ripple at all |

A shipping game's ground at one metre from the eye has visible structure at every scale: turf and
clover, gravel, the granular crust of sand, cracks and small stones. The crate's ground has one
noise octave at 0.8 m⁻¹ with ±25 % amplitude fading out by 60 m (`terrain.md`, Model 2 and
Deviation 19), which is why the near ground is smooth and the distance is smooth: the only
high-frequency signal is a single Perlin band that the pixel footprint aliases once it is far
enough away, so it is faded out before it can do harm, and the mid-distance has nothing.

## Sources

- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668` and
  `s2007-advances-course-notes-1-6-mb`, read in full for this note). The defect and the fix are
  both stated there: texture-compositing terrain "often result[s] in lack of detail either up
  close, in a distance, or at high angles" (abstract); the fix is "combine low-resolution unique
  color maps … with multiple tiled detail maps for high-frequency details that are blended in at
  certain distance to the camera" (p. 1). §5.2.3: detail on masks is "tiled grayscale textures …
  combined with the lower resolution mask", "a good compromise" between "few texture fetches"
  and "ALU operation complexity", because procedural fBm costs ALU while textures cost fetches;
  the Overlay blend (Listing 2) "does not affect areas where the base procedural mask is 0.0 or
  1.0". This note follows the second half — the *detail* is a function of the noise the crate
  already has, but band-limited per octave rather than a single octave faded by hand.
- **Lagae, A., Lefebvre, S., Cook, R., DeRose, T., Drettakis, G., Ebert, D., Lewis, J. P.,
  Perlin, K., Zwicker, M. (2010)**, *A survey of procedural noise functions*, Computer Graphics
  Forum 29(8) (`10.1111_j.1467-8659.2010.01827.x`, already cited by `wind.md`; §5.2 read here).
  §5.2.1, *frequency clamping*: "when noise patterns are obtained as a weighted sum of band-pass
  noises, a first approach is to cancel the contribution of bands whose frequency is too high"
  (citing Norton, Rockmore & Nielson 1982). It "works best if the noise is narrowly band-pass …
  Perlin noise is only weakly band-pass, making frequency clamping difficult to tune". §4: a
  sum of weighted band-limited layers is the standard way to control a spectrum, and the bands
  "have little overlap in the frequency domain and can be seen as a spectral basis". §5.2 also
  records Cook & DeRose 2005's finding that "Perlin noise is prone to problems with aliasing and
  detail loss, because it is only weakly band-limited", which is exactly the crate's symptom: one
  weak band that aliases at distance and is therefore faded away, leaving nothing.
- **Perlin, K. (1985)**, *An image synthesizer*, SIGGRAPH (via Lagae et al. §2 and this crate's
  `noise.wesl`): gradient noise "is an approximation to white noise band-limited to a single
  octave"; the crate's `fbm3` sums octaves at lacunarity 2, gain 0.5, so each octave is one
  band and can be weighted independently. That is what makes frequency clamping cheap here.
- **Musgrave, F. K., Kolb, C. E., Mace, R. S. (1989)**, *The synthesis and rendering of eroded
  fractal terrains*, SIGGRAPH (`10.1145_74334.74337`, as in `terrain.md`): lacunarity λ = 2 and
  persistence, the octave structure whose bands this note's weights index.
- **Galin, É. et al. (2019)**, *A review of digital terrain modeling*, CGF 38(2)
  (`10.1111/cgf.13657`, as in `terrain.md`): the turbulence sum `Σ a_i n(φ_i p)`, i.e. detail as
  weighted bands of the same base noise.
- **Kittler & Darula 2014** and the crate's own `docs/research/terrain.md` §Model 2 for the
  existing albedo-detail constants (±25 %, 0.8 m⁻¹) and `terrain.md` Deviation 19 for the mask
  detail (4 octaves from 48 m).

## Model adopted

**1. Detail is a sum of octaves, each faded by its own footprint.** The ground's detail albedo
modulation becomes

```
d(p) = 1 + A · Σ_k w_k · n_k(p) / Σ_k w_k
```

with `n_k` the crate's `fbm3` octaves (the same noise as the terrain's shape, seeded the same
way), `w_k = 0.5^k` the usual persistence, and `A` the amplitude. Each octave's frequency is
known: the base octave's lattice is 1 / 0.8 m, so octave `k` has a feature size `0.8 / 2^k` m.

**2. Frequency clamping, per octave.** Octave `k`'s weight is multiplied by the fraction of the
pixel footprint that can resolve it. The footprint is the screen-space derivative of the ground
position, `max(length(dpdx(pv)), length(dpdy(pv)))` in metres per pixel; octave `k` is kept while
its feature size is at least two footprints (`2 · footprint < 0.8 / 2^k`), and its weight falls
to zero over the band between one and two footprints:

```
keep_k = 1 − smoothstep(0.5, 1.0, footprint · 2^k / 0.8)
```

This is Lagae et al. §5.2.1's frequency clamping applied to the crate's own octave sum. It
replaces the old hard fade (`1 − smoothstep(40, 120, dist)`), which threw away every band the
moment the camera moved away instead of discarding only the bands that alias.

**3. Two independent detail scales.** One fBm octave sum at 0.8 m⁻¹ carries the mid-frequency
grain the crate has today (1 m features and coarser). A second, finer sum at 8 m⁻¹ carries the
near-field grain (12 cm features: soil crumbs, turf clumps, gravel), visible only within a few
metres where its footprint is small enough. Both are band-limited by (2); neither has a hand-set
fade distance.

**4. The normal follows the detail.** The bump perturbation stays the analytic derivative of the
same noise (`perlin3_d`), scaled by each octave's kept weight, so a surface that loses its
albedo grain at distance also loses its shading grain and cannot shimmer. The normal's
perturbation amplitude is `0.04` (unchanged) at the coarse scale and `0.02` at the fine one.

**5. The layer's colour keeps its structure.** The detail multiplies the layer colour, so grass
keeps grass's hue and sand keeps sand's; a separate low-frequency, *contrast-only* modulation
(already present at ±25 %) is unchanged. The fine octaves carry no colour of their own.

**6. Both paths, one function.** `TerrainExtension` (`render.wesl`) and `GroundExtension`
(`ground.wesl`) share the function through `terrain::common`, so a game's own mesh gets the same
ground. Nothing changes for a camera over 120 m up: every octave's `keep` is zero there.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| Detail frequency, coarse | 0.8 m⁻¹ (1.25 m features) | `terrain.md` Model 2 (unchanged) |
| Detail frequency, fine | 8 m⁻¹ (12.5 cm features) | new: the near-field grain; [INFERENCE] from the 1 m eye height |
| Octaves, coarse / fine | 3 / 2 | `terrain.md` Model 2 (3); fine 2 keeps the cost at 5 taps |
| Amplitude, coarse / fine | ±25 % / ±15 % | coarse unchanged; fine chosen so a sunlit grass patch stays inside ±20 DN |
| Band limit | `keep_k = 1 − smoothstep(0.5, 1, footprint·2^k/λ)` | Lagae et al. 2010 §5.2.1 (frequency clamping) |
| Bump, coarse / fine | 0.04 / 0.02 | coarse unchanged (`terrain.md` Model 2); fine half, so 12 cm grain does not read as rock |
| Distance at which all detail is gone | ~120 m at 1080p | computed from (2) with the horizontal footprint, not a constant |

## GPU mapping

No new pass, buffer, binding or texture: the detail is the existing `fbm3`/`perlin3_d` evaluated
at two scales with per-octave weights, plus one `dpdx`/`dpdy` per fragment (already implied by the
texture taps). The fine octave's two fBm calls are skipped by a branch on `keep > 0`, which is
false for every pixel beyond a few metres, so the distant ground pays nothing (the shader is
uniform-free on that branch per pixel; a wavefront where any lane is near keeps the branch, and
the cost is bounded by the near field).

## What great looks like

- Standing on grass at 1.7 m: the ground reads as *turf* — clumps a hand's width across, a
  little darker in the hollows, lighter on the crowns, with the shading following the same
  clumps. At 20 m the clumps have become a soft mottle, not a shimmer.
- Sand at 1.7 m: granular, a fine grain that catches the light, with the dunes' large shapes
  still reading as the dune's own curvature. From 200 m the sand is smooth and clean.
- Rock: the grain follows the strata and does not crawl when the camera moves.
- Snow: the detail is under the snow. A snow-covered ground shows the snow's own smooth surface
  (the snow albedo blends over it), with the buried ground's grain visible only in a trench
  floor where the trail's parallax reaches it.
- Wet ground: the grain darkens and glosses with the water, so a puddle's edge reads against dry
  grain rather than a flat colour change.
- No aliasing at any distance or grazing angle, and no popping as the octave weights cross their
  band: the weight is a smoothstep, not a hard cut.
- The cost stays inside the material's existing budget: one extra `fbm3` at 2 octaves in the near
  field, nothing in the distance.

## Validation targets

1. **Near-ground variance (capture, `terrain_tour` hills at 6 m, noon).** A 200 × 100 px patch of
   near grass has a standard deviation of at least **4 DN** (measured 1.07 before), and no pixel
   of it differs from its 3 × 3 mean by more than 25 DN (no salt).
2. **Distance is unchanged (capture).** The same scene's ground beyond 120 m is within 2 DN of
   the pre-change capture, per channel, because every octave is clamped there.
3. **No aliasing (capture, grazing and aerial).** `hills` at 2 m grazing and at 300 m aerial:
   no pixel-to-pixel flicker in a 2-frame difference (mean |Δ| under 1 DN on static ground), and
   no moiré in a 4× crop of the mid-distance.
4. **Sand (capture, `dunes` at 6 m).** The near sand's standard deviation at least 4 DN; the dune
   silhouette unchanged.
5. **Snow (capture).** A snow-covered ground's near patch stays smooth (its standard deviation
   within 1 DN of the pre-change capture), because the snow albedo covers the detail.
6. **Both paths (GPU test).** `GroundExtension` on a game mesh at 2 m shows the same near
   variance as `TerrainExtension` over the same ground, within 20 % of the terrain's.
7. **Cost (bench).** `main_opaque_pass_3d` at High, 1080p, the camera 6 m above hills, within
   5 % of iteration 27's logged minimum.

## Deviations from this plan

1. **The detail is procedural, not a tiled texture atlas.** Andersson's answer to the same defect
   is tiled detail *textures*, and the crate has no texture assets beyond `stars.bin` and the
   moon map (a crate rule and a wasm-size concern). Lagae et al. §5.2.1's frequency clamping gets
   the same "no aliasing, no detail loss" result from the octaves the crate already evaluates
   analytically, at the cost of ALU in the near field only.
2. **Two scales, not one.** A single fBm at 0.8 m⁻¹ cannot carry 12 cm grain without aliasing the
   moment the camera is 3 m up; a single fBm at 8 m⁻¹ leaves the mid-distance bare. The two
   sums' bands overlap by design (the coarse covers 1.25 m and up, the fine 12.5–25 cm).
3. **The fine octave is skipped by a branch, not by a distance constant.** A constant would have
   to be tuned per field of view and resolution; the footprint test is exact for any of them, so
   the same shader is right at 1080p and at 4K and under an orthographic camera (whose footprint
   is constant in metres).

## Pending index

- Andersson 2007 and Lagae et al. 2010 were read from the corpus copies already in the repository;
  no new ingestion this iteration.
