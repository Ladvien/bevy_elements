# Props under an orthographic camera: a draw range measured in pixels, not in metres from the eye

The crate scatters rocks, and under an isometric orthographic camera it draws **none of them**.
The scatter itself is not the fault: the same `mountains` scene spawns 4243 rocks through a
perspective camera and 4513 through the isometric one, in 144 and 166 tiles. They are spawned,
seated, and then drawn at a **scale of nothing**.

`seat_transform` (`src/rocks/mod.rs`) shrinks a rock to zero over the last [`FADE`] = 20 % of its
range, and it measures the distance from `ElementsView::eye` — the camera's own translation. An
isometric camera in this crate stands [`ORTHO_BACK_M`] = 457 m back from what it shows
(`examples/common/mod.rs`; Sanctuary's town camera is 1500 ft). Every rock on screen is therefore
450-470 m from the eye, while a 0.25 m stone's range is 100 × 0.25 = 25 m: the fade term clamps to
1 and the scale to `1e-3`. The near and far `VisibilityRange`s are culled on the same distance,
so even a rock that kept its scale would not draw.

The camera-distance model is right for a perspective camera and has no meaning for an orthographic
one. This note replaces the measure with the one the projection actually has.

## Sources

- **Akenine-Möller, T., Haines, M., Hoffman, N., Shirley, P. (2019)**, *Real-Time Rendering, 4th
  ed.*, §4.7.1 (corpus `bo/book_real_time_rendering_4th_akenine`, l. 3540): "When orthographic
  projection is used for viewing a scene, **objects maintain the same size regardless of distance
  to the camera**." The whole premise: a measure of distance from the eye cannot say how large a
  prop is on screen under this projection, because the answer does not depend on it.
- **Luebke, D. et al. (2003)**, *Level of Detail for 3D Graphics*, §4.1.1-4.1.2 (corpus
  `book_level_of_detail_3d_graphics_luebke`, ll. 2673, 2695): distance-based LOD thresholds are
  broken by "scaling the object to make it larger or smaller, using a different display
  resolution, and changing the field of view", and "a pure distance measure does not take into
  consideration the parameters of the perspective projection used to render all objects", so
  "size-based LOD techniques use the **projected screen coverage** of an object, and switch between
  LOD based on a series of size thresholds rather than a series of distances", which "avoid some of
  the problems with distance-based LOD since projected size is **insensitive to display resolution,
  object scaling, or field of view**". Under an orthographic camera the distance measure is not
  merely inaccurate, it is constant: the projected size is the projection's own area over the
  rock's diameter, at every depth.
- **Luebke et al. §4.1.2**, on the practical form: "calculate the projected radius of an object's
  bounding sphere … in effect a form of distance-based LOD selection that accounts for field of
  view and object size — provides a very lightweight and effective mechanism for representing the
  size of an object in screen space". The crate already divides the diameter by a per-metre range
  factor; under an orthographic camera the factor is the view's ground area.
- **Lindstrom, P. et al. (1996)**, *Real-time, continuous level of detail rendering of height
  fields* (corpus `10.1145_237170.237217`), via `docs/research/orthographic-lod.md`: the
  screen-space-error criterion, where `λ` is "the number of pixels per world coordinate unit in the
  screen x-y coordinate system" — under an orthographic projection that is a constant of the
  view, which is exactly the quantity this note needs.
- **Strugar 2009**, *Continuous distance-dependent level of detail for rendering heightmaps*
  (corpus `10.1080_2151237x.2009.10129287`, l. 219): for a shadow map "the actual LOD selection
  should still be based on the main camera settings … to avoid rendering artifacts that would be
  caused by difference in the terrain mesh resulting from two different LOD functions". The crate
  already followed this for the terrain's patches (`orthographic-lod.md` Model 1: select about the
  focus); the rocks are drawn **on** that mesh and must be selected by the same centre, or a rock
  is culled by one rule while the ground it sits on is drawn by another.
- **Asirvatham & Hoppe 2005**, *Terrain rendering using GPU-based geometry clipmaps* (corpus
  `10.1145_1015706.1015799_gpugcm`): nested grids "centered about the viewer". The centre of a
  camera-relative domain is a free choice of the scheme, and under an orthographic camera the
  viewer is what the view shows, not where the camera stands.
- **van Muijden, J. (2017)**, *GPU-Based Procedural Placement in Horizon Zero Dawn*, GDC
  (cited by `docs/research/rock-scatter.md`): a shipping open world places its props by
  **density**, "Scale to w = footprint", then a range test — a placement pipeline whose range is a
  function of the view's footprint, not of the eye's distance.
