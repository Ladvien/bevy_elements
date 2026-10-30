## [Unreleased]
### Fixed

- **Rainbows are drawn by rain, not by falling snow (#51).** The bow's gate, drop size and curtain took `Weather::precipitation_mm_h`, the water equivalent of rain and snow together, so a full double bow stood over falling snow opposite a low sun, and sleet drew the bow of its whole rate. The gate and drop size now take the rain, `precipitation_mm_h · (1 − snow_fraction())`, and the curtain shares its light between rain and snow by their extinction (the crate's laws, snow some twenty to thirty times as opaque per mm/h), so only the rain's part makes the bow. Measured on the Apple M1 Max (`tests/gpu_rainbow.rs`, 5 mm/h, sun 21.8°): at −5 °C the primary's band adds 0 (was 0.067 on a sky of 0.59, the frame now byte-identical with the bow off); at 0.5 °C (half snow) 0.0091, an eighth of the rain's 0.071 (was 0.068); rain is unchanged (byte-identical frame). The bow fades continuously through the sleet band instead of vanishing at −1 °C. `CloudParams` gains `shower` (a uniform tail; no API change). Assumed decision on #51. `docs/research/rainbow.md` Deviations 4 and 5.
- **`main` builds again.** The merge `f0ffdc6` kept both copies of what it joined: two re-exports of the cloud and lightning items, a second `perlin3_periodic` and `fbm3_periodic` in `noise.wesl`, a stale `sky_dimming` and its test in `celestial`, and older Lightning and Precipitation README sections. They are removed; `README.md` and `src/` are as they were before the merge.

### Changed

- **The night sea's horizon is closer to the sky above it.** The night fill's lift now rises from 20° to 60° over the horizon (was 8° to 40°) and keeps 5 % of itself at the horizon (was 15 %), so a sea at a grazing view reflects a lobe that no longer reaches the lifted sky: the sea at midnight is 2.3x the sky just over the horizon (was 4.1x). A horizontal surface's night irradiance is unchanged (3.26e-3 against 3.29e-3); a vertical one gets about 0.50 of the uniform lift (was 0.69). `docs/research/night-sky-light.md` Deviation 6.
### Added

- **`SurfaceCover`: a fixed snow and wetness for one material, with a CPU query (#49).** `WeatheredExtension::with_cover(SurfaceCover)` / `set_cover(Option<..>)` replace the scene's snow and base wetness on that material with two `CoverBand`s (amount, steepest slope with a soft edge, height window with a soft edge), so a volume's mesh (or any prop, or a preview) wears snow and wet without the weather; `None` returns the scene's weather, and mutating the live asset hot-reloads it. `SurfaceCover::snow_at` / `wetness_at` answer the same coverage per normal and height with no GPU. `serde` derives with defaulted fields. The voxel snow slab (geometry) still follows the weather. `docs/research/surface-cover.md`.
- **Rainbows.** A shower opposite a low sun draws Airy's primary and secondary bows in the sky composite (`RainbowSettings::strength`, default 5, 0 switches it off; `CloudPlugin` inserts it). The phase function is built at start-up from Airy theory over a Marshall–Palmer drop-size law (a 512 × 16 `Rgba16Float` table over 110–170° and median drop radii of 0.02–1.2 mm, 21 wavelengths, the sun's disc blurred in): red outside and violet inside, reversed in the fainter secondary, Alexander's dark band between, a pale wide bow for a drizzle and a narrow vivid one for a heavy shower. The curtain's optical depth is read from the cloud weather map along the ray (taps at 1.5, 3, 6 and 12 km, under the cloud base), the sun's light is the camera's, through the atmosphere and the cloud's beam, so a sunset bow is red-gold and a high sun, a dry sky or an overcast one draws none. One more branch in the composite fragment, none outside the 110–170° ring. `docs/research/rainbow.md`, `tests/gpu_rainbow.rs`.
- **`Environment::terrain_normal` and `terrain_slope_deg`.** The ground's unit normal and slope at a world XZ, from the one gradient the trail kernel and `sinkage_at` already use (`docs/research/terrain-normal.md`); unit-free, so a foot world answers as a metre world. `None` off the terrain and before the first readback.
- **`RestoreTerrain { snapshot, region_m }`: put a region of the live terrain back as a snapshot held it.** The undo of a Smooth, Flatten or Noise stroke (no `SculptTerrain::inverse`), or of any stroke, without respawning the terrain and so without losing the water, the cover maps or the trails. Cells whose centres lie in the metre-frame `Rect` take the snapshot's bedrock (`surface - sediment`) and sediment back; the rest are untouched. Applied in order with strokes, one edit per frame; the render world writes the rows with `write_buffer` ahead of the pack, bounds and readback kernels (no new kernel). A snapshot from another grid (or the coarser readback above 1024² cells) is dropped with a warning; `validate()` checks the rect. For bevy_emerge #99. `docs/research/terrain-restore.md`.

## [0.7.6] - 2026-10-30
### Added
- **`ProceduralTerrain::sample_surface(size, cell_m)`: a preset's surface on the CPU.** Row-major heights, m, of the generator `generate.wesl` runs, with no app, no GPU and no `TerrainPlugin`: a tool can bake `hills(seed)` into a height file, a game can preview or test a preset. Deterministic per seed; agrees with the kernel within the sampling tolerance of `terrain.md`, not bit for bit. The test mirror's generator and its lattice noise moved out of `#[cfg(test)]` to make it public (`terrain/model/surface.rs`, `gpu/perlin.rs`). Fixes #48 (bevy_emerge #99). `docs/research/terrain-surface-cpu.md`.

## [0.7.5] - 2026-10-30
### Added
- **`Environment::terrain_snapshot()`, `TerrainSnapshot::to_heightmap()`, `terrain()`, `height_range_m()`, `centre_m()`: save a sculpted ground and respawn it.** The snapshot is the crate's last height readback (surface and sediment, metre frame); `to_heightmap` is an `R32Float` image plus the range that decodes it, `terrain` a `Terrain` over it at the grid's extent and resolution, `centre_m` where its entity belongs. The surface comes back within f32 rounding (GPU test: 2e-4 m after a 5 m raise); the sediment layer is not restored. For bevy_emerge #99 (saving a project's sculpt, undoing Smooth, Flatten and Noise strokes). `docs/research/terrain-heightfield-save.md`.

## [0.7.4] - 2026-10-29
### Added
- **`SculptTerrain::inverse()`**: the stroke that undoes a `Raise` or `Lower` (the other kind, same amount, centre, radius, hardness); `None` for `Smooth`, `Flatten` and `Noise`, which depend on the ground they met and are undone from a snapshot of the heights. For an editor's undo stack (bevy_emerge #99).

## [0.7.3] - 2026-10-29
### Added

- **`SculptTerrain`: height brushes on the live terrain.** A message with a metre-frame centre, a radius, a hardness (plateau fraction; a smoothstep fall to the rim) and a `SculptKind`: `Raise`, `Lower`, `Smooth`, `Flatten { height_m, blend }`, `Noise { amplitude_m, feature_m, seed }`. A new kernel (`sculpt.wesl`) edits the bedrock layer at the head of the terrain's compute pass, so the pack, leaf bounds and height readback of the same frame carry the edit; cells at or beyond the radius are bit-identical. `SculptTerrain::validate()` returns the `SetupError` naming a bad field, `sculpt_weight` is the brush weight. Built for bevy_emerge #99 (terrain brushes, pieces following the ground); a game can dig and landscape with it. `docs/research/terrain-sculpt.md`.

## [0.7.2] - 2026-10-29

### Added

- **`SurfaceSetup` and `TerrainSetup`: the ground, water and cover settings, and the terrain's source, as saved, validated, live-appliable values.** `SurfaceSetup` bundles `HydrologySettings`, `ErosionSettings`, `SnowSettings`, `WetnessSettings`, `WaterSettings`, `WaveSettings`, `RockSettings`, `DustSettings` and `TrailSettings`; `validate()` returns the same `SetupError { field, problem }` as `EnvironmentSetup`, `apply_to_world(..)` validates first, changes nothing on error and writes a resource only when it differs (an unchanged re-apply does not rebuild the rocks' scatter), and `capture_from_world(..)` reads the running settings back. `TerrainSetup { source, extent_m, resolution, external }` describes a `Terrain` as data (`TerrainSetupSource::{Procedural, Flat, Heightmap { path, height_range_m }}`): `terrain(&AssetServer)` validates and builds the component, loading a height map linearly as one channel, and `capture(&Terrain)` reads one back. Built for an editor that sculpts and previews a map's ground and saves it as a file (`docs/research/surface-setup.md`).

