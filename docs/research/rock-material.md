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

**1. The ground's detail is projected along the surface's own frame, not the world XZ plane.**
The detail's texture coordinate becomes the surface's position expressed in a frame built from
the surface normal: with `t = normalize(n × up)` the tangent and `b = n × t` the bitangent, the
detail is sampled at `(p · t, p · b)`. On a flat surface this is the XZ plane (unchanged); on a
45° face the noise's lattice lies *in the face*, so its features are round on the face and the
weave disappears. The world position is still what the noise is a function of, so nothing slides
when the camera moves. This is triplanar mapping's *first* axis generalised to the local frame
(Weiß et al. 2020 §3.1's `T_x(y,z)` idea, with the projection chosen by the normal instead of
blending three projections).

**2. A rock material, separate from the ground's.** The rock layer gets:
- **strata**: a banding of the albedo along a *vertical* axis with a slowly turning strike, so a
  cliff face reads as bedded rock. The band coordinate is the world height warped by a low
  frequency field, so the beds dip and undulate instead of being a stripe pattern.
- **a second, darker rock colour** mixed in by the strata, so the cliff has two rock tones
  rather than one washed one.
- **fractures**: a cellular (Worley) network of thin dark lines — the joints along which rock
  breaks. It is the one feature of a cliff that a player reads as "solid" at a distance.
- **a grain of its own**: the same band-limited octaves, but with the rock's amplitude and its
  own seed, so the rock's surface does not share the grass's tuft structure.

**3. The detail's footprint is measured in the surface's own frame.** The frequency clamping
compares the pixel's footprint against the octave's feature size; on a slope the footprint in
the ground plane is not the footprint on the surface. The clamped footprint becomes the
*larger* of the two axes' spans, so a band is cancelled when *either* axis cannot resolve it.

**4. The layer weights get a large-scale variation.** The masks vary only with slope and
sediment, so on a plain (slope < 26°, sediment < 0.3 m) the weights are constant and the ground
is one colour over the whole view. A low-frequency field modulates the *rock and soil* weights
against the *grass and sand* ones, so a plain gets patches: grass, drier grass, bare earth. It is
contrast-only, at a scale of tens of metres, and it moves the layer boundaries, not the colours.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| Detail's projection frame | the surface's tangent/bitangent frame, `t = normalize(n × up)` | Weiss et al. 2020 §3.1; Lagae et al. 2010 §5 |
| Strata band spacing | 3.5 m of world height | fitted to the capture (Deviation 1) |
| Strata warp amplitude | 1.2 m of height over 60 m of distance | fitted (Deviation 1) |
| Second rock tone | 0.62 of the rock albedo, warm-shifted | fitted (Deviation 1) |
| Fracture cell size | 2.4 m | fitted (Deviation 1) |
| Fracture line width | 0.06 of a cell | fitted (Deviation 1) |
| Rock's own grain amplitude | 0.55 of the rock's albedo, 1.1 m and 0.12 m scales | fitted (Deviation 1) |
| Layer variation scale | 45 m | fitted (Deviation 2) |
| Layer variation depth | 0.35 of the grass/soil weight | fitted (Deviation 2) |

## GPU mapping

- The tangent frame is two cross products and a normalize in the fragment shader: no new
  binding, no new buffer.
- The strata are one `perlin3` evaluation (a warp) and one `perlin2_d` (the band coordinate's
  own detail), plus a `smoothstep`: no new binding.
- The fractures are one `worley3` call at 2.4 m — a 3×3×3 lattice search, the same cost as the
  ground's existing `clump_scale` call.
- The rock's grain reuses `ground_detail`'s octave sums with different constants and a
  different seed, so it costs one more octave sum on the rock pixels only (the rock weight is
  already known).
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

1. **No weave on a slope**: the near-ground band of `terrain_tour` `mountains` at 2 m and 6 m
   has a structure-tensor coherence below 0.45 and no direction with more than 35 % of the
   band's gradient energy, at seed 42 and seed 7.
2. **The rock reads as rock**: the face's saturation (max−min of RGB) is at least 22 (against
   15.6 today) and its luminance spread over a 300 × 200 patch is at least 9 DN (against 5.8).
3. **Strata are horizontal in world space**: the dominant gradient direction of the luminance on
   a 40° face is within 12° of the horizontal, and the bands' spacing in world height is
   3.5 m ± 20 % (measured on a straight face from two eye heights).
4. **The mountain is not one colour**: over a 300 × 200 patch of the massif from 900 m, the
   luminance spread is at least 9 DN and the saturation spread at least 6.
5. **The plain has patches**: over a 400 × 200 patch of the plain at 6 m the luminance spread is
   at least 4 DN with the *mean* within 1.5 DN of the baseline's.
6. **The grass is unchanged**: the flat ground's near-field sd stays between 3.5 and 7 DN, and
   the mean brightness within 1 DN of the baseline's.
7. **Defaults unchanged where they should be**: the `WeatheredMaterial` and the game's own
   `GroundExtension` ground keep their current look, and every existing GPU test passes.
8. **Cost**: `elements_terrain` and the main opaque pass within 10 % of the baseline's
   minimum, at the same clocks.

## Deviations from this plan

*(filled in as the implementation settles; every changed default is recorded here with its
citation.)*

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
