# A game keeps its own look and lights

Issues #9, #10 and #11. An art-directed game (Sanctuary: palettes, a fixed ambient, its own lights) wants the weather (rain and snow, wetness, cloud shadows, the wind) without the crate taking over its look. Today `ElementsCamera` inserts Bevy's physical atmosphere, two fogs, an exposure and a zero ambient and forces `Msaa::Off`, with no way to skip any of it; `CelestialPlugin` always spawns and drives a sun and a moon light, so a game that lights its scene itself cannot have `SunState` (and `SurfaceWeathering` dries at a sun factor of 0 without it); and the globals buffer a game's own material would bind is crate-private.

## Sources

- Patry 2021, *Real-time samurai cinema* (Ghost of Tsushima), SIGGRAPH Advances (corpus `s2021-advances-pdf-66-mb`, ll. 229–267): a physically based sky and lighting, but "lighting can deviate from physical correctness", "artists can globally adjust to achieve desired look" with knobs "typically done at the granularity of the weather and time of day states"; the art direction "called for 'stylized realism'". A shipped weather system lets the look be the game's while the weather stays the system's.
- Schneider & Vos 2015, *The real-time volumetric cloudscapes of Horizon: Zero Dawn* (corpus `s2015-advances-ppt-944-mb-pdf-5-mb-2`, ll. 39–41, 120, 707): "art direct-able" is a stated requirement, and the weather system has "controls to bias the simulation to keep things art direct-able".
- Tatarchuk 2006, *Artist-directable real-time rain rendering in city environments* (corpus `s2006-advances-materials-6`): the rain effects are separate, individually controllable pieces (particles, streaks, ripples, wet materials), which is what a game picks from.
- Bevy 0.20 (vendored source): `extract_atmosphere` drops a view's atmosphere state, `AtmosphereTextures` included, when the camera has no `AtmosphereSettings` or the world no `Atmosphere` (`bevy_pbr-0.20.0/src/atmosphere/mod.rs:207-227`); `AtmosphereEnvironmentMapLight`'s observer adds and removes `GeneratedEnvironmentMapLight` (`environment.rs:207-280`); `Camera` does not require `Exposure` (`bevy_camera-0.20.0/src/camera.rs:377-385`). The crate's own map of what reads each component is the scout report summarised in the Model.

## Model adopted