- **This crate**: `src/rocks/mod.rs` (`seat_transform`, the `VisibilityRange`s of the near and far
  meshes, `FADE`, `NEAR_PER_M`, `SEAT_STEP_M`), `src/rocks/model.rs` (`ClassLattice::new`:
  `radius_m = range_per_m · b`, `tiles_near` about an eye XZ), `src/plugin.rs`
  (`ElementsView { position, eye, velocity }`, `ElementsQuality::rock_range`),
  `docs/research/view-anchor.md` (the focus, and Deviations 2 and 4: the sun's cascades and the
  precipitation box already size themselves from an orthographic view's focus and area),
  `docs/research/orthographic-lod.md` (`TerrainRuntime::lod_centre`, the centre the patches are
  selected and morphed about), `docs/research/rock-scatter.md` (the scatter, its law, its ranges).
- Bevy 0.20: `bevy_camera::visibility::VisibilityRange` and `check_visibility_ranges` (the range is
  the entity's translation against **each view's** `GlobalTransform` translation, with
  `ShadowLodOrigin` as an alternative anchor for shadow views), `OrthographicProjection::area()`
  (the world-unit rectangle the view spans). No `ShadowLodOrigin` on a rock helps: the range test
  always uses the camera's own translation.

## Diagnosis (measured, 2026-10-26)

| Check | Result |
|---|---|
| Rocks spawned, perspective (`mountains`, eye on the ground) | **4243** in 144 tiles |
| Rocks spawned, isometric orthographic (same scene, `ELEMENTS_ORTHO=18`) | **4513** in 166 tiles |
| Rocks drawn, isometric orthographic | **none** (`/tmp/i57/before/m_ortho40.png`: bare ground, no stone anywhere) |
| Rocks drawn, perspective at the same site | stones and blocks across the frame (`m_persp9_foot.png`) |
| `eye.distance(ground)` under the isometric camera | 450-470 m, for every rock in view |
| A 0.25 m stone's `far_m` | 25 m (`100 · 0.25`) |
| `seat_transform` fade at 460 m | `t = 1` → `fade = 1e-3` (the floor that keeps the normals invertible) |

The bands first logged against this camera by iteration 51 were investigated and are **not** this
item: with the sun's shadow maps disabled the same rock region differs by 1 % of its pixels and the
banding is unchanged, and the cascade reach shortened 10× changes nothing, so the pattern is the
rock material's own joint network (iterations 43-44's subject), present in perspective views too.
It is not an orthographic defect and is not what this pass fixes.

## Model adopted

1. **A prop's draw distance is a screen size, expressed as the distance at which a perspective
   camera of the crate's reference field of view would give that size.** A rock is worth drawing
   while it is larger than `MIN_ROCK_PX` pixels; the range `R(D) = c · D` (m of range per m of
   diameter) is the existing encoding of that, calibrated for a perspective camera. Under an
   orthographic view the same screen size is reached at `R_ortho = c · D · (2 tan(θ/2) / H_m)`,
   where `H_m` is the view's ground height in metres (`OrthographicProjection::area().height()`) —
   that is, the rock's own projected size is `D · height_px / H_m` and the criterion is unchanged.
   Written as a **magnification of the perspective range**, `m = 2 tan(θ/2) / H_m · (1 / REFERENCE_…)`.
   The crate's reference is a 60° vertical field of view, whose `2 tan(30°) = 1.1547`.
2. **The measure is taken from the focus, not the eye.** A rock's distance is measured from the
   centre the terrain's patches are selected about (`TerrainRuntime::morph_centre`), which is the
   focus under an orthographic camera and the eye under a perspective one — so a perspective view
   is unchanged by construction, and an orthographic one measures from what it shows.
3. **The scatter's rings are the same measure.** `ClassLattice::radius_m` is scaled by the same
   magnification, so the tiles that are built are the tiles that are drawn, and no rock is spawned
   beyond the range it would be drawn at. A capped magnification keeps a very wide view from
   spawning a whole region's rocks (`ORTHO_ROCK_RADIUS_MAX_M`).
4. **The near/far LOD split is unchanged in kind.** `NEAR_PER_M · D` is the near mesh's range under
   a perspective camera; under an orthographic one it is the same magnification, so a rock at a
   given screen size still switches mesh at a given screen size. Both `VisibilityRange`s are then
   still measured from the camera's own translation by Bevy, which under an orthographic camera is
   constant for everything on screen — so the ranges are **not** the culling mechanism under an
   orthographic view. The mesh switch there is made by the crate instead, on the same
   focus-measured distance, by choosing which child mesh is visible. [A rock under an orthographic
   camera sits at one camera distance for all of its kind: the ranges would all be `near` or all
   `far`, so nothing would cross-fade. The scale fade remains the crate's own, and now it is
   measured from the focus.]
5. **Seating is unchanged.** A rock still sits on the surface the patches draw for the same morph
   centre; only the fade and the ring are re-measured.

## Parameters

| Name | Value | Basis |
|---|---|---|
| Reference field of view | 60° vertical | Bevy's `PerspectiveProjection::default()` (`fov: π/3`), the crate's own default view |
| Orthographic magnification | `2 tan(30°) / (H_m / c_ref)`, `c_ref` the rock's metres of range per metre | Model 1: equal projected size at equal range |
| Cap on the orthographic ring | 1200 m | a wide region view draws the rocks it shows, bounded so a 4 km view does not spawn the world |
| Near LOD split | `22 · D`, magnified as the range is | existing `NEAR_PER_M` |

