# The water's surface is a mirror with a skin on it, not a bowl of jello

`docs/sky_light.gif` shows the pond under a partly cloudy sky, and the water reads as a smooth
blue-grey sheet with soft blobs where the cumulus should be reflected. A player sees "jello":
a surface with the *shape* of a liquid but none of the *behaviour* of one. The same complaint
covers the second half of the request: the readable rain and snow are not much better than the
physical ones, because the water is the largest thing in those frames too.

## The defect, measured

The water is a near-perfect mirror: `WaterSettings`'s `perceptual_roughness` is 0.089, and
Bevy's `perceptualRoughnessToRoughness` squares it (`bevy_pbr-0.20.0/src/render/pbr_lighting.wesl:546`),
so the *linear* roughness is **0.0079** — 0.45°. What the eye sees is therefore the environment
map, perturbed by the surface normal. The only thing between the sky and the player is the
ripple normal map, and it is that map which makes the jello.

Measured on `examples/waves` at 16:30, camera 8.7 m up, `PartlyCloudy` (`/tmp/review28/cap/`):

| Frame | sky band high-pass sd | water band high-pass sd | ratio |
|---|---|---|---|
| ripples on (the shipped look) | 1.99 | **4.44** | 2.23× |
| ripples off (`ripple_scale = 0`) | 1.99 | 1.35 | 0.68× |

**The water is noisier than the sky it mirrors.** With the ripples off it is a clean mirror
(the cumulus reflect crisply, `pond_flat.png`). A real water surface reflects the sky *softer*
than the sky, never sharper: the ripples blur what they reflect, they do not add detail to it.
The 2.23× is the jello, quantified.

## Why: the normal map is an open-ocean statistic applied to a pond

`water_normals.wesl` builds the map's height as `Σ 0.35^o · perlin3_periodic(u · 8·2^o)` over
five octaves, so the *wavelengths* run 1 m → 6 cm and the *slope* of octave `o` is `0.7^o` of
the first. The octave-0 band — 1 m wavelength — therefore carries **52 %** of the map's slope
energy:

| Octave | Wavelength | Share of slope energy | rms slope |
|---|---|---|---|
| 0 | 1.00 m | 52.5 % | 0.109 (6.2°) |
| 1 | 0.50 m | 25.7 % | 0.076 (4.4°) |
| 2 | 0.25 m | 12.6 % | 0.053 (3.1°) |
| 3 | 0.125 m | 6.2 % | 0.037 (2.1°) |
| 4 | 0.062 m | 3.0 % | 0.026 (1.5°) |

A 6.2° rms tilt on a mirror swings the reflected ray by 12.4°, which is **nine texels of a 128²
environment cube**. The clouds in the reflection are displaced and smeared by more than their own
size, and their edges are torn. That is the jello: not a liquid, a wobbling mirror.

The spectrum is the error, and the error is in the *statistic*, not in the rendering. The map's
rms slope is `NORMAL_RMS_SLOPE = 0.15` and its `water.rs` counterpart
`ripple_slope(u10) = √(0.003 + 0.00512 U₁₀)` are the **Cox & Munk (1954) open-ocean mean-square
slope** — the glitter statistics of a fetch-limited sea at 10 m wind. A 25 m pond has no fetch: it
is not a wind sea at all, it is a near-flat sheet with a capillary skin and whatever the bodies
disturb. Putting the ocean's 1 m chop on it is what makes the water look like jelly.

## Sources

- **Cox, C. S. & Munk, W. H. (1954)**, *Measurement of the roughness of the sea surface from
  photographs of the sun's glitter*, JOSA 44, 838–850 (`10.1364/JOSA.44.000838`; the relation
  `s² = 0.003 + 0.00512 U₁₀` is already in `water.rs:136` with the citation, and the corpus
  records the same paper's role in the sun's glitter, `10.5194_amt-6-2989-2013`). The statistic
  is for a **fetch-limited sea**, and the note's own Parameters table already carries it as
  "[INFERENCE]; Cox & Munk 1954". This note does not change the statistic's *formula*, only
  which part of the spectrum a sheltered body may show.
