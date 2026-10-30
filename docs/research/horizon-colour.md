# The horizon is the sky's colour, not the fog's

Every frame whose ground does not reach the horizon draws a **flat, perfectly uniform slab** where
the ground ends, and the slab is the camera's `DistanceFog` colour. That is right in principle — a
below-horizon ray past a finite terrain is air, and air converges on the fog colour — but the colour
is wrong: it is a fixed hue, `FOG_COLOUR = (0.75, 0.80, 0.85)`, times the scene's *horizontal*
illuminance, while the sky at the horizon is Rayleigh- and Mie-extinguished air whose radiance and
hue follow the sun's altitude and the azimuth. The two diverge exactly when the sun is low.

Measured on `main` at `1cf5b4d`, mac_air, `terrain_tour`, 400 m above a plain looking level (so the
slab fills the lower half), 2560 × 1406, the slab sampled over 1 200 × 420 px and the sky 4 px above
its edge:

| hour | sun altitude | sky just above | slab | Δ luminance | Δ max channel |
|---|---|---|---|---|---|
| 06:00 | (below the horizon) | 228, 230, 224 | 207, 211, 214 | −16 | 20 |
| 09:00 | 30° | 214, 223, 225 | 224, 226, 229 | +6 | 10 |
| 12:00 | 68° | 201, 212, 218 | 224, 226, 229 | **+17** | **23** |
| 17:00 | 25° | 228, 234, 232 | 224, 226, 229 | −4 | 7 |
| 18:30 | 4° | 231, 227, 215 | 193, 196, 200 | **−28** | **38** |
| 20:00 | −2.2° | 150, 128, 136 | 127, 114, 109 | **−21** | **26** |

at eye level on the mountains, where the ground stops 500 m out and the slab is 10× wider:

| hour | sky above | slab | Δ luminance | Δ max channel |
|---|---|---|---|---|
| 05:30 | 227, 225, 213 | 149, 147, 125 | −81 | 88 |
| 12:00 | 224, 227, 229 | 159, 171, 154 | −65 | 75 |
| 19:00 | 227, 210, 186 | **97, 96, 83** | **−116** | **130** |

The slab's own colour has a per-channel standard deviation of **0.0–0.5 DN** over thousands of
pixels: it is one value, not land, and its edges are knife-sharp (a 64 DN step at its top, 41 DN at
its bottom against the terrain). At 2× (`/tmp/i61/crops/pc_zoom.png`) it reads as a pale painted sea
under a blue sky, and at dusk as a dark grey one under a peach sky.

**This is a documented validation target that fails.** `fog.md` Validation 12: "the distance-fog
colour of distant terrain at the horizon must show no visible dark band against the sky just above it
at 19:30 and no bright band at 23:00. A band means the exposure rule needs the sky-to-illuminance
ratio **or the atmosphere's horizon colour**." The crate's own GPU test
(`the_horizon_sky_is_the_fog_colour_under_a_deck`) passes only because it runs under `Snow`, where a
full deck's colour happens to sit near the fog hue.

## Sources

- **Wenzel, C. (2006)**, *Real-time atmospheric effects in games*, SIGGRAPH *Advances* ch. 6
  (corpus `s2006-advances-materials-11`, §6.4–6.5, ll. 80–98). Used, and it is the model this note
  adopts: "To make the two models cooperate, we need a way to determine a fog color that can be used
  with accompanied volumetric fog value to blend scene geometry nicely into the sky." His method:
  "all samples taken during the sky texture update for directions resembling the horizon are
  additionally averaged and stored for later use"; the fog then blends the framebuffer against "the
  average of the horizon samples calculated on the CPU". He also reports the simplification this note
  takes: "using all of the horizon samples to produce a better matching fog color seems tempting but
  it was found that the difference to the average of all horizon samples is barely noticeable and
  doesn't justify the additional overhead in shading computations." **The fog colour must be measured
  from the sky itself, at the horizon, and one colour for the whole horizon is enough.**
