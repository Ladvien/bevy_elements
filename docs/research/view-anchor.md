# A focus for the camera-relative domains, and orthographic cameras

Issue #7. An isometric or top-down game renders through a fixed orthographic camera that stands far back from what it shows (Sanctuary: 1500 world units). Every camera-relative domain of this crate centres on the camera's position, so under such a camera the rain, the wind field, the ground fog, the strikes, the trails and the waves all sit 1500 units away from anything on screen, and the cloud march builds every ray from a single eye point, which an orthographic view does not have.

## Sources

- Marschner & Shirley, *Fundamentals of Computer Graphics*, §4.3 and §4.3.1 "Orthographic Views" (corpus `book_fundamentals_computer_graphics_marschner`, ll. 2346–2389). An orthographic view's rays "all have the direction −w"; "even though a parallel view doesn't have a viewpoint per se, we can still use the origin of the camera frame to define the plane where the rays start", each ray starting at its pixel's location on that plane: "same direction, different origins". A perspective view's rays share the origin and differ in direction.
- Akenine-Möller et al., *Real-Time Rendering* 4th ed., §4.7.1 (corpus `book_real_time_rendering_4th_akenine`, ll. 3518–3540): orthographic projection "does not affect the w-component"; "objects maintain the same size regardless of distance to the camera".
- Puig-Centelles, Ripollés & Chover 2009, *Creation and control of rain in virtual environments*, The Visual Computer 25:1037–1052 (`10.1007/s00371-009-0366-9`, fetched from SciDB, uploaded to the home-still inbox). The rain container "is initially centered on the observer … continuously updated in order to follow the user's movements", shaped to the observer's field of view; when the observer looks at rain from outside it ("Far Rain"), the container "is centered in the part that is visible to the user" and resized "to cover the whole part … that the observer can see" (§4.2–4.3). Particle size and count follow a level of detail by distance from the eye (§5).
- Puig-Centelles et al. 2011, *Rain Simulation in Dynamic Scenes*, IJCICG (`10.4018/jcicg.2011070102`, same path): the GPU version; the container "permanently tracks the viewer", positioned from the camera position and orientation as "an extended frustum".
- Tatarchuk 2006, *Artist-directable real-time rain rendering in city environments* (corpus `s2006-advances-materials-6`): separate particle systems placed in the environment and culled by the frustum, rather than one system glued to the eye.
- Losasso & Hoppe 2004, *Geometry clipmaps* (corpus `10.1145_1015706.1015799`): nested grids centred on the viewer, updated toroidally as it moves. This crate's wind, trails and waves windows use the same scheme (`wind.md` "Toroidal domain"); the scheme does not care what the centre is.
- Bevy 0.20 (vendored source): orthographic views are detected in WGSL as `view.clip_from_view[3].w == 1.0` (`bevy_pbr-0.20.0/src/cluster/cluster_raster.wesl:113`); `OrthographicProjection::default_3d()` has `near = 0`, `far = 1000` (`bevy_camera-0.20.0/src/projection.rs:782-790`); the LOD position `view.lod_view_world_position` is the camera's translation (`bevy_render-0.20.0/src/view/mod.rs:1244-1250`). Bevy's own atmosphere builds its rays as perspective ones, from the near-plane position through the eye (`bevy_pbr-0.20.0/src/atmosphere/functions.wesl:335-357`), so its sky and aerial-perspective LUTs are perspective-only.

## Model adopted

