# Trails on a game's own ground (issue #26)

`TrailsPlugin` (`trails.md`) presses footprints, ruts and tracks into a centimetre height field that follows the camera, and draws it on the crate's terrain and on `GroundExtension`. Both read the crate's `Terrain` and its ground-cover maps: the field asks them for the ground's height (is a sole in contact?), its snow depth and density, its sediment, slope and wetness. A game that owns its ground (its own heightfield, its own ground mesh in `WeatheredMaterial`, no crate `Terrain`) gets nothing: the plugin idles (`src/trails/mod.rs`, `ensure_trails`). Issue #26 asks for trails on that ground, a gameplay query for them, and for the window to follow the `ElementsFocus` of a strategy camera. This note covers what changes; the deformation model itself (Bekker–Wong sinkage, the CRREL cap, rims, slumping, the weather erasing the prints, the parallax drawing) is `trails.md`'s and is not repeated.

## Sources

- **Barré-Brisebois 2014**, *Deformable Snow Rendering in Batman: Arkham Origins*, GDC (slides `colinbarrebrisebois.com/wp-content/uploads/2022/06/gdc2014-deformable_snow_rendering.pdf`, read as text: `/tmp/i27/pdf/bao.txt`). The case of this issue exactly: "Raycast on a terrain / Modify terrain mesh. We don't have terrain. We have rooftops and streets." Snow lies on *snow surfaces*, meshes the artists mark, each with its own heightmap whose UVs align with an ankle-high orthographic frustum ("0-1 range, simplifies heightmap-to-displacement"), resolution `min(512, ¼ · surface size)`; "for every active snow surface: figure out if [a] surface-affecting object is on the surface" (per-surface actor lists); "add [a] small value to the heightmap to make snow gradually replenish (since it's snowing)"; two material stages (undeformed and deformed snow) blended by the heightmap with relief mapping on consoles, tessellation on PC; < 1 ms on PS3/360; heightmaps allocated and freed by the player's position and visibility.
- **Tran 2018**, *Creating Snow Trails in Unreal Engine 4* (Kodeco, `kodeco.com/5760-creating-snow-trails-in-unreal-engine-4`, read): the engine-agnostic recipe games use on arbitrary ground: a world-space render target "store[s] trails in a defined area rather than the entire world", centred on the player, projected onto the ground material by world XY ("center the render target so that the middle is at (0, 0) in world space … then convert from world space to UV space"), and only materials that sample it show trails. Its follow-ups (Gawron 2018, *Interactive Snow Shader*, ArtStation; `roymasad/Unreal-Snow-Deformation`, GitHub: "some objects can deform snow, some objects act as snow surfaces") keep the same split: a surface opts in.
- **Hanák 2021** (`trails.md`, Sources): the sliding window of *Rise of the Tomb Raider* (1024² × 4 cm) and *Horizon Zero Dawn: The Frozen Wilds*; *The Last of Us Part II* draws its prints on whatever mesh the character stands on as decals with parallax occlusion mapping and a written depth.
- **Tatarchuk 2006**, parallax occlusion mapping (corpus `s2006-advances-materials-10`): "parallax occlusion mapping can be successfully used on procedurally generated height fields" (p. 50); the march `trails.md` adopted, which needs only a height field and the surface's own plane, not a terrain.
- **CRREL 90-9** and **Alvarado et al. 2022** (`10.48550_arxiv.2209.10215`, home-still; `trails.md`): the shallow-snow compaction cap needs only the snow's depth and density; Alvarado's model is per footprint, ground-agnostic.
- **`bevy_flora` 0.46 `FloraGround`** (`bevy_flora-0.46.0/src/ground.rs`): the family's way for a game to say where its ground is: `FloraGround::new(|xz| height)` with world XZ in and world height out, "an analytic height or a heightmap lookup, not a ray cast". `TrailGround` takes the same shape so a game hands both crates one closure.
- In the crate: `view-anchor.md` (the focus the camera-relative domains centre on), `readback-pacing.md` and `cloud-shadow-query.md` (the readback pattern of the `Environment` queries), `snow-season.md` (the point snowpack `SurfaceWeathering` and a pack a game inserts).