- **Decima engine, height fog (Kohei of Kojima Productions)**, SIGGRAPH 2017 *Advances*
  (corpus `s2017-advances-pdf-slides-2`, §"Height fog", ll. 508–760). Used: the two-model split this
  crate already has (a precomputed atmosphere for the far field, an analytic height fog for the near
  field) needs *one* light source for both — `L = L_background·α + (L_sun + L_amb)(1 − α)` with
  `L_sun` and `L_amb` "supplied … from the precomputed atmospheric scattering model", not invented.
  The fog's own ambient is `L_amb = (Σβ_si / Σβ_ti) · L_sky^LUT`: the sky's light at the fog's own
  height, which is what makes a fogged surface "blend nicely into the sky" (Wenzel's phrase) rather
  than into a colour of its own.
- **Hillaire, S. (2020)**, *A scalable and production ready sky and atmosphere rendering technique*,
  CGF 39(4) (corpus `10.1111_cgf.14050`; the model Bevy's `Atmosphere` implements). Used: the model
  produces the sky's in-scattering per view direction, which is what the horizon colour must be read
  from — not a constant hue.
- **Bevy 0.20 source**, the exact path a sky pixel takes (`~/.cargo/registry/src/index.crates.io-*/`):
  `bevy_pbr-0.20.0/src/atmosphere/render_sky.wesl:44-70` (a sky pixel is the sky-view LUT sampled at
  its direction, `sample_sky_view_lut`, plus the sun disc; the LUT is computed by
  `sky_view_lut.wesl:20-37` with `raymarch_atmosphere(…, ground: true)`), and
  `atmosphere/functions.wesl:129-134` (`sample_sky_view_lut(r, ray_dir_as)` maps a direction in
  *atmosphere space* through `sky_view_lut_r_mu_azimuth_to_uv`). The LUT is `Rgba16Float`, its
  in-scattering in the same units the crate's cloud pass already divides by exposure.
- **The crate's own `fog.md`** Validation 12 and Deviations 3–5: the fog colour is
  `hue × (E/π) × exposure` *because* `DistanceFog::color` is added after exposure
  (`bevy_pbr-0.20.0/src/render/fog.wesl:64-78`); the note names "the atmosphere's horizon colour" as
  the remedy this iteration takes.
- **`clouds.md` Deviations 28–29** (already in the crate): the below-horizon sky past a finite
  ground is `scene · T + fog · (1 − T)`, the same as every mesh — and the crate already carries the
  camera's fog colour and extinction into the cloud pass as `CloudParams::haze`. The slab this note
  fixes is the `exp(−σ · max(ground, 32 km)) → 0` limit of exactly that expression, i.e.
  `CloudParams::haze.xyz`.

## Model adopted

**1. The fog colour becomes the sky's horizon colour.** `DistanceFog::color` is set, per camera per
frame, to the mean radiance of Bevy's *own* sky over a band of directions along the horizon — the
same quantity Wenzel averages out of his sky pass, and the same one Decima's `L_amb` stands for. No
new pass and no new colour model: the crate asks the renderer what its sky is.

**2. It is measured on the GPU from the sky-view LUT, once per frame, into a small readback.** The
LUT is already there and is the exact function `render_sky.wesl` samples for a sky pixel. The crate
adds one tiny compute kernel that samples it over `N` azimuths at a small elevation and reduces to
one RGB value, then reads the result back. `N` = 32 azimuths at +0.5° elevation (Wenzel's
"directions resembling the horizon"), which is under a thousand LUT fetches — noise against any
frame.

**3. The measured colour replaces the analytic `hue × E/π × exposure`.** The analytic form stays as
the value used before the first readback, so a camera's first frame is no worse than today, and as
the fallback when the cloud pass is disabled (a game that took the atmosphere away has no sky to
measure and keeps the analytic hue, exactly as it keeps its own clear colour).

**4. It is `DistanceFog::color`, so it fixes every consumer at once.** Every mesh's far field, the
cloud pass's `ground_haze` slab, the composite's `sky_through_clouds`, and the sky probe's composed
cube read this one value (`clouds/mod.rs` `haze()`). The slab and the sky then agree by construction
rather than by coincidence.

**5. Only the colour changes; the extinction does not.** Koschmieder's `3.912/V` stays (Deviation 4
of `fog.md`: the extinction is a property of the medium, not of the light). This keeps the visibility
contract, every existing transmittance test, and the energy conservation of Deviation 3.

## Parameters

| Quantity | Symbol | Value | Unit | Source |
|---|---|---|---|---|
| Horizon band elevation | `HORIZON_BAND_DEG` | +0.5 | ° | Wenzel's "directions resembling the horizon"; above Bevy's 0.5° sky-LUT row spacing so no row is the ground's |
| Azimuths averaged | `HORIZON_AZIMUTHS` | 32 | — | Wenzel: the mean of the horizon samples is enough |
| Readback interval | — | `ElementsReadback::interval` (250 ms) | s | the crate's existing throttle; a sky does not change in 4 frames |
| Analytic fallback | — | `fog_hue(h) × (E<sub>scene</sub>/π) × exposure` | linear | today's `fog::update_distance_fog` |

## GPU mapping

One compute kernel, `elements_fog_horizon`, dispatched once per frame at 1 workgroup of 32 threads:
thread `i` takes azimuth `2π i / 32`, builds the atmosphere-space direction for elevation
`HORIZON_BAND_DEG`, and samples Bevy's `sky_view_lut` with `sample_sky_view_lut`'s own mapping; a
workgroup reduction writes the mean to a 16-byte `ShaderBuffer`. The main world reads it back through
the crate's existing `Readback` path (`src/gpu/readback.rs`) and writes `DistanceFog::color` from it.
The kernel needs `AtmosphereTextures` (the sky-view LUT) and the view uniform for the camera's
position; it is skipped for a view with no atmosphere, which keeps the analytic hue.

Storage-buffer budget: unchanged (one 16-byte buffer, read-only in the fog pass; the crate's
materials are untouched).

## What great looks like

- Where the ground ends, the sky continues: **no edge at all** at the horizon in a clear frame, at
  any hour. The slab is not a colour that happens to be near the sky's; it is the sky's.
- Under a low sun the band is warm like the sky above it (peach under peach at 18:30), and at dusk it
  is dark like the dusk sky, not a grey one.
- Under a full deck the band is the deck's own horizon colour, so a whiteout stays white.
- The frame's lower half never reads as a painted sea: at 2× a viewer cannot find where the terrain
  stops.
- The fog on *distant terrain* (a ridge 1.5 km out) sits in the same hue as the sky behind it, which
  is what `fog.md` Deviation 3's exposure rule was approximating.

## Validation targets

1. **The slab against the sky at 12 h, eye level** (`terrain_tour`, mountains seed 7, `ELEMENTS_AGL=1.7`):
   the mean absolute difference between the slab and the sky 4 px above its edge is **≤ 6 DN** and
   the max channel difference **≤ 10 DN** (today: 65 and 75).
2. **The slab at 19 h** (same scene): ≤ 10 DN mean, ≤ 16 DN max (today: 116 and 130), and the slab
   keeps the sky's warmth: `R − B` within 0.6 of the sky's.
3. **The slab under `Fog`**: ≤ 12 DN (today: 97), i.e. a 300 m visibility still agrees at the
   horizon.
4. **The slab at 06:00, 09:00, 17:00, 18:30 and 20:00** (400 m up, level): ≤ 12 DN mean at every
   hour, and the sign of the residual changes no more than it does today at 17:00.
5. **Noon and the 400 m cumulus frame are otherwise unchanged**: the slab's own value moves (it must),
   the terrain, the clouds and the sky above the band differ by ≤ 2 % of pixels by more than 2 DN.
6. **The mean is a sky, not a constant**: with the sun's light off (night) the colour's luminance is
   under 5 % of the noon value; the colour's hue at 18:30 differs from noon's by ≥ 0.1 in
   `(R − B)/luminance`.
7. **CPU unit tests**: the direction set is uniform in azimuth, every elevation is the same, and the
   mean of a constant sky is that constant; `fog_hue`'s analytic value is still exactly what it was
   (the fallback path).
8. **Cost**: the kernel is under 5 µs on the crate's bench host; the pass adds no bind group to any
   material.

## Deviations from this plan

1. **The readback and the new compute kernel were not built.** The plan's Models 1-2 measured the
   horizon's mean on the GPU over 32 azimuths and wrote it into `DistanceFog::color` from the main
   world. The build instead reads the cube the crate already composes (`sky_probe`, the cloud pass's
   environment cube) *inside* `ground_haze`, at the ray's own azimuth. Three reasons, all measured:
   - **Per azimuth is strictly better than one mean.** Wenzel reports the mean is enough because his
     sky is a low-resolution gradient; this crate's dusk horizon runs from a peach sunward to a cool
     anti-solar side, and one mean of that is a band of its own. The azimuth sweep
     (`/tmp/i62/after/az_xp`, `az_xn`, `az_zn`, `az_zp`, 19 h, 400 m up) shows the slab following
     the sky's own hue round the compass.
   - **No new pass, no readback, no latency.** `DistanceFog::color` from a readback lags the exposure
     by a frame and needs a kernel plus a storage buffer; the cube fetch is already bound to the
     shader that draws the slab.
   - **The slab is the only thing this change moves.** `DistanceFog::color` still comes from
     `fog::update_distance_fog`'s analytic hue, so every mesh's far field, and `fog.md` Validation
     12's "distant terrain at the horizon", are **unchanged by this pass** and still open. What is
     fixed is the flat slab the cloud pass fills where the ground ends — the 130 DN band the finding
     measured. Reading the fog colour itself from the sky is the next item, not this one.
2. **The taps are two, not 32, and the horizon value is extrapolated to zero elevation.** Sampling
   the cube 0.5° up the sky (the plan's band, chosen for Bevy's sky-LUT row spacing) is a steep
   gradient in twilight: at 20 h the slab's lip read 127.8 DN against a sky of 117.2. The shipped
   form takes two taps, at 0.5° and 1.0° (`HORIZON_SAMPLE_BAND` = 0.0087 rad and twice it), and
   extrapolates linearly to the horizon as `2·r(h) − r(2h)`. Measured on `big` against the sky 4-12
   rows above the terrain's edge: 2.2 DN at 20 h, 4.3 DN at 19.5 h, and 0.7-2.0 DN at every hour from
   dawn to 22 h (`/tmp/i63/{before,after}/hi_h*`).
   A wider pair is *worse*, not better: measured in this pass by building the same tree with
   `HORIZON_SAMPLE_BAND` = 0.0175 rad (taps at 1° and 2°), the 20 h lip reads **3.2 DN** against the
   shipped pair's **2.2 DN**, and 19.5 h reads **6.9 DN** against **4.3 DN**; noon is 2.5 against
   2.0 (`/tmp/i63/wide/` against `/tmp/i63/after/`). The sky's gradient steepens toward the horizon,
   so a linear extrapolation taken from too high up overshoots. The cube's own resolution sets the
   floor — a 128² face spans 0.7° per texel, so a tap closer than that bilinearly averages the cube's
   ground in and the slab goes dark (measured at 0.1°: 12.9 against a sky of 14.2).
3. **The night fill is not added.** The environment cube carries `params.atmo.w · NIGHT_SKY_COLOUR`
   so the lighting cube is not black at night; the screen's sky does not carry it. Adding it drew a
   grey sea (9.6, 12.4, 15.5 DN) under a black sky (2, 1, 1). The slab reads the cube without it.
4. **Parameter table.** `HORIZON_SAMPLE_BAND` is 0.0087 rad (0.5°), with the second tap at twice it,
   so `HORIZON_BAND_DEG` = 0.5 rather than the table's +0.5 in the plan's *sky-LUT* mapping;
   `HORIZON_AZIMUTHS` is 2 (not 32, and they are elevations, not azimuths), and the readback
   interval and the analytic fallback are not used: the slab has no fallback path, because a ray
   that meets the planet always has the cube.

## Pending index

`distill` is down (192.168.1.110:7434, 502 — `big`, which hosts it, does not answer `ping`), so
`distill_index` is skipped (`docs/research/corpus.md`). The corpus was read from the Garage bucket
through signed `curl` (`~/.cache/bevy_elements_corpus/markdown`). Read but **not indexed**:
`s2006-advances-materials-11`, `s2017-advances-pdf-slides-2`,
`book_real_time_rendering_4th_akenine`, `s2021-advances-pdf-66-mb`.
