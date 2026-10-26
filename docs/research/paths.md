# Paths and roads: worn ground a game lays across the terrain

The crate presses what a *point* does to the ground — a boot, a hoof, a wheel, one `Imprint` at a time — into a centimetre window that follows the camera (`docs/research/trails.md`). It has nothing for the ground a *route* wears: the desire line across a meadow, the two ruts a cart leaves along a lane, the gravel road that crosses a valley. A game that ships a village has to lay its paths itself, and today the crate gives it no surface for them: the terrain's layers are rock, sand, soil and grass by slope, sediment and noise, and the trail field only holds what an entity pressed in the last window's worth of movement.

This note is the research for a **path surface**: routes a game (or the crate, from its own wear) declares across the terrain, each a polyline with a width, a kind and a depth, drawn into the terrain's own shading as worn, compacted, bare ground with its own colour and roughness, and — where it is deep enough — a rut the terrain's geometry follows.

## Sources

- **Issue and owner's ask** (this iteration): "Can we please focus on roads, path, trails, etc." The crate's trails work is the pressed-footprint half; the laid-route half is missing.
- **Pouyat, Page-Dumroese, Patel-Weynand & Geiser 2020**, *Forest and Rangeland Soils of the United States Under Changing Conditions* (Springer, open access; corpus `10.1007_978-3-030-45216-2`, ll. 1456, 1464, 1466). The trail paragraph used for the model: "Soil erosion is the most widespread trail impact, because trails typically rely on, or result in, a **bared soil surface**"; "motorized and equestrian travel will displace trail soil surfaces more than human foot traffic (Cessford 1995; Newsome et al. 2004; Pickering et al. 2010)"; "**Surface compaction** from recreation activities on unauthorized routes can impede infiltration into soil. This compaction **dries out soil along these tracks and paths** (Settergren and Cole 1970), which can limit vegetative growth"; trail building "protect[s] natural soil and vegetation communities **by concentrating traffic**". That is the physical chain a path surface has to show: bare soil, compacted, drier, no vegetation, wider with heavier traffic.
- **Sumner, O'Brien & Hodgins 1999**, *Animating sand, mud, and snow* (corpus `10.48550_arxiv.2302.08683`): the height-field ground model the trails already use — a plastic carve with a rim of the displaced material, and slump to the material's angle. A road's ruts are the same field at a larger scale and a constant depth along a polyline rather than a swept ellipse.
- **Tatarchuk 2006**, *Practical parallax occlusion mapping* (corpus `s2006-advances-materials-10`): the linear march with a piecewise-linear intersection and soft self-shadows that `trails/surface.wesl` already implements; a rut's visible depth is drawn through the same march, so a path needs no new geometry.
- **Andersson 2007**, *Terrain rendering in Frostbite using procedural shader splatting* (corpus `10.1145_1281500.1281668`, §5.2.4-5.2.5): terrain materials are composited in-shader from **masks** — a height/slope/base-texture set plus "dynamic mask textures" that decals render into, at "4 pixels per meter, because the mask only needs to contain rough circular gradients"; a burnt dirt material made visible over a road by a destruction mask. The precedent for the technique chosen here: a low-resolution mask field, sampled in the terrain's one shader, not a second pass or a mesh.
- **Barré-Brisebois 2014** (snow deformation, GDC; via `trails.md`): snow and ground wear are height maps at a few centimetres per texel, a sliding window, drawn by the surface's own material. A path is the same kind of field at a coarser texel (a metre) with a much longer memory.
- **Green 2007**, *Improved alpha-tested magnification for vector textures* (SIGGRAPH course; via RTR4 §15.4.3, corpus `book_real_time_rendering_4th_akenine` ll. 18976-18996): a coverage **distance field** — per texel, the signed distance to the nearest edge, bilinear-sampled and turned into coverage by a `smoothstep` — antialiases an edge from a coarse grid without aliasing at any zoom. The path mask is a distance field for the same reason: a 1 m texel would otherwise give a 10 cm path a staircase edge.
- **RTR4 §4.7.1, §9.13** (mask textures, material blending; corpus `book_real_time_rendering_4th_akenine`): a terrain "model include[s] dirt, roads, ground cover … each with its own material BRDF", blended in the shading pass.
- **This crate**: `src/terrain/render.wesl` (the fragment's layer weights and the albedo build; the trail march), `src/terrain/common.wesl` (`layer_weights`, `turf_palette`, `ground_detail`), `src/trails/` (the window, the kernels, `TrailState`, `surface.wesl`), `src/terrain/material.rs` (`TerrainMaterialParams`, the `GROUND_TEXTURES`/`GROUND_COLOURS` pipeline-key precedent), `docs/research/ground-colour.md` (the turf palette: a worn tone already exists) and `docs/research/ground-textures.md` (a layer's albedo, relief and roughness multipliers).
- Not obtained: Cessford 1995, Newsome et al. 2004, Pickering et al. 2010, Olive & Marion 2009, Marion 2006, Settergren & Cole 1970 — cited by the synthesis above, whose statements are what the model uses. Recorded under *Pending index*.