## Model adopted

1. **`TrailGround`** (resource): the game's ground, `TrailGround::new(|xz: Vec2| height)` (world XZ to world height, world units, `FloraGround`'s shape; `TrailGround::flat(y)`), and whether it is sand (`with_sand()`; soil otherwise, which turns to mud with the wetness). While it exists the trails run on it, with or without a crate `Terrain` (a game that inserts it says the ground is its own).
2. **The ground under a texel** is uniform over the window (Barré-Brisebois's surfaces carry one snow): snow depth and density from `SurfaceWeathering`'s point pack, the wetness from `SurfaceWeathering`, sand or soil from `TrailGround`, no rock. The point pack is the authority on new snow, as the cover map is on a terrain: a trench fills by what its depth gained (Barré-Brisebois's "replenish since it's snowing", here from the weather's own snowfall), melt clamps it to the snow, wind smooths it, wet sand and mud slump.
3. **Contact** is tested on the CPU, once per imprint: the sole presses while it is no more than 0.1 m above `TrailGround`'s height plus the snow at its centre (the terrain path's rule; Barré-Brisebois's "is the object on the surface"). The kernel then treats it as in contact.
4. **`WeatheredMaterial` draws trails when it opts in**, `WeatheredExtension::with_trails()` (Barré-Brisebois's snow surfaces; Tran's ground material): the same parallax march as `GroundExtension` over the fragment's own plane (from its geometric normal), the hit's position, normal, self-shadow and openness in the lighting, compacted snow on a trench floor and the material's own colour where a trench reaches the bottom of the snow, water on the floors of ruts in saturated ground. A face steeper than 60° draws none (a wall is not walked on). Off by default: a roof over a path would otherwise show the path's prints. Forward rendering only (the self-shadow needs the lights).
5. **The window follows the focus, and the materials know where it is.** The kernels already centre the window on `ElementsView::position` (the `ElementsFocus`, else the camera), but every material faded the field by its distance from the *camera*, so an isometric camera 1500 m back faded every trail. `TrailState::centre` publishes the window's centre; the terrain and ground materials take it in their uniform, `WeatheredMaterial` in the shared globals.
6. **A wider window for a far camera:** `TrailSettings::texel_m` (default: the quality's, 5 cm at High) trades detail for reach at the same memory, as Barré-Brisebois scales his heightmap's texel by the surface: 8 cm gives 82 m at High's 1024², enough for an isometric camera that sees 70 m of ground.
7. **`Environment::trail_depth(Vec2) -> Option<f32>`**: how far the ground at a world XZ is pressed below the undisturbed surface, m (0 on a rim or untouched ground inside the window; `None` outside it and before the first readback). A fifth kernel writes, on readback ticks only, the deepest point of each k × k block of texels (k = max(2, N/512): 10 cm cells at High, 512² of them, 1 MB) with the window's origin in a header, so the answer is placed by the frame that computed it, not the frame that asked.
8. **`SurfaceWeathering::with_snowpack(depth, density, wetness)`**: a pack a game inserts (a level that starts under snow), treated as one read back from a save (the cover maps, if any, are seeded from it). It replaces the crate's test-only constructor.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| Contact tolerance | 0.1 m above `TrailGround` + snow | `trails.md` |
| Snow on game ground | `SurfaceWeathering` depth and density | `snow-season.md`; CRREL |
| Steepest face drawn | 60° (n.y ≥ 0.5, faded over 0.5–0.6) | [INFERENCE] (walls are not walked on) |
| `texel_m` override | none by default; 8 cm suggested for an isometric camera | Barré-Brisebois (texel by surface); RotTR 4 cm |
| Readback cell | k = max(2, N/512) texels, deepest point | [INFERENCE] (≤ 1 MB per readback) |
| Readback rate | `ElementsReadback::interval` (4 Hz) | `readback-pacing.md` |

## GPU mapping

- `TrailParams` gains `ground: vec4` (snow depth m, snow density, wetness, sand 0/1) and a mode flag; with it set, `ground_at` returns the uniform ground and `surface_y` the plane of the imprint's own CPU contact test, and the terrain and cover bindings hold one-cell placeholders. `TrailGpu` owns the handles it binds (the terrain's and cover's buffers, or the placeholders), so the render world no longer needs `TerrainGpu` or `CoverGpu`.
- `trail_readback`: one thread per readback cell, after `trail_relax`, on its own layout (the uniform, the state, the readback buffer) so the other four kernels keep seven storage buffers; dispatched only on a readback tick.
- `ElementsGlobals` grows from six to eight `vec4` (96 → 128 B): `trail` (texel m, texels, 1 when bound, the march's bound) and `trail_light` (the beam's share, the window's centre XZ, the snow depth), the same two vectors the terrain uniform carries. It is rewritten in place each frame, so the moving centre re-prepares no bind group. `WeatheredExtension` gains a uniform word (trails on) and one texture and sampler (bindings 104, 105): no storage buffer, so the fragment stage's eight are untouched.
- Terrain and ground materials: the window centre goes into `trail_light.yz`, which their uniforms already carry.

## What great looks like

The issue's dawn: under the isometric camera at 35°, after a night of a walker crossing fresh snow on a game's own plane, a line of distinct prints curving between the houses, legible as footprints from the strategy view, shadowed on their sun side at a low sun; at noon still legible. Up close, through a perspective camera on a game's own hilly mesh, the prints and ruts exactly as good as on the crate's terrain: crisp walls in snow, the dark ground in a rut through thin snow, rounded rims in sand, glossy floors in mud. Nothing on the roofs or walls above the path; no edge where the window ends, anywhere on screen; a trail that stays put when the focus moves.

## Validation targets

1. **Acceptance (GPU, `tests/gpu_trails.rs`).** No `Terrain` entity; `TrailGround::flat(0.0)`; `SurfaceWeathering::with_snowpack(0.10, 150, 0)`; an `Imprint` (walker sole, 760 N) moved 20 m along the ground: `Environment::trail_depth > 0.05` every 0.5 m along its path; no `Terrain` entity at the end.
2. **Contact on a sloped game ground (GPU).** `TrailGround::new(|xz| 0.1 · x)`: a sole that follows the slope presses; the same sole 0.3 m above it presses nothing.
3. **New snow fills a print on game ground (GPU).** Raising the inserted pack from 10 to 15 cm shallows a trench by 5 cm (±0.5 cm).
4. **The query (GPU and CPU).** `None` before the first readback and outside the window; `Some(0.0)` on untouched ground inside it; the readback's header places a moved window correctly (decode unit test).
5. **Window centre (visual and GPU).** Under the isometric camera following a walker, prints across the whole visible window, none faded by the camera's distance; `ElementsFocus` moved 10 m keeps the print at its world position (the existing window test, on game ground).
6. **`WeatheredMaterial` draws the print (GPU pixel test).** Top-down onto a snowed weathered plane with `with_trails()`, the pixels over a pressed sole differ from the flat snow beside it; without `with_trails()` they do not.
7. **Visual.** `isometric` with its walker pressing prints (Snow grown 12 h, then Clear at 7.5 h and noon; pitch 30° and 35°), and `trails` with `ELEMENTS_GROUND=mesh` (a game's own hilly mesh in `WeatheredMaterial`, no `Terrain`) at 2–20 m in snow, sand and mud: GREAT in the critique, crops at 3–4×, before and after.
8. **Cost.** `elements_trails` on game ground no more than on the terrain (the uniform ground skips every map lookup); the readback kernel ≤ 0.02 ms on its ticks; the weathered material's march the cost `GroundExtension` measured (`trails.md` Validation 8). The terrain path unchanged.

## Deviations from this plan

(none yet)

## Pending index

- Barré-Brisebois 2014 (GDC slides, no DOI): read from the PDF's text; not in home-still.
- Tran 2018, Gawron 2018, `roymasad/Unreal-Snow-Deformation`: web pages, no DOI.
- `distill_search` through the MCP bridge returns HTTP 404 (as in `trails.md`); searched with `hs distill search` instead.
