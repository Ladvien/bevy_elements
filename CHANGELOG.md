# Changelog

All notable changes to bevy_elements. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate is pre-1.0, so any release may break the API.

## [Unreleased]

### Fixed

- **Snow now lies on the ground the examples show, and a still can show it.** `weather_tour` under `Snow` and `Blizzard` rendered olive-drab ground where the boxes and columns beside it were white: the harness's 2 km plane was a bespoke material that sampled only the cloud shadow map. The examples' ground is now `WeatheredMaterial` — it darkens and glosses in rain, runs with streaks and whitens under snow as a consumer's mesh does — and its plane reaches 40 km, so the ground meets the sky without the sliver of unlit below-horizon atmosphere a 2 km plane leaves at the horizon (a 35 DN dark shelf under `Overcast` and `PartlyCloudy`, now gone). A held clock (`ELEMENTS_HOUR`) integrates no game time, so no material could ever whiten in a still; `ELEMENTS_GROW_HOURS=n` now runs `n` game hours of the held preset before the first frame and rewinds the clock, leaving the hour, sun and sky as the example's own (`docs/research/snow-cover.md`, Deviation 14; `docs/research/clouds.md`, Deviation 28).
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

[0.5.0]: https://github.com/Ladvien/bevy_elements/compare/v0.1.0...v0.5.0