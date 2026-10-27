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
| Detail frequency, fine | 11.1 m⁻¹ (9 cm features) | new: the near-field grain; fitted to the 4 DN target (Deviation 4) |
| Octaves, coarse / fine | 4 / 3 | coarse 4 closes the 15–40 m gap (Deviation 5); fine 3 |
| Amplitude, coarse / fine | ±20 % / ±85 % | fitted: 4 DN near-field target, mean within 1 DN (Deviation 4) |
| Band limit | `keep_k = 1 − smoothstep(0.5, 1, footprint·2^k/λ)` | Lagae et al. 2010 §5.2.1 (frequency clamping) |
| Bump height, coarse / fine | 0.045 m / 0.018 m | Deviation 6: a height amplitude, not a slope |
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

1. **The detail noise is 2D in the ground plane, not 3D at the surface point.** The plan's Model 1
   inherited `terrain.md` Deviation 3 ("triplanar procedural detail noise is a 3D noise at the
   world position"). Measured on `mountains` at 6 m, a 3D slice streaked the cliff faces with
   diagonal bands: a 3D noise sampled on a tilted surface is not band-limited (Lagae et al. 2010
   §5: "sampling a 3D noise function at a 2D surface will not result in a band-limited texture"),
   and on a 60° face the lattice's vertical axis compresses by 2×. A 2D gradient noise in XZ
   (`noise::perlin2_d`, new) is isotropic in the plane the detail lives on, and the faces take no
   ground detail anyway.
2. **The detail is faded by the surface's upness as well as by the footprint.** Ground grain on a
   near-vertical rock face is wrong twice: the projection is compressed along the view direction
   there, and rock's detail is its strata, not turf. `smoothstep(0.55, 0.9, n.y)` removes it from
   the steep faces (the diagonal streaks above were reproduced exactly this way). The rock's
   own material detail is unchanged and remains a separate, future piece.
3. **The band limit uses the XZ footprint divided by the surface's upness, not the 3D footprint.**
   The detail is a function of XZ, so a pixel's XZ span is what determines which of its bands
   alias; on a slope the same 3D footprint covers more XZ, and the 3D length under-filters it.
4. **The fine scale is Perlin at 9 cm, not Worley at 12 cm.** The plan's clump scale (Worley cells
   with a `smoothstep` crown) read as a regular halftone grid on sand at 6 m and as a woven
   fabric on rock, even with Galin's domain warping; a Worley lattice at the pixel's own scale has
   structure too regular to hide. A decorrelated 3-octave Perlin sum at 9 cm is indistinguishable
   from noise at every angle tested and needs no warp. The final parameters (`DETAIL_FINE_M`
   0.09 m, `DETAIL_FINE_AMP` 0.85, `DETAIL_COARSE_AMP` 0.20, `DETAIL_COARSE_OCTAVES` 4) are fitted
   to the 4 DN near-field target on `terrain_tour` hills, plains and dunes at 6 m and 2 m.
5. **The coarse scale's amplitude is 0.20, not the old 0.25.** The 4th octave (15.6 cm) the plan's
   Validation 1's gap needed was added, and with it the same total contrast as before at 0.25
   produced a heavier mottle than the one band it replaced; 0.20 with four bands measures the
   same far-field and more near structure.
6. **The bump is a height amplitude, not a slope.** The plan's `0.04` was a multiplier on the
   noise's derivative; the gradient of a height field is `amplitude · ∂n/∂x / feature`, so the
   same look is `DETAIL_COARSE_HEIGHT_M = 0.08` with the derivative divided by the feature size.
   The first build used the raw derivative and turned the ground black (a slope of 0.36 at 12 cm).

7. **The detail is procedural, not a tiled texture atlas.** Andersson's answer to the same defect
   is tiled detail *textures*, and the crate has no texture assets beyond `stars.bin` and the
   moon map (a crate rule and a wasm-size concern). Lagae et al. §5.2.1's frequency clamping gets
   the same "no aliasing, no detail loss" result from the octaves the crate already evaluates
   analytically, at the cost of ALU in the near field only.
8. **Two scales, not one.** A single fBm at 0.8 m⁻¹ cannot carry 12 cm grain without aliasing the
   moment the camera is 3 m up; a single fBm at 8 m⁻¹ leaves the mid-distance bare. The two
   sums' bands overlap by design (the coarse covers 1.25 m and up, the fine 12.5–25 cm).
9. **The fine octave is skipped by a branch, not by a distance constant.** A constant would have
   to be tuned per field of view and resolution; the footprint test is exact for any of them, so
   the same shader is right at 1080p and at 4K and under an orthographic camera (whose footprint
   is constant in metres).

10. **A host may set the detail's amplitude and the camera's finest resolvable feature (#42).** The crate derived the footprint from `dpdx`/`dpdy` alone, so a game whose art direction or zoom turns the 9 cm band into one-texel salt-and-pepper (Sanctuary's isometric ground, red-channel stddev 26.8 on open grass) had no host-side remedy: `TerrainLook`'s tint is multiplicative on top of the grain. Two knobs on `TerrainLook`, both pure uniform writes:
    - `detail` (default 1): a multiplier on the albedo swing, on the bump gradient, and on the `coarse`/`fine` values the turf palette reads as its clump, so a lowered `detail` loses grain, relief and the palette's fine ragged edge together. At 0 the albedo is the palette's own colour field and the normal the hill's. Wet-ground sheen, snow and the path's own colours are other fields and do not read it.
    - `finest_m` (default `None`): the smallest world feature the host's camera resolves, m. The band limit's footprint becomes `max(derived, finest_m)`, so an octave of feature `f` keeps 1 at `f >= 2 * finest_m` and 0 at `f <= finest_m` (the same `detail_keep` ramp, Lagae et al. 2010 §5.2.1). A `finest_m` above `DETAIL_COARSE_M` therefore drops the coarse band's base octave and every octave below it. The road's speckle (`paths.wesl`, `path_grain`) takes the same `detail` multiplier and the same footprint floor, with its texel-resolution speckle dropped by the floor only when `finest_m` is set, so a defaulted host is unchanged.
    - Default arithmetic is unchanged: `max(footprint, 0.0)` and `× 1.0` are exact.
    - **The issue's `finest_m: 0.04` cannot reach its 8-level target under this rule**: the fine band's base octave is 9 cm, so at a 4 cm footprint its ratio is 0.44 and it is kept whole (only the 4.5 cm octave, ratio 0.89, fades to 12 %). The same issue states that a 9 cm feature is about one pixel at Sanctuary's zoom; that is a footprint of 9 cm, and `finest_m = 0.09` drops the base octave (ratio 1.0). The number to pass is the camera's metres per pixel. The acceptance figure is measured on a capture and recorded in the log.
    - The `ground_detail` debug view of the issue (per-band colour, `keep` readout) is not part of this change.

## Pending index

- Andersson 2007 and Lagae et al. 2010 were read from the corpus copies already in the repository;
  no new ingestion this iteration.
