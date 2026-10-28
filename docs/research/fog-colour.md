# The distance fog's colour is the sky's horizon, not a constant hue

Iteration 86 found why a still sea (and any distant ground) shows a 1-px line at the horizon whose sign flips with the hour (+13 levels at noon, −39 at 19:30): `fog::update_distance_fog` colours `DistanceFog` with `FOG_COLOUR` (0.75, 0.80, 0.85) × the scene's illuminance × exposure, a constant hue, while Bevy's sky at the horizon follows the sun. At 13-18 km the clear-air visibility (40 km) has already put 70-80 % of a pixel into the fog colour, so the mismatch is the horizon line. `horizon-colour.md` fixed the slab past a finite ground (the cloud pass reads the sky cube per azimuth) and named this as the open half: "Reading the fog colour itself from the sky is the next item". `fog.md` Validation 12 ("no dark band against the sky at 19:30, no bright band at 23:00; the remedy is the atmosphere's horizon colour") has failed since it was written.

## Sources

- **Wenzel, C. (2006)**, *Real-time atmospheric effects in games*, SIGGRAPH *Advances* (corpus `s2006-advances-materials-11`, §6.4-6.5): the fog colour is "the average of the horizon samples" taken from the sky pass; one colour for the horizon is enough ("the difference to the average of all horizon samples is barely noticeable"). Adopted, with the average taken over the azimuths the camera faces rather than all round.
- **Decima height fog, SIGGRAPH 2017 *Advances*** (corpus `s2017-advances-pdf-slides-2`, "Height fog"): fog and sky must share one light source, `L = L_bg·α + (L_sun + L_amb)(1 − α)` with the lights taken from the precomputed atmosphere, not invented. A fog coloured by the sky's own radiance is that identity at the horizon.
- **Hillaire (2020)**, *A scalable and production ready sky and atmosphere rendering technique*, CGF 39(4) (corpus `10.1111_cgf.14050`): the sky's in-scattering per direction is what a distant surface converges on.
- **`docs/research/horizon-colour.md`** (this crate): the cloud pass's `horizon_sky` (two taps at 0.5° and 1.0°, extrapolated to zero elevation, scaled by the sky gain, without the night fill) is the measured, tuned way to read the horizon from the sky cube; this note reuses it unchanged (the 2 DN-level slab agreement in its Deviation 2).
- **`docs/research/fog.md`** Deviations 3-5 and Validation 12.
- **Iteration 86 log** (`docs/loop-log.md`): the horizon row's sign and size at 12, 19.5, 21 and 0 h, the cause by elimination, and the rejected attempts (skirt fade; dropping clear-air extinction).

## What great looks like

- Where distant ground, the sea or its skirt meets the sky there is no line at any hour: the far field fades into the sky's own horizon colour, warm under a low sun on the sunward side, cool away from it.
- At dusk a distant ridge is peach against a peach sky, not a grey wedge; at night it sinks into the dark sky instead of standing in a lit-grey fog.
- Under a deck, in fog and in dust the fog stays the weather's own grey, brown or white: a blue horizon colour never leaks into a whiteout.
- Turning the camera moves the fog's tint with the sky behind the view, with no pop at the 4 Hz readback.

## Model adopted

