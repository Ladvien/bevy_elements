# The sun under broken cloud: local shadows, a steady sky, and a hidden disc

Two defects of one cause, found by the iteration-15 review and reproduced on `main` (`7dd1ef0`) by iteration 16:

1. **Standing in one cloud's shadow takes the whole sky with it.** The sun's and the moon's `DirectionalLight`s carry `CloudShadows::camera_transmittance`, the mean of the cloud shadow map over the 9 × 9 texels (70 m) around the camera (`clouds.md` Deviation 24). Bevy's atmosphere is lit by every directional light's `color · illuminance` (`bevy_pbr-0.20.0/src/atmosphere/functions.wesl:200, 232, 475`), so when a lone cumulus shades the camera the sky dome, the aerial perspective over every distant hill, the environment map (reflections, the water) and the volumetric fog all dim with it, and every `StandardMaterial` surface in the landscape loses its sun. The composite's `sky_gain = total / T̄` (`celestial.md` Deviation 18) restores the dome only to `0.25 + 0.75 T̄`, and when `T̄` reaches 0 the light is literally 0 and nothing is left to restore: `waves` at 17.5 h on 21 June under `Clear` (cover 0.05) shows a black sky over a grey horizon band at frames 1200 and 1800 (`/tmp/i16/before/long.png`), and the ground, the far shore and the water go dull at the same time.
2. **The sun disc shines through a thick cumulus.** The cloud march stops once the view transmittance falls below `MIN_TRANSMITTANCE = 0.01` and composes the sky behind with that last 1 %. The sky is ~10⁴ cd/m², but Bevy's sun disc is ~1.6 × 10⁹ cd/m² (Hillaire 2016 §4.1.2: 120 000 lx over 7.1 × 10⁻⁵ sr), so 1 % of it is still a saturated white dot in the middle of the cloud's dark base: `day_night` at 17.5 h (`/tmp/i16/before/dn_dot.png`).

## Sources

