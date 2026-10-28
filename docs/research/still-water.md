# Still water — research note (issue #44)

A body of water a host declares at a fixed level (a sea, a lake, a moat), drawn by `WaterPlugin` and answered by `Environment::water_depth`, with no simulation behind it. This iteration ships the drawn surface and the CPU query; the wave contact, the hydrology idle and the grid-edge continuation are listed under Deviations.

## Sources

- `docs/research/water-rendering.md` (the water pool, the absorb and surface passes, Beer–Lambert on the prepass thickness, the foam rules): everything about how the surface looks is reused unchanged. Its sources: Vlachos 2010 (corpus `s2010-advances-pdf-slides-10`), Mei, Decaudin, Hu 2007 (DOI `10.1109/pg.2007.15`: terrain and water share one height field `h = b + d`, the surface is `b + d`), Pope & Fry 1997 (absorption).
- `docs/research/terrain.md` (leaf bounds: surface min and max per patch node, used here to select patches under the level) and `docs/research/units.md` (the metre frame).
- Corpus search for a still-sea method (`grep -ril "sea of thieves"`, `"Crest"`) found no document; the choice below follows from Mei et al.'s single height field and needs no new source. [INFERENCE: no shipping-game write-up was read for this note.]

## Model adopted

The still level `L` is one more water surface, `L`, next to the simulated surface `b + d`. Over a ground height `b` the drawn column is `max(d, L − b)`: where the simulation has water above `L` it shows, elsewhere the still sea. The same expression sits in the vertex stage (the displaced height `b + depth`) and in the fragment's `column_at` (the discard at 3 mm, the edge fade, the foam from thickness are unchanged), so the shore fades and foams through the existing rules, and the absorb and surface passes colour it with the existing parameters.

Patch selection: a selected patch is drawn when its simulated depth max is ≥ 3 mm **or** `L − (lowest ground in the patch) ≥ 3 mm`, from `TerrainRuntime::water_box` (surface min). The patch's `Aabb` top is raised to `L`.

CPU: `Environment::water_depth` returns the higher of the simulated depth and `max(L − terrain_height, 0)`, so it works without `HydrologyPlugin`. The level is in world units (`ElementsUnits`), in the frame of `Environment::terrain_height`.

## Parameters

- `StillWater { level: f32 }`, a resource; absent means no still water. World units. Updating it rewrites one uniform lane (`WaterParams::still.x`, metre frame, −1e9 when absent): no buffer or terrain is rebuilt.

## What great looks like

A shore that turns from the sand's colour through blue-green to deep blue as the depth grows (red goes first), a thin foam line where the ground crosses the level, no row of missing water at a patch seam, the sky and sun in the surface at grazing angles.

## GPU mapping

No new pass, buffer or binding. One `vec4` in the water uniform. Per-fragment cost: four more `terrain_cells.x` taps in `column_at` (already read for the vertex), one `max`.

## Validation targets

1. Mean red under 0.3 m of still water ≥ 5× the mean red under 8 m (Beer–Lambert at `absorption.x = 0.45`: about 32×); top-down over a sloping shore.
2. `Environment::water_depth` equals `max(L − bed, 0)` within 1 cm over a test terrain, and agrees with the drawn absorption depth.
3. A foot world (`units_per_metre = 3.2808`) with `level = 0` draws and answers as the metre world.
4. `wet_patches`: only patches with ground under the level are drawn (unit test).

## Deviations from this plan

- (3, done in iteration 81) Zero hydrology substeps with a still level: `hydrology::model::still_idle` holds the scheduler at 0 while `StillWater` is set, no bake runs, rain and every source rate are ≤ 0, and a depth readback has shown a mean depth under `QUANTUM_M` (5 µm; the crate's own "no water" floor). The readback lags by up to its interval, so water a host writes into the layers is noticed one readback late. Thermal slumping runs inside the substeps and so pauses while idle; it resumes with the first rain or source. [INFERENCE: no corpus source covers this; it is a scheduling rule, not a model.]
- Not in this iteration, issue #44 stays open: (5) the sea past the terrain grid's border (the shader clamps its taps to the grid edge); (6, done in iteration 82) the foot-world GPU check: `gpu_still_water::gpu_a_still_level_in_a_foot_world_draws_and_answers_as_in_metres`. (2, done in iteration 80) `WaterContact` rings on still water: `waves.wesl` `water_at` takes `max(simulated, level − ground)` from `params.sim.w`, the same expression as the water shader, no new binding.

## Pending index

None (no new corpus document).