1. **A 32-azimuth ring of the sky's horizon radiance is computed on the GPU every frame** by a third entry point of the cloud march shader (`cloud_horizon_ring`, one workgroup of 32 threads, appended to the sky-probe pass), each thread running the existing `horizon_sky` for its azimuth. It uses the probe pass's bind group plus one 512-byte storage buffer. No new pass, no new texture, no new bind group on any material.
2. **It is read back through the crate's throttled readback** (`ElementsReadback`, 4 Hz) into `EnvironmentSnapshot::horizon`. The ring is radiance, independent of the camera's heading and its exposure, so it needs no per-frame update.
3. **Each frame the CPU averages it toward the camera's heading** (weights cos² of the angle to the heading within ±90°, a plain mean for a camera looking straight up or down) and multiplies by the camera's exposure: that is `DistanceFog::color`. The heading changes at frame rate; only the sky's slow drift waits for the readback.
4. **The sky's share is bounded by the weather.** `horizon_share = (1 − air)·(1 − cloud_cover)`, with `air = 1 − exp(−σ_excess·32 km)` the same law the sky's weather air uses (`sky_air_extinction`, `clouds.md` Deviation 29). The fog colour is `lerp(analytic, sky, horizon_share)`: a clear sky with the clear visibility takes the sky, a full deck or a thick fog keeps the analytic colour (the deck's illuminance-derived grey), partial cover is in between. The analytic colour is also the value before the first readback and for a camera with no probe.
5. **Only the colour changes**: the extinction (`3.912/V`), the cloud `haze` plumbing, the visibility contract and every transmittance test are untouched. The slab's `ground_haze` already mixes `horizon_sky` with `haze`; with the fog colour now the horizon radiance the two agree by construction.

## Parameters

| Quantity | Symbol | Value | Unit | Source |
|---|---|---|---|---|
| Azimuths in the ring | `HORIZON_RING` | 32 | - | Wenzel: the mean of the horizon samples is enough; 11.25° is finer than the sky's azimuth structure |
| Heading weight | cos² within ±90° | - | - | art direction: the view sees its own half of the horizon (no source; Deviation 1) |
| Sky share | `(1 − air)(1 − cover)` | 0..1 | - | the sky's own weather-air law, `clouds.md` Deviation 29 |
| Ring extrapolation | `HORIZON_SAMPLE_BAND` | 0.5° and 1° | - | `horizon-colour.md` Deviation 2 |
| Readback interval | `ElementsReadback::interval` | 250 ms | s | the crate's throttle |

## GPU mapping

`cloud_horizon_ring` is `@compute @workgroup_size(32, 1, 1)` in `march.wesl`, bound to `march.wesl`'s binding 15 (`array<vec4<f32>, 32>`, read_write, `w` = 1 once written). It is dispatched once after `cloud_sky_probe` in the same compute pass, with the probe's bind group, so it runs only for a view that has a probe. 32 threads, 64 cube fetches: noise against the probe's 6 × 128² texels. Storage buffers per stage are unchanged for every material (the buffer is in the cloud pass's own bind group, a compute stage with 1 storage buffer).

## Validation targets

1. **The horizon row of the still-sea fixture** (`gpu_still_water`, eye 30 m up, 8° down, centre column; iteration 86's `/tmp/i86` method) differs from the sky row above it by ≤ 8 DN at 12, 19.5 and 21 h (iteration 86: +13, −39, grey-over-red).
2. **The same row at midnight**: the sea below the horizon is within 15 DN of the sky (iteration 86: a grey slab 40-60 DN above a black sky).
3. **Fog (`WeatherPreset::Fog`) and a full deck (`Snow`)** are within 6 DN of what they were: the share is 0 there. `gpu_a_fogged_sea_is_the_fogs_colour_not_the_waters` still passes.
4. **A 360° heading sweep** at 19.5 h: the fog colour's R/B ratio is larger toward the sun than away from it.
5. **CPU**: the ring's mean of a constant sky is that constant; the heading average of a one-sided ring is dominated by the side the camera faces; a camera looking straight down gets the plain mean; `horizon_share` is 1 for `Clear`, 0 for `Fog` and for cover 1.
6. **Cost**: one workgroup of 32 threads per frame; no change to any material's bindings.

## Deviations from this plan

1. **Heading weighting** is art direction without a source (Wenzel reports one colour for all the horizon samples); recorded as a deliberate choice because this crate's dusk horizon runs from peach sunward to cool anti-solar (`horizon-colour.md` Deviation 1 measured the same).
2. **The ring is read back, not sampled in the fog shader.** Bevy's mesh fog takes one `DistanceFog::color` per camera from the main world (`bevy_pbr-0.20.0/src/render/fog.wesl`), so a per-pixel sky colour would need a new mesh-fog path; the average toward the heading is Wenzel's own design.

## Pending index

`distill` rejects new documents with an ONNX out-of-memory error (2026-10-16); no new documents were added this pass (every source is a corpus stem already read in iterations 61-63).
