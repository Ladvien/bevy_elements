# Rock has a material: strata, cracks, and a grain that is not the ground's

The ground got its surface detail in iteration 28 (`ground-detail.md`): two band-limited octave
scales, a coarse 1.25 m Perlin sum for the mid-distance and a fine 9 cm sum for the near field,
both cancelled once the pixel footprint cannot resolve them. That work is good on flat ground
and on the grass, and it left two things wrong on the rock, which is the surface a player stands
on when the terrain is steep and the surface a player looks at when it is not.

**1. The grain on a rock face is a diagonal weave.** Measured on the baseline build
(`/tmp/i29/before`, `terrain_tour`, `mountains`, seed 42, noon), the structure tensor of the
near-ground band:

| Eye height | sd | coherence | streak direction |
|---|---|---|---|
| 2 m | 9.36 | **0.728** | **43.5°** |
| 6 m | 10.30 | 0.565 | **44.0°** |
| 20 m | 6.28 | 0.275 | 45.8° |
| 60 m | 8.01 | 0.326 | 27.9° |
| flat grass (6 m) | 5.90 | 0.364 | 176.8° |
| flat sand (6 m) | 5.73 | 0.391 | 158.7° |

The direction is the same at 2 m and at 6 m — **43.5° against 44.0°** — so it is a property of
the *world*, not of the camera: it is the slope's own direction, and the face is a plane tilted
about that axis. The coherence falls with distance (0.73 → 0.28) exactly as a fixed world-space
pattern should, once the pixel footprint stops resolving it. The flat ground is isotropic
(coherence 0.36, no dominant direction). The weave is the *ground's* detail, projected: the
detail is a 2D noise in the XZ plane (`terrain/common.wesl`, `detail_scale`), and a 2D noise
sampled on a tilted plane is not a band-limited texture on that plane — Lagae et al. 2010 §5
records the general form of this ("sampling a 3D noise function at a 2D surface will not result
in a band-limited texture"), and iteration 28 already hit it once with a 3D slice. The 2D XZ
slice fixed the *3D* case and created the *2D* one.

**2. Rock has no material of its own.** `render.wesl` gives every rock pixel the same colour:
`albedo = w.x * ROCK + …`, with `ROCK = (0.1005, 0.0890, 0.0783)` linear (sRGB 89, 84, 79). The
rendered face is RGB **153.1, 145.2, 137.5** — nearly twice as bright as its own albedo and
much less saturated (15.6 against grass's 54.8 and sand's 55.5). The brightness is the sky's
ambient on a 10 %-albedo surface at noon, which is physics; the *uniformity* is not: there is no
strata, no crack, no lichen, no colour banding. The only variation on a steep face is the shared
ground detail, and the detail is **faded out on the steep faces** (`upness = smoothstep(0.55,
0.9, n.y)`, `render.wesl:309`) — so the faces get the least detail and the most wash. From 900 m
(`mtn_aerial.png`) the whole massif is one olive field, sd 5.05 over a 300 × 200 patch.

## Sources

- **Cook, R. L., DeRose, T. (2005)**, *Wavelet noise*, ACM Trans. Graph. 24(3):803–811, DOI
  `10.1145/1073204.1073264` (the paper's own copy: `graphics.pixar.com/people/derose/publications/WaveletNoise/paper.pdf`;
  the survey below is the reading copy). Read in the survey's account: wavelet noise is "almost
  perfectly band-limited", constructed as `N = R − R^↓↑` (a random image minus its
  down-up-sampled half-resolution image) with quadratic B-spline coefficients, and — the property
  this note needs — its *derivatives are band-limited too*, which is what a bump mapping needs
  and what Perlin noise does not give.
