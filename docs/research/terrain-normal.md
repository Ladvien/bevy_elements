# Terrain normal and slope queries

A game placing a prop, aligning a vehicle or a footstep, or deciding whether a slope will slide, needs the ground's tilt at a point. `Environment` answers height, water, snow and sinkage but not the normal: the host re-derives it from `terrain_height` samples with its own step, which disagrees with the crate's own (the trail kernel's slope, the terrain shader's normal).

## Sources

- Corpus search for method phrases (`central difference`, `heightfield normal`) finds only the rendering derivations the crate already uses; the maths is textbook and needs no citation. Recorded, not searched further.
- `src/terrain/model.rs` `TerrainSnapshot::slope_deg`: the gradient by central differences over ±1 cell (one-sided at the edge) at the four surrounding cell centres, interpolated bilinearly, the same gradient `terrain_pack` and the trail kernel use.

## What great looks like

`terrain_normal` and `terrain_slope_deg` agree with the slope the trail kernel and sinkage law already use (one gradient, one source), are exact on an analytic plane, unit length, continuous across cell borders, and carry no unit conversion: a slope is dimensionless, so a foot world answers as a metre world.

## Model adopted

1. One private gradient `TerrainSnapshot::gradient(p)` (dh/dx, dh/dz), extracted from `slope_deg`.
2. `slope_deg = atan(|g|)` (unchanged results), `normal = normalize(-gx, 1, -gz)`.
3. `Environment::terrain_normal(Vec2) -> Option<Vec3>` and `Environment::terrain_slope_deg(Vec2) -> Option<f32>`: `None` outside the terrain, with no terrain and before the first readback.

## Parameters

None.

## GPU mapping

None: CPU queries on the last readback.

## Validation targets

- A plane `h = 0.5 x` (slope 26.565 deg) answers the same normal and slope at every interior point and at an edge cell (the one-sided difference is exact on a plane).
- A flat ground answers `(0, 1, 0)` and 0 deg; the normal is unit length and its angle from +Y equals the slope.
- A world of 3.28084 units per metre answers the same normal at the matching point.
- `None` off the grid.

## Deviations from this plan

None.

## Pending index

None (no new corpus document).
