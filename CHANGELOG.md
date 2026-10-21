# Changelog

All notable changes to bevy_elements. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate is pre-1.0, so any release may break the API.

## [Unreleased]

### Added

- **A game lights its own night.** The crate simulates a day and a night, a sun and a moon, a sky
  and a metered exposure, and a game that put a character in that world at night had **nothing
  to light them with**: no query said how bright the night was in the camera's own terms, and
  nothing said what power a torch had to be to read against the exposure the crate meters. The
  crate's own machinery for this existed and was private — the lightning writes its bolt's
  `PointLight` in physical units from the flash's luminance and the bolt's material colour from
  the same flash through the camera's `Exposure` — so it is now public, in one path:
  - `ElementsExposure`, a resource holding the camera's EV100 and its exposure factor,
    refreshed every frame. The celestial plugin writes the *followed* camera's exposure there,
    whether the crate metered it or the game drove it (`ElementsCamera::auto_exposure` off).
  - `Environment::exposure()`, `display_value(luminance)`, `luminance_for_display(display)`,
    `light_intensity(luminance, distance, albedo)`, `light_illuminance(intensity, distance)`
    and `night_light_colour()`.
  - The `light` module, with the same functions in free form.

  A scene-linear value of **0.18 is 18 % grey, the middle of the picture**, and the crate's
  metering is built so that an 18 % grey surface reads at 0.18 *at every hour* — the exposure
  is applied inside Bevy's PBR shader before the tonemapper, so it is the tonemapper's input
  that the crate's numbers name. The power a light needs scales *inversely* with the
  exposure: the same lamp wants 62 253 lm at noon and 2.8 lm at midnight for the same picture,
  so a game that hard-codes a lamp's lumens is wrong by five orders of magnitude between the
  two, and a game that asks the crate is never wrong.

  Bevy maps *both* a point light's and a spot light's power to `Φ / 4π` ("we choose to use the
  same 4\*PI divisor in both cases so that toggling between point light and spot light keeps lit
  areas lit equally", `bevy_pbr-0.20.0/src/render/light.rs:711`), so a spot's cone decides only
  *where* the light lands and not how bright it is on the axis: one power function serves both.

  No new pass, no new buffer, no new binding. The `isometric` example places a street lamp and
  a hand-held light and balances them with the crate's numbers: the town's frame contrast rises
  from **2.44 DN to 14.06 DN**, and the noon capture is unchanged (143.03 against 142.62, a
  0.41 DN difference, the standard deviation within 0.03). See `docs/research/game-lights.md`.

### Changed

- **The night sky is blue, not brown.** The sky at 22 h with no moon was **sepia**: its linear
  RGB was (0.00297, 0.00195, 0.00099), a red/green ratio of 1.52, and the crate's own
  `twilight.md` says the night should be "Nothing brown". The cause is not what iteration 12
  found and fixed — that was Bevy's default `ClearColor`, and the crate replaced it with the
  night sky's own radiance. The clear colour *is* now neutral (the probe reads
  `LinearRgba(0.0035603, 0.0035603, 0.0035603)`) and the sky is still brown. The clear colour is
  what lies **behind** the atmosphere, and the atmosphere is drawn over it with its
  transmittance — measured against that neutral clear colour, (0.904, 0.640, 0.353) at the
  zenith and falling with elevation. That is Rayleigh extinction, which takes the blue out of the
  line of sight most, and it is *warm*, so a neutral clear colour behind it is a brown sky. The
  clear colour is now the *scotopic* colour divided by that transmittance, so what comes through
  is the cool blue-black the rod's eye expects (Jensen et al. 2001 §7 on the blue shift, and the
  scotopic curve, whose peak is at 507 nm). The ambient lights the ground with the sky's glow as
  it reaches the ground — through the atmosphere — so it uses the scotopic colour directly; the
  clear colour is the glow *behind* it and must be pre-extinction. Measured: the zenith's
  red/green 1.411 → **0.783** and its blue/green 0.551 → **1.197**, the horizon still warmer
  (red/green 2.119 → 1.626), the sky probe's path bluer too (blue/green 1.179 → 1.266), the dusk
  and the dawn byte-identical, and the cost nil (the mountains' opaque pass 0.147 ms against
  0.147 ms: the fix is one `vec3` written per camera per frame).

- **A moonless night is not a black screen: the night sky lights the ground.** The isometric town
  at 20 h rendered at **1 DN** — the sky, the ground, the houses' windows, everything — and a
  player cannot see their character. The cause was precise: the night sky's own illumination
  (the airglow, zodiacal light and starlight that lie *behind* the atmosphere) is the crate's
  `ILLUMINANCE_FLOOR_LUX = 0.0005 lx`, and the crate used it for the exposure meter's floor and
  the background's clear colour, while the **ambient that illuminates surfaces** came from
  `cloud_ambient`, which is zero when neither the sun nor the moon is up. The sky was lit and the
  world on it was not. The night sky's radiance is now part of the ambient *and* of the sky
  probe's cube (which is display-referred, so the radiance is converted with the camera's
  exposure, as the fog's haze is), in the *scotopic* colour — a cool blue-green, Jensen et al.
  2001's blue shift and the rod's curve, whose peak is at 507 nm. Measured on the isometric
  town in `Clear`: 20 h **0.907 → 24.763 DN** and 22 h **0.739 → 22.443 DN**, with the blue/red
  ratio rising to 1.06; noon and the twilight are byte-identical, and the cost is nil (the
  fix is on the CPU: the mountains' opaque pass 0.251 → 0.253 ms). A test asserted the bug
  ("a night with no moon stays dark whatever the cloud") and now asserts the correction.

- **The ground has more than one colour: a meadow is not one green.** The crate's grass was a
  single green — the hue's spread over a near-ground patch was 0.010 on the plains and 0.016 on
  the hills — so the ground was one colour from the near grain to the horizon. The turf is now a
  **palette of four tones** (fresh grass, the yellow of a dry patch, the straw of a bleached
  one, the earth of a worn place) mixed by a score, exactly Papavasiliou's model (JCGT 4(1)
  §3.3): a low-frequency field decides how much of each tone is *around* and a finer one
  decides which tone this piece of ground *wears*, and the score is their product. The four
  tones are mixed as *shares* that sum to one, so the mixture is a convex combination of them
  and the ground's mean colour and brightness cannot drift — the alternatives each failed (the
  unweighted mean of the tones drifted the mean brightness 3.2 DN, and normalising the mixture
  by its own colour cancelled the hue entirely). The palette's two fields are the ground's
  *own* — the large-scale field the layer variation already uses, and the turf's coarse and
  fine detail fields — so the palette's only new noise evaluation is its selection field, and
  the cost is **0.137 ms → 0.144 ms** on the mountains' opaque pass. Measured: the hue's
  spread 0.0097 → 0.0226 on the plains, 0.0258 → 0.0339 on the hills, with the mean
  brightness within 0.2 DN and the rock byte-identical. A game's own ground on
  `GroundExtension` gets the same variation.