- **Hillaire 2016**, *Physically Based Sky, Atmosphere and Cloud Rendering in Frostbite* (SIGGRAPH course notes; local text `/tmp/i16/pdf/hillaire2016.txt`). §5.9: "volumetric cloud shadows are baked into a 2D texture storing transmittance and projected onto the world. It is applied on all opaque, transparent surfaces, and also sampled by our emitter system". §5.9.2, eq. 22: the aerial perspective under clouds is `L_AP · Tr_cloud + L_cloud`, with `Tr_cloud` the cloud layer's **mean transmittance over the hemisphere** seen from the camera, not the shadow at the camera's own position. §4.1.2: the sun disc's luminance from 120 000 lx and its 0.545° diameter.
- **Wenzel 2006**, *Real-time Atmospheric Effects in Games* (CryEngine 2; corpus `s2006-advances-materials-12`): "Cloud shadows are cast in a single full screen pass", per pixel from the scene depth, on the surfaces only.
- **Ubisoft Montréal 2025** (SIGGRAPH Advances; corpus `s2025-advances-materials-3`): the sun shadow of a surface "combine[d] with cloud shadows" per surface point; the sky's own light kept as its own term (sky visibility) and rescaled only by the sky's intensity.
- **Patry 2021**, *Samurai Cinema* (Ghost of Tsushima; corpus `s2021-advances-pdf-66-mb`): sky lighting as `sky luminance × sky visibility`; the sky luminance is global, the visibility local.
- **Schneider & Vos 2015**, *The Real-time Volumetric Cloudscapes of Horizon Zero Dawn* (corpus `s2015-advances-ppt-944-mb-pdf-5-mb-2`): "Once the alpha of the image reaches 1 we don't need to keep sampling so we stop the march early" — the early exit is an opaque pixel, alpha 1, not a 99 % one.
- **Mol & van Heerwaarden 2025**, *Mechanisms of surface solar irradiance variability under broken clouds*, ACP 25:4419 (`10.5194/acp-25-4419-2025`, stored and read). Under shallow cumulus "the shadows are typically dark (50 % to 80 % lower than clear-sky SSI) and the transitions to sunlit areas on the order of tens of metres"; the diffuse irradiance under broken cloud is at or above the clear-sky diffuse (forward, side and downward escape enhance it), "values below even clear-sky diffuse irradiance" only under a cumulonimbus anvil. A cloud's shadow over the observer does not dim the sky they see.
- **Mol, van Stratum, Knap & van Heerwaarden 2023**, *Reconciling observations of solar irradiance variability with cloud size distributions*, JGR Atmospheres (`10.1029/2022jd037894`, stored and read): cloud shadow sizes follow a power law, with a cloud-edge transparency length of ≈ 25 m, and shadows smaller than ≈ 750 m are common — smaller than the camera's 70 m average is not rare.
- **Kasten & Czeplak 1980** (Solar Energy 24:177; `celestial.md` Deviation 18): the mean global irradiance `G_clear (1 − 0.75 N^3.4)` is a mean over many points under a cover `N`, which is what the lights' factor `0.25 + 0.75 T̄` reproduces when `T̄` is a regional mean beam.
- **Bevy 0.20 source.** No per-light opt-out of the atmosphere; light render layers only gate shadow casting (`bevy_pbr-0.20.0/src/render/light.rs:1456-1460`), not shading (`pbr_functions.wesl:583-641` loops every directional light). `DirectionalLightTexture` (light cookies, `bevy_light-0.20.0/src/directional_light.rs:176`) would project the shadow map onto every PBR surface, but it rides the clustered decals, which need binding arrays and are off on Metal and WebGPU (`bevy_pbr-0.20.0/src/decal/clustered.rs:575-584`): the consumer game runs on macOS, so it cannot be the one path.
- This crate's notes: `clouds.md` ("Cloud shadow map and the sun's illuminance", Deviation 24), `celestial.md` Deviation 18 (`sky_gain`), `twilight.md` Model 3 (the set sun), `terrain.md` §4 and `weathered`'s mirror (the per-pixel ratio), `fog.md` Deviation 7 (the volume's beam compensation).

## Model adopted

1. **The lights carry the regional beam.** `T̄` becomes the mean direct transmittance of the whole cloud shadow map, the 8 km square around the camera (Hillaire's hemisphere mean, eq. 22, taken over the ground the camera sees rather than the sky; Kasten & Czeplak's mean over points). It is the mean of the map's 256² query grid, which is already read back every frame for `Environment::cloud_shadow`, smoothed over 1 s as before, and `1 − cover^3.4` until the first readback. The field is renamed `CloudShadows::mean_transmittance`, because it no longer describes the camera. Everything that took the camera value takes this one: the sun's and the moon's illuminance, the camera's `AmbientLight` (`0.25 (1 − T̄)`), the exposure (`scene_illuminance`), the cloud composite's `sky_gain`, the clouds' ambient and airlight, the precipitation's light and the fog volume's beam compensation.
2. **The shadow stays on the surfaces, per pixel.** The crate's materials (terrain, water, `WeatheredMaterial`, `GroundExtension`, the trails) already scale their lit result by `(0.25 + 0.75 t) / (0.25 + 0.75 T̄)` with `t` the map at the pixel; with `T̄` now regional, a pixel in a cumulus shadow under a mostly clear sky drops to `0.25 / (0.25 + 0.75 · 0.97) ≈ 0.26` of the sunlit level (Mol & van Heerwaarden's 50–80 % lower), the sunlit pixels beside it stay at ≈ 1.02, and the sky, the haze and the reflections do not move. Under a full deck `T̄ → 0` and the model is what it was.
3. **The dome's restoration is unchanged in form**: `sky_gain = total(T̄) / factor(T̄)`, with the regional `T̄`. It now cannot reach a black sky unless the whole 8 km is shaded, and then the deck covers the dome.
4. **An exhausted march is opaque.** When the march stops at `T < 0.01`, `T` becomes 0 and the in-scattered radiance is divided by `1 − T_exit`, the column's remaining 1 % assumed to scatter like the 99 % it met (≤ 1 % brighter than before; Schneider's alpha → 1). The sky behind, sun disc included, is then hidden by any cloud the march gives up on; a cloud thinner than τ = 4.6 still lets the disc through as a dazzling white, as the real sun does through thin altostratus.
5. **The camera kernel goes.** `cloud_shadow_camera`, its 16-byte buffer and its readback have no consumer left and are deleted.

## Parameters

| Name | Value | Source |
|---|---|---|
| Shadow map extent (the regional mean's area) | 8 km square, centred on the camera | `clouds.md` (`SHADOW_EXTENT_M`) |
| Query grid averaged | 256² cells, 31.25 m | `cloud-shadow-query.md` |
| Smoothing of `T̄` | 1 s time constant | unchanged (`CAMERA_TRANSMITTANCE_TAU_S`, renamed) |
| Before the first readback | `1 − cover^3.4` | Kasten & Czeplak, unchanged |
| March exit | `T < 0.01` → `T = 0`, `L /= 1 − T` | Schneider & Vos 2015 |

## GPU mapping

No new pass. The `cloud_shadow_camera` dispatch (one 9 × 9 workgroup re-marching 81 texels of the shadow) and its readback are removed; the mean is a CPU sum of the 65 536 floats the coarse readback already decodes (≈ 20 µs, measured under Validation 5). The march gains one branch after its loop.

## What great looks like

A summer afternoon of fair-weather cumulus: walking into a cloud's shadow, the ground and the objects around you go cool and soft while the hills a kilometre off stay in sun, the sky stays the same blue, the far shore and the water's reflection do not dim, and you can watch the shadow's edge slide across the land. Looking at the sun behind a cumulus, the cloud's edge burns silver and its body is grey; no disc shows through it. Through the thin edge of the cloud the disc may glow. Under a full deck, nothing changes from today: no beam, soft light, a grey dome.

## Validation targets

1. **The light does not follow the camera's shadow (GPU, `tests/gpu_clouds.rs`).** Under `Clear` with a cover that leaves both shade and sun in the map, the camera moved between the darkest and the brightest 70 m spot of the same field: the sun's illuminance differs by at most 2 % and the mean of a clear-sky region of the frame by at most 5 %. Before: the illuminance follows the camera's 9 × 9 mean (≈ 0 vs ≈ 1).
2. **The light carries the map's mean.** Clear and Overcast: `|direct − mean(T)| ≤ 0.01`, and the overcast beam is still at most 5 % of the clear one; the ambient is still `0.25 (1 − T̄)` of the clear-sky illuminance.
3. **An opaque cloud hides what is behind it (CPU mirror, `clouds::model`).** A column the march gives up on returns `T = 0` and a finite radiance at most `1/0.99` of the integrated one.
4. **Visual.** `waves` at 17.5 h, frames 600/1200/1800: a blue sky and an unchanged horizon; `day_night` at 17.5 h: no disc in the cloud, a silver lining; a `terrain_tour`/`weather_tour` sequence under `PartlyCloudy` with a shadow crossing the camera: the sky's mean luminance steady to within 5 % while the near ground darkens and the distant terrain stays lit.
5. **Performance.** `elements_clouds_*` passes no slower than iteration 15's run; the readback mean's CPU cost recorded.

## Deviations from this plan

## Pending index

`10.5194_acp-25-4419-2025` and `10.1029_2022jd037894` stored by `hs paper download` (conversion by the watcher). Perez et al. 1993 (`10.1016/0038-092x(93)90017-i`, the all-weather sky luminance model) not obtained: no open copy.