1. **A focus.** `ElementsFocus` is a component on the followed `ElementsCamera`: `ElementsFocus::Entity(e)` (that entity's `GlobalTransform` translation) or `ElementsFocus::Point(p)`. Without it, or when the entity has no transform, the focus is the camera. It is "the part that is visible to the user" of Puig-Centelles: what the domains centre on.
2. **Two positions.** The followed view carries the focus (`position`, what every simulation domain centres on) and the eye (`eye`, the camera's translation). The focus centres the precipitation box and its draw fade, the wind window and the gusts, the ground fog volume, the strike disc and the thunder distance (the listener is where the game's attention is), the cloud weather and shadow maps, the cover's mean around the view, the trails and waves windows, and the hydrology's nearest blockers. The eye stays where a viewpoint is meant: the lightning bolt's on-screen width, the camera velocity that turns a falling drop into a streak.
3. **Orthographic rays.** The cloud march, resolve and composite build each ray from the view matrices, not from one eye point: the direction is the unit vector between the pixel's points on the near plane (reverse-Z depth 1) and at depth 0.5, which for a perspective view is the ray through the eye and for an orthographic view is the view direction for every pixel; the origin is the eye for a perspective view and the pixel's point on the near plane for an orthographic one (Marschner & Shirley's "same direction, different origins"). Every distance the shaders compare (the cloud entry against the scene depth, the reprojection of the shell entry) is measured from that origin.
4. **Precipitation under an orthographic view.** A drop's on-screen width `d · focal_px / w` is already right: `w = 1` and `clip_from_view[1][1] = 2 / height`, so `focal_px` is pixels per metre (RTR4: size independent of distance). The near-plane clip on `w` is skipped (an orthographic `w` is 1 everywhere), the soft-particle fade compares view-space depths reconstructed through `view_from_clip` (exact for both projections), and the direction to the drop that orients its phase function is the view direction.
5. **The bolt width.** The pixel size at distance `d` is `d · 2 tan(fov/2) / height_px` for a perspective camera and `area.height() / height_px` for an orthographic one, whatever the distance.

## Parameters

| Parameter | Value | Note |
|---|---|---|
| Focus default | the camera | perspective users see no change |
| Ray direction depths | near plane (1.0) and 0.5 (reverse-Z) | finite for an infinite perspective and an orthographic far plane |
| Orthographic test | `clip_from_view[3].w == 1` | Bevy's own test |

No extent changes: the precipitation box stays 60 m, the fog volume 240 × 120 × 240 m. A view wider than them shows rain and fog only around the focus; scaling them to the view is issue #8's units and a later item.

## GPU mapping

- `common.wesl`: `ray_dir` and `scene_distance` give way to `view_ray(uv, world_from_clip, cam, orthographic) -> Ray { origin, dir }` and `scene_distance(uv, depth, world_from_clip, origin)`. The march, resolve and composite use the ray's origin wherever they used `view.world_position`. The march's planet centre stays straight below the ray's origin (for an orthographic view, a few hundred metres of offset on a 6 360 km planet).
- `draw.wesl`: a `centre` field in `DrawParams` for the box fade; `view_from_clip` for the soft-particle depth; the view direction under orthographic.
- CPU: `ElementsView { entity, position, eye, velocity }`; `follow_camera` resolves the focus. `pixel_size(distance)` in `lightning/visual.rs` replaces the perspective-only pixel angle.
- No new passes, no new bindings; per pixel, one more unprojection in the cloud shaders.

## What great looks like

An isometric orthographic camera, 1500 m back and 35° down, over a rainy plain: streaks fill the screen around the focus with the same width at every depth, the ground fog sits on the ground under the focus, splashes ring the ground in view, and clouds overhead cast shadows that slide across it. Panning the focus keeps the rain continuous (the box wraps; no respawn burst). A perspective camera renders exactly as before.

## Validation targets

1. **Focus centring (CPU).** With an orthographic `ElementsCamera` 1500 m from its `ElementsFocus`: the precipitation box centre, the wind window's centre, the ground fog volume's centre and the strike disc's centre are the focus, not the camera (one test per domain; strikes within `STRIKE_RADIUS_M` of the focus).
2. **Default unchanged.** With no `ElementsFocus`, every centre is the camera, and every existing test passes unchanged.
3. **Rays (CPU mirror of `view_ray`).** Under `Projection::Orthographic` the origins of two pixels differ and their directions are equal (to 1e-6); under `Projection::Perspective` the origins are the eye and the directions differ.
4. **Bolt width.** Non-zero and finite under an orthographic projection, proportional to the projection's area height and independent of distance.
5. **GPU, orthographic view.** The cloud pass renders a non-empty layer under an orthographic camera (the march does not degenerate), and the precipitation draw writes streaks around the focus (`tests/gpu_precipitation.rs`).
6. **Performance.** `elements_clouds_march` within 10 % of iteration 18's 0.513–0.539 ms (High, perspective).

## Deviations from this plan

## Pending index

Uploaded to the home-still inbox for conversion and indexing: `10.1007_s00371-009-0366-9`, `10.4018_jcicg.2011070102` (read from local `pdftotext`). The rest is in the corpus.
