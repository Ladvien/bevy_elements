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

**Versions.** Both stay at 0.0.11 (Deviation 1).

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
- `isomesh` 0.0.10's API for every item this crate uses (`Sdf`, `ChunkId`, `ChunkLayout`, `chunk::dirty::{DirtySet, mark_edit}`, `marching_cubes::MarchingCubes`, `MeshBuffer`, `fields::Sphere`, `brush::smooth_min`) is compared with 0.0.11's.

## Deviations from this plan

1. **The `isomesh` feature does not accept 0.0.10.** The plan was to widen the `isomesh` requirement to `>=0.0.10, <0.0.12`: every item this crate uses is identical in both (`Sdf` trait and `ChunkLayout` signatures diffed, the rest found at the same paths). But a widened core requirement builds nothing new. bevy_isomesh 0.0.11 requires `isomesh = "0.0.11"`, which for a `0.0.x` version means exactly 0.0.11, so every build resolves the core to 0.0.11; a second copy at 0.0.10 would make `SnowyField`'s `Sdf` impl another trait than the one bevy_isomesh samples. bevy_isomesh 0.0.10 itself is built on Bevy 0.19 (pin parity, AGENTS.md "Plays well with the family" 1) and has neither `VoxelVolume::replace_field` nor `IsomeshSystems`, both used here. So both stay at 0.0.11, a published Bevy 0.20 release; a game on a path checkout labelled 0.0.10 moves it to 0.0.11 (or patches with `[patch.crates-io]` from a checkout versioned 0.0.11).
2. **`VoxelSnow` has no `resolution`.** The plan gave it one; the columns are one per voxel, and wider by doubling only past `ElementsQuality::terrain_max_resolution` a side (`snow_columns`), which is what a wall wants, so the field was dropped (`VoxelSnow` is a unit component).
3. **Across a cliff the slab takes the nearest column's top and snow, not the bilinear blend (found in the capture review).** `voxel-snow.md` samples the tops and the depths bilinearly, which the voxel hill never noticed: no step on it is steeper than a voxel. On crenellated walls (0.25 m voxels, 1 m merlons) a top blended between a walk column (5 m) and a merlon column (6 m) floated the slab's underside 0.2 m above the walk in front of every merlon, and a top blended between a wall column and the "no top" of the air beside it hung the slab down the wall's face: black wedges at every merlon's foot and a wavy seam along each wall's base (`/tmp/i23/seq/frame0079.png`). `SnowyField` now checks the four taps it blends: when they differ by more than one voxel `c` it takes the nearest column's top and depth, so a step keeps a sharp edge and the slab sits on both sides of it; elsewhere nothing changes (`the_slab_sits_on_both_sides_of_a_cliff`). On a `VoxelGround` this affects only slopes steeper than 45° per cell, which the shed exposure keeps nearly bare anyway.

## Pending index

No new external sources: the model reuses this crate's notes.