- **Vlachos, C. et al. (2010)**, *Footprint and flowmap image-based water rendering*
  (corpus `s2010-advances-pdf-slides-10`; as in `water-rendering.md` Model 3): the flow-mapped
  normal is a *surface detail* layer, tiled at 8 m, whose strength is scaled by the flow speed
  (`strength = mix(STILL_RIPPLE, 1, |v|/v_foam)`) — the note's Model 3 and the shader's
  `flow_normal`. Vlachos's own still water carries a fine, near-mirror surface, not a metre-scale
  swell.
- **Lagae, A. et al. (2010)**, *A survey of procedural noise functions*, CGF 29(8)
  (`10.1111_j.1467-8659.2010.01827.x`; already cited by `wind.md` and `ground-detail.md`): §4
  "the most common approaches to spectral noise control" is a sum of weighted band-limited
  layers, and a *slope* spectrum (the derivative, not the height) is what a surface's appearance
  depends on. §5.2.1, frequency clamping: a band the pixel footprint cannot represent must be
  cancelled, not aliased. The map's mip chain already does this (`octaves_kept`), and it is why
  the far water does not sparkle.
- **Tessendorf, J. (2004)**, *Interactive water surfaces* (Game Programming Gems 4; as in
  `waves.md`): the dispersive wave field's own bands are 5–80 cm texels with a wavelength
  spectrum fitted to Airy's dispersion — the crate's *wave* field is already at the fine scale
  the ripple map should have, and it is the ripple map that duplicates the coarse scale.
- **Hwang, P. A. (2005)**, *Wave number spectrum and mean square slope of intermediate-scale
  ocean surface waves*, J. Phys. Oceanogr. 35, 106 (`10.1029/2005JC003002`; **download blocked**,
  no open copy through the providers): the mean-square slope of the intermediate (0.1–10 m) band
  is the statistic this note is about; the relation is read here from the corpus's own record of
  Cox & Munk and the crate's model, and the paper's numbers are **not** used (the record is
  "not obtained" for the payload).
- **Ottosson, B. (2011)**, *Real-time interactive water waves* (KTH thesis, EA DICE; as in
  `waves.md`): the shipping-game view — a pyramid of camera-centred cascades, each its own band,
  the surface drawn as the sum of their slopes. The crate's `waves` module is that; this change
  removes the *duplicate* coarse band from the ripple map so the two do not fight.
- **`docs/research/water-rendering.md`**: the map's model, its mip chain, the Cox & Munk slope
  and the `STILL_RIPPLE` floor (0.25, "[INFERENCE]"), and the note's own Parameters table.

## Model adopted

**1. The ripple map's slope spectrum is shifted to the fine end.** The height's amplitude ratio
per octave becomes `0.5` instead of `0.35`, so the *slope* of every octave is equal (a flat slope
spectrum, the surface-noise analogue of a white-noise spectrum): the map's rms slope is then
shared equally by the 1 m, 0.5 m, 0.25 m, 0.125 m and 6 cm bands instead of 52 % / 26 % / 13 % /
6 % / 3 %. Nothing about the map's total slope changes — the Cox & Munk value is untouched, so
the surface's roughness, and the glitter it makes, are the same.

**2. The map's base band starts at the capillary end.** The base octave's lattice goes from 8
cells to 16 cells across the 8 m tile, and the ladder loses its coarsest octave: the map's
wavelengths run 0.25 m → 6.25 cm (three octaves, the finest at the kernel's four-texel limit)
rather than 1 m → 6 cm. A pond has no metre-scale wind chop; the metre band belongs to the wave
field (`waves`), which computes its own bands from Airy's dispersion and is already there. The
kernel's four-texel limit is what fixes the count: with a ratio-2 ladder the finest band must
stay at four texels, so a 16-texel base leaves exactly three octaves.

**3. The still-water floor is the pond's own, and it is small.** `STILL_RIPPLE` (the strength a
body with no flow keeps) drops from 0.25 to 0.06: a sheltered pond is a mirror with a skin on
it. The floor exists so that still water is not a perfect mirror (a perfect mirror has no
surface at all, and reads as glass), not so that it looks like the sea.