- **The rock has a material, and the ground's grain no longer weaves the cliff faces** (below):
  the rock faces were a woven herringbone (coherence 0.71 at a fixed 43.5° world direction),
  the rock had no material of its own, the ground's large-scale variation gives a plain
  patches of bare earth, and the slope band's soil is broken into patches and tapers into the
  rock.

- **Rock has a material, and the ground's grain no longer weaves the cliff faces.** The rock
  faces were a woven herringbone: the streak direction was 43.5° at 2 m and 44.0° at 6 m — the
  *same* direction at two eye heights, which is the signature of a lattice in world space —
  and the coherence was 0.71. The cause was the ground detail's 2D noise in the world XZ plane:
  that plane is the *projection* of the surface onto it, so on a slope the lattice is stretched
  along the slope and shows through. The ground's **turf** keeps the 2D noise (it is right on
  the near-horizontal surfaces) and is now gated off before the surface is steep enough to
  weave it — closed at 37° — and the **rock's** material, the crate's 3D noise at the 3D surface
  point, which has no projection at all, takes over there; the rock's ramp opens at 30°. The
  rock's material is bedded on a *tilted* plane (the world's height would band the massif with
  its own contours, which is a topographic map), a second warmer bed tone, a fracture network
  of ridged noise (a Worley cell network bounds cells and read as a giraffe's hide), and the
  rock's own grain and relief. The ground's large-scale variation gives a plain patches of
  bare earth through the meadow, which Andersson's Overlay detail cannot do — it "does not
  affect areas where the base procedural mask is 0.0 or 1.0", and a plain's grass mask is 1
  everywhere. The slope band's soil is broken into patches and tapers into the rock, so the
  warm brown that ringed every grass patch is gone: the pink was 16.75 % of the 2 m face and
  9.67 % of the 6 m one before, 0.00 % now, and the orange rim from 1415 pixels to none. The
  mountains' opaque pass is **0.127 ms against the baseline's 0.136** — the mask's fBm went
  from four octaves to one, and that pays for the rock's material. No public Rust API changes:
  the shaders' `terrain::common` imports are the interface.

- **The layer masks move**: the rock's ramp is 30°–38° (was 35°–45°) and the soil's band
  16°–26° tapering from 22° (was 26°–36°), so the ground's turf is gone before the surface
  is steep enough to weave it and the soil thins as the rock rises.

- **The water's surface is a mirror with a skin on it, not a bowl of jello.** The ripple normal
  map's spectrum was an open-ocean statistic applied to sheltered water: 52 % of its slope
  energy sat in the 1 m octave, and a 6.2° rms tilt on the water's near-mirror surface (0.089
  perceptual roughness, 0.0079 linear) swung the reflected ray 12.4° — nine texels of the 128²
  environment cube — so the clouds in the reflection were torn into blobs. The map's slope
  spectrum is now flat (every octave the same slope, Lagae et al. 2010 §4) over three octaves
  from 0.25 m to 6.25 cm, the metre band is left to the wave field that already computes it from
  Airy's dispersion, and the still-water strength drops from 0.25 to 0.06, because a sheltered
  pond is a mirror and the open sea is not. The map's total rms slope is unchanged (Cox & Munk
  1954, 0.150, checked by the CPU and GPU oracles), so the surface is as rough as the wind
  statistic says and the glitter it makes is the same. Measured on `examples/waves` at 16:30
  under `PartlyCloudy`: the water band is 2.23× noisier than the sky it mirrors before, 1.37×
  after, and the water's mean brightness is unchanged (97.15 against 97.11 DN). The wakes are
  untouched (`gpu_waves` passes unchanged). `WaterSettings::ripple_scale` and the flow's own
  strength are unchanged, so a running current is as rough as before.

- **The ground has surface detail at a metre** (below): the terrain and `GroundExtension` shade
  with two band-limited octave scales instead of one 0.8 m⁻¹ noise band faded by distance. Each
  octave is cancelled once the pixel footprint cannot resolve it (Lagae et al. 2010's frequency
  clamping), so the near field reads as turf and granular sand (near-patch standard deviation
  1.07 → 4.26 DN on `terrain_tour` hills at 6 m) while ground beyond 120 m is unchanged to
  0.002 DN. The detail is a 2D noise in the ground plane, not a 3D noise sampled on the surface:
  the 3D slice is anisotropic on slopes and streaked the cliff faces. `bevy_elements::noise`
  gains `perlin2_d` (2D gradient noise with its analytic derivative); `terrain::common` gains
  `ground_detail`, `detail_keep` and `detail_scale`. No public Rust API changes: the shaders'
  `terrain::common` imports are the interface.

