# Still water — research note (issue #44)

A body of water a host declares at a fixed level (a sea, a lake, a moat), drawn by `WaterPlugin` and answered by `Environment::water_depth`, with no simulation behind it. This iteration ships the drawn surface and the CPU query; the wave contact, the hydrology idle and the grid-edge continuation are listed under Deviations.

## Sources

- `docs/research/water-rendering.md` (the water pool, the absorb and surface passes, Beer–Lambert on the prepass thickness, the foam rules): everything about how the surface looks is reused unchanged. Its sources: Vlachos 2010 (corpus `s2010-advances-pdf-slides-10`), Mei, Decaudin, Hu 2007 (DOI `10.1109/pg.2007.15`: terrain and water share one height field `h = b + d`, the surface is `b + d`), Pope & Fry 1997 (absorption).
- `docs/research/terrain.md` (leaf bounds: surface min and max per patch node, used here to select patches under the level) and `docs/research/units.md` (the metre frame).
- Corpus search for a still-sea method (`grep -ril "sea of thieves"`, `"Crest"`) found no document; the choice below follows from Mei et al.'s single height field and needs no new source. [INFERENCE: no shipping-game write-up was read for this note.]

## Model adopted

The still level `L` is one more water surface, `L`, next to the simulated surface `b + d`. Over a ground height `b` the drawn column is `max(d, L − b)`: where the simulation has water above `L` it shows, elsewhere the still sea. The same expression sits in the vertex stage (the displaced height `b + depth`) and in the fragment's `column_at` (the discard at 3 mm, the edge fade, the foam from thickness are unchanged), so the shore fades and foams through the existing rules, and the absorb and surface passes colour it with the existing parameters.

Patch selection: a selected patch is drawn when its simulated depth max is ≥ 3 mm **or** `L − (lowest ground in the patch) ≥ 3 mm`, from `TerrainRuntime::water_box` (surface min). The patch's `Aabb` top is raised to `L`.

Past the grid (issue #44 acceptance 5): the terrain grid is finite and its taps clamp to the edge sample, so the sea continues outward at the depth `L − (edge ground)`, which is what the clamp already computes. Four extra tiles in a pinwheel (west and east strips run the grid's full depth plus the reach, north and south the rest) reach `WATER_SKIRT_REACH_M` (100 km, past the horizon of a camera 700 m up) beyond the grid's last samples; each is drawn by the same two passes (a `MeshTag` of `SKIRT_TAG` makes the vertex stage place the 33 × 33 mesh by the transform's scale instead of by CDLOD level, and skip the morph and the last-sample clamp). Where the edge ground is under the level the fragment sees `L − bed` exactly as inside; where it is dry the fragment discards. The depth prepass holds nothing past the grid, so the thickness there reads as the 1 km cap and the absorb pass is the deep-water colour, which is the right colour for an open sea at the horizon. The cloud composite painted every pixel with no mesh below the horizon with its fogged-ground slab (`ground_haze`, a replace blend), which hid the water; `CloudParams::sea` and `sea_box` let the composite leave a pixel whose ray meets the still plane inside the sea's region (grid plus reach) to the water. The skirt keeps the camera's distance fog on (a bed-less pixel has none from a bed). Cost: eight draws of 2048 triangles (four tiles × two passes), only while `StillWater` is set. [INFERENCE: no corpus source; Crest and Sea-of-Thieves-style ocean tiles extend a plane to the horizon the same way, but none was read.]

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
- (5, done in iteration 83) The sea past the terrain grid's border: see Model adopted, "Past the grid". The sea ends at 100 km, not at infinity; past it the composite's ground slab shows as before. (6, done in iteration 82) the foot-world GPU check: `gpu_still_water::gpu_a_still_level_in_a_foot_world_draws_and_answers_as_in_metres`. (2, done in iteration 80) `WaterContact` rings on still water: `waves.wesl` `water_at` takes `max(simulated, level − ground)` from `params.sim.w`, the same expression as the water shader, no new binding.

- (7, iteration 84) The terrain's own edge seen through a shallow still sea. A bed that ends at the grid's border is visible through clear water as a hard teal-to-deep-blue step (iteration 83's frame, row 153: the bed's last row, below the water's edge row 139 because the bed lies 8 m under the surface), and the join of the in-grid film with the skirt stepped by 8-10 levels. Beer–Lambert has no term for it: past the bed's end the prepass holds depth 0 and the thickness saturates at `MAX_THICKNESS`. Model adopted: while a `StillWater` is set, the bed point under a fragment (the prepass depth along the same view ray) within `EDGE_BAND_M` = 32 m of the grid's outermost samples is blended into the deep colour the skirt shows: the absorb pass's transmittance is scaled by `bed_seen = smoothstep(0, band, d)` (`d` the metre distance of the bed point to the nearest edge), the surface pass's body alpha is `mix(1, 1 − exp(−σ_s t_eff), bed_seen)`, and the share `1 − bed_seen` of the camera's fog, which the bed no longer carries, is applied in the film. A first attempt that added 300 m of thickness near the edge moved the step instead of removing it (an exponential's visible part is its tail: the bed's green came back within 4 rows); scaling the transmittance is linear in what the eye sees. An art-direction choice, not a measured model: a real sea has no edge. [INFERENCE: no source read; the 32 m was chosen by the capture, see the log.] Hydrology water (no `StillWater`) is untouched (`bed_seen` = 1).
- (8, iteration 85) Fog in front of water. The bed is fogged in the opaque pass, then the absorb pass (`AlphaMode::Multiply`) multiplies the whole pixel by the Beer–Lambert transmittance `x`, so the fog radiance between the eye and the surface was also tinted by the water's absorption: a fogged sea drew a saturated teal (chroma 0.83 in linear radiance at row 200 of the edge scene) under a grey sky. Fog is a medium in front of the surface, so it must not be absorbed. Model adopted: the surface pass fogs its own film by the camera's fog and adds back the fog share the multiply removed, `Lf · (1 − x)` per channel, with `Lf` the fog radiance over the pixel (`main_pass_post_lighting_processing` on a black film) and `x` the absorb pass's own factor. The fog radiance is taken at the surface's distance, not the bed's (the difference is the water's thickness, metres against fog lengths of hundreds). No source: the compositing identity `F·(bed·x + film) + Lf` follows from Bevy's blend equations; art direction. Result 0.145 (`gpu_a_fogged_sea_is_the_fogs_colour_not_the_waters`). This also applies to simulated water: the same shader.

## Pending index

None (no new corpus document).
