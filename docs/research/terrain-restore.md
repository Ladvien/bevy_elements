# Restoring a region of the terrain from a snapshot

bevy_emerge's editor (issue #99 there) undoes a raise or a lower with `SculptTerrain::inverse`. Smooth, Flatten and Noise have no inverse (they depend on the ground they met), and `TerrainSnapshot` can only respawn the whole terrain, which drops the water, the cover maps and the trails. An editor needs to put the ground of one stroke's footprint back as it was, in place.

## Sources

- Corpus search for method phrases (`undo brush`, `heightfield edit history`, `sculpt undo`) finds nothing relevant: the library holds simulation and rendering work. Recorded, not searched further. The technique (an undo patch holding the touched region's prior values, applied by overwriting) is the standard one in terrain editors and needs no citation for its maths.
- `docs/research/terrain-sculpt.md` (a stroke edits the bedrock; surface = bedrock + sediment) and `docs/research/terrain-heightfield-save.md` (Deviation 1: sediment was not restored by a respawn; the snapshot already carries it).
- `AGENTS.md`: a request made to the render world is a plain field of the extracted resource with an epoch, answered through an atomic; `ShaderBuffer::with_size` layers keep a stale main-world copy once the GPU has written them, so a layer is never rewritten from the main world.

## What great looks like

A stroke, then a restore of its footprint from a snapshot taken before it, leaves every cell of the footprint exactly as it was (bedrock and sediment, to f32 rounding) and every cell outside it as the stroke left it, with no seam at the rect's edge, and the drawn ground, the rocks, the cover and `Environment::ground_height` follow on the next frame like they follow a stroke.

## Model adopted

1. `RestoreTerrain { snapshot: TerrainSnapshot, region_m: Rect }`, a `Message`. Every cell whose centre lies in `region_m` (metre-frame XZ) takes the snapshot's bedrock (`surface - sediment`) and sediment.
2. Applied in order with the strokes, one edit per frame, so "stroke, restore, stroke" keeps its order. One queue of edits; the stroke kernel's epoch and a new restore epoch are both caught up before the next edit is handed over.
3. The render world writes the region's rows into the bedrock and sediment layers with `RenderQueue::write_buffer` in the terrain's `prepare`, before the frame's kernels run, then bumps `changes` so the pack, bounds and readback kernels follow. No new kernel.
4. A snapshot whose grid differs from the live one (size, cell, or origin by more than a thousandth of a cell) is dropped with a warning: the readback is coarser than the simulation above 1024², and a snapshot from another terrain has other cells. `validate()` checks the rect without a terrain, so an editor can check first.

## Parameters

None.

## GPU mapping

No kernel. Two row-wise `write_buffer` calls per region row (bedrock, sediment), each `width x 4` bytes at offset `(z * size.x + x0) * 4`.

## Validation targets

- CPU: the cell range of a rect (centres in, edges clamped, empty rect gives no cells); `validate` rejects a non-finite or empty rect.
- GPU (`tests/gpu_sculpt.rs`, Metal): snapshot, Smooth+Raise strokes, `RestoreTerrain` of a rect covering part of the hill: cells inside the rect equal the before-snapshot's surface within 2e-4 m, cells outside still carry the stroke (the hill's height there), and sediment inside equals the snapshot's.

## Deviations from this plan

None.

## Pending index

None: no document was added to home-still.