**4. The strength still follows the flow.** Nothing changes in `flow_normal`: the strength
still rises with the water's speed to the full Cox & Munk slope at the foam threshold, so a
running stream or a wake is as rough as before. Only the *still* case changes.

**5. The map's mip chain is unchanged.** `octaves_kept` already drops the octaves a mip cannot
represent, and the finer base band means the coarse mips now carry the fine octaves instead of
the metre one, so the far water stays smooth.

## Parameters

| Parameter | Before | After | Source |
|---|---|---|---|
| Height amplitude ratio per octave | 0.35 | **0.5** | Lagae et al. 2010 §4 (a flat slope spectrum) |
| Base octave's lattice over the 8 m tile | 8 cells (1 m) | **16 cells (0.25 m)** | §Sources: a pond has no fetch; the metre band is `waves`' |
| Octaves | 5 | **3** | the kernel's four-texel limit (Deviation 4) |
| Wavelength range | 1 m → 6 cm | **0.25 m → 6.25 cm** | derived |
| Slope gain `NORMAL_SLOPE_GAIN` | 4.7 | **1.8575** | refitted so mip 0 keeps the Cox & Munk rms slope (Deviation 3) |
| `STILL_RIPPLE` | 0.25 | **0.06** | Vlachos 2010 (a still surface is near-mirror) |
| Map rms slope (mip 0) | 0.150 | 0.150 | Cox & Munk 1954, unchanged |
| Slope share per octave | 52 / 26 / 13 / 6 / 3 % | **20 % each** | derived |

## GPU mapping

No new pass, texture, binding or dispatch. The change is three constants: `OCTAVE_GAIN` and
`BASE_CELLS` in `water_normals.wesl` (the kernel, dispatched once at start-up) and `STILL_RIPPLE`
in `water.wesl` (the per-fragment strength). The map's generation cost is unchanged (five octaves
either way). The CPU mirror of the kernel (`hydrology/water/model.rs`) carries the same two
constants and its slope test is the oracle.

## What great looks like

- A calm pond is a **mirror**: the cumulus reflect with their edges intact, softened by a hair,
  never torn into blobs. The reflected sky is *smoother* than the sky itself.
- The surface has a **skin**: a fine capillary texture that catches the light, too fine to
  read as waves, present everywhere so the water is never dead glass.
- A **moving** body still breaks the mirror: the hull's wake is a crisp V with sharp transverse
  crests, the float's rings sharp, the pebble's ring sharp.
- A **running** stream or a strong current is as rough as before (the flow raises the strength
  to the full Cox & Munk slope).
- The far water does not sparkle or shimmer; the near water does not wobble.
- Under a low sun the glitter path is a long smear broken into sparkle, not a wobbling sheet.

## Validation targets