### Changed

- `WaterSettings`, `TrailSettings` and `WaveSettings` derive serde (feature `serde`), and every settings struct in `SurfaceSetup` deserialises with `serde(default)`, so a file may give one field of a section. Previously a partial section was a missing-field error.

## [0.7.1] - 2026-10-29

### Added

- **`EnvironmentSetup`: the sky and weather as one saved, validated, live-appliable value.** It bundles `WorldClock`, `GeoLocation`, `Climate` and the forced weather preset (with `serde`: any format, absent fields default, unknown fields are an error). `validate()` returns a `SetupError { field, problem }` naming the field that broke its range; `apply(..)` / `apply_to_world(..)` validate first, change nothing on error, and force the weather only when the forced preset differs, so reloading an unchanged file does not restart a blend; `capture(..)` / `capture_from_world(..)` read the running state back to save. New `WeatherDirector::forced()` reads the forced preset the director kept private. Built for an editor that previews a map's environment live and saves it (`docs/research/environment-setup.md`).

## [0.7.0] - 2026-10-29

### Fixed

- **A moonless night's sea no longer glows over a black sky.** The sky cube carries the night sky's light times the camera's exposure so that surfaces are lit (`docs/research/night-sky-light.md`, Deviation 2), and Bevy multiplies the cube by the exposure again, so a specular reflection of it was lifted twice: a `StillWater` sea at a grazing view drew 42-80 DN (about 100x) under a sky of 5-8 DN, a grey slab with a hard line at the horizon. The lift now fades in with elevation (8 to 40 degrees, `night_fill` in `march.wesl`); toward the horizon the cube is the sky the screen draws, and a normalisation keeps a horizontal surface's irradiance as it was (3.283e-3 against 3.286e-3 measured). The sea at midnight is 4.1x the sky above it (was ~100x). A vertical surface's night fill falls to about 0.6 of what it was, so a wall reads darker than a roof under a moonless sky. Measured on the Apple M1 Max (`gpu_a_night_sea_is_not_brighter_than_the_sky_it_reflects`, `the_night_fill_lifts_the_light_on_surfaces_and_not_the_horizon`).
- **The distance fog's colour is the sky's horizon, so distant ground, the sea and its skirt meet the sky without a line at any hour.** `DistanceFog::color` was a fixed hue times the scene's illuminance; the sky at the horizon follows the sun, so the far field showed a 1-px bright or dark line at the horizon whose sign flipped with the hour. A third entry point of the cloud march shader (`cloud_horizon_ring`, one workgroup, run after the sky probe) now computes the sky's horizon radiance in 32 azimuths (the same `horizon_sky` the below-horizon slab reads); the throttled readback carries it to the main world and each frame the fog takes its average toward the camera's heading (cos² weights), times the exposure. The share is `(1 − air)(1 − cloud_cover)`, so a deck, fog or dust keeps the weather's own colour, and a camera with no probe or before the first readback keeps the analytic colour. Measured on the Apple M1 Max (`tests/gpu_still_water.rs`, `gpu_the_far_sea_meets_the_sky_without_a_line`): the largest one-row line at the horizon of the edge scene fell from 9 / 18 / 9 levels at 12 / 19.5 / 21 h to 0.3 / 1.6 / 3.5. Not changed: at midnight the sea is still a grey slab under a black sky (the ground and water are lit by the night fill, the sky is not). `docs/research/fog-colour.md`.
- **Water seen through fog is no longer tinted by its own absorption.** The absorb pass multiplied the fogged bed, so the fog in front of the surface took the water's teal: a `Fog` preset drew a saturated teal sea (chroma 0.83) under a grey sky. The surface pass now fogs its film and adds back the fog share the absorb pass removed (`hydrology/water.wesl`); the same sea reads 0.145. Applies to simulated water and `StillWater` alike.
- **The terrain's border is no longer a line through a shallow `StillWater` sea (#44).** A bed that ends at the grid's border showed through clear water as a hard teal-to-deep-blue step, and the in-grid water joined the skirt with a 8-10 level step. While a `StillWater` is set, a bed point within 32 m of the grid's outermost samples now fades into the deep colour (the absorb transmittance and the surface body alpha, with the share of camera fog the bed no longer carries applied in the film). Measured on the Apple M1 Max (`tests/gpu_still_water.rs`): the largest adjacent-row step of the edge scene's centre column fell from 0.085 to 0.023 in linear radiance. Hydrology water is untouched.

### Added

- **A `StillWater` sea runs past the terrain grid's edge to the horizon (#44, fifth slice).** Four extra water tiles (`WaterSkirt`, two passes each, drawn only while a `StillWater` is set) carry the sea 100 km beyond the grid's last samples at the depth the clamped edge ground gives (`level − edge ground`), through the same absorb and surface passes, and take the camera's distance fog (they have no bed behind them to carry it). The cloud composite leaves the pixels whose ray meets the still plane inside that region to the water: before, a pixel with no mesh below the horizon was painted with the fogged-ground slab, which a water pass could not cover. `CloudParams` gains `sea` and `sea_box` (a uniform tail; no API change). Measured on the Apple M1 Max (`tests/gpu_still_water.rs`): from 170 m inside the edge, the largest step between adjacent rows under the horizon is 0.085 with the skirt and 0.40 without it (the ground disc shows past the edge), and a foot world draws the same column as the metre world. Past 100 km the composite's ground slab shows as before.

