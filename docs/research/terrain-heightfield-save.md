# Saving and restoring a sculpted heightfield

bevy_emerge's issue #99 ("Sculpt terrain, and pieces sit on it") has an editor that raises, lowers, smooths, flattens and roughens a live terrain with `SculptTerrain`. The edits live only on the GPU: a project that is saved and reopened, or an undo of a Smooth, Flatten or Noise stroke (which have no `inverse`, they depend on the ground they met), needs the heights back on the CPU and a way to put them back. The crate already reads the surface back every time it changes (`TerrainSnapshot`, behind `Environment::terrain_height`); it has no public door to it and no way to turn it into a terrain.

## Sources

- Corpus search for method phrases (`heightfield serialization`, `terrain save`, `heightmap export`) finds nothing: the library holds simulation and rendering work. Recorded, not searched further.
- `docs/research/terrain.md` (layers, readback, "Heightmap import": the kernel resamples the red channel, exact when source and destination sizes agree, per `heightmap.wesl` `axis_tap` with ratio 1).
- `docs/research/terrain-sculpt.md` (bedrock is what a stroke edits; surface = bedrock + sediment).
- `AGENTS.md`: readback is box-averaged by 2 above 1024², so a snapshot may be coarser than the simulation grid.

## What great looks like

A terrain carved with strokes, saved as a snapshot, and respawned from it comes back with the same heights to within f32 rounding at every cell centre, at the same extent and position, with no seam, step or shifted origin, so a saved sculpt is indistinguishable from the live one.

## Model adopted

1. `Environment::terrain_snapshot() -> Option<TerrainSnapshot>`: the last readback (cheap clone, the heights are `Arc`ed). `TerrainSnapshot` is already public and exported.
2. `TerrainSnapshot::height_range_m() -> Range<f32>`: lowest and highest stored surface height (widened to 1 m when level, so a range is never empty).
3. `TerrainSnapshot::to_heightmap() -> (Image, Range<f32>)`: an `R32Float` image of the surface, one texel per cell, value `(h − lo) / (hi − lo)`, with the range that decodes it, exactly what `TerrainSource::Heightmap` consumes.
4. `TerrainSnapshot::terrain(&self, images: &mut Assets<Image>) -> Terrain`: a `Terrain` (extent = cells × cell size, resolution = the snapshot's cells) over that image. `TerrainSnapshot::centre_m() -> Vec3` is where its entity belongs in the metre frame (`x, y0, z`; multiply by `ElementsUnits::units_per_metre` for a world `Transform`).

Sediment is not restored: a heightmap terrain starts with no sediment, the whole surface becomes bedrock. Surface heights, which drive everything a game queries and draws, are exact; the soil layer over rock is lost. Recorded under Deviations.

## Parameters

None. The snapshot's grid is the readback's (2× coarser than the simulation above 1024² cells); a restored terrain has the snapshot's resolution.

## GPU mapping

None new. The restore uses the existing heightmap generator; the snapshot uses the existing readback.

## Validation targets

- Test (GPU, `tests/gpu_heightfield_save.rs`): carve a 5 m hill into a flat 128² terrain, take the snapshot, despawn the terrain, spawn `snapshot.terrain(..)` at `centre_m`, and read `Environment::terrain_height` at every cell centre: equal to the snapshot's heights within 2e-4 m (f32 normalisation error: range 5 m × 1.2e-7 plus the kernel's multiply), and outside the hill exactly the original level.
- CPU: `to_heightmap` of a ramp holds `(h − lo)/(hi − lo)` at every texel and the range returned decodes it; a level snapshot has a non-empty range and zeros; `terrain()` carries the extent and resolution of the grid.

## Deviations from this plan

1. Sediment is not part of the restore (a `Heightmap` source sets it to zero). Restoring it needs a second image and a kernel change; the surface is what a saved sculpt is for.

## Pending index

None: no document was added to home-still.
