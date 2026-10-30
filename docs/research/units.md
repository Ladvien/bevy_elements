# A world-units scale (issue #8)

A game whose world unit is not the metre (Sanctuary's is the foot, 3.2808 units to the metre) gets weather sized and timed for metres: rain in a box a third of the size it should be, wind cells, clouds, fog and lightning 3.28 times too small, a sky whose aerial perspective is three times too strong, point lights three times too bright up close. `ElementsUnits { units_per_metre }` tells the crate how many world units make a metre.

## Sources

- Gregory, J. (2018). *Game Engine Architecture*, 3rd ed. (corpus `Game Engine Architecture, Third Edition…`), §12.4 (l. 20151): "Most rigid body dynamics simulations operate in the MKS system of units"; §11 texel density (l. 16013) "depends entirely upon your choice of world units". The engine's world unit is the game's choice; the simulations' is SI.
- Lagarde, S., de Rousiers, C. (2014). Moving Frostbite to physically based rendering 3.0, SIGGRAPH course notes (the 2014 *Physically Based Shading in Theory and Practice* course, DOI `10.1145/2614028.2615431`). Open PDF from the author's site (`course_notes_moving_frostbite_to_pbr_v32.pdf`, 121 pages), read with `pdftotext -layout`: §4.1 "Coherent light units: all light should be expressed in the same units"; §4.2 eq. 13 `E = I / distance²` "requires that distance units are homogeneous (m, cm, mm, …) across the lighting calculation", "In Frostbite 1 unit = 1 meter". A punctual light's intensity for a given illuminance therefore scales with the square of the length unit.
- Bevy 0.20 source, `bevy_pbr-0.20.0/src/atmosphere/mod.rs:243-255`: the atmosphere's `world_to_atmosphere` is the inverse of its entity's scale and translation (the 0.19 migration removed `scene_units_to_m`): a world in feet needs the `Atmosphere` entity's `Transform` scale = 3.2808 for its LUTs to be in metres.
- `FloraUnits` (bevy_flora 0.46, `src/units.rs`): the family's precedent. "A field that sizes or times the crate's own simulation is in metres; a position, or a size that addresses the world, is in world units"; default 1.0 changes nothing.
- This crate's inventory of the boundaries (two read-only surveys, iteration 24): about 120 places in Rust where a metre computation meets a `Transform`, the camera or a world-space component, and 13 of 42 shaders that turn a world position from Bevy's view or mesh into a metre quantity. AGENTS.md "Units are SI at every boundary".

## Model adopted

**One metre frame.** The crate already simulates in metres everywhere. Its internal frame becomes *world ÷ units_per_metre*, same origin and axes, and nothing inside changes: grids, cell sizes, altitudes, fall speeds, extinction per metre, physics. Every conversion sits at a boundary:

1. **Reading the world.** The camera and focus (`ElementsView`, the one place every camera-relative domain reads), and every `GlobalTransform` the crate reads (blockers, water sources, contacts, imprints, the terrain entity, voxel volumes, orthographic areas) go through `ElementsUnits::to_metres`, which pre-scales the affine by `1/u`.
2. **Writing Bevy's world.** What the crate inserts or drives in world units: the crate-spawned `Atmosphere`'s scale (`u`), `DistanceFog` (distances × `u`, extinction ÷ `u`), the ground-fog `FogVolume` (transform × `u`, density ÷ `u`), shadow cascade distances (× `u`), the lightning flash `PointLight` (range × `u`, intensity × `u²`, eq. 13), the bolt mesh (scale `u`), the terrain and water patches and their bounds.
3. **The public API.** Positions are world units: `Transform`s, `ElementsFocus::Point`, the `Environment` query inputs and `terrain_height`'s output, `StrikeLightning::at`, `LightningStrike`/`Thunder::position`, `LightningFlash::position`, `WindGust::origin`. Everything else stays SI, as AGENTS.md requires: `Thunder::distance_m`, depths, speeds, `Terrain` extents and heights, `WaterContact`/`Imprint`/`PrecipitationBlocker` sizes (local, scaled by their transforms), and the resources that describe the crate's own grids (`ElementsView`, `WindState`'s window, `TerrainState`'s grid, `CloudShadows`), which are in the metre frame and say so.
4. **Shaders.** Each render shader that takes a world position from Bevy (`view.world_position`, `in.world_position`, `world_from_clip` rays, instance matrices) divides it by `u` on entry and multiplies the positions it hands back (clip position, `pbr_input.world_position`, reprojection) by `u`. The factor travels in the pass's own uniform: `CloudParams`, the precipitation `DrawParams`, `TerrainMaterialParams`, `GroundParams`, `WaterParams`, `WindFieldParams`, and `ElementsGlobals` (which grows by one `vec4`, 80 → 96 bytes).

**Runtime changes.** The simulations do not see the factor, so changing it at runtime moves only the boundaries: the camera-relative windows (rain box, wind grid, fog, clouds) follow on the next frame. A terrain keeps the metre-frame origin it was built at until it is rebuilt (its `Transform` changing rebuilds it).

**Bad values.** A non-positive or non-finite `units_per_metre` is read as 1.0 (`ElementsUnits::per_metre`), with one warning.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| `ElementsUnits::units_per_metre` | 1.0 | issue #8; FloraUnits |
| Foot | 3.2808 | 1 ft = 0.3048 m (exact) |
| Point light intensity in world units | `I · u²` | Lagarde & de Rousiers 2014 eq. 13 |
| Atmosphere entity transform | translation `(0, −R·u, 0)`, scale `u` | Bevy 0.20 `world_to_atmosphere`; `Atmosphere` docs (`bevy_light-0.20.0/src/atmosphere.rs:25-70`) |

## GPU mapping

No new passes, buffers or bindings: one `f32` in each affected uniform (a free component where one exists) and one more `vec4` in `ElementsGlobals`. A multiply per world position on entry to a shader and one on exit.

## What great looks like

The same scene built in feet (every `Transform` and the camera multiplied by 3.2808, `ElementsUnits { units_per_metre: 3.2808 }`) renders the same picture as in metres: the same clouds at the same apparent height, rain of the same density and streak length, fog at the same distance, the same shadows and sky, and the same answers from `Environment` (wind in m/s equal, heights × 3.2808).

## Validation targets

- With `units_per_metre = 3.2808`, `Thunder::distance_m` of a strike 3280.8 units away reads 1000 (issue #8).
- With 3.2808, the precipitation box is 196.8 units wide (issue #8).
- The default 1.0 leaves every existing test unchanged.
- A terrain, wind and cover scene in feet answers `Environment` queries as the metre scene does at the corresponding points (heights × 3.2808, wind equal) (GPU).
- The quickstart in feet matches the quickstart in metres to within the frame-to-frame noise of the clouds and particles (capture).
- `per_metre` reads 0, −1, NaN and ∞ as 1.0.

## Deviations from this plan

1. **Correction, iteration 25: the atmosphere's translation is the planet's centre.** This note read only the scale in `world_to_atmosphere`. Bevy's `Atmosphere` documents its entity's `GlobalTransform` as the planet's centre and, when that transform is still default on add, moves it `inner_radius` below the origin. The spawn here gave the entity a `Transform::from_scale(u)`, and transform propagation wrote that transform's identity translation back over Bevy's placement, so the planet's centre sat at the world origin and the camera inside the planet: in the `waves` example under Clear the sky became a beige dome of radial streaks and the pond reflected it (an app whose propagation ran after the hook; others kept Bevy's placement, which is why the iteration-24 captures looked right). The crate's atmosphere `Transform` is now `(0, −R·u, 0)` with scale `u` (`plugin::planet_transform`), at spawn and when the units change; `plugin::tests::the_crate_atmosphere_centres_the_planet_below_the_origin` fails on the old transform. `docs/research/sky-light.md` Deviation 6.

## Pending index

- Lagarde & de Rousiers 2014 read from the author's PDF; not converted into the corpus.