- **The followed camera's sky light and reflections come from the clouds' sky probe** (below): its `AmbientLight` no longer carries the cloud-scattered light while the probe is live, and the terrain's and `WeatheredMaterial`'s overcast shade factor keeps only its relief cue, `overcast_relief` in `bevy_elements::surface_cover` (the dome's own shading now comes through the environment map; `snow-cover.md` Deviation 16). Overcast scenes lose the flat white sheen the uniform ambient's boosted specular laid over every surface: grass under the `waves` overcast reads 115 DN against 134, a metal box's sides mirror the ground (101 DN) under a top that mirrors the sky (193 DN). `CloudQuality` has a `probe_edge` field; a struct literal needs it.
- **`ElementsGlobals` is 128 bytes** (a sixth `vec4` whose `x` is `ElementsUnits::units_per_metre`; a seventh and eighth carrying the trail window, its centre, the beam's share of the light and the snow depth, `trail` and `trail_light` in the WESL struct): a game material binding `ELEMENTS_GLOBALS_BUFFER` declares the new fields, and a struct literal of `ElementsGlobals` needs `snow_depth_m`, `trail_texel_m`, `trail_texels`, `trail_centre` and `trail_direct_share`. `LightningStrike::position`, `Thunder::position`, `LightningFlash::position`, `WindGust::origin`, `ElementsFocus::Point` and the `Environment` query positions are world units (unchanged at the default), and the grids the crate publishes (`WindState`'s window, `TerrainGrid`, `CloudShadows`) are in the metre frame. The WESL import `elements_wind` takes a world position (its `WindFieldParams` gained `units`).
- **`TrailSettings` has a `texel_m` field** (a struct literal needs `..default()`), and the WESL `bevy_elements::trails::surface::trail_fade` takes the window's centre (`TrailState::centre`, also in the terrain and ground uniforms' `trail_light.yz` and the globals' `trail_light.yz`) instead of the eye. `SurfaceWeathering::with_snowpack` replaces nothing public.
- **`SnowyField` takes the nearest column's top and snow across a cliff** (four tops more than a voxel apart) instead of blending them: a top blended across a step floated the slab's underside off the lower columns, a dark sheet at a merlon's foot or down a wall's side. Smooth ground is unchanged. `docs/research/voxel-volumes.md` Deviation 3.
- **The `flora` feature depends on `bevy_flora` 0.46** (was 0.30; issue #12), so a game on 0.46 builds one copy and `ElementsFloraPlugin` writes the `FloraWind`, `FloraSeason` and `FloraWetness` its trees read. No API change on this side: `GustFront` speed and width are metres under 0.46's `FloraUnits`, which is what `WindGust` carries. A game on a git revision of `bevy_flora` patches it with `[patch.crates-io]`.
- **`Weather` has a `snow_possible` field** (1 unless a `SnowSeason` excludes snow), which `Weather::snow_fraction` multiplies; a struct literal needs it or `..default()`. `Climate` has `snow_season` (`None` by default, `#[serde(default)]`, so saved climates load). The WGSL `bevy_elements::climate::snow::snow_fraction_at(t_c, possible)` takes the factor as a second argument. Ground-cover maps created on a terrain (and re-created when it is rebuilt) now start from the point snowpack of `SurfaceWeathering` instead of bare ground.
- **`ElementsCamera` has fields beyond `auto_exposure`** (`atmosphere`, `distance_fog`, `volumetric_fog`, `ambient`, `msaa_off`); a struct literal needs `..default()`. Migration: `ElementsCamera { auto_exposure: false }` becomes `ElementsCamera { auto_exposure: false, ..default() }`. The precipitation particles now take their direct light from the lights marked `Sun` and `Moon` (whoever owns them) and the crate's computed sun only when none is marked; the default scene is lit as before.
- **`WindState::at_camera` is now `at_focus`**: the wind at what the domains centre on, the camera unless it carries an `ElementsFocus`. Migration: rename the field.
- **Clouds are white: multiple scattering by diffusion.** Clouds were lit by single scattering and three Wrenninge octaves, which against Monte Carlo transport in a cloud slab return a sixth of the light of a thick top and almost none of a base: cumulus were grey-khaki from the ground, khaki from above and black underneath at sunset. The octaves are replaced by photon diffusion between the cloud's lit face and the cloud behind it, along the light (Jensen et al. 2001's extrapolated boundaries, `z = 2/(3(1 − g))`; one extra base-shape sample 300 m behind each sample), with an albedo fitted to 96 Monte Carlo slabs (`docs/research/cloud-lighting-mc.py`): the march is within 0.57–1.12× on every thick top and a mean squared log error of 0.067 against 0.82 before (`the_slab_matches_monte_carlo`). Sunlit cumulus are now white, their bases grey, sunset undersides lit orange, a noon overcast deck about three times brighter and neutral. The clouds' ambient now takes the sky's colour only; the sunlight it stood in for is the diffusion term. `elements_clouds_march` 0.480–0.524 → 0.513–0.539 ms at High (+3–7 %, alternating runs at 1740–1800 MHz). `docs/research/cloud-lighting.md`, `docs/cloud_lighting.png`.
- **Cumulus billow.** The fair-weather cumulus of `Clear` and `PartlyCloudy` rendered as kilometre-wide smooth pancakes with stacked shelves on their flanks: inside a coverage blob the density was solid, the cumulus height gradient was a flat slab whose corners and the dome texels' kinks showed as terraces, and the detail erosion (0.2 of 128–512 m cells) only roughened it. Cumuliform clouds (type ≥ 0.25) are now carved by Nubis Evolved's envelope model (Schneider 2022, 2023; Bauer 2019): a flat base at the condensation level, a C¹ rounded top whose height follows the blob, convective cells from the base noise that split a blob into separate thermals (closing into a deck as the cover reaches 1), and a value erosion `√saturate((N − (1 − D)) / D)` of a billowy Worley composite (base tile 4096 m, detail tile 640 m: lobes of a kilometre, turrets of 80–320 m) that gives cauliflower tops and crisp silhouettes. The detail noise fades to its mean where the march's step is too long to resolve it. Stratus decks are unchanged; the share of the ground in cumulus shade still matches `cloud_cover` (0.377 at 0.4 over the field, mirror test). `elements_clouds_march` 0.437–0.474 → 0.489–0.492 ms at High (+6 %), the shadow map +0.008 ms. `docs/research/cumulus-shape.md`, `docs/cumulus.png`.
- **Twilight exists, and the sunset is not blown out.** The sun's light was switched off at −2°, and Bevy's atmosphere is lit only by directional lights, so from −2° to −18° (over an hour of every summer evening) the sky had no light at all. The sun now lights the atmosphere down to −18°, falling below the horizon at the rate Hulburt (1953) measured for the zenith twilight sky (0.427 decades per degree; checked on a linear target to within 0.65–1.0×); it lights no surface once its disc has set (Bevy's own visible-disc factor), its shadow maps are off below −0.5°, and the precipitation draw takes it only while its disc is up. Civil and nautical twilight now render as they should: a blue to blue-purple zenith (the ozone in Bevy's medium), an orange glow on the sun's side, the Belt of Venus on the other, the afterglow shrinking into night. The automatic exposure, metered on illuminance, is now capped by the sky toward the sun (Patry 2021): from a sun of about 25° down through twilight the brightest sky stays at 3.2 × display peak, where it reached 6–20, so the golden hour keeps its colour and the ground reads as silhouettes against it. The afternoon under a clear sky is metered as before; a camera standing in a cloud shadow no longer opens up past the sunlit sky (PartlyCloudy at noon 0.4 stop darker). `docs/research/twilight.md`, `tests/gpu_twilight.rs`.
- **The night sky is no longer brown.** Bevy's default `ClearColor` (sRGB 43, 44, 47) is written behind the sky before any exposure, so it showed through the atmosphere as a constant warm-brown sky of about 40 DN at night. A camera that kept the default clear colour is now cleared every frame to the night sky's own faint glow (1.6×10⁻⁴ cd/m², the sky that gives the exposure's illuminance floor) at its exposure; a camera with its own `ClearColorConfig` keeps it.
- **Clouds take the colour of the light that fills them.** The clouds' ambient (the diffuse light on their shaded sides) was a neutral grey whatever the sky, so after sunset clouds were grey cardboard against a violet or navy sky. Its colour is now that of the diffuse light it stands for: Bevy's own sky (16 directions of the camera's atmosphere environment map over the upper hemisphere, undimmed by the cloud shadow, plus the night sky's glow behind it) plus the sunlight the cloud above has scattered out of the beam; the brightness is unchanged. Thick decks and cumulus bases stay neutral by day, the golden-hour base warms under its orange rim (`(64, 62, 60)` → `(74, 59, 56)`), the civil-twilight base turns violet-grey (`(57, 57, 59)` → `(62, 55, 62)`), nautical-twilight clouds mauve, and night clouds keep the night sky's hue. A camera without `AtmosphereEnvironmentMapLight` keeps the neutral ambient. About +0.005 ms on the cloud march at 1080p (`docs/research/clouds.md` Deviation 30, `tests/gpu_cloud_ambient.rs`).
- **`CloudShadows::camera_transmittance` is now `mean_transmittance`, the cloud shadow map's mean over its 8 km** (and `ElementsGlobals::camera_transmittance` is `mean_transmittance`). It is what the sun's and moon's lights carry; a custom shader that scales its lit colour by `(0.25 + 0.75 T) / (0.25 + 0.75 T̄)` keeps working with the new field. Migration: rename the field; if you used it to ask whether the camera stands in shade, ask `Environment::cloud_shadow_at(camera_position)` instead. A plain `StandardMaterial` no longer dims with the shadow over the camera; use `WeatheredMaterial` for props that should take the cloud shadow (the `waves` example's bodies now do).

### Added

- **Trails on a game's own ground (issue #26).** A `TrailGround` (resource: the ground's world height at a world XZ, the shape of `bevy_flora`'s `FloraGround`, soil or `with_sand()`) runs the trails with no crate `Terrain`: snow, its density and the wetness are `SurfaceWeathering`'s (new snow in the point pack fills the prints; prints made at night last into the morning), contact is tested against the game's height, and nothing is spawned. `WeatheredExtension::with_trails()` draws them on any mesh by the same parallax march as the terrain (compacted snow, the material showing where a print reaches the bottom of the snow, water on saturated rut floors; faces up to 60°, forward rendering). `Environment::trail_depth(Vec2)` answers how deep the ground is pressed, from a readback of the deepest point per 10 cm (a fifth kernel on readback ticks, `elements_trails_readback`). `TrailSettings::texel_m` widens the window for a far camera. `SurfaceWeathering::with_snowpack(depth, density, wetness)` starts a scene under snow. A sole walked 20 m over a game's flat ground under 10 cm of snow leaves `trail_depth` 7.0 cm all along it (the CRREL cap), with no `Terrain`. `docs/research/trails-game-ground.md`, `tests/gpu_trail_ground.rs`, the `isometric` example's walker (prints in the town's own snowed ground; `ELEMENTS_SNOWPACK`, `ELEMENTS_TRAIL_TEXEL`), `ELEMENTS_TERRAIN=mesh` in `trails` and `game` in `bench_elements`, `docs/trails_game_ground.png`, `docs/trails_game_ground.gif`.
- **A look for rain and snow under far-off cameras (issue #25).** `PrecipitationLook` (resource, defaults are today's physical look) restyles the precipitation draw and nothing else: `rain_fall_scale` and `rain_wind_scale` scale a streak's fall and wind drift apart, `rain_tint` its colour and opacity, `flake_size_spread` draws each flake at a log-normal size about its own (clamped to ±2σ), `flake_min_half_width_px` sets the smallest flake. `PrecipitationLook::readable()` is the stylised look for orthographic and strategy cameras: under an orthographic camera pitched 35° the `Rain` preset's streaks stand 15° from vertical instead of 63° (sleet), and snow is flakes of visibly different sizes instead of identical dots. `docs/research/precipitation-look.md`, `docs/precipitation_look.png`, `ELEMENTS_PRECIPITATION=readable` in every example and `ELEMENTS_PITCH` in `isometric`.
- **`WeatherDirector::forecast` (issue #27):** the scheduler's preset for each of the coming hours, from the same seeded chain and without changing the director; the weather then comes as forecast, forced or not.
- **`SurfaceWeathering: PartialEq` (issue #23):** compares the whole snowpack, its private water equivalent, density and last stepped hour included (not whether it was just read back from a save), so a game's save can derive `PartialEq` and a serde round trip compares equal.
- **Clouds in the environment map: reflections and sky light under cloud.** Bevy's environment map was the clear sky, so water, puddles, wet ground and every glossy surface mirrored a blue sky under cumulus, and under overcast the water was brighter than the sky it reflects (`(230, 238, 240)` under `(226, 226, 226)`: the uniform cloud ambient's specular term, boosted 2.2× for its diffuse term). `CloudPlugin` now marches the clouds into a cube around the followed camera (the size of its atmosphere cube, 128² by default), composes it with Bevy's atmosphere cube as the screen composite composes the sky (sky gain, the weather's air; toward the ground, Bevy's ground lit by the dome's total light), and hands it to the camera's `GeneratedEnvironmentMapLight`, which Bevy filters into the reflections and the irradiance. One texel of every k × k block per face is marched a frame at half the screen march's steps (k = 4, 4, 4, 2 from Low to Ultra at 128²) and blended into its history; the cube is handed over once every texel has been marched. 0.062 ms at High (RTX 3090); the screen march drops a duplicate sky irradiance per ray, so the clouds' total rises 0.056 ms. Ponds mirror the cumulus and their blue gaps, the overcast water is a grey mirror darker than its sky, and the overcast dome lights tops above sides above undersides (1 : 0.67 : 0.37). A camera without the crate's atmosphere, or not followed, is unchanged. `docs/research/sky-light.md`, `tests/gpu_sky_probe.rs`, `docs/sky_light.png`, `docs/sky_light.gif`.
- **A world-units scale (issue #8).** `ElementsUnits { units_per_metre }` (default 1.0; 3.2808 for a world in feet) tells the crate how many world units make a metre. The crate keeps simulating in metres in a frame of world ÷ units and converts only where it meets the world: the followed camera and focus, every `Transform` it reads (blockers, water sources, contacts, imprints, the terrain, voxel volumes), what it writes into Bevy's world (the spawned `Atmosphere`'s scale, `DistanceFog` and the ground-fog `FogVolume`, shadow cascades, the lightning bolt and its light, intensity × u², the terrain and water patches), and the 13 shaders that take a world position from Bevy. Positions at the API are world units; lengths, depths and speeds stay SI. A strike 3280.8 units away in feet reads `Thunder::distance_m` 1000, the rain box is 196.8 units wide, and a foot world answers `Environment` as the metre world does (heights within 2.3e-5 m, wind within 0.12 %, the same cloud shadows). Non-positive or non-finite values read as 1.0. `docs/research/units.md`, `ELEMENTS_UNITS` in the examples' harness (`docs/units.png`), `tests/gpu_units.rs`, `tests/lightning.rs` `a_strike_in_a_world_of_feet_is_a_thousand_metres_off`.
- **Snow on many voxel volumes without a terrain (issue #13).** `VoxelSnow` on any number of `VoxelVolume`s puts `VoxelGround`'s snow slab on each (one column per voxel, the same hold, edit rescans and field swaps) and spawns no terrain and no hydrology. A column's depth is the ground-cover map's at its world position where a crate terrain with `GroundCoverPlugin` lies under it, else `SurfaceWeathering`'s snowpack on exposed flat ground, times its top's shed-slope exposure. Volumes may be moved, turned about the vertical and scaled uniformly (chunks as children of the volume); a tilted one gets no snow; a volume with both components is a `VoxelGround`. `VoxelPlugin` runs `VoxelSnow` without the terrain, hydrology and cover plugins. Both `isomesh` crates stay at 0.0.11: bevy_isomesh 0.0.10 is built on Bevy 0.19 and bevy_isomesh 0.0.11 pins its core exactly. `docs/research/voxel-volumes.md`, the `voxel_walls` example (`docs/voxel_walls.png`, `docs/voxel_walls.gif`), five system tests and three model tests in `voxel::tests` and `voxel::model`, `the_slab_sits_on_both_sides_of_a_cliff`.
- **A snow season on `Climate` (issue #15).** `Climate::snow_season = Some(SnowSeason { first: (11, 14), last: (3, 31) })` (local calendar, inclusive, may wrap the year) keeps snow out of the rest of the year: the scheduler never picks Snow or Blizzard there and all precipitation falls as rain, through `Weather::snow_possible`, which every reader of the rain–snow split scales by (the point model, the cover and hydrology kernels, the particles, the fog and the weathered materials). The phase changes over the hour before the first day and after the last; a forced Snow or Blizzard still snows; lying snow melts as usual; inside the season nothing changes, hour for hour. `Climate::with_snow_season`, `Climate::snow_possible`, `docs/research/snow-season.md`, `tests/snow_season.rs` (a cool-temperate November with no snow before 14 November and the same rain, the season identical to no season, a −3 °C rain hour, forcing, the boundary ramp), `ELEMENTS_CALENDAR=november` and `ELEMENTS_SNOW_SEASON=off` in `weathered_town` (`docs/snow_season.png`, `docs/snow_season.gif`).
- **The snowpack survives a save (issue #14).** `SurfaceWeathering` serializes with the `serde` feature: the snow, its density and the wetness carry on after a load, exactly as unsaved when the loaded clock is within an hour of the saved one, and neither melting nor growing across the gap when it is not. With a terrain, the ground-cover maps are re-seeded from the loaded pack (each cell's snow times its slope and shelter exposure, its wetness times its rain exposure) by a one-off `cover_seed` kernel after the shelter mask, and a map readback in flight across it is dropped. `tests/save.rs` (`a_restored_snowpack_continues_the_same_snow`, `a_snowpack_loaded_a_day_later_neither_melts_nor_grows`, `files_from_before_the_snow_season_load`), `cover::gpu_tests::gpu_a_restored_snowpack_seeds_the_maps`, `ELEMENTS_SAVE` / `ELEMENTS_LOAD` / `ELEMENTS_EV100` in `weathered_town`.
- **A game keeps its own look and lights (issues #9, #10, #11).** Every part of the camera's dressing is a flag on `ElementsCamera`, on by default: `atmosphere` (Bevy's sky; without it the cloud layer is not drawn on the camera but the cloud shadows still fall), `distance_fog`, `volumetric_fog`, `ambient` and `msaa_off`. A flag that is off inserts nothing and leaves the game's own component alone; turning one off at runtime removes only what the crate inserted. `ElementsCamera::bare()` turns them all off. `DepthPrepass` is inserted while a plugin that reads it (clouds, precipitation, water) is added, and the cloud passes skip a camera that keeps MSAA. `CelestialSettings { lights, sky }` stops `CelestialPlugin` spawning and driving the sun and moon lights and the moon disc and stars, while `SunState` stays current (so `SurfaceWeathering` still dries in the sun); a game's own `Sun`-marked light is never touched, lights the rain and snow (with the camera's own `AmbientLight`) and casts the cloud shadows. `ELEMENTS_GLOBALS_BUFFER` is the crate's 80-byte globals storage buffer (wetness, snow cover, rain intensity, cloud shadow placement, ripple and streak phases, wind, light direction) for a game's own material extension to bind. `docs/research/integration.md`, `ELEMENTS_LOOK=game` in the `isometric` example (`docs/game_look.gif`), `tests/gpu_dressing.rs` (the default set unchanged, a bare camera and no lights keeping the game's fog, ambient, MSAA and sun, runtime flips), `tests/gpu_weathered.rs` `a_game_material_reads_the_globals_buffer`.
- **A focus for the camera-relative domains, and orthographic cameras (issue #7).** `ElementsFocus::Entity(e)` or `ElementsFocus::Point(p)` on the `ElementsCamera` centres the rain and snow box, the wind window and its gusts, the ground fog, the strikes and the thunder distance, the cloud weather and shadow maps, the trails and waves windows and the surface weathering's mean on that entity or point instead of the camera; without it nothing changes. Orthographic cameras work: the cloud march, resolve and composite build each ray from the view matrices (parallel rays starting on the plane through the focus, so an isometric camera's stand-off is not air and a camera high above a low deck does not start inside it), the bolt keeps two pixels of core at every distance, and for an orthographic followed camera the distance fog is linear from the nearest ground the view shows (`orthographic_fog_falloff`, `orthographic_near_depth`), the sun and moon cast one shadow cascade around the focus, the rain box grows to the view's footprint (up to 240 m) with its drops drawn without the distance fade, and the ground-fog volume is off. A perspective camera is unchanged; the terrain's level of detail stays on the eye. `docs/research/view-anchor.md`, the new `isometric` example (`docs/isometric.gif`), `tests/gpu_precipitation.rs` `the_domains_centre_on_the_focus_of_an_orthographic_camera`, `tests/gpu_cloud_ambient.rs` `an_orthographic_camera_sees_the_deck_a_perspective_one_does`, `tests/lightning.rs` `strikes_and_thunder_centre_on_the_focus`.
- **Waves: ripples and wakes from bodies in the water** (`WavesPlugin`, in `ElementsPlugins`). A `WaterContact { half_extents }` (an ellipsoid, any rotation) on any entity presses the hydrology's water with the pressure head of its occupied column (its draft for a shallow hull, at most `1/k` for a deep leg the waves do not reach; Colen & Kolomeisky 2021) and, while it crosses the surface going down, Bernoulli's impact head `v²/2g`. At rest the water stays flat around it; moving it leaves a wake inside Kelvin's 19.47° (measured edge ≤ 19.1° in `tests/gpu_waves.rs`); bobbing or dropped in it sends out dispersive rings. The surface is a linear capillary-gravity wave field in three camera-following cascades a factor of four apart (High: 512² of 5, 20 and 80 cm, out to 410 m), stepped by Tessendorf's iWave scheme with a 13 × 13 kernel fitted at start-up to Airy's dispersion (within 1.5 % for waves of 3 to 12 texels; Canabal et al. 2016) and forced by the bodies' head split into the bands like Ottosson's pyramid. Waves reflect off the shores, slow in the shallows, follow the current, break beyond `ak` = 0.44 (Jeschke & Wojtan 2017) into foam that fades over a few seconds, and the water's surface and absorb passes draw their slopes and the foam as lace. Wave faces that would reflect below the horizon now reflect the horizon instead of Bevy's tan ground disc. 0.29 ms a frame at High with 8 bodies and currents everywhere, 0.23 ms on still water (RTX 3090). `WaveSettings`, `WaveState`, the `waves` example, `ELEMENTS_CONTACTS` and `ELEMENTS_WAVES=off` in `bench_elements`, `docs/research/waves.md`.
- **Trails: footprints, ruts and tracks in snow, sand, soil and mud** (`TrailsPlugin`, in `ElementsPlugins`). An `Imprint { half_extents, load_n }` on any entity presses the ground under its translation while it is within 0.1 m of the surface, as deep as the ground gives (Bekker–Wong sinkage with Wong's constants; in snow no deeper than it compacts to 500 kg/m³, CRREL 90-9; at most a 0.30 m loose layer), with a rim of the displaced material it cannot compress (Alvarado et al. 2022: 10 % in snow, all of it in dry sand); it carves the whole path swept between frames, so a fast wheel leaves an unbroken rut. A 77.5 kg walker leaves 24 cm in 30 cm of fresh powder, 3.4 cm in dry sand, 0.5 cm in dry soil, 30 cm in mud. New snow fills a trench by what the cover map gained, melt clamps it to the snow, the snow drift and the sand flux smooth it, and sand and mud slump to their angles (Sumner et al. 1999). The field is a camera-following window of centimetre texels (High: 1024² × 5 cm) in four compute kernels, 0.119 ms at High with 16 wheels pressing (RTX 3090, minimum over the run), drawn per pixel by the terrain material and `GroundExtension` with parallax occlusion mapping (Tatarchuk 2006; one sample per texel, refined on a cubic B-spline of the field), a soft self-shadow and an occlusion term; compacted snow, the ground under thin snow, the soil under grass and water on the floors of wet ruts show through. `TrailState` publishes the field for custom ground shaders (`bevy_elements::trails::surface`). The `trails` example, `ELEMENTS_IMPRINTS` and `ELEMENTS_TRAILS=off` in `bench_elements`, `tests/gpu_trails.rs`, `docs/research/trails.md`.
- **`ELEMENTS_START` in the `day_night` example** starts the running day at that hour (a dusk time-lapse: one game hour is 300 frames), and `ELEMENTS_PRESET` holds a weather preset (default `Clear`).

### Fixed

- **Trails are drawn around the focus, not the camera.** The trail window follows the `ElementsFocus` (else the camera), but the terrain and ground materials faded the field by the distance from the camera, so under a camera that stands back from its focus (the isometric camera 1500 m off) every trail was faded out. The materials now take the window's centre from `TrailState::centre`.
- **The crate's atmosphere sits on its planet again.** Bevy reads an `Atmosphere`'s `GlobalTransform` as the planet's centre; the units change of this release gave the spawned atmosphere a `Transform` of scale `u`, and transform propagation then put the planet's centre at the world origin, the camera inside the planet: the `waves` example's sky under `Clear` was a beige dome of radial streaks, reflected in the pond. The atmosphere's `Transform` is the centre `inner_radius · u` below the origin, scaled by `u`, at spawn and when `ElementsUnits` changes (`plugin::tests::the_crate_atmosphere_centres_the_planet_below_the_origin`).
- **Clouds seen from above are no longer khaki.** The cloud march's aerial perspective for a ray going down took the transmittance of a path that met the ground, blue gone; it now takes the pair along the reversed ray (Bruneton's `GetTransmittance`), and the cumulus field below a high camera is white over the haze.
- **A cloud's shadow over the camera no longer darkens the sky.** The sun's and moon's `DirectionalLight`s carried the cloud transmittance of the 70 m around the camera, and Bevy lights its atmosphere with them, so under a partly cloudy sky every cumulus that shaded the camera dimmed the whole sky, the haze over every distant hill, the reflections and every `StandardMaterial` with it — to a black sky in `waves` at 17.5 h and a grey one in `terrain_tour` under `PartlyCloudy`. The lights now carry the shadow map's mean over the 8 km around the camera (Hillaire 2016's hemisphere mean; Kasten & Czeplak's mean over points), and the terrain, water, ground and `WeatheredMaterial` keep their own per-pixel shadow relative to it: walking into a shadow, the ground around you darkens and the sky stays blue (sky luminance steady to 0.9 % over a 20 s `PartlyCloudy` sequence, against 27 % before). Under a full deck nothing changes. The camera's 9 × 9 shadow kernel and its readback are gone. `tests/gpu_clouds.rs` `a_cloud_over_the_camera_leaves_the_lights_alone`, `docs/research/broken-cloud-light.md`.
- **The sun no longer shines through a thick cloud.** The cloud march stops once the view transmittance falls below 1 %, and kept that last 1 % of the sky behind it; the sun disc is about 10⁵ times the sky, so it burned through a cumulus's dark base as a white dot. A march that stops there is now opaque (Schneider & Vos 2015), its scattered light renormalised by at most 1 %.
- **Each domain plugin works on its own.** `WindPlugin`, `FogPlugin`, `CloudPlugin` and `GroundCoverPlugin` read `Climate`, `WeatherDirector`, `CelestialFrame` or `SurfaceWeathering` that only `ClimatePlugin` or `CelestialPlugin` created, so `ElementsPlugins.build().disable::<…>()` panicked on a missing resource; each now inserts, if absent, every resource its systems read. The wind, precipitation, cover and water shaders import the terrain's `terrain::common` library, which only `TerrainPlugin` registered: the wind kernel without a terrain plugin failed to compile and the field never went live. The library is now registered with the shared GPU core. `tests/gpu_plugins.rs`.
- **Ground fog and lightning stand on the terrain.** The ground-fog volume was placed at y = 0–120 m whatever the ground, so over a terrain it hung below the hills or in the valleys' air; its bottom is now the terrain surface under the camera (snapped to the volume's texel plane), y = 0 outside a terrain. Drawn lightning strikes attached at y = 0 and now attach at the terrain height (`docs/research/lightning.md`); scripted `StrikeLightning` keeps its point.
- **A terrain's resolution never exceeds the quality cap.** When the extent and resolution gave non-square cells, the rows were re-derived from the columns' cell without the cap, so 256 × 4096 m at High got 512 × 8192 cells; the coarser axis's cell now binds and both axes are re-derived from it (512 rows, 32 columns).
- **An erosion bake interrupted by a terrain rebuild is restarted.** Rebuilding the hydrology resources (a quality change, a new `Terrain`, the terrain removed and re-added) kept only the queued `ErodeTerrain` requests and dropped the running one, whose `TerrainBaked` was then never written; the running bake now goes back to the head of the queue and restarts in full on the new layers.
- **A terrain change is never lost to pipelined rendering.** The request to repack the terrain's render buffers, bounds and height readback was a shared flag the main world set and the render world cleared after its pass; a set from the next main frame could land before that clear and be wiped, so the last change of a burst (hydrology, snow melt, dust, a voxel edit) was neither drawn nor read back. It is now a counter extracted with the frame (`terrain.md` Deviation 20).
- **The point snow model no longer collapses after the cover map overrides it.** Where the ground-cover map covers the camera it publishes its own `SurfaceWeathering::snow_depth_m`, and the point model then integrated from that public depth: a map depth of 0 over a point pack of 15 mm gave a density of about 1.5×10⁷ kg/m³, so once the map stopped covering the camera the point model read a few micrometres of snow for days. The point model now integrates from its own pack.
- **`ElementsFloraPlugin` works without `WindPlugin`.** Its wind driver read `WindGust` messages that only `WindPlugin` registered, so the `flora` hook alone panicked; it now registers them itself.
- **An inactive camera is not followed.** The environment followed the `ElementsCamera` with the lowest `Camera::order` even when that camera had `is_active: false`; inactive cameras are now skipped (and not counted in the several-cameras warning).
- **Rain and snow stay drawn while the clock is paused.** With `Time<Virtual>` paused the precipitation pass switched off and the particles vanished while the clouds kept drawing; the particles now hold still and stay drawn.
- **The wind readback accepts a larger buffer.** Bevy's readback pool can return a buffer longer than requested; the wind snapshot discarded it instead of decoding the grid's length.

## [0.5.3] — 2026-10-16

### Added

- **The weather can be saved and restored.** `WeatherDirector` derives `Reflect`, `PartialEq` and, with the `serde` feature, `Serialize`/`Deserialize`. Saved with `WorldClock`, `Climate` and `GeoLocation` and inserted into a fresh app, it continues the same scheduler chain, forced preset and blend in progress, where a fresh director restarted at `Clear` and walked a different chain. `docs/research/weather-save.md`, `tests/save.rs`.

## [0.5.2] — 2026-10-16

### Fixed

- **Gameplay queries stay fresh in an app with no window.** Nothing paced an app that renders only to images (a headless server, a simulation tool, every GPU test): its frames were submitted faster than the GPU retired them, the latency of each readback doubled every few requests, and in the ground-cover suite no readback arrived again after three seconds, so `wind_at`, `water_depth`, `snow_depth`, `wetness`, `cloud_shadow_at` and the map-backed `SurfaceWeathering` answered with the state of the first seconds. A frame with no window now waits for its own GPU work, so every readback lands on the next frame; windowed apps, which presenting already paces, are untouched. This also fixes the three GPU tests that failed on `main` since the loop's iteration 4 (`cover::gpu_wetness_soaks_and_dries_on_the_formula`, `cover::gpu_surface_weathering_follows_the_map`, `voxel::gpu_voxel_snow_covers_the_top`). `docs/research/readback-pacing.md`.

## [0.5.1] — 2026-10-16

### Added

- **`Environment::cloud_shadow_at(Vec3)` and `Environment::illuminance_at(Vec3)`**: the cloud's shadow and the horizontal illuminance (lx) at a world position, for stealth, shade-seeking, solar or growth gameplay. The cloud shadow kernel writes every fourth texel of the shadow map into a 256² grid (31.25 m cells over the map's 8 km) that the 4 Hz readback copies; the query projects the position along the light as the ground's material does, and is 1 outside the map. The illuminance is the clear-sky sun and moon light times Kasten & Czeplak's `0.25 + 0.75 · T` at that point. Clouds only: terrain and meshes do not shadow either answer. `docs/research/cloud-shadow-query.md`.
- **`cloud_shadow_probes` example**: a grid of unlit markers coloured by `cloud_shadow_at` (yellow open, blue shadowed) over the cloud shadows on the ground, with the illuminance at the grid's centre on screen.

### Fixed

- **`day_night` now frames the sun and the moon.** The example's camera was aimed west-south-west
  at a fixed bearing, and the vision review measured every body outside that frustum for the
  whole sweep — sunrise at azimuth 54–65° behind the observer, noon at 180°/68° up, sunset at
  304° past the right edge, the waxing moon at 150–175° south: the domain that exists to render
  those discs never showed one. The camera now aims at the sun while it is above the horizon,
  at the moon once the sun is down and the moon is up, and back at the evening heading when
  neither is up; a paused hour (`ELEMENTS_HOUR`) aims once so the still is framed, and holding
  the right button releases the aim.
- **Snow relief reads under an overcast sky.** Saturated snow rendered as a flat wash (0.56 DN
  of standard deviation across 840 k pixels of terrain from the air) because the ambient is
  uniform over normals and the beam is ~0 under cloud. The crate's materials now shade the lit
  colour by the Moon & Spencer (1942) CIE overcast dome — brightest at the zenith, the horizon a
  third of it, integrated with the ground's bounce into the anchors top 1 / side 0.68 / bottom
  0.5 — ramped by the cloud map's transmittance over each ground point, so clear air is an exact
  no-op (controls within dither) and a full deck gets the whole shape. The showcase's boxes and
  columns join its ground on `WeatheredMaterial`, so the shading reaches them: their sides read
  9 DN darker than their tops under Snow while the tops do not move. Under Overcast 559 652
  terrain pixels move ≥ 3 DN (13 726 ≥ 7) where none did before (`docs/research/snow-cover.md`,
  Deviation 15).

- **A dust storm streams.** The dust sprites were drawn at their 2 mm grain size — 0.02–0.1 px
  anywhere in the 60 m box — so the sub-pixel coverage rule faded them out and the storm read as
  static fog (zero pixels below 150 against a 224 background across a 1150 × 140 sky band).
  Sprites are now drawn at a 0.35 m cluster scale with their own 0.5 opacity (below a flake's
  0.8, so a cluster sprite is a veil, not a puff); the grain size, the sprite extinction, the
  counts and the motion are unchanged (`docs/research/dust.md`, Deviation 15).
- **The horizon shows no dark band where the ground ends before it.** Past the edge of a finite ground (quickstart hills from 6 m eye, any terrain that does not reach the horizon) the below-horizon sky drew Bevy's unlit planet term: `ground_haze` fogged over the bare planet's near root — 23 m at 5° below the horizon from a 2 m eye, a transmittance of ~1 — so a 45-row band read 157–171 against sky 243 and fogged ground 229 under `Snow` and `Rain`. The fog distance is now floored at the march range (32 km): a below-horizon pixel with no mesh shows ground the fog has fully carried, because nothing nearer than the far field is visible beyond a finite terrain. New GPU test `the_horizon_sky_is_the_fog_colour_under_a_deck` pins both sides of the horizon (`docs/research/clouds.md`, Deviation 28).
- **Snow now lies on the ground the examples show, and a still can show it.** `weather_tour` under `Snow` and `Blizzard` rendered olive-drab ground where the boxes and columns beside it were white: the harness's 2 km plane was a bespoke material that sampled only the cloud shadow map. The examples' ground is now `WeatheredMaterial` — it darkens and glosses in rain, runs with streaks and whitens under snow as a consumer's mesh does — and its plane reaches 40 km, so the ground meets the sky without the sliver of unlit below-horizon atmosphere a 2 km plane leaves at the horizon (a 66 DN drop to a band floor 16 DN below the ground under `Overcast` and `PartlyCloudy`, now gone). A held clock (`ELEMENTS_HOUR`) integrates no game time, so no material could ever whiten in a still; `ELEMENTS_GROW_HOURS=n` now runs `n` game hours of the held preset before the first frame and rewinds the clock, leaving the hour, sun and sky as the example's own (`docs/research/snow-cover.md`, Deviation 14; `docs/research/clouds.md`, Deviation 28).
- **The sky carries the weather's air, so a whiteout has no horizon line.** The distance fog fogs every mesh to the fog colour as it recedes, but the sky kept the cloud deck's own radiance: under `Blizzard` the horizon read as a 40-level step (sky `(186, 193, 198)` above fogged ground `(224, 228, 230)`), and the same step showed on real terrain and from the air. The composite now fogs sky pixels by `σ_air = σ_fog − 3.912/40 km`, read from the camera's own `DistanceFog` and with the clear-air floor removed, so the sky converges on the same fog colour the ground does, and `Clear` (40 km) is a bit-exact no-op. `fog::sky_air_extinction` is the one authority for the floor (`docs/research/clouds.md`, Deviation 29).
- **The ground-fog volume no longer darkens the sky it cannot light.** Bevy lights a `FogVolume` from the `DirectionalLight`s alone, which under cloud carry only the beam `T̄` — zero at full cover. The volume therefore removed the sky's light and returned none of it, so a short `Weather::visibility_m` turned the whole sky dark: a noon blizzard rendered as a dark hole (measured zenith `(46, 47, 50)`). Its density now fades by the beam's share of the scene's light and its intensity rises by the reciprocal, so it keeps its radiance-to-opacity ratio and simply fades out where Bevy cannot light it; the sky and the distance fog draw the whiteout (`docs/research/fog.md`, Deviation 7). `Clear` and thin-cloud presets are unchanged.

## [0.5.0] — 2026-10-14

Everything from 0.2 through 0.5 lands in one release. Every system was studied in a committed research note first (`docs/research/`, one per system: the papers, the equations adopted, every deviation with its source, and the numbers the tests assert).

### Added

- **Heightfield terrain (`TerrainPlugin`).** A `Terrain` from procedural presets (`hills`, `mountains`, `dunes`, `plains`), a height map or level ground, generated on the GPU into per-cell layers and drawn with a CDLOD patch pool (Strugar 2009) and a slope/sediment layer-blending material; `TerrainRender::External` still simulates when your own mesh holds the surface, shaded through `GroundExtension`. `Environment::terrain_height` reads the height back.
- **Hydrology and erosion (`HydrologyPlugin`).** Flooding, rivers and lakes as one shallow-water field on the terrain grid (Mei et al. 2007 virtual pipes with a depth-dependent pipe area and Manning friction), `WaterSource` and `WaterSink`, rain split into rain and snow per cell, hydraulic erosion with flux-form sediment transport, thermal erosion that leaves rock faces standing on game time, and `ErodeTerrain` bakes of simulated time. `Environment::water_depth` and `water_velocity`.
- **Water rendering (`WaterPlugin`).** The water drawn over the terrain by a second CDLOD patch pool: a per-channel Beer–Lambert *absorb* pass with thickness from the depth prepass corrected for refraction, and a *surface* pass with the Fresnel reflection of the sky, the silt body colour, foam and flow-mapped ripples (Vlachos 2010) from the simulated velocity; the sun's path through the water shades the bed.
- **Snow and wetness maps (`GroundCoverPlugin`).** Snow as a depth and water equivalent per cell, stepped on game time: each cell's own rain–snow split, slope shed and blocker shelter, densification, wind drift, smoothing, and degree-day melt that arrives in hydrology's water exactly (Hock 2003). Wetness from rain and evaporation. The terrain and ground materials shade snow, wet darkening and gloss, and puddles with ripples; `SurfaceWeathering` (and so `WeatheredMaterial`) follows the maps' mean around the camera. `Environment::snow_depth` and `wetness`.
- **Weathered materials (`WeatheredPlugin`).** `WeatheredMaterial` for any mesh: wet darkening and gloss, rain streaks, up-facing snow on flora's curve, and the cloud shadows, fed by one shared `ElementsGlobals` buffer with a per-material `exposure` for shelter.
- **Dust storms (`DustPlugin`).** Aeolian transport on the terrain's sediment layer: Paris et al.'s capacity with Venturi speed-up and a lee shadow, a flux line scan that conserves sand exactly, and hydrology's thermal kernels avalanching at the sand's repose angle; dust sprites in the precipitation buffer and a fog tint while `Weather::dust` is up.
- **Snow on voxel volumes (feature `isomesh`, `VoxelPlugin`).** `VoxelGround` on a `bevy_isomesh` `VoxelVolume` scans the column tops into a crate-internal `Terrain` (`TerrainRender::External`) the snow, wetness and hydrology run on, and a `SnowyField` adds the snow depth back as a slab over the tops, swapped per column when it moves half a voxel and remeshing only the chunks that changed; tunnels stay clear. Erosion is off for voxel ground. Needs `isomesh`/`bevy_isomesh` 0.0.11.

### Changed

- Overcast light: the sun and moon `DirectionalLight`s carry the cloud's beam transmittance and the diffuse skylight remainder is the camera `AmbientLight`, so object shadows soften under cloud; the sky reads grey, not blue.
- The far field of the clouds is lit and seen through the air exactly as the marched deck is at the fade boundary, and a ray that meets the planet past the edge of a finite ground draws the camera's distance fog instead of a cloud.
- Terrain layer blend: Andersson-style Overlay detail on wider ramps, so rock, soil and grass meet in irregular mottled transitions instead of iso-slope blobs.
- Snowflakes are soft discs of bounded elongation, not dashes; snowflakes' streak is capped at one diameter.
- `profile.dev` sets `debug = 0`: Bevy examples' debuginfo links OOM CI runners.

### Examples

New: `terrain_tour`, `external_ground`, `flood`, `lake`, `erosion_bake`, `weathered_town`, `ground_cover`, `dust_storm`, `voxel_snow` (feature `isomesh`), and `quickstart` now stands on hills. `bench_elements` times every pass, with `ELEMENTS_TERRAIN=hills|mountains|dunes|lake|none`.

[0.5.1]: https://github.com/Ladvien/bevy_elements/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/Ladvien/bevy_elements/compare/v0.1.0...v0.5.0