- **The hydrology idles under a `StillWater` with nothing wet (#44, third slice).** With a `StillWater` set, no bake, no rain (`Weather::precipitation_mm_h` ≤ 0), no `WaterSource`, and a depth readback that found the terrain dry (mean depth under 5 µm, the crate's delivery quantum), `run_hydrology` schedules zero substeps and clears the scheduler's backlog, so a still sea costs the simulation nothing and resuming does not burst. Rain, a source, a bake or water on the ground resumes it. Thermal slumping of loose sediment is part of the substeps and pauses while idle. Measured on the Apple M1 Max (`tests/gpu_hydrology.rs`, `a_still_sea_with_nothing_wet_runs_no_substeps`): 0 substeps over 60 frames at `time_scale` 1 after the first readback, more than 0 within 60 frames of 30 mm/h rain. Still not done: a sea past the terrain grid's edge, a foot-world GPU check.
- **`WaterContact` rings and wakes on `StillWater` (#44, second slice).** The wave kernels' `water_at` takes the higher of the simulated depth and the still level over the ground (one lane, `WaveParams::sim.w`), so a body dropped on a still sea presses it, sends out rings and leaves a wake, and the shore mask follows the still shoreline. Measured on the Apple M1 Max (`tests/gpu_waves.rs`, `a_body_dropped_on_a_still_sea_sends_out_rings`): a 0.25 m sphere dropped on a 1 m still level with no simulated water raises 31.7 mm of wave at 1 m within a second (0 before; the issue asks for 1 mm). Still not done: a hydrology that idles with only a still level, a sea past the terrain grid's edge, a foot-world GPU check.

- **`Environment::sinkage_at(position, half, load_n)`: the depth a step would press (#39).** The trail kernel's own Bekker–Wong sinkage (CRREL-capped in snow, sand, soil to mud by wetness, nothing on rock) evaluated on the last readback's ground, with an `Imprint`'s pressure and plate width, so a game gates its footprints on the depth the ground will take and not on a copy of the physics. It takes the contact's half extents as well as the load (the issue's `(position, load_n)` has no area to derive a pressure from). To carry the snow's density the cover readback is now `vec4` per cell (depth, wetness, water equivalent, spare): 16 B per readback cell instead of 8. Measured on the Apple M1 Max (`tests/gpu_trails.rs`): the answer is the kernel's target (snow 0.2400 m, wet soil 0.0379, dry soil 0.0048, dry sand 0.0339) and the trail field's deepest texel sits 0.1–3.6 % under it (0.2365, 0.0373, 0.0047, 0.0327), never over; the issue's 1 mm holds on soil and not in snow (3.5 mm) or on sand (1.2 mm). A `TrailGround` a game supplies is not read. `paths_resist_prints` is not built.

- **`Environment::ground_look_at` and `GroundLook`: what the ground's cover is at a point, in the shader's own terms (#41, first slice).** `GroundLook { wet, puddle, snow, drawn_wet }` is the wetness map, the puddle mask of the simulated water depth (`WetnessSettings`' thresholds), the snow cover, and the wetness the surface model wears on bare ground (`min(max(wet, puddle), 1.4)`), from the last readback. A host's decal fades with `drawn_wet` and agrees with the ground it lies on. Not in it: the grass canopy and the rut-floor puddle. `FootprintPressure` (a ramp type) is not built.

- **`StillWater`: a body of water at a fixed level, drawn by `WaterPlugin` and answered by `Environment::water_depth`, with no simulation (#44, first slice).** `StillWater { level }` (a resource, world units) draws water of depth `level − surface` wherever the terrain is below it, through the existing absorb and surface passes; the vertex and fragment stages take `max(simulated depth, level − surface)`, patch selection adds the patches whose lowest ground is under the level, and a patch's box is raised to it. The level is one uniform lane, so changing it rebuilds nothing. `Environment::water_depth` returns the higher of the simulated depth and `level − ground`, so it needs no hydrology. Measured on the RTX 3090 (`tests/gpu_still_water.rs`): over a flat bed, mean red under 0.3 m is 15.5× the mean red under 8 m (Beer–Lambert alone predicts 32×; the bed's own wet shading and the sun leg are in both), and the depth answers 0.300 m and 8.000 m. **Not done, so #44 stays open:** `WaterContact` rings (the wave kernel reads the simulated depth), zero hydrology substeps with only a still level, and the sea past the terrain grid's border. The `terrain_tour` example takes `ELEMENTS_STILL=<level>`.

- **`stone_mesh`, `stone_facet_tones`, `stone_palette`, `STONE_SHAPES`, `StoneDetail`: the facet-dressed stones for a host's own scatter (#43).** `stone_mesh(shape, StoneDetail::Near | Far)` is the mesh `RocksPlugin` draws (1280 or 320 triangles, long axis X one unit long, vertex colours carrying each joint facet's palette tone, the mottle and the base's darkening); a host spawns it with its own `StandardMaterial` (vertex colours multiply `base_color`) and scales it by the stone's length. `stone_facet_tones(shape)` is the per-facet table both meshes read and `stone_palette()` the four tones. None needs `RocksPlugin`; `shape` wraps at `STONE_SHAPES` (12). The scatter's own meshes are unchanged (same builder).

- **`RockSettings::allowed`: keep the scatter to a host's region (#43).** `Some(rects)` (`bevy::math::Rect`, x = world X, y = world Z, metres) keeps a rock only where its position is inside at least one rectangle; `Some(vec![])` keeps none and builds none; `None` (the default) is the old scatter. The kept rocks are the unrestricted scatter's own, in the same places, so a region moves none and a saved game's rocks stay put; a rock inside the region but under an orthographic view's diameter threshold is still not drawn. A change rebuilds the tiles. Measured on the RTX 3090 (`tests/gpu_rocks.rs`): 133 rocks unrestricted, 126 of them inside a 120 m block, 126 kept with the region, 0 with an empty one. **Migration:** a `RockSettings { .. }` literal needs `..default()`. The per-facet palette export for a host's own rocks, the issue's second ask, is not built; #43 stays open.

- **`TrailSettings::min_depth_m`: a host sets the shallowest print that is pressed (#39).** An imprint whose Bekker–Wong sinkage on the texel's own ground is below it leaves no trench and no rim, so a game that wants prints only on soaked soil, sand and snow sets 0.01 and no longer duplicates the crate's wetness maths; snow deeper than 1 cm always presses, dry sand (3.4 cm) keeps its prints. Default `0.0` is the old behaviour. **Migration:** a `TrailSettings { .. }` literal needs `..default()`. `paths_resist_prints`, the issue's route rule, is not built (`Environment::sinkage_at` is, above).

- **`TerrainLook::detail` and `TerrainLook::finest_m`: a host steers the ground's grain (#42).** `detail` (default 1) multiplies the albedo grain's and the bump's amplitude in both bands and the route grain's; 0 leaves the palette's colour and the hill's shape with no grain and no bump, and the turf palette's clumps (colour, not grain) stay. `finest_m` (default `None`) is the width in metres of one screen pixel on the ground as the game's camera knows it: where it exceeds the footprint the crate derives, it replaces it in the ground detail's and the route grain's band limit, and above the coarse band's feature it drops that band too. Both ride the material's uniform like the tint, so a write is not a pipeline compile. The defaults multiply by exactly 1 and floor at 0. **Migration:** a `TerrainLook { tint, saturation }` literal needs `..default()`. The example harness reads `ELEMENTS_DETAIL` and `ELEMENTS_FINEST_M`. Not included: the issue's band-by-band debug view.

- **`Environment::terrain_drawn_height` and `TerrainGrid::drawn_height`: the height of the drawn triangles.** `terrain_height` is the bilinear of the cell centres, but the level-0 patches draw two triangles per quad (split along `(1, 0)-(0, 1)`), which differ inside a quad by up to a quarter of its twist: 0.7 ft at a crease on Sanctuary's 9.375 ft town grid, 0.15 % of points over 0.15 ft. The new query follows the mesh, equals the bilinear at every cell centre and quad edge, and is continuous across quads (#40).

### Fixed

- **Despawning the terrain entity no longer logs one WARN per pooled patch (#47).** The patch pool's slots are the terrain's children, so a host's despawn of the terrain already took them, and `clear_pool` then despawned each remembered slot with `despawn()`, which warns when the entity is gone: 256 `Entity despawned` lines per restart at `High`. It uses `try_despawn()`; a pool rebuilt under a live terrain still removes its old slots. Test `terrain::pool_tests` counts the ECS warnings (8 for 8 slots before, 0 after) with a `log` dev-dependency.

- **The `trails` example lays Stone and packed-snow routes** (`ELEMENTS_PATHS=stone|packed`), so every `PathKind` has a capture path. Captured on the RTX 3090: Gravel and Stone routes draw after the #38 fix (a grey road, a pale slab band on turf); a Snow route is a faint compacted band in snow.

- **`tools/wesl-check` now compiles the 11 shaders that import from Bevy.** It reported
  `celestial::moon`, `celestial::stars`, `clouds::{composite,march,resolve,shadow}`,
  `hydrology::water`, `precipitation::draw`, `terrain::{ground,render}` and `weathered::weathered`
  as "not checked (module not found)" and exited 0, because WESL joins `import bevy_pbr::…` onto the
  importing package (`bevy_elements/bevy_pbr`) and the router never asked the stubs. Each Bevy
  namespace is now mounted under both spellings, the stubs declare Bevy 0.20's own structs and
  function signatures (`View`, `PbrInput`, `StandardMaterial`, `VertexOutput`, `apply_pbr_lighting`, …),
  and a missing module is a failure, not a benign "not checked". All 44 modules compile; a wrong call
  into Bevy or a missing field fails the check with exit 1. Fixes #37.

- **`PathKind::Gravel`, `Stone` and `Snow` routes draw.** `paths::mask_image` clamped the kind lane to 0..1
  where the shader reads it over 0..4 (`texel.y * 4.0`), so every kind above `Earth` encoded as 255,
  decoded as kind 4 and painted nothing (#38). The lane is now stored `/ 4`, rounded up so `floor` keeps
  kind 3, with a rut-depth fraction held at 0.97 so a deep rut cannot change a route's kind.

- **The air past the ground is the sky's own column, not the fog's hue.** Where a frame's ground does
  not reach the horizon, the crate drew a flat slab in the camera's `DistanceFog` colour, a fixed
  `(0.75, 0.80, 0.85)` scaled by horizontal illuminance, while the sky at the horizon is Rayleigh- and
  Mie-extinguished air whose radiance and hue follow the sun's altitude and azimuth — so the two
  diverged exactly when the sun was low (measured 116 DN low at 19:00, 130 DN in the worst channel at
  dusk). `ground_haze` now takes the sky, sampled by `horizon_sky(dir)` from the sky cube in the ray's
  own azimuth above the horizon, and the weather's air mixes it toward the fog colour by its own
  extinction, the law the ground below already obeys. The GPU test asserts the invariant: the sky above
  the horizon and the slab below are the same colour in three azimuths at five hours, within 8 % of
  luminance and 0.15 of warmth (measured on `big`: 1.4 % at noon, 1.0 % at golden hour, 1.7 % at a low
  sun, 7.3 % just after sunset).
  `docs/research/horizon-colour.md`, `262207d`.

- **The night horizon is not a grey sea, and the dusk horizon has no bright lip.** The sky cube carries
  a night fill so the *lighting* cube is not black at midnight; read back into the slab it drew
  (9.4, 12.5, 15.5) DN under a sky of (1.3, 1.0, 1.0) — a visible grey band across every night horizon,
  uniform to a per-channel sd of 0.00. That
  fill is no longer added, and the horizon's radiance is extrapolated to zero elevation from two taps
  half a degree apart, because a single tap a little above the horizon reads the twilight gradient (the
  slab's lip was 10.6 DN bright at 20 h). Midnight now reads (0.7, 0.7, 0.7) against (1.9, 1.0, 1.0), and
  the 20 h lip 2.2 DN. The GPU test covers midnight, where a black sky must leave a black slab.

## [0.6.0] — 2026-10-27

> **Published tarball note.** The crate on crates.io also carries `262207d`, "the air past the ground is
> the sky's own column" — the horizon slab now samples the sky in the ray's own azimuth instead of using
> one fixed fog hue (a 116 DN error at dusk before it). That fix was uncommitted when the crate was
> packaged, so it is in the published source but landed after tag `v0.6.0`; it is gated here (380 tests,
> 0 failing) and is in `[Unreleased]` below. Tag and crate differ only by that commit.

Nineteen days and 197 commits after 0.5.3. The four things a game will notice:

**Clouds are lit properly, and the sky is not one colour.** Multiple scattering by diffusion makes a
cloud white rather than grey, cumulus billow, twilight exists and the sunset is no longer blown out, the
night sky is blue and lights the ground instead of being a black screen, and a cloud's shadow over the
camera no longer darkens the sky. Clouds also appear in the environment map, so reflections and sky
light under cloud agree with what is drawn overhead.

**Roads, and trails.** `PathNetwork` / `PathsPlugin` rasterise a route into an 8-bit coverage distance
field that the terrain material reads per fragment: turf gives way to compacted earth with the route's
own grain, a dish cross-section whose verge catches a grazing sun, snow cleared where traffic runs, and
each wheel line's rut. That block compiles only into a terrain whose network has routes, so a terrain
without paths pays nothing (measured within ±2 % of a no-network scene). `Environment::surface_at`,
`nearest_path` and `on_path` answer a game's questions about them. Separately: trails — footprints, ruts
and tracks in snow, sand, soil and mud — and waves, ripples and wakes from bodies in the water.

**Rock is a material, not a colour.** A rock face carries a palette of different rock, a succession of
bedded layers, two families of joints, fractures, a grain and a relief of its own; a cliff breaks into
irregular fragments of many sizes and a distant range is many rocks. Scattered stones take the cliff's
palette per facet rather than one tone over the whole body — following the shape's own joint planes, so
the colour changes at the break rather than across the face.

**Night, cameras and the ground under them.** A game lights its own night; `ElementsCamera` gained
fields beyond `auto_exposure`; a focus now drives the camera-relative domains, which is what lets an
orthographic camera work at all; and a snowpack survives a save.

**Breaking.** `TerrainSnapshot` carries the sediment; `ElementsGlobals` is 128 bytes;
`CloudShadows::camera_transmittance` is now `mean_transmittance` (the shadow map's mean over its 8 km);
`WindState::at_camera` is `at_focus`; `TrailSettings` has a `texel_m` field; `Weather` has
`snow_possible`; `SnowyField` takes the nearest column's top and snow across a cliff. The `flora`
feature depends on `bevy_flora` 0.46.

The full, unchanged per-change record follows.



### Changed

- **A scattered stone wears the cliff's palette per facet, not one rock over its whole body.** Every
  vertex of a shape carried one tone, so a boulder below a cliff was greyer than the face it fell
  from — its top facet's saturation 0.020 against the face's 0.15 at the same light — and the hue it
  showed was the light's rather than the rock's. The tones now follow the shape's own **joint
  planes**: the soft minimum's carrier names each vertex's facet, and a vertex's tone blends over
  the joint's own narrow width (0.004 of the shape's radius), so a facet keeps its rock over its
  whole face and the colour changes at the break. Each shape's area-weighted brightness is
  normalised to the palette's 1 by a scalar, so every rock keeps its hue and the scatter keeps the
  ground's brightness. The mesh stays indexed and nothing per pixel changed; the palette table is
  the **shape's**, shared by both levels of detail (see Fixed).
  `docs/research/stone-material.md`.

### Fixed

- **A scattered stone no longer changes colour where its mesh switches.** The facet palette was
  derived from the mesh's own triangle areas, so the near (1280-triangle) and far (320-triangle)
  mesh of one rock were handed **different** palette tones — 19° of hue apart at the worst vertex,
  3.7° on one shape's mean. The meshes switch abruptly at `22 · D`, so a stone changed colour as the
  camera moved, which the eye reads as the rock flickering. The tone table is now computed once per
  shape from its canonical tessellation and stored on `RockGeometry`, so both meshes wear the same
  rocks to the bit; the near mesh is unchanged. `docs/research/stone-material.md`.
- **Scattered rocks are drawn under an orthographic camera.** An isometric game saw none of them:
  every rock was seated, then shrunk to nothing, because the fade measured from `ElementsView::eye`
  and an isometric camera stands 457 m back from everything in view (a 0.25 m stone's range is 25 m).
  Bevy's `VisibilityRange` culls by the same camera distance, so even a rock that kept its scale
  would not have drawn. The measure is now the view's own: under a perspective camera the
  per-metre range is unchanged; under an orthographic one the same screen size is a threshold on the
  rock's **diameter** (distance says nothing about a rock's size under that projection), the ring is
  the view's ground footprint capped at 1200 m, and the fade and the near/far mesh choice are taken
  from the terrain's LOD centre — the same `ElementsFocus` the patches are selected about — so a
  rock is culled by the rule the ground under it is drawn by. `docs/research/orthographic-rocks.md`.
- **`RockStats` counts the rocks drawn, not the rocks placed**, so a game reading it under an
  orthographic camera (which drops rocks below its diameter threshold) gets the number that is on
  the ground.

### Added

- **Roads and paths: routes a game lays across the terrain, worn into its ground** (the user's
  ask across iterations 52-55). `PathNetwork` holds `Path`s — a polyline of world XZ points, a half
  width, a `PathKind` (`Earth`, `Gravel`, `Stone`, a packed `Snow` track) and a rut depth — which
  `PathsPlugin` (in `ElementsPlugins`) rasterises once, off the frame, into an **8-bit coverage
  distance field** over the terrain's extent: four texels across the narrowest route, sampled
  bilinearly, so a 70 cm footpath's edge is smooth at any zoom. The terrain material reads it per
  fragment and draws the ground the route has worn: the turf gives way to bare, compacted earth with
  the route's own grain (a per-texel speckle the mask carries at its own resolution plus noise up
  close), a dish cross-section whose verge stands proud and catches a grazing sun, snow cleared
  where traffic runs, and each wheel line's rut as a dark, compacted, smoother groove. A route is the
  ground it crosses, worn — trodden sand on a dune, the snow's own colour in snow, the route's earth
  or stone only on turf and soil. `PathState` publishes the mask for custom shaders. The block is
  compiled only into a terrain whose network has routes (`PATHS`), so a terrain without paths pays
  nothing (measured within ±2 % of the same scene with no network). See `docs/research/paths.md`;
  the `trails` example takes `ELEMENTS_PATHS=lane|track|road|all`, `bench_elements` too.

- **Asking a game where its roads are: `Environment::surface_at`, `nearest_path`, `on_path`**
  (`docs/research/path-query.md`). Iteration 55 shipped laying routes; a game could not ask about
  them. `surface_at(xz)` answers a `PathSurface` — the nearest route's `kind`, its `coverage` (1
  inside, 0 past its edge: the *shader's own* smoothstep, so the answer and the picture cannot
  disagree), `half_width_m`, `rut_depth_m` and `rut` (0-1, the shader's Gaussian about the wheel
  lines) — the surface classification shipping engines call a *collision material* (Gregory 2018).
  `nearest_path(xz)` adds the signed distance to the route's **edge** and the nearest point of its
  centre line; `on_path(xz)` is the predicate. Exact and current (the network is CPU data, not a
  readback), and 1000 queries over a 40-route village cost 1.04 ms. Both new types carry `serde` and
  `Reflect`.

### Changed

- **Breaking: `TerrainSnapshot` carries the sediment.** The terrain's readback now copies the
  sediment thickness beside the surface height (`TerrainState::height_readback` is an
  `array<vec2<f32>>` of surface and sediment per cell, written only when the terrain changes),
  and `TerrainSnapshot` has a public `sediment` field and a `sediment(Vec2)` query: the
  scattered rocks keep off sand with it. Code that builds a `TerrainSnapshot` by struct literal
  adds `sediment` (an `Arc<[f32]>` of the surface's length).

- **The rock's material skips the fields it cannot use, and the picture is the same.** The crate's rock is the most
  expensive thing it draws: the mountains' opaque pass is 0.72–0.79 ms with the rock's
  material and 0.13–0.38 ms without it, so the material is **roughly half the pass**, and the
  crate's notes have warned about it since 0.5.0.

  The material was not wasteful in the way a leak is wasteful — every one of its fields is
  band-limited, and every one of those band-limits is correct. The waste was that the **field
  was evaluated where its own band-limit had already cancelled it**, and that the noise
  computed a **derivative** most of its callers never used.

  Two fields are now gated by the band-limit of the feature they feed: the **bed's region**
  (the slow field that biases the bed's colour, and which is thrown away wherever a fragment
  resolves) and the **second joint set's segment field** (the field that breaks the joint
  set's planes into segments, and which was fetched over the whole face although the joint is
  *absent* over most of it). And `perlin3` is now the **value-only** noise: the derivative is a
  second large polynomial, the crate makes five evaluations per rock pixel, and four of them
  use the value alone. The change is **crate-wide** — the hydrology's phase noise, the
  terrain's generator, the surface cover's field and the noise's own `fbm3` all call `perlin3`.

  Measured: the mountains' opaque pass, the minimum over six baseline and five interleaved
  trials, **0.253 ms on the baseline and 0.245 ms on this one** — no slower; the 3 % gap is
  inside the card's noise. The picture is **the same**: every angle differs by under 0.55 % of
  its pixels (the GPU's own non-determinism) and every mean is identical to three decimals,
  and the value-only noise is **bit-identical** by test. See
  `docs/research/rock-cost.md`.

- **A cliff carries slabs and chips: a second joint set, and a spacing that varies.** The
  crate's cliff was a dense field of same-sized angular chips — a shattered safety-glass
  window, not a rock face. At 3 m the face is a uniform mosaic of pieces a hand's width across,
  and a real cliff carries a metre-wide slab, a fist-sized block and a chip *in the same wall*.

  The cause was in the crate's own numbers: the block network's **cells** already varied — the
  p10 0.25 m and the p90 1.31 m, a 5.24:1 spread — so the *rendering* of them was what
  flattened them. Every cell was drawn with the same joint width and the same relief, so a big
  cell and a small cell read as the same kind of piece.

  And the geology says why a cliff has more than one *kind* of piece. Verdian et al. (2021): the
  initial size distribution "should strongly reflect the size distribution of latent blocks …
  which is set by the **spacing and orientation of fractures, foliations, and bedding
  planes**" — a block is the intersection of **three** families of planes. And Strauhal &
  Zangerl (2021) name the simplification the crate made: "the block size distribution is
  estimated by **assuming that the fractures are fully persistent**". The crate's single,
  always-persistent cell network is exactly that simplification.

  So the cliff now has **two joint sets**: the cell network's, and a second family of parallel
  planes across the bedding. A family of planes is a **period**, and a period is a coordinate —
  so the second set costs a dot product, a division and a `fract`, and *no second lookup*,
  which is what makes it affordable on the crate's hottest path. The planes **wander** and they
  **end**, because a joint set's fractures are non-persistent, and a perfectly straight plane
  set reads as a wire fence across the face.

  Measured: the blocks' edge-orientation resultant at dusk falls from **0.312 to 0.219** and the
  gradient's direction's resultant from **0.057 to 0.017** — the lattice's regularity is
  broken. The pieces' areas spread at 8 m (the p75/p25 ratio 2.29 → 3.35). The view from 120 m
  is unchanged to four decimals, and the `plains` preset differs in **0** of 921 600 pixels. See
  `docs/research/block-sizes.md`.

- **A distant range is many rocks: the beds carry the colour to the horizon.** The crate's
  palette reached the cliff's *fragments* and stopped there, and the fragments are band-limited
  away at 120 m — so a distant massif was still one cool grey, every massif the same, the
  beds' banding showing only as faint value stripes.

  That was by design: iteration 36 established that **the beds carry the distance**, the bed's
  *edge* band-limited away and the bed's *tone* not. The design was right and the bed's *tone*
  was the hole in it — it was a **scalar**, and a scalar can only make a rock lighter or
  darker.

  The bed's tone is now a **colour**, drawn from the fragments' palette by the bed's index, and
  the geology says how. Debret et al. (2011) on spectrocolorimetry: the **Q7/4 ratio** — "the
  ratio between the % reflectance value at 700nm to that at 400nm" — "provides the numerical
  description of the general slope of VIS", and it is the standard quantitative description of a
  sediment's colour, running "from 1.5 to 6" across the sediments they measured. And the
  mechanism: "goethite is **yellow** while hematite is **red**". A bed's colour is its **iron**,
  and the iron is a draw on the bed's index — the same construction the crate's beds already
  use for their thickness and their tone, with the quantity changed from a brightness to a
  **hue**.

  Measured: the massif's hue's resultant falls from **0.759 to 0.683** at 300 m and from
  **0.672 to 0.596** at 120 m, and the hue's spread rises by 7° and 6.5°. The mean moves
  **0.3 DN** at 120 m and not at all at 300 m, the face at 3 m differs in **0.64 %** of its
  pixels with an identical mean, and the `plains` preset differs in 9 of 921 600 pixels, by
  1 DN. See `docs/research/bed-colour.md`.

- **The cliff is not one grey: a rock face carries a palette.** The crate's cliff was a single
  cool grey, from the near face to the horizon: at dusk and at dawn the face was a **single
  hue** — the circular resultant of the hue angles 0.998 and the spread 2.1° — and the
  rock-only patch at 3 m had a chroma's median of 0.0157, which is nearly grey. It read as
  *concrete*.

  The cause was structural, not a matter of tuning: the crate's rock system was **monochrome by
  construction**. The bed's tone is a scalar, the block's tone is a scalar, and the second
  tone is a fixed `vec3` — and a scalar can only make a rock lighter or darker, never *another
  colour*.

  The geology says the rock's own colour is not the point — the **surface's** colour is.
  Tansem & Storemyr (2020) on the red-coated rocks of Alta: "it is highly likely that the rock
  art was originally carved on rocks with **red iron films, rocks that are now generally
  gray**" — the same rock wears a red film in one place and is grey in another. And Krinsley
  et al. (1998) on desert varnish: "a thin (< 100 to 500 μm thick) coating of ferromanganese
  oxides, clay minerals, and trace elements" — a coating *on* the rock, layered and patchy.

  So the model is the crate's own, from the ground's palette: a **palette of tones mixed as
  convex shares** by a score. The turf has four tones; the rock has four rocks — a cool
  basalt, an iron-stained sandstone, a pale limestone, a dark ferromanganese varnish — and a
  **rind**: the film of iron and manganese on the surface, a separate layer with its own
  colour and its own patchy coverage.

  Measured on the face at 3 m: the hue's resultant falls from **0.800 to 0.656**, the hue's
  spread rises from **27.6° to 39.7°**, and the chroma's median rises from **0.0157 to
  0.0275**. The mean luminance moves **0.38 DN** — the palette's equal-weight mean is exactly
  (1, 1, 1), so a game that changes colour does not change its lighting. The `plains` preset
  differs in 7 of 921 600 pixels, by 1 DN: the meadow is untouched. See
  `docs/research/rock-palette.md`.

- **The cliff is not a quilt: a rock face breaks into irregular fragments.** The crate's blocks
  came out as a **quilt of identical lozenges** — a regular lattice of same-size diamonds,
  every block the same shape with the same dark notch on the same side, in rows. It read as
  wallpaper.

  The cause was a one-line regression: the commit that replaced the block network's warp with
  the noise's own *gradient* left the line as `let warp = vec3<f32>(0.0);`, so **the warp has
  been off since 0.5.2** and the cells have been a perfect lattice ever since.

  But the warp alone is not the fix, and the measurements say so:

  - The warp's **amplitude** was calibrated for the noise's *value* and not for its
    *gradient*. The gradient's mean magnitude is 0.94, so 0.09 displaced a cell's boundary by
    **0.06 of a cell** — a fifth of what it needed. The amplitude is now 0.30 m.
  - The warp's **scale** was the cell's own, so it *scrambled* the lattice instead of bending
    it: the cell's index stopped being monotonic along a straight cut, and the joint was
    crossed **zero times in 30 m**. It now varies over nine cells.
  - The cells of a network are all the **same size**, so a warped lattice is still a lattice of
    equal cells. The size spread comes from the network's feature points being drawn *towards*
    their cell's centre, so the cells are unequal.
  - The joint's **width** is a **Weibull** draw, the shape the fracture spacing's distribution
    follows (Verdian et al. 2021), and not a linear one.
  - A **second generation** of blocks — the scree's own construction — was tried and
    **rejected**: a seam every few pixels turned the face into a dark speckle.

  Measured on a face at 3 m: the blocks' **edge-orientation resultant** — how strongly the
  blocks' edges agree on a direction, which is the quilt's signature — falls from **0.431 to
  0.277**, and the material's own high-pass **rises** from 6.67 to **8.02 DN**. The view from
  120 m is unchanged to four decimals (the material's own high-pass 4.3440 DN before and
  after), so the distance is calm, and the `plains` preset is byte-identical, so the meadow is
  untouched. See `docs/research/rock-fragments.md`.

### Fixed

- **Under an orthographic camera the terrain near the focus is drawn at full detail, on the
  height map's own samples** (issue #36). The CDLOD patches were selected by distance from the
  camera's eye and morphed by Bevy's `lod_view_world_position`, the eye too; an isometric
  camera stands hundreds of metres back from what it shows, so the ground on screen was drawn
  in level-1 and level-2 patches and level 0 never appeared, and figures standing on the
  height map sank into it: in an isometric view of the hills a quarter of the posts standing
  on `Environment::terrain_height` had their 10 cm foot swallowed, half on the mountains. Under
  an orthographic followed camera the patches are now selected and morphed about its
  `ElementsFocus` (without one, about where its view ray first meets the ground), in every
  view — the main pass, the depth prepass and the shadow views read the same centre from the
  terrain's and the water's uniform, and the scattered rocks seat on the same surface. A
  perspective camera selects and morphs about its eye, as before.

  The patches' vertices also moved half a cell, onto the height samples (the cell centres
  that `terrain_height`'s bilinear interpolates): before, a level-0 vertex sat on a cell's
  corner and was the mean of four samples, so even level 0 drew a smoothed map, 0.15 m off
  the bilinear at the 90th percentile on the hills (4 m cells) and 0.62 m on the mountains.
  On the samples the drawn surface is the bilinear at every vertex and departs from it only
  inside a quad, by at most a quarter of the quad's twist: 0.047 m at the 90th percentile on
  the hills, 0.20 m on the mountains. The drawn terrain now spans the cell centres, half a cell
  inside the grid's extent on each side. See `docs/research/orthographic-lod.md`.

- **Wet ground is no longer a mirror.** Every surface the crate wets went to roughness 0.1 at
  full wetness, so in rain a meadow became a choppy grey lake, the hills at dusk looked
  snow-covered, a cliff turned to crumpled chrome and the dunes went white in the middle
  distance. Against the same rain on dry ground the meadow's 10–60 m band measured 187 against
  124 (luminance) and a median saturation of 0.032 against 0.308: rain made the ground brighter
  and greyer, the opposite of what water does. Now a porous surface takes Lagarde's limited wet
  sheen, roughness × (1 − 0.4 · wetness · porosity) (0.54 for a soaked rough surface), and the
  water film at 0.1 is kept for smooth and metal surfaces and for a wall drenched by a rain
  streak. Its albedo darkens by the water-film physics per channel,
  (1 − r_e)(1 − r_i) ρ / (1 − r_i ρ), instead of a flat half: a dark channel loses more than a
  bright one, so wet colour is more saturated and wet sand is darker and browner rather than
  greyer (near sand: saturation 0.258 against 0.245 dry). The terrain's grass is a canopy whose
  wax beads the rain and takes `bevy_flora`'s wet-leaf rule (saturation × 1.25, brightness kept,
  no gloss), so a wet meadow is a deeper green (the same band: 123 and 0.384); the canopy is read
  from the albedo's excess green (Woebbecke et al.'s `2g − r − b`), so the soil between the
  blades darkens as soil and the shader carries nothing new across the rock's material. A rock's joints
  and fractures now occlude its sky reflection by the same visibility that darkens them, so a
  wet cliff keeps its dark joint network under the sheen (dry, it changes by a few levels).
  `WeatheredMaterial` and `GroundMaterial` take the new porous rule too (a soaked rough wall at
  0.54); puddles, ripples and snow are unchanged. See `docs/research/wet-ground.md`.

- **Rain no longer rings on dry grass.** Every rain impact within 20 m of the camera drew an
  expanding white ring, so a storm littered the meadow with 30 cm white circles. A ring is a
  wave on free water: on a live terrain an impact now rings only where the terrain holds
  standing water at least `WetnessSettings::puddle_min_m` deep (a puddle, a river, a lake). A
  blocker's top face and the plane of a scene without a terrain ring as before.

- **The `weathered_town` example's ground reaches the horizon.** Its 600 m slab ended in a hard
  line against the rain haze once the wet ground stopped mirroring the sky; it is 40 km now,
  like the other examples' ground.

- **The CPU mirror's `fract` was the wrong one.** Bevy's `Vec3::fract` is `x − trunc(x)` and so
  is **negative** for a negative coordinate, while WGSL's is `x − floor(x)` and is always in
  [0, 1). The two disagreed about where every cell's wall was, and the terrain model's joint
  weight came out at **zero** on the negative half of the world.

- **The massif has a succession of beds, not a flat blue-grey wash.** Over a rock-only
  200 × 200 patch of the mountains from 120 m the rock's spread was a max−min of **1.00 DN**,
  where the crate's own `rock-material.md` asks for 9, and at 300 % the rock was a smooth
  wash with a soft, formless mottling. The green turf and the ochre soil read clearly at the
  same distance, so it was not the resolution that was failing — the rock had no structure to
  resolve.

  The cause was in the crate's own numbers: the bed's coordinate is
  `(dot(p, TILT) + warp · 3.5) / 4.5` and the warp is the ground's 45 m-scale field in
  [−1, 1], so **the warp displaced the bed coordinate by up to 0.78 of a bed** — most of a
  bed's own thickness. A 4.5 m grid distorted by a 45 m field is a set of blotches, not a set
  of bands, and the mask picked a random half of the smeared field. And every bed was
  identical, so the cliff was a stripe pattern, and a stripe pattern whose stripes are
  sub-pixel at 120 m is a flat tone.

  The beds are now a **warped lattice** — the boundary between the beds whose index is `i`
  and `i + 1` falls at `i + warp(i)`, and a bed's thickness is the gap between two
  boundaries — with a **per-bed tone** drawn from the bed's index with the crate's own
  integer hash, because *a bed is a colour*: a sandstone, a shale, a limestone. The bed's
  **edge** is band-limited away with the distance and the bed's **tone** is not, which is the
  whole trick: at a distance the cliff shows its succession and not its stripes.

  Measured: the material's own contribution at 120 m is a standard deviation of **3.41 DN**
  and a max−min of **21.2 DN**, and the face at 3 m carries bands of ±7 DN alternating. The
  `plains` and `hills` presets are **byte-identical**: the meadow is untouched. The cost is
  **+3.5 %** on the mountains' opaque pass. See `docs/research/bedding.md`.

- **The cliff is made of blocks.** The crate's rock had a material — bedding, joints, a grain —
  and the material was nearly *invisible*: it contributed an sd of **1.72 DN** to a face at 3 m,
  and at 200 m the massif was **one colour** (a max−min of 1.00 DN over a 300 × 200 patch,
  where the crate's own `rock-material.md` asks for 9). The reason was structural: every
  feature was a *multiplier on the albedo of one continuous surface*. A cliff is not a texture.
  It is a pile of blocks.

  A rock mass is a set of blocks held in one piece (Zhang et al. 2020, *Geofluids* 2020:6679317:
  "discontinuous in geometry while continuous in mechanics"), and a cliff face shows the blocks.
  They are the **cells of a 3D cellular network**, in the rock's own frame — a 2D network in the
  ground plane would stretch on a slope and weave, exactly as the crate's turf does. The
  network's coordinates are **squashed along the bedding plane's normal**, so the cells stretch
  into slabs: a rock breaks along *joint sets*, families of parallel planes, so its pieces are
  aligned rather than a random heap (La Pointe & Hudson 1985).

  A block is **shaded, not outlined**: its own tone (the cell's random factor), its own
  **facet** — a rock block's face is flat, and the *step* is at the joint — and a **joint** that
  is the cell's own *boundary*, the second nearest feature point's distance. The crate's rock
  note is right that a cell network's boundaries read as a giraffe's hide; that is a statement
  about drawing the boundaries as *lines*, and a block is a volume.

  The network's coordinates are **warped** before the lookup, so the cells' boundaries wander
  and the blocks are irregular fragments instead of a mosaic, and the joint's **width** is the
  cell's own draw, which is what sets the block's **size** — a cell with a wide joint is a
  small piece of rock and one with a narrow joint is a big slab. The blocks are band-limited by
  the *joint's width* and not by the cell, the crate's own rule for the fractures: a crack is a
  line a fraction of a cell wide, so it aliases long before the cell does.

  The **beds** carry a value contrast now. The second tone was (1.06, 0.93, 0.76), whose
  luminance is 0.95, so the beds moved the luminance by 1.9 % and the massif was one colour; a
  bed is a *darker* band, and the value is what carries at a distance. The tone is now
  (0.80, 0.73, 0.66) at a mix of 0.55, a 14 % albedo drop.

  The blocks are **tabular** — elongated along the bedding, because a rock breaks along *joint
  sets* — and the network's coordinates are **warped** before the lookup, so the blocks are
  irregular fragments and not a mosaic. The joint's **width** is the cell's own draw, which is
  what sets each block's **size**.

  Measured: the material's own high-pass at 3 m goes from **1.72 to 5.74 DN** (the note's target
  is 3), and at 90 m the frame is **byte-identical** to the baseline's — the blocks are
  band-limited away, so the distance is calm. The `plains` and `hills` presets are
  **byte-identical**: the meadow is untouched.

  The cost is a logged trade: the mountains' opaque pass, which is the pass the blocks are
  in, goes from **0.141 ms to 0.181 ms** (+28 %) — the crate's rule is no regression without a
  justified trade, and this one buys a material that went from an sd of 1.72 DN to 5.74 DN,
  paid only where the rock shows. See `docs/research/rock-blocks.md`.

- **The scree is a scatter of stones, not a band of mixed colour.** The stone that has weathered
  off a cliff lies below it, and the crate rendered the boundary between the turf and the rock
  as a **smooth wash** — a colour tint toward the rock over a slope band, with no stone
  anywhere in it. At 2 m the ground at the rock's foot read as a uniform sandy grey with a
  fine even noise, and the grass met the stone over a monotone 72-pixel ramp: a dusty surface,
  not rock. A scree is *stones of many sizes with turf between them*, and that is what it is
  now.

  The stones are the **cells of a 2D cellular network**: the nearest feature point is a
  stone's centre and the distance to it is the stone's radius. The crate's own rock note
  records why that is the right structure — and why the *same* network cannot make the
  fractures: "A Worley cell network cannot make them — it bounds *cells*, so a face reads as a
  giraffe's hide". Used the other way round, as the *stones* rather than as the *cracks*, it
  is the missing piece.

  Two networks are nested — a coarse one for the big stones and a fine one for the small ones,
  the fine ones only where the coarse ones do not cover — because a real talus has a long tail
  (Verdian et al. 2021, *Earth Surface Dynamics* 9:1073: the finest talus clasts are 2 mm
  against a median of 10 to 575 mm, and the median moves by more than 10-fold between sites).
  One network at one size reads as *pebbles on grass*.

  Each stone has its own **relief** — the distance field's gradient domes it, so the sun rakes
  its edge and it reads as a thing rather than a stain — and its own **contact shadow** on the
  matrix beside it, so the stones sit *in* the dirt. The ground between them is the scree's
  own matrix, not the turf: a scree has no grass in it.

  The band is the soil's own band up to the rock's ramp (16°–40°), an apron rather than a
  line: a narrow band traced the hillside's break as a **contour line**, the one thing a
  scree is not. The scatter is band-limited by the cell, so at 90 m it is a tone and not a
  corduroy.

  Measured: the `plains` preset at 6 m is **byte-identical** (the mean 170.725 and the
  standard deviation 26.575 before and after) — the meadow is untouched; the hillside's
  high-frequency energy rises from 6.415 to 8.299; and at 3 m the stones are individually
  countable. See `docs/research/scree.md`.

### Added

- **A record of recent days: `ClimateHistory`** (issue #31). `Environment` answered only the
  present, so a game could not ask what the last few days were like. `ClimateHistory` (a
  resource, inserted by `ClimatePlugin`) records each local calendar day from the `Weather` as
  the clock runs: the mean, minimum and maximum temperature, the mean relative humidity, the
  precipitation and snowfall totals and the hours observed (`DayConditions`).
  `Environment::recent_days(n)` returns the completed days of the last `n` and
  `recent_climate(n)` a `ClimateSummary` with the ETCCDI's daily indices — wet days (≥ 1 mm,
  `WET_DAY_MM`), frost days (minimum < 0 °C, `FREEZING_C`) and icing days (maximum < 0 °C) —
  the hour-weighted means, the extremes and the totals. A window of 0 is empty, a longer one is
  the whole record, no days is `None`. It keeps 31 days (`ClimateHistory::with_capacity`); a
  step of up to a day is recorded, a longer jump leaves a gap, a rewind forgets the days after
  it. With `serde` it is part of the weather's save: restored onto the saved clock it continues
  the same days (`tests/save.rs`). Deterministic for a climate and a seed, and the same days at
  a quarter of the frame step within 0.02 °C (`tests/climate_history.rs`). See
  `docs/research/climate-history.md`.

- **`TerrainLook`: a game grades the owned terrain's ground** (issue #35). The terrain's shader
  writes its own albedo, so a game that grades its materials per palette could not grade the
  ground: at night the meadow's green showed through a navy veil as olive. `TerrainLook {
  tint, saturation }` (a resource, default identity) grades the bare ground — rock, soil, grass,
  sand and scree, textured or not — and the scattered rocks lying on it: the saturation first
  around the colour's own linear Rec. 709 luminance (0 grey, above 1 oversaturated, negative
  taken as 0), then the tint, so a cooling tint survives a desaturation; before the ground
  cover, so snow and standing water keep their colours and wet grass still beads the rain. The
  default renders the ground as before (within a float's rounding, below one 8-bit level; a
  branch that skipped it bit-exactly cost the 4K opaque pass 6 %, the branchless operator
  nothing measurable), and a change is a uniform write, so a palette blend can write it every
  frame without compiling a pipeline (tested over 240 frames). `terrain_tour` and
  `bench_elements` take `ELEMENTS_TINT` and `ELEMENTS_SATURATION`, `terrain_tour` also
  `ELEMENTS_GRADE_GLIDE`. See `docs/research/terrain-look.md`.

- **PBR textures set on the terrain.** Underfoot the meadow was a soft green felt.
  `TerrainTextures` on the `Terrain` entity takes, per ground layer (sand, soil, grass), a
  `GroundLayerTextures`: a base colour and, optionally, a normal (+Y up; `flip_normal_map_y` for
  DirectX maps), roughness (G), occlusion (R) and height (R) map, as a glTF material or a PBR
  library stores them, and the tile's size. The crate packs them itself, off the frame (an
  async task, 0.5 s for two layers of five 1024² maps), into physically based detail over its
  procedural layers: an albedo detail (a multiplier with mean 1), a height, a normal, and
  roughness and occlusion as multipliers with mean 1, deriving whatever a set leaves out from
  its base colour; the base colour is a detail over the crate's calibrated colour, and
  `with_colour(share)` lets the set's own mean colour replace the layer's at every distance (a
  library's albedo is rarely calibrated: two CC0 libraries' sets measured 2-6.5 times the
  crate's and bleach the hills taken whole). `GroundLayerTextures::photograph()` de-lights and
  tiles a photograph or a generated image. The maps are hex-tiled (Mikkelsen 2022: offset taps
  in eight orientations, so a tile never repeats), blended between the layers by their heights
  (grass blades stand out of the soil), and band-limited by the pixel's footprint, so nothing
  sparkles; a layer left `None` stays procedural. Inserting, changing or removing the component
  re-textures the ground at runtime, the previous set rendering until the new one is packed, so
  a set is a stylistic variant a game can swap. The examples wear golem-generated soil and
  grass photographs from `assets/ground/golem/` (excluded from the package;
  `ELEMENTS_GROUND_TEXTURES=0` leaves them off, `ELEMENTS_GROUND_SWAP=S` toggles them,
  `ELEMENTS_GROUND_COLOUR=c` sets their colour share); the sand waits for wind ripples. Cost:
  the textured path is compiled only into the pipeline of a material that has a set
  (`TerrainExtensionKey`), so without one the ground costs what it did; with one the eye-level
  meadow's opaque pass rises 26 % (4K, interleaved, P5 clocks), and a set that takes its own
  colour runs its own pipeline (`GROUND_COLOURS`), since the colour lanes read in the hot path
  cost a further 8 %. See `docs/research/terrain-textures.md` and
  `docs/research/ground-textures.md`.

- **Rocks on the ground.** The crate's ground was an empty carpet: every stone in it was
  painted into the terrain's material, so nothing on a meadow, a hillside or below a cliff had a
  silhouette or cast a shadow. `RocksPlugin` (in `ElementsPlugins`) scatters boulders and stones
  over the `Terrain` as meshes: angular talus blocks in heaps below the cliffs, rounded rocks
  over the meadow (sparse on the flat, more on steep hillsides and in rocky patches), none on
  sand or on the cliffs. Their sizes follow Golombek's exponential rock-abundance law (rocks of
  diameter `D` or more cover `k·exp(−q(k)·D)` of the ground), their shapes are jointed blocks
  with the axis ratios measured on talus (Verdian et al. 2021), and each sits on the surface the
  CDLOD patches actually draw, a quarter to a third buried. A rock is drawn to a distance in
  proportion to its size (`ElementsQuality::rock_range`, 100 m per metre of diameter at High)
  and shrinks away over the last fifth of it, so it never pops. They are drawn with
  `WeatheredMaterial`, so they take snow, wetness and cloud shadows. `RockSettings` sets the
  covers, the range and the seed; `ScatteredRock` marks each rock for a game's colliders;
  `RockStats` counts them. See `docs/research/rock-scatter.md`.
- **`WeatheredExtension::with_grain(feature_m)`**: a stone grain for any static mesh, the
  terrain rock's band-limited 3D noise as a mottle at `feature_m` and a speckle at an eighth of
  it, varying the albedo and bending the shading normal, with no texture or UVs. Every existing
  `WeatheredMaterial` is unchanged (the grain is off by default).
- **`terrain_tour`: `ELEMENTS_FLY=8`** glides the camera forward at 8 m/s, held above the
  ground, for moving captures. **`bench_elements`: `ELEMENTS_EYE`/`ELEMENTS_LOOK`/`ELEMENTS_AGL`**
  place the camera, **`ELEMENTS_ROCKS=off`** leaves the rocks out, and the scene line reports
  Bevy's `per_view_shadow_pass`.

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

- **Asking a game where its roads are: `Environment::surface_at`, `nearest_path`, `on_path`**
  (`docs/research/path-query.md`). Iteration 55 shipped laying routes; a game could not ask about
  them. `surface_at(xz)` answers a `PathSurface` — the nearest route's `kind`, its `coverage` (1
  inside, 0 past its edge: the *shader's own* smoothstep, so the answer and the picture cannot
  disagree), `half_width_m`, `rut_depth_m` and `rut` (0-1, the shader's Gaussian about the wheel
  lines) — the surface classification shipping engines call a *collision material* (Gregory 2018).
  `nearest_path(xz)` adds the signed distance to the route's **edge** and the nearest point of its
  centre line; `on_path(xz)` is the predicate. Exact and current (the network is CPU data, not a
  readback), and 1000 queries over a 40-route village cost 1.04 ms. Both new types carry `serde` and
  `Reflect`.

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

- **Asking a game where its roads are: `Environment::surface_at`, `nearest_path`, `on_path`**
  (`docs/research/path-query.md`). Iteration 55 shipped laying routes; a game could not ask about
  them. `surface_at(xz)` answers a `PathSurface` — the nearest route's `kind`, its `coverage` (1
  inside, 0 past its edge: the *shader's own* smoothstep, so the answer and the picture cannot
  disagree), `half_width_m`, `rut_depth_m` and `rut` (0-1, the shader's Gaussian about the wheel
  lines) — the surface classification shipping engines call a *collision material* (Gregory 2018).
  `nearest_path(xz)` adds the signed distance to the route's **edge** and the nearest point of its
  centre line; `on_path(xz)` is the predicate. Exact and current (the network is CPU data, not a
  readback), and 1000 queries over a 40-route village cost 1.04 ms. Both new types carry `serde` and
  `Reflect`.

### Changed

- Overcast light: the sun and moon `DirectionalLight`s carry the cloud's beam transmittance and the diffuse skylight remainder is the camera `AmbientLight`, so object shadows soften under cloud; the sky reads grey, not blue.
- The far field of the clouds is lit and seen through the air exactly as the marched deck is at the fade boundary, and a ray that meets the planet past the edge of a finite ground draws the camera's distance fog instead of a cloud.
- Terrain layer blend: Andersson-style Overlay detail on wider ramps, so rock, soil and grass meet in irregular mottled transitions instead of iso-slope blobs.
- Snowflakes are soft discs of bounded elongation, not dashes; snowflakes' streak is capped at one diameter.
- `profile.dev` sets `debug = 0`: Bevy examples' debuginfo links OOM CI runners.

### Examples

New: `terrain_tour`, `external_ground`, `flood`, `lake`, `erosion_bake`, `weathered_town`, `ground_cover`, `dust_storm`, `voxel_snow` (feature `isomesh`), and `quickstart` now stands on hills. `bench_elements` times every pass, with `ELEMENTS_TERRAIN=hills|mountains|dunes|lake|none`.

[Unreleased]: https://github.com/Ladvien/bevy_elements/compare/v0.7.1...HEAD
[0.7.1]: https://github.com/Ladvien/bevy_elements/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/Ladvien/bevy_elements/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/Ladvien/bevy_elements/compare/v0.5.3...v0.6.0
[0.5.1]: https://github.com/Ladvien/bevy_elements/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/Ladvien/bevy_elements/compare/v0.1.0...v0.5.0