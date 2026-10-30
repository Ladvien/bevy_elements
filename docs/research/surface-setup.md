# SurfaceSetup and TerrainSetup: the ground, water and cover settings as saved, validated, live-appliable values

bevy_emerge's terrain and water panels (issues #99 and #100 there) store this crate's own settings
types in a game's project and edit them live. `EnvironmentSetup` did that for the sky; the ground has
nine settings resources (hydrology, erosion, snow, wetness, water, waves, rocks, dust, trails), three
without serde (`WaterSettings`, `TrailSettings`, `WaveSettings`), no validation, and a `Terrain`
component that cannot be saved because a height map is a `Handle<Image>`. This note adds the same
shape for the ground, in its general form: a level loader, a debug console and a test harness need
the same. No physics changes.

## Sources

- The crate's own notes define each setting's range: `docs/research/hydrology.md` (Manning n
  0.03-0.05, step), `wetness.md`, `snow` in `cover`, `dust.md` (talus, Bagnold threshold, Kok
  saturation length), `rock-scatter.md` (covers are Golombek's area fractions, so 0-1),
  `water.md` (index of refraction >= 1, Cox & Munk slope), `terrain.md` (generator parameters).
- Corpus search for method phrases (`preset`, `save state`) returns nothing applicable: this is API
  plumbing, not a model. Not searched further; `docs/research/environment-setup.md` is the precedent.
- The editor's issues #99 and #100 (read 2026-10-29): a height map "imported", "saved with the map,
  in the crate's format"; settings are "the crate's serde types ... RON files".
- Bevy 0.20: `ImageLoaderSettings { is_srgb, expand_grayscale }` for a linear single-channel load
  (`bevy_image-0.20.0/src/image_loader.rs`), the terrain's documented requirement.

## What great looks like

A host reads a file, gets a value or an error naming the field and the bound it broke, applies it to
a running app, and an unchanged re-apply (a hot-reloaded file after an unrelated edit) changes no
resource, so the rocks' scatter is not rebuilt for nothing. It captures the running state back into
the same value. A level names its ground (procedural preset, flat, or a height-map path) as data and
the host builds the `Terrain` from it. No visual change: nothing rendered differs.

## Model adopted

- `SurfaceSetup { hydrology, erosion, snow, wetness, water, waves, rocks, dust, trails }`, serde with
  `default` and `deny_unknown_fields`. `validate()`, `apply_to_world(&mut World)` (validate first;
  write a resource only when it differs), `capture_from_world(&World)` (a missing resource reads as
  its default).
- `TerrainSetup { source: TerrainSetupSource, extent_m, resolution, external }` with
  `TerrainSetupSource::{Procedural(ProceduralTerrain), Flat(f32), Heightmap { path, height_range_m }}`.
  `terrain(&AssetServer)` validates and builds the `Terrain`, loading a height map linearly and as one
  channel; `capture(&Terrain)` returns the setup and errors for a height map with no asset path.
- `WaterSettings`, `TrailSettings`, `WaveSettings` gain their serde derives (behind `serde`).
- `apply_to_world` inserts a missing resource rather than erroring (unlike `EnvironmentSetup`): a
  game may omit dust or waves, and an inserted resource of a plugin that is absent is inert; a
  plugin added later keeps it (`init_resource`).

## Parameters

No defaults change. Bounds, each from the setting's own doc: strictly positive `step_s`,
`manning_n`, `tile_m`, `flow_cycle_s`, saturation length, snow densities and settle days, extent;
finite and >= 0 for rates, coefficients and scales; `[0, 1]` for covers, albedo, roughness, silt
tint and ridged; angles of repose in (0, 90); `shed_slope_deg` ordered within [0, 90]; refractive
index >= 1; `puddle_full_m >= puddle_min_m`; rock regions finite with min <= max; `octaves >= 1`
(the generator stops itself at the Nyquist limit, so there is no upper bound to enforce).

## GPU mapping

None. No shader, buffer or binding changes; no benchmark applies.

## Validation targets

- Every default validates; each bound rejects a value just outside it, naming the field; the edges
  validate.
- An invalid setup writes nothing.
- An unchanged re-apply leaves every resource's change tick alone; changing one setting marks that
  resource only.
- `capture` after `apply` returns what was applied; a JSON round trip equals the original; an
  unknown field is rejected; absent fields take defaults.
- `TerrainSetup::terrain` then `capture` returns the setup for each source kind; a height map with
  no path and an invalid setup error with the field.

## Deviations from this plan

None: no default changes.

## Pending index

None; no new corpus document.