1. **The camera's dressing is a set of opt-outs, each default on.** `ElementsCamera { auto_exposure, atmosphere, distance_fog, volumetric_fog, exposure, ambient, msaa_off }`:
   - `atmosphere`: `AtmosphereSettings` and `AtmosphereEnvironmentMapLight`, one Earth `Atmosphere` in the world while any camera wants it, and the night-sky clear colour. Off: no sky; the cloud layer is not drawn on that view (it needs the atmosphere's transmittance and aerial-view LUTs) but the cloud shadows, which do not, still fall.
   - `distance_fog`: a `DistanceFog` the crate drives from the visibility. Off: none inserted, and one the game put there itself is left alone.
   - `volumetric_fog`: `VolumetricFog`, which draws the ground fog and the light shafts.
   - `exposure`: `Exposure::SUNLIGHT` (and `auto_exposure` drives whatever `Exposure` the camera has, as before).
   - `ambient`: a per-camera `AmbientLight` the crate drives with the cloud-scattered sky light. Off: Bevy's or the game's ambient.
   - `msaa_off`: `Msaa::Off`. Off: the game's MSAA stays, and the cloud and precipitation passes skip that view (they read the depth prepass single-sampled).
   - `DepthPrepass` is not a flag: it is inserted while a plugin that reads the depth prepass (clouds, precipitation, water) is added.
2. **Only what the crate inserted is removed.** Flipping a flag off at runtime removes the component only if the crate put it there (an `ElementsDressing` record of the crate's insertions); flipping it on inserts it if absent. A system on `Changed<ElementsCamera>` does both, so a game can toggle the physical sky in its options.
3. **The celestial maths does not need lights.** `CelestialSettings { lights, sky }` (a resource, both default on): `lights` spawns and drives the sun and moon `DirectionalLight`s; `sky` spawns the moon disc and the star field. `SunState` and `CelestialFrame` (sun and moon directions, altitudes, phase, illuminances) are computed every frame either way, so `SurfaceWeathering` dries in the sun and `Environment::is_daylight` holds. With `lights` off the crate touches no `DirectionalLight`, a game's own `Sun`-marked one included. The light the precipitation particles take from the sun and moon is computed from the same state (the clear light, the twilight fall and the cloud's mean beam), not read back from the lights.
4. **The globals are public.** `ELEMENTS_GLOBALS_BUFFER` (a `Handle<ShaderBuffer>`) is the 80-byte storage buffer `WeatheredPlugin` keeps current (wetness, snow cover, rain intensity, the cloud shadow map's placement, ripple and streak phases, the wind, the light direction), laid out as `bevy_elements::surface_cover::ElementsGlobals`; `CloudShadows::map` is the shadow map (already public). A game's own material extension binds the buffer at any free slot and imports the struct.

## Parameters

| Flag | Default | Inserts | If off |
|---|---|---|---|
| `atmosphere` | true | `AtmosphereSettings`, `AtmosphereEnvironmentMapLight`, an `Atmosphere`, the night clear colour | no sky; clouds not drawn on the view; shadows unaffected |
| `distance_fog` | true | `DistanceFog` (driven) | the game's or none |
| `volumetric_fog` | true | `VolumetricFog` | no ground fog volume, no shafts |
| `exposure` | true | `Exposure::SUNLIGHT` | the game's or Bevy's |
| `ambient` | true | `AmbientLight` (driven) | the game's or Bevy's |
| `msaa_off` | true | `Msaa::Off` | clouds and precipitation skip an MSAA view |
| `CelestialSettings::lights` | true | sun and moon lights (driven) | no light touched |
| `CelestialSettings::sky` | true | moon disc, star field | none |

## GPU mapping

No new passes. The cloud pass's view query also requires `Msaa::Off` (it would otherwise bind a multisampled depth to a single-sampled layout). Everything else is component insertion on the CPU.

## What great looks like

The `isometric` town with `ELEMENTS_LOOK=game`: the game's own warm key light, a flat palette ambient and a plain clear colour, no physical sky, no exposure change, no fog; and on top of that look, the crate's rain and snow streaks, wet darkened walls and roofs, cloud shadows sliding over the ground and snow building on the roofs. The default look is unchanged.

## Validation targets

1. **Every flag off** (`ElementsCamera` with all six off, `CelestialSettings { lights: false, sky: false }`): none of the dressing components is inserted, no `Atmosphere`, no `DirectionalLight` and no moon or star mesh is spawned, and a game's own `DistanceFog`, `AmbientLight` and `Sun`-marked light keep the values it gave them.
2. **Defaults** produce exactly today's component set on the camera (a regression test listing it).
3. **Runtime flips**: turning `atmosphere` off removes the crate's `AtmosphereSettings` and `AtmosphereEnvironmentMapLight` and back on restores them; turning `distance_fog` off on a camera whose `DistanceFog` the game inserted keeps it.
4. **SunState without lights**: with `lights` off, `SunState` follows the clock (altitude at noon and midnight as with lights), and `SurfaceWeathering` dries in the sun.
5. **The globals from outside the crate** (GPU): a material extension defined in a test binds `ELEMENTS_GLOBALS_BUFFER`, imports `ElementsGlobals` and draws the wetness it reads; the pixel follows `SurfaceWeathering::wetness`.
6. **Defaults unchanged**: every existing test passes; performance within 10 % of iteration 19.

## Deviations from this plan

1. **There is no `exposure` flag.** `bevy_render` registers `Exposure` as a required component of `Camera3d` (`bevy_render-0.20.0/src/camera.rs:69`), so a camera always has one and the crate's `insert_if_new(Exposure::SUNLIGHT)` never applied, before this change either (found by `a_bare_camera_without_lights_keeps_the_games_look`, which saw an `Exposure` on a camera the crate had dressed with nothing). Whether the crate touches the exposure is `auto_exposure`'s alone; `ElementsCamera` has five dressing flags (`atmosphere`, `distance_fog`, `volumetric_fog`, `ambient`, `msaa_off`) and the Model's `exposure` row does not exist. `ElementsCamera::bare()` turns all five and `auto_exposure` off; the precipitation pass also needs `Msaa::Off`, so a game that wants rain on its own look keeps `msaa_off` on.
2. **A game's own `Sun`-marked light lights the rain and casts the cloud shadows (found in the capture review).** With the crate's lights off, the particles kept the crate's computed sun (about 50 klx) and sky while the town was lit by the game's 6 klx key light at Bevy's default exposure: stark white streaks and flakes on a palette scene (`/tmp/i20/r1/sheet.png`, bottom row). The particles now take their direct light from the light marked `Sun` (and `Moon`), the crate's or the game's, in its direction and at its illuminance (the crate's computed light only when no light is marked), and their sky light from the camera's own `AmbientLight` when the crate does not drive it (`π · 0.452 · brightness · luminance`, what Bevy's ambient gives a white diffuse surface). The cloud shadow map, when the crate's lights are off and a `Sun`-marked light exists, is cast along that light, so cloud shadows and the game's building shadows agree. A game marks its key light `Sun` to opt in; the crate never writes to it with `lights` off.
3. **A bare camera keeps MSAA, so the rain needs `msaa_off` kept on.** The precipitation pass reads the depth prepass single-sampled and skips (with one warning) an MSAA view; `ElementsCamera::bare()` turns `msaa_off` off with everything else, so the example's game look is `ElementsCamera { msaa_off: true, ..ElementsCamera::bare() }`. The cloud passes now skip an MSAA view too (they would have bound a multisampled depth to a single-sampled layout).

## Pending index

All sources are in the corpus (indexed).