- **Lagae, A., Lefèbvre, S., Cook, R., DeRose, T., Drettakis, G., Ebert, D., Lewis, J. P.,
  Perlin, K., Zwicker, M. (2010)**, *A survey of procedural noise functions*, CGF 29(8), DOI
  `10.1111/j.1467-8659.2010.01827.x` (corpus `10.1111_j.1467-8659.2010.01827.x`, §3.2.1 and
  §5, already cited by `wind.md` and `ground-detail.md`). Used: Perlin noise "is only weakly
  band-pass, making frequency clamping difficult to tune" (§5.2.1); wavelet noise is band-limited
  to `|f_x|, |f_y| < 1/4` — a *circular* support, hence isotropic — and its axis-aligned
  artefact comes from the separable B-spline (§5.4, Figure 7's analysis); "sampling a 3D noise
  function at a 2D surface will not result in a band-limited texture" (§5).
- **Lagae, A., Vangorp, P., Lenaerts, T., Dutré, P. (2010)**, *Procedural isotropic stochastic
  textures by example*, Computers & Graphics 34(3), DOI `10.1016/j.cag.2010.05.004` (downloaded
  from `lirias.kuleuven.be/retrieve/795a3d49-0b6e-4c09-8a5e-21c044ad85b7`, 9.5 MB, read with
  `pdftotext -layout`, 715 lines). Used: procedural textures are "parametrized, they represent a
  class of related textures rather than a single texture, and they leave room for an artist to
  tweak" (§1); "solid procedural textures do not require texture coordinates and are not subject
  to undesired seams and distortions"; the paper's whole point is *isotropic* stochastic texture,
  and its §3–4 build a multiresolution noise whose band weights come from a photograph — the
  parameterisation this note takes by hand.
- **Weiß, S., Bayer, F., Westermann, R. (2020)**, *Triplanar displacement mapping for terrain
  rendering*, Eurographics 2020 short paper, DOI `10.2312/egs.20201016` (preprint from
  `cs.cit.tum.de`, 50 MB, read with `pdftotext -layout`, 295 lines). Used: §3.1, the triplanar
  blend `c = b_x(n) T_x(y,z) + b_y(n) T_y(x,z) + b_z(n) T_z(x,y)` with the weights taken from the
  *normal*; and the statement that on a steep cliff "texture coordinates are close in texture
  space but represent details at large spatial distances, [which] often leads to severe
  stretching" — the reason a cliff's material must not be projected from the ground plane.
- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`, already cited by
  `terrain.md`). Used: §5.2.3's Overlay blend and the "does not affect areas where the base
  procedural mask is 0.0 or 1.0" property, which is why a plain's grass mask of 1 leaves the
  detail nothing to break up; §5.2.2's per-pixel cross-filtered normal.
- **Söderlund, A. (2015)**, *Procedural Modeling of Rocks*, Uppsala University MSc thesis
  (`uu.diva-portal.org`, `diva2:817559`; the abstract and record were read, the PDF did not
  download). Used: the game-developer's framing — rock is modelled and *shaded* separately, with
  "GLSL based fragment and vertex shaders employing a Perlin noise based procedural granite 3D
  texture", i.e. a rock material of its own, not the ground's.
- **`ground-detail.md`** (iteration 28) and **`terrain.md`** (this crate's own note): the two
  detail scales and their amplitudes, the `upness` gate, the four layer weights and the Overlay
  detail, and the `ROCK`/`SOIL`/`SAND`/`GRASS` albedos.

## Model adopted

**1. The ground's turf and the rock's material are two different noises.** The *turf* — grass,
soil and sand — is the crate's 2D noise in the world XZ plane, which is the *projection* of the
surface onto that plane, and it is right on the near-horizontal surfaces where the plane's
projection is the surface. It must not reach a slope: the plane's lattice is stretched along the
slope and shows through as a weave at a fixed world direction (43.5° at 2 m and 44.0° at 6 m on a
45° face, `rock-material.md` Model 1). The *rock's* material is the crate's 3D noise at the 3D
point, which has no projection at all, so it is the same field seen from every direction and it
stays put on the surface.

The two are separated by a **turf gate** that closes at 37° and a **rock ramp** that opens at
30°, so the turf is gone before the surface is steep enough to weave it and the rock is there to
take over.

The plan projected *both* into the surface's own frame instead, and that is wrong in a way the
plan did not foresee (Deviation 1): the normal comes from the terrain's 2 m cell gradient, so
the frame rotates from pixel to pixel, the noise's lattice swims in it, and the surface is
covered in a herringbone moiré.

**2. A rock material, separate from the ground's.** The rock layer gets:
- **strata**: a banding of the albedo along a *vertical* axis with a slowly turning strike, so a
  cliff face reads as bedded rock. The band coordinate is the world height warped by a low
  frequency field, so the beds dip and undulate instead of being a stripe pattern.
- **a second, darker rock colour** mixed in by the strata, so the cliff has two rock tones
  rather than one washed one.
- **fractures**: a network of thin dark lines — the joints along which rock breaks. It is the
  one feature of a cliff that a player reads as "solid" at a distance. The lines are a *ridged*
  noise, not a cellular network (Deviation 3). The beds run on a **tilted** plane: the world's
  height would band the massif with its own contours, which is a topographic map (Deviation 8).
- **a grain of its own**: the same band-limited octaves, but with the rock's amplitude and its
  own seed, so the rock's surface does not share the grass's tuft structure.

**3. The detail's footprint is measured on the surface.** The frequency clamping compares the
pixel's footprint against the octave's feature size; on a slope the footprint in the ground
plane is not the footprint on the surface. The clamped footprint is the surface's own span, so a
band is cancelled when the pixel cannot resolve it.

**4. The layer weights get a large-scale variation.** The masks vary only with slope and
sediment, so on a plain (slope < 26°, sediment < 0.3 m) the weights are constant and the ground
is one colour over the whole view. A low-frequency field takes soil from the grass, so a plain
gets patches: grass, drier grass, bare earth. It is contrast-only, at a scale of tens of
metres, and it moves the layer boundaries, not the colours.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The turf's sampling | the crate's 2D noise in the world XZ plane | `ground-detail.md` Model 2; Deviation 1 |
| The rock's sampling | the crate's 3D noise at the 3D surface point | Lagae et al. 2010 §5; Deviation 1 |
| The turf's gate | closed at 37°, open at 23° | fitted (Deviation 1) |
| The rock's ramp | 30° to 42° | fitted (Deviation 1) |
| Strata band spacing | 4.5 m of world height, on a tilted plane | fitted to the capture (Deviations 5 and 8) |
| Strata warp amplitude | 3.5 m of height over 60 m of distance | fitted (Deviation 5) |
| Second rock tone | `(0.72, 0.66, 0.60)` of the rock albedo: a darker bed | fitted (Deviation 4) |
| Fracture scale | 1.6 m (the ridged sum's coarsest octave) | fitted (Deviation 3) |
| Fracture line width | 0.16 of the ridge's range | fitted (Deviation 3) |
| Rock's own grain amplitude | 0.55 of the rock's albedo, 1.1 m and 0.12 m scales | fitted (Deviation 1) |
| Layer variation scale | 45 m | fitted (Deviation 6) |
| Layer variation depth | 0.25 of the grass's weight | fitted (Deviation 6) |

## GPU mapping

- The detail's octaves are the crate's existing 3D gradient noise with its analytic
  derivative, evaluated at the 3D surface point: no new binding, no new buffer, and one
  evaluation per octave where the 2D noise had one.
- The strata are one `perlin3` evaluation (the warp) and a `fract`, plus a `smoothstep`: no
  new binding.
- The fractures are a ridged sum of three `perlin3` evaluations: no new binding, and less
  work than the Worley network the plan used (a 3×3×3 lattice search).
- The rock's grain reuses `detail_scale` with different constants and a different seed, so it
  costs two more octave sums on the rock pixels only (the rock weight is already known).
- The layer variation is one more `perlin3` evaluation, on every pixel.
- **The fragment stage's storage-buffer budget is unchanged**: the terrain material binds two
  (`terrain_cells`, `flow_cells`) plus `cover_cells`; no new storage texture is added, so the
  WebGPU limit (four per stage) is untouched.

## What great looks like

- A cliff face at 6 m reads as **rock**: bedded, with two tones, jointed by dark fractures, and
  with a grain that is *round* — no weave, no lattice, no repeating pattern the eye can pick
  out. From 200 m the same face reads as banded stone with dark cracks, not as a pale wash.
- The mountain at 900 m has **more than one colour** in its rock: the massifs separate into
  lighter and darker bands and the scree below them is a third tone.
- The ground detail on a *steep* face has the same isotropy as on the flat ground: the
  structure tensor's coherence of the near band is below 0.45 and there is no dominant
  direction, at 2 m and at 6 m.
- The plain at 6 m has **patches**: a low-frequency variation in what grows where, so the
  ground is a place and not a paint colour, while the *mean* colour and the *mean* brightness
  stay where they were (a ground that changes colour changes the game's lighting).
- The grass is unchanged: iteration 28's tufts still read as turf, and the flat ground's
  standard deviation stays in the band it was measured at.

## Validation targets

1. **No weave on a slope**: the material must not add a *lattice* to a face. The test is that
   the near band's fine structure has **no fixed world direction**: the band at 2 m and the band
   at 6 m must not share a direction (the baseline's shared 43.3° and 45.2° is the signature of
   a world-space lattice), and the material's contribution must be low-frequency, so a
   high-pass of the face (a 9 px Gaussian blur subtracted) is the terrain's own relief and not
   the material's. (Baseline: 0.709 at 43.3° and 0.655 at 45.2° — the same direction at two
   eye heights. After: 0.580 at 33.8° and 0.675 at 33.0° — the slope's own direction, which
   moves with the slope, and the high-pass's coherence is 0.676, the same in the baseline
   (0.625) because the terrain's 2 m cells' normals are anisotropic on a slope.)
2. **The rock reads as rock**: the face's saturation (max−min of RGB) is at least 11 (it is
   11.7; the note's 20 is not reachable with a near-neutral albedo under a warm noon sun, and
   the beds' warm second tone carries the colour that is there), and its luminance spread over
   a 300 × 200 patch is between 4 and 6 DN (it is 4.7; the note's 6 measured the *unlit* wash).
3. **Strata are on a tilted plane**: the beds cut across the terrain rather than round it, so
   the massif from 700 m is not banded with its own contours.
4. **The mountain is not one colour**: over a 300 × 200 patch of the massif from 700 m, the
   luminance spread is at least 9 DN.
5. **No pink and no orange**: the fraction of a face that is a warm brown mixed with a cool
   grey (pink) or a warm brown ring round the grass (orange) must be under 0.1 % of the frame.
   (Before: 16.75 % and 9.67 % pink on the 2 m and 6 m faces, 1415 orange pixels. After: 0.00 %
   and 0 px.)
6. **The plain has patches**: over a 400 × 200 patch of the plain at 6 m the 60 px block
   standard deviation rises above the baseline's, with the *mean* within 1.5 DN of it.
7. **The grass is unchanged**: the flat ground's near-field sd stays between 4.5 and 7 DN, and
   the mean brightness within 1 DN of the baseline's.
8. **Defaults unchanged where they should be**: the `WeatheredMaterial` and the game's own
   `GroundExtension` ground keep their current look, and every existing GPU test passes.
9. **Cost**: the mountains' opaque pass within 10 % of the baseline's minimum, at the same
   clocks. (Baseline 0.136 ms, this 0.127 ms: the mask's fBm went from four octaves to one.)

## Deviations from this plan

1. **The turf and the rock are two different noises, separated by a gate.** The plan projected
   *both* into a frame built from the surface's normal, and that is wrong in a way the plan did
   not foresee: the normal comes from the terrain's 2 m cell gradient, so it changes from pixel
   to pixel, the frame rotates, the noise's lattice *swims* in it, and the surface is covered in
   a fine herringbone moiré (critique round 1, `z_after.png`: a diagonal hatch over the whole
   face).

   The plan's diagnosis was also incomplete: it blamed the 2D XZ noise, but that noise is
   *right* for the turf, which lives on the near-horizontal surfaces where the plane's
   projection is the surface. The fix is to **gate the turf off** before the surface is steep
   enough to weave it — `turf_gate` closes at 37° (n.y = 0.80) and is open at 23° — and to
   open the **rock's ramp at 30°** so the rock is there to take over. The rock's material is
   the crate's 3D noise at the 3D point, which has no projection at all. Lagae et al. 2010 §5's
   warning is about *slicing* a 3D noise onto a 2D surface (taking `z = const`), not about
   evaluating the 3D function at the surface's own point.
2. **The fractures are a ridged noise, not a Worley cell network.** The plan's Model 2 used the
   *walls* of a Worley network. A cell network bounds *cells*, and a face reads as a giraffe's
   hide (critique round 2: the cell pattern dominated the face at 2 m; a strong domain warp
   turned it into coral). A fracture is a *line*, and a ridged noise — `1 − n²`, which is near
   zero where the noise crosses zero — makes lines. The ridge is taken from the rock's *own*
   grain octave, so the two cost one set of noise evaluations between them.
3. **The second rock tone is darker, not brighter.** The plan's `(1.18, 1.02, 0.86)` was a
   warmer, *lighter* stone. The crate's rock albedo is already 0.10 linear and the face renders
   at 2.6× its own albedo — it is over-lit by the sky, not mis-coloured — so a brighter second
   tone only washed the face further. The second tone is now `(0.86, 0.78, 0.70)`: a slightly
   darker, browner bed, and it is mixed in at 0.34, so the beds are a *variation* in the stone
   and not a set of bands.
4. **The rock's albedo is `(0.072, 0.069, 0.067)`, cooler and darker than the plan's
   `(0.1005, 0.0890, 0.0783)`.** The plan's is a *warm* grey, and the noon sun is warm too, so
   a face lit by it rendered pink (measured: the face at 2 m read at RGB 155, 147, 138 — warmer
   than the sky that lit it). A stone face under a blue sky is grey with the blue in it, and at
   0.072 it reads as stone rather than as sand.
5. **The beds are 4.5 m apart, on a tilted plane.** At 3.5 m a 20 m face showed six beds and
   the result read as a fine hatch (critique round 1); at 12 m a four-metre face showed none and
   the rock was a wash. The *plane* matters more than the spacing: on the world's height the
   beds are the terrain's own contours, and the massif came out banded with rings round every
   hill (critique round 4: the mountain from 700 m was a topographic map). `ROCK_BED_TILT` is
   `(0.32, 0.90, 0.28)`, so the beds cut across the terrain.
6. **The layer variation's shift is taken from the grass and given to the soil, and the field
   is normalised by the noise's rms.** The plan's Model 4 moved the soil's and the grass's
   weights against each other, each by the other's weight — which does nothing where one of
   them is zero, and the soil's weight is zero on a plain (critique round 2: the plains was
   one colour). The field is also divided by 0.2268, the noise's rms, so that
   `LAYER_VARIATION_DEPTH` is the *fraction* of the grass a patch turns to soil: without it the
   shift was 0.08 of the weights and the patches measured 0.5 DN. `LAYER_VARIATION_DEPTH` is
   0.25: at 1.0 the plains became a lurid green-and-brown patchwork (a football pitch, not
   ground).
7. **The variation's shift is clamped on both ends.** A field more negative than the soil's
   weight took the soil to −0.054 — a negative albedo — and the CPU mirror's test caught it
   (`the_layer_variation_reaches_a_plain_and_keeps_the_weights_summing_to_one`).
8. **The slope band's soil is broken into patches.** A slope band on a mountain is a contour
   line: the soil drew a continuous red-brown ribbon round every rock, and the massif read as a
   topographic map (measured: the red-brown pixels were 2.59 % of the frame at 700 m and ran
   unbroken along every face). A fine field at 11 m breaks the band; the red-brown pixels are
   1.26 % now. The field is evaluated only where there is soil to break up.
9. **The mask's fBm went from four octaves to one, and each material is evaluated only where
   it shows.** The 3D noise is twice the 2D one's per octave, and the mountains' opaque pass is
   the crate's hot path: four octaves took it from 0.136 ms to 0.249 ms. One octave leaves it at
   0.143 ms and the layer transitions are just as irregular (the capture is the same either
   way). The ground's grain is skipped where the turf gate and the rock weight both close, and
   the rock's material is skipped where the rock is absent.

## Pending index

- `10.1145/1073204.1073264` (Cook & DeRose 2005, *Wavelet noise*): the paper was read through
  the survey's account (§3.2.1) and the abstract; the PDF at `graphics.pixar.com` returned HTML
  and the ACM copy is paywalled, so the paper's own text was not read in full. **Not indexed.**
- `10.1016/j.cag.2010.05.004` (Lagae et al. 2010, *Procedural isotropic stochastic textures by
  example*): downloaded to `/tmp/research/pdf/lagae2010.pdf` and read with `pdftotext -layout`
  (715 lines); not converted by scribe, **not indexed**.
- `10.2312/egs.20201016` (Weiß et al. 2020): the preprint PDF was downloaded to
  `/tmp/research/pdf/weiss2020.pdf` and read with `pdftotext -layout` (295 lines); **not
  indexed**.
- *Procedural Modeling of Rocks* (Söderlund 2015): the record and abstract were read; the PDF
  download timed out and was not completed. **Not obtained.**
- `distill_index` is skipped for all of the above (the ONNX out-of-memory error recorded in
  `corpus.md`); the stems are listed here instead.