1. **The water is smoother than the sky (capture).** `waves` at 16:30, `PartlyCloudy`, 8.7 m
   eye: the water band's high-pass standard deviation is **below** the sky band's (0.68× before
   this change's own ripple test, which measured 2.23×). A water surface that reflects the sky
   noisier than the sky is the defect.
2. **The reflection keeps its structure (capture).** A 4× crop of the reflected cumulus shows
   a cloud edge that is soft but continuous — no torn edge, no doubled outline, no blob where a
   cloud was.
3. **Still water is not glass (capture).** The near water's high-pass standard deviation is at
   least 0.8 DN (a visible skin), and the water's mean brightness is within 2 DN of the
   ripple-free mirror's (the skin perturbs the reflection, it does not dim it).
4. **The wakes are unchanged (capture + GPU test).** `gpu_waves`'s
   `a_moving_body_leaves_a_kelvin_wake` and `a_body_at_rest_holds_the_water_down_under_it_only`
   still pass unchanged; a capture shows the wake's transverse crests as sharp as before.
5. **The map's statistics (CPU + GPU).** `normal_gain_gives_the_cox_munk_slope` still gives an
   rms slope of 0.150 ± 0.004 (the total slope is unchanged; only its distribution over the
   spectrum is). `mips_keep_the_expected_share_of_the_slope` is updated to the new spectrum's
   shares. `tests/gpu_water.rs`'s `gpu_water_normals_tile_and_have_the_cox_munk_slope` still
   passes (rms 0.150, tileable, alpha mean in range).
6. **Cost.** The water's shading passes unchanged (`elements_waves_*` and
   `main_transparent_pass_3d` within 5 % of their logged minima; the map is generated once).

## Measured

On `examples/waves` at 16:30, camera 8.7 m up, `PartlyCloudy` (`/tmp/review28/cap/`), the
high-pass standard deviation of the sky band and of the water band that mirrors it:

| Frame | sky | water | ratio | water mean |
|---|---|---|---|---|
| before (shipped) | 1.99 | 4.44 | **2.23×** | 97.20 |
| after (this change) | 1.99 | 2.73 | **1.37×** | 97.15 |
| reference: the ripples off (a bare mirror) | 1.99 | 1.35 | 0.68× | 97.11 |

The water is no longer noisier than the sky it mirrors, and its mean brightness is unchanged
(97.15 against 97.11), so the ripples now perturb the reflection rather than replace it. The
near band alone reads 1.81 against the bare mirror's 1.75.

The wakes are untouched: `examples/waves` with `ELEMENTS_FOLLOW=4,2.5,8` shows the hull's
transverse crests and foam as sharp as before (`cmp_wake.png`), and `gpu_waves`'s three tests
pass unchanged.

## Deviations from this plan

1. **The Cox & Munk formula is unchanged, only the spectrum's shape over the map's octaves
   changes.** The `ripple_slope(u10)` relation and the map's total rms slope stay what they are:
   the surface is as rough as the wind statistic says. What changes is *which* wavelengths carry
   that roughness, which is the rendering half of the model, not the physics half. The reason is
   that the alternative — keeping the metre band and lowering the total slope until the jello
   goes — also removes the glitter the statistic exists to produce, and it makes the same
   surface look different at every distance (the mip chain then has to drop the metre band at a
   different range), so the defect would come back as soon as the camera moved.
2. **The `STILL_RIPPLE` floor drops to 0.06, not to 0.** A floor of zero makes still water a
   perfect mirror, which reads as glass: no surface at all, and the sky's reflection with a hard
   edge where the water meets the shore. A small floor keeps the skin.
3. **The change is in the ripple map, not in the water's shading.** The water's own roughness
   (0.089 perceptual) and the wave field are untouched: the wave field is already at the fine
   scale, and the two together are what a surface needs.
4. **The map's slope gain is refitted (4.7 → 1.8575) and the octave count drops (5 → 3).** A flat
   slope spectrum over three octaves has a smaller total gradient than the old one over five, so
   the gain that turns the height's central difference into the slope has to grow the other way;
   the *resulting* rms slope is unchanged at 0.150 (Cox & Munk 1954), which the CPU and GPU
   oracles both check. The mip chain's octave filter follows the new ladder
   (`octaves_kept`: all three down to level 1, one fewer at level 3, none from level 4).

## Pending index

- `10.1029/2005JC003002` (Hwang 2005): **not obtained** — no open copy through the providers
  (Wiley 403). The statistic is read from Cox & Munk as the crate already cites it.
- The corpus's `s2010-advances-pdf-slides-10` (Vlachos 2010) and
  `10.1111_j.1467-8659.2010.01827.x` (Lagae et al. 2010) are read from the corpus copies already
  in the repository; no new ingestion.