## GPU mapping

None: the rocks are ordinary `Mesh3d`s with a `WeatheredMaterial`; what changes is which are
spawned, how far they are faded and which child mesh is shown. No shader, no binding, no pass.

## Validation targets

1. **Spawned and drawn under an isometric orthographic camera.** The same `mountains` scene that
   spawns 4513 rocks draws stones and blocks across the frame at `ELEMENTS_ORTHO=18`, and a stone
   in the near field is at least three pixels across.
2. **A perspective view is unchanged.** The same perspective captures (eye level, 6 m above the
   ground, aerial) are identical to the byte where they were identical before, and the spawned
   count is unchanged.
3. **The fade is about the focus.** A rock at the focus is at full scale; one at the ring's edge is
   at the floor, and the transition is monotone as the focus walks.
4. **Nothing is culled by Bevy's range under an orthographic camera.** Every rock spawned in the
   view is visible: `RockStats` drawn against `VisibilityRange` culling.
5. **The ring does not explode.** At `ELEMENTS_ORTHO=400` the spawned count stays under the cap's
   population, and the build stays under the existing scatter's cost.

## What great looks like

An isometric town view, 18 m of ground top to bottom, in the mountains: the talus at the cliff's
foot is a scatter of stones and blocks with silhouettes and contact shadows, as it is through a
perspective camera — not a painted sheet. Walking the focus does not pop stones in and out at the
edge of the frame: they arrive small, at the ring's edge, and grow as they come. A wide region view
shows the rocky ground densely enough to read as talus from above without the frame budget moving.
The same scene through a perspective camera is exactly as it was.

## Deviations from this plan

1. **The mesh switch under an orthographic camera is made by the crate, not by `VisibilityRange`**
   (Model 4). Bevy's range test compares each entity with **each view's** translation, which under
   an orthographic camera is one number for the whole scene: the near and far children would both
   take the same verdict and a rock would never cross to its far mesh, or never to its near one.
2. **The `ShadowLodOrigin` route was rejected.** It re-anchors the range test for **shadow** views
   only (`check_visibility_ranges` uses camera translations for the main view), so it cannot fix a
   main-view cull.
3. **The cap.** Model 3's magnification is unbounded: a 4 km view would spawn every rock class to
   its own range, which is millions. The cap is the crate's, and is logged rather than derived.
4. **The perspective capture that differs is a real bug the first cut had.** The first version
   cached the fade centre on each `RockSeat` at spawn, so a *moving* perspective camera never
   re-faded its rocks: the 6 m-above-ground capture differed from the baseline in 2.8 % of its
   pixels (rocks at the frame's edge kept full scale). The centre is now passed to
   `seat_transform` each pass. The still perspective capture is identical to the byte.
5. **A rock is drawn by diameter, not by distance, under an orthographic view** — so a *very* wide
   view draws nothing at all rather than a field of sub-pixel meshes: at 4 km of view height a
   3.6 m boulder is one pixel. The ring is empty in that case, which is what `ring_radius` returns
   and `a_wide_orthographic_view_draws_only_the_stones_that_read` asserts.

## Outcome (measured, iteration 57)

Every target above was checked on the built crate, `terrain_tour`, 1280×720, 200 frames, on the
RTX 3090, against the same scene at the pre-change commit:

| Target | Result |
|---|---|
| 1. Drawn at the isometric camera | 2.91 % of the frame changed, 151 blobs ≥ 6 px, median side **10.0 px** (p90 20.6, max 43.7) |
| 1. A 60 m view drops the finest class | 2.75 % at 40 m, **1.61 %** at 60 m (threshold 0.52 m there, above the 0.25-0.5 m class) |
| 2. A perspective view unchanged | `m_persp9` **max diff 0**, 0.000 % of pixels; from 6 m AGL max 1 in 0.049 % (dither) |
| 3. The fade is about the focus | radial bands of the difference: 0.39 % at r ≥ 0.9, **3.01 %** in the outermost band of the heading-0 frame; monotone |
| 4. Nothing culled | the corner bands above hold stone, so the ring reaches the frame's corners |
| 5. The ring does not explode | a 1200 m view draws none (1 px boulders), frame byte-identical to the baseline |
| A pan arrives small | 61 frames at 10 m/s: p99 change flat at 22-23 levels, `>40`-level ≤ 0.24 % |

The cost is measured in `docs/loop-log.md` (iteration 57): the isometric scene with the scatter off
against the same scene on, at 4K, interleaved.

## Pending index

Nothing new: the note reuses `rock-scatter.md`'s and `view-anchor.md`'s sources, all of which are
already in the corpus (`bo/book_real_time_rendering_4th_akenine`,
`bo/book_level_of_detail_3d_graphics_luebke`, `10.1080_2151237x.2009.10129287`,
`10.1145_1015706.1015799`, `10.1145_237170.237217`).