## Model adopted

1. **A route is a polyline with a width and a kind.** `Path { points: Vec<Vec3>, half_width_m, kind, depth_m }` — a lane, a footpath, a cart track, a gravel road — laid on the terrain by the game and published as a resource. The surface is drawn, never a mesh: the terrain material reads the routes at each fragment, as the trails field already is. A route's points are world XZ on the ground (its own Y is ignored: the ground is the terrain's), so a route follows the terrain instead of cutting through it.
2. **A mask field, not per-fragment distance to every route.** The routes are rasterised once (on the CPU, when they change) into a **path mask** over the terrain's extent: per texel a signed distance to the nearest route edge (Green's coverage distance field, so an edge antialiases from a coarse grid) and the route's kind and depth. The terrain reads `distance` and turns it into coverage; a texel holds the *nearest* route, which is what a worn ground is: overlapping routes of different kinds are rare and the nearest wins.
3. **Resolution: 1 m a texel.** A path is 0.6-4 m wide and its edge is 0.1-0.3 m of fade, so a 1 m texel with a distance field resolves the edge to about a centimetre of antialiasing; a 2048 m terrain is 2048² texels (16 MB at `rgba16float`, 4 MB at `rgba8unorm` with the distance scaled). [INFERENCE: the width-to-texel ratio; Andersson's destruction mask is 0.25 m a texel for cruder gradients.] The mask is generated only when the routes change; a still route costs one texture sample in the fragment.
4. **What the surface is.** Inside a route, by coverage:
   - **Bare ground**: the layer weights move from grass and soil to the route's own worn earth — the crate's `SOIL` darkened and desaturated toward the gravel of its kind (`path kind`: earth lane, gravel road, stone road). The turf's palette is cut where the coverage is high, so no grass survives in the middle of a lane.
   - **Compaction**: lower roughness (0.96 → 0.86), a flatter micro-relief (the procedural grain's gradient × (1 − 0.7 · coverage): a trodden path has no clods), and — in snow — a shallower, denser snowpack than the field around it (the compaction the source names: snow on a path is packed by the traffic).
   - **Ruts**: the route's `depth_m` (a cart track 6 cm, a road 3 cm) presses the field along the route's *edges*, where wheels run: two bands at ±0.6 of the half width for a cart track and a road, one centre band for a footpath (Sumner's carve, at a constant depth along the polyline, no rim: a road's displaced material has long since been pushed aside). The terrain's trail march draws it like any rut — the same parallax, the same self-shadow — so a deep rut reads as a trench and a shallow one as a depression with no extra code path.
   - **Drainage**: a road on a side slope has its outer edge cut and its inner edge built up (a bench), which is the one place a path moves the ground's *shape*: at most 0.3 m at the route's edges, faded by the coverage. [INFERENCE: from the source's "siting trails on side slopes where water bars and drainage-control features can limit erosion"; the bench depth is a game's `depth_m`.]
5. **Wear the weather cannot erase, and wear it can.** A laid route's mask is the game's and persists; what the traffic *adds* stays in the trails field, which the weather fills and smooths as it does today. So a path is worn bare (persistent) and its ruts soften (the field) — the two systems stay separate, each with its own memory.
6. **Procedural wear (optional, `PathsPlugin`).** The crate can also grow paths from wear: an `Imprint`'s trail depth accumulated into the path mask as *worn* coverage, so a route walked enough becomes a bare path (the desire line). This is a second, slow accumulator on the CPU readback of the trail field (the deepest `h` per readback cell already exists), and it is opt-in (`PathSettings::wear`).

## Parameters

| Parameter | Value | Basis |
|---|---|---|
| Mask texel | 1 m (2048² over a 2 km terrain) | Andersson's 0.25 m destruction mask, scaled for a path's 1-4 m width [INFERENCE] |
| Mask format | `rgba16float`: signed distance (m), kind (0 earth, 1 gravel, 2 stone, 3 snow track), depth (m), wear (0-1) | Green's distance field; kinds are the surfaces |
| Edge fade | 0.12 m + the kind's own edge | a path's edge is trodden, not cut: 12 cm of grass thinning [INFERENCE] |
| Default half widths | footpath 0.35 m, lane 1.2 m, cart track 1.0 m with two ruts at ±0.6, gravel road 2.5 m | a walker's single track, a farm lane, a cart's wheelbase (1.5 m) [INFERENCE from `trails` example] |
| Rut depth | footpath 1.5 cm, cart track 6 cm, gravel road 3 cm | Sumner's carve at the traffic's load; CRREL/Bekker give the sinkage of one pass, a route is many [INFERENCE] |
| Roughness | 0.96 → 0.86 by coverage | compacted, finer surface |
| Grass cut | coverage above 0.6 | "bared soil surface" |
| Compaction (snow) | snow depth × (1 − 0.5 coverage), density raised | the source's compaction |
| Bench | up to the route's depth, at the edges, on side slopes | [INFERENCE] |
| Pipeline key | `PATHS` when any route exists, so a terrain with none pays nothing | the `GROUND_TEXTURES` precedent (AGENTS: a branch that never runs still costs) |

## GPU mapping

- A new `path` lane in `TerrainMaterialParams`: x the mask's extent (m), y its texels a side, z 1 when a mask is bound, w the wear's share. The mask is `texture_2d` binding 109 with 110 the sampler (an extension's bindings are the material's; the terrain currently binds 100-108).
- `render.wesl`: after the layer weights and the ground detail, a `path_at(p)` samples the mask's distance, kinds and depth, and mixes the albedo, the roughness scale, the grain's gradient, the snow cover and the trail carve. The pipeline key (`TerrainExtensionKey::paths`) compiles the block only into a material with a mask, exactly as the ground textures' block is.
- The ruts go into the trails field at the mask's depth (a per-frame stamp of the routes into the trail window on the CPU, or a `path` field the march also reads: the trail kernel stamps them as the window moves, so a rut follows the route as the camera travels).
- `GroundExtension` (a game's own ground) reads the same lane when a mask is set.

## What great looks like

A gravel road crossing the hills: a 5 m band of pale grey stone over the turf, its edges ragged with grass thinning into it over 10-20 cm, a shallow crown and two wheel ruts with dark, damp floors, loose stones at the verge, no grass in the wheel tracks, and the turf's clods flattened across it. A footpath up a slope: a single trodden line 70 cm wide, bare dark earth, a slight bench cut into the side slope, faint boot ruts, grass closing over it in patches where traffic is light. A cart track across a meadow: two dark ruts 1.5 m apart with a grassy strip between, exactly as wide as the cart. In snow: a path packed and grey against the fresh white, its ruts blue-shadowed. At every angle the edges are smooth (a distance field, not a texel staircase), the surface is worn to the same depth down its length, and nothing pops as the camera moves.

## Validation targets

1. **The mask (CPU, `paths::model`).** A straight route 2 m wide at 1 m texels has its coverage 1 inside 0.9 × the half width, 0 beyond 1.1 ×, and monotone between; a route at any angle has the same edge width (the distance field is isotropic); two routes crossing take the nearer's kind; a route outside the terrain's extent is clipped; the mask's generation is deterministic.
2. **Coverage from distance (CPU mirror of the shader).** `coverage(d) = smoothstep(half_width, half_width − fade, d)`: 1 inside, 0 outside, 0.5 at the edge, and no coverage where no route is.
3. **The surface (GPU, `a_path_wears_the_ground_bare`).** On the hills' meadow with a 2 m earth lane: the albedo at the centre is the path's earth (within a level of the CPU's mix), the grass weight at the centre is below 0.05, the roughness is 0.86, and 3 m away every value is the unpathed ground's.
4. **The ruts (GPU, `a_cart_track_cuts_two_ruts`).** A cart track 1 m half width, 6 cm depth: the trail field at ±0.6 m from the axis is 6 cm down (±20 %), the grass strip between them is untouched, and no rut appears off the route.
5. **Edges (visual).** At 1.5 m above the ground and looking along a diagonal route: no texel staircase on the edge, and the coverage's 0.12 m fade reads as grass thinning rather than a cut line.
6. **The default is free.** A terrain with no route renders exactly as before (≤ 1 level on 0.05 % of the pixels), and its opaque pass is unchanged within noise (the block behind the shader def).
7. **Performance.** The terrain's opaque pass within 2 % of the baseline with a mask bound (one texture sample and a few mixes), and the mask's generation under 30 ms for 2048² on the CPU.

## Deviations from this plan

None yet: the note precedes the implementation. Deviations found in the capture review are recorded here as they are fixed.

## Pending index

Not in the store (cited by the synthesis and recorded rather than read): Cessford 1995, Newsome et al. 2004, Pickering et al. 2010, Olive & Marion 2009, Marion 2006, Settergren & Cole 1970. Every other source above is in the corpus.
