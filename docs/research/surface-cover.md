# Surface cover: a fixed snow and wetness for one material

Issue #49 (consumer: an editor drawing a carved isomesh volume): a serde settings type for snow and wetness coverage (amount, slope and height thresholds) applicable to a mesh, hot-reloadable, with a CPU coverage query usable headless. The weather maths already exists (`wetness.md`, `snow-cover.md`); this note adds only the *selection* of where a given amount lies, so there is no new physics and no new corpus source.

## Sources

- `docs/research/snow-cover.md` "Coverage and shading": the flora curve `snow · smoothstep(0.85 − 0.6 snow, +0.2, n.y)` that the weather-driven path keeps.
- `docs/research/wetness.md` "Shading, shared by all three extensions": the film, unchanged; the cover only supplies the wetness the film is given.
- Shipping practice: slope-and-altitude masks for snow are the standard terrain-shader control (a "snow line" and a "max slope" per material in every engine's terrain layer); no paper is needed for a smoothstep mask.

## What great looks like

A cover set to 1 reads exactly as the weather's full snow or full wet film (same albedo and roughness as the scene's own), with a snow line that is a soft band, not a hard stripe, and steep faces shedding along a soft edge.

## Model adopted

`CoverBand { amount, max_slope_deg, slope_soft_deg, min_height_m, max_height_m, height_soft_m }` evaluated per face as `amount · ramp(max_slope − angle, slope_soft) · ramp(h − min, height_soft) · ramp(max − h, height_soft)`, with `ramp(v, s) = smoothstep(0, s, v)` and a step at `v ≥ 0` when `s = 0`. `angle` is the geometric normal's angle from up. `SurfaceCover { snow, wetness }` packs two bands into the material's uniform; the shader replaces `g.weather.x` and `mesh_snow_cover(g.weather.y, n.y)` with the bands while the material has a cover. `CoverBand::coverage` is the CPU form and `SurfaceCover::{snow_at, wetness_at}` the queries.

## Parameters

Defaults: a band is bare (`amount` 0), unbounded in height, `max_slope_deg` 180, hard edges. `CoverBand::snow` is 45° with 15° soft; `CoverBand::wet` is 150° with 30° soft (everything but an underside).

## GPU mapping

Four `vec4`s in the existing `WeatheredParams` uniform (binding 100) plus a flag; no new binding, so the fragment stage's storage-buffer budget is unchanged. The ramp is the same function in `weathered.wesl` (`band_coverage`); `tests/gpu_weathered.rs::a_surface_cover_weathers_a_dry_scene` compares the pixels with the plain material at the weather's own values.

## Validation targets

- A wetness-1 band reports 1.0 on an up normal and 0 under the slope limit's far side (unit test, sphere normals).
- 1x, top view of a cube: cover snow 1 equals the plain snow pixel within 1 level; a snow line below the top bares it; cover wetness 1 equals the plain wet pixel within 1 level; `None` returns the dry pixel.

## Deviations from this plan

None. Assumed decision (owner may override): the cover is a field of the material extension, not a component on a volume entity, because a volume's chunk meshes take whatever material the host assigns and a material is the only thing both the editor and a game own for certain; absolute amounts replace the weather's rather than scale them, so an editor preview does not depend on the weather. Not done: the voxel snow *slab* (geometry) still follows the weather.

## Pending index

`docs/research/surface-cover.md` cites no new corpus stems.
