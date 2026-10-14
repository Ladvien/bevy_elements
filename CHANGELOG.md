# Changelog

All notable changes to bevy_elements. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate is pre-1.0, so any release may break the API.

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

[0.5.0]: https://github.com/Ladvien/bevy_elements/compare/v0.1.0...v0.5.0