# Snow on many voxel volumes, without a terrain (issue #13)

A game whose walls are many `VoxelVolume`s over its own ground wants a snow slab on every wall, and wants the crate to own no terrain. `VoxelGround` (`docs/research/voxel-snow.md`) makes one volume *the* ground: it spawns the crate's single `Terrain` over the volume, and the ground-cover maps, hydrology and water run on it. That cannot be repeated per wall (one terrain exists at a time, `terrain/mod.rs` keeps one `TerrainState`), and a game with its own ground does not want it at all.

## Sources

- `docs/research/voxel-snow.md` (this crate, read in full): the column-top scan (highest zero crossing of the base field, 8 bisections), `SnowyField` (`min(base, max(y − (t + s), (t − c) − y))`), per-column sample-and-hold with the `0.5 c` swap threshold, the chunks a changed column reaches, the edit rescans, and Limitations (a)–(h), all reused unchanged.
- `docs/research/snow-cover.md` (this crate): the shed-slope exposure `1 − smoothstep(α₀, α₁, α)` with `SnowSettings::shed_slope_deg` (Fearing 2000 §4.1, angle of repose 15°–90° by snow type, read for that note), and the point model in `SurfaceWeathering` (snow on exposed flat ground at the reference elevation: accumulation, settling, degree-day melt).
- `docs/research/snow-season.md` (iteration 21): the point pack as the snow on exposed flat ground (`SurfaceWeathering::pack_seed`), which the cover maps are seeded from times each cell's exposure.
- bevy_isomesh 0.0.11 (`plugin.rs:218-237`): `VoxelVolume::replace_field`, `VoxelChunk` ("lives on its own entity, with the volume as its parent"); the game spawns the chunk entities, so a chunk's mesh sits in the volume's frame and the volume's `GlobalTransform` takes field space to the world.
- bevy_isomesh 0.0.10 (docs.rs source, `Cargo.toml.orig`): depends on Bevy 0.19 (`bevy_asset`, `bevy_mesh`, `bevy_app`, `bevy_ecs`, `bevy_tasks`, `bevy_platform` = "0.19"), has no `replace_field` (0.0.11 `plugin.rs:225`) and no `IsomeshSystems` (0.0.11 `lib.rs:15`). isomesh 0.0.10 and 0.0.11 (both unpacked in the registry): `Sdf`, `ChunkId`, `ChunkLayout::{new, cells, cell_size, base_sample, world_of_sample, chunk_of}`, `fields::Sphere`, `brush::smooth_min` identical.
- Issue #13 (filed from Sanctuary): 1 ft cells, 16-cell chunks, identity transforms, the game's own heightfield; acceptance "two volumes each get a snow slab, and no `Terrain` is spawned" and "builds against isomesh 0.0.10 and 0.0.11".

## Model adopted

**`VoxelSnow`, a component on any number of volumes.** It puts the `VoxelGround` snow slab on the volume and nothing else: no terrain, no hydrology, no water. The column grid is the volume's own field-space XZ box with one column per voxel (`VoxelSnow::resolution` overrides it), scanned once as `VoxelGround` scans; edits rescan the marked chunk columns the same way; the slab, the held depths, the swap threshold and the dirty-chunk derivation are the same code. A volume that has both components is a `VoxelGround` (one warning).

**Where its snow comes from.** Per column, at the column's world position (the volume's `GlobalTransform` applied to the column top's centre):

1. the **ground-cover map** where one covers that point (a game's own terrain with `GroundCoverPlugin`): its depth there, so a wall under a `PrecipitationBlocker` roof, whose ground the map shelters, stays bare, and the walls follow the ground's melt;
2. otherwise the **point snowpack** of `SurfaceWeathering` (exposed flat ground at the reference elevation).

The depth is multiplied by the column top's own **shed-slope exposure** (the slope from the neighbouring tops, `SnowSettings::shed_slope_deg`), so a pitched cap sheds as a roof does, and divided by the volume's scale to field units. The held depths are refreshed only when the source changed: a new map readback, or the point pack moved by more than a quarter voxel since the last refresh, so idle volumes cost nothing per frame.

**Transforms.** A volume may be translated, turned about the vertical and scaled uniformly: its columns stay vertical in the world, and only the world position and the depth's scale change. A tilted or non-uniformly scaled volume gets no snow (one warning): its field-space columns are not the direction snow falls.

**Versions.** `isomesh` accepts `>=0.0.10, <0.0.12` (its API this crate uses is identical in both). `bevy_isomesh` stays at 0.0.11: 0.0.10 is built on Bevy 0.19 (the pin-parity rule forbids it) and lacks two APIs the feature uses.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| Columns | one per voxel over the volume's XZ box (`VoxelSnow::resolution` overrides, capped at `ElementsQuality::terrain_max_resolution`) | voxel-snow.md (one terrain cell per voxel) |
| Swap threshold | `0.5 c` | voxel-snow.md |
| Refresh trigger | new cover readback, or the point pack moved by > `0.25 c` | this note |
| Shed slope | `SnowSettings::shed_slope_deg` (default) | snow-cover.md |
| Upright tolerance | `\|up · Y\| > 0.9999`, scale components within 0.1 % | this note |

## GPU mapping

None: the slab is CPU work (the scan and the swap) and bevy_isomesh's meshing, as for `VoxelGround`. The cover map is read from the existing readback (`EnvironmentSnapshot::cover`).

## What great looks like

Two walls of a small fort in a snowfall, one turned 30° and raised on the game's own plain: both grow a cap of snow on their tops as the weather snows, the crenels and the wall walk white, the faces bare, and the caps melt away together on a warm day; the world has no crate terrain.

## Validation targets

- Two volumes with `VoxelSnow` each get a slab whose depth follows the point pack (within the swap threshold) and no `Terrain` exists (headless test).
- A translated and yawed volume's slab sits on its own tops (its column grid is in field space) and samples the cover map at its world position (unit test of the mapping).
- A tilted volume gets no slab and warns once.
- A volume with both `VoxelGround` and `VoxelSnow` is a voxel ground.
- Removing `VoxelSnow` restores the user's field and remeshes the chunks.
- `cargo build --features isomesh` with `isomesh` 0.0.10 resolved (`cargo update -p isomesh --precise 0.0.10` is impossible while bevy_isomesh 0.0.11 requires `isomesh = "0.0.11"`; the check is that this crate's own use compiles against 0.0.10's API, see Deviations).

## Deviations from this plan

1. **"Builds against isomesh 0.0.10 and 0.0.11" is met for `isomesh` alone.** `bevy_isomesh` 0.0.10 depends on Bevy 0.19 (pin parity, AGENTS.md "Plays well with the family" 1) and has neither `VoxelVolume::replace_field` nor `IsomeshSystems`, both used by this feature. A game on a path checkout of bevy_isomesh patches it (`[patch.crates-io]`) with a Bevy 0.20 build whose version is 0.0.11 or a 0.0.11-compatible API.

## Pending index

No new external sources: the model reuses this crate's notes.
