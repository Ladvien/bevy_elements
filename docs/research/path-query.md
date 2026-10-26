# Path surfaces: asking a game where its roads are

Iteration 55 shipped the ability to *lay* routes: a `PathNetwork` of `Path`s rasterised into a
coverage distance field that the terrain material draws (`docs/research/paths.md`). Nothing lets a
game *ask* about them. `Environment` answers `terrain_height`, `water_depth`, `snow_depth`,
`wetness`, `trail_depth` and `cloud_shadow_at`, but a game that put a town on a road network cannot
answer the three questions it will ask every frame:

1. **Which surface is under this point?** — for footstep, hoof and tyre sounds, for movement speed,
   for the dust a wheel kicks, for whether a boot sinks.
2. **Where is the nearest road, and how far?** — for AI that prefers roads, for building and prop
   placement that keeps off them, for spawn points and patrol routes.
3. **Is this point on a road at all?** — the predicate most placement code actually wants.

Without them a game re-implements the polyline maths itself, against the same network the crate
already holds, and its answer drifts from what is drawn.

## Sources

- **Gregory 2018**, *Game Engine Architecture*, 3rd ed. (corpus `Ga/Game Engine Architecture…`).
  §12 (animation events): "when the left or right foot touches the ground, a footstep sound and a
  'cloud of dust' particle effect could be initiated" — the trigger is the *event*, but the sound's
  identity comes from the surface under it. §"Collision materials": "many games implement a collision
  shape categorization mechanism similar in many respects to the material system used in the
  rendering engine … the term *collision material* … associate with each collidable surface a set of
  properties that defines how that particular surface should behave … Collision properties can
  include sound and particle effects, physical properties like coefficient of restitution or
  friction coefficients". §"Physical materials" (Unreal): "visible geometry can be tagged not only
  with visual materials, but with physical materials as well. The former defines how the surface
  looks, the latter defines how it reacts to physical interactions, including impact sounds". Naughty
  Dog's *polygon attributes*: "collision geometry can be tagged with polygon attributes (PATs for
  short), which define certain physical behaviors like footstep sounds". The model adopted is their
  common shape: **a surface classification, queried per point, that a game maps to sound, friction
  and speed** — the crate supplies the classification and the geometry, the game the assets.
- **This crate's own precedent**, which fixes the API's shape and its freshness contract:
  `Environment::trail_depth` (`src/query.rs`) answers from a CPU snapshot of GPU state, up to a
  readback interval old, and a game reads it for exactly these purposes ("a game can tell where
  something walked and how long ago it snowed"). `terrain_height`, `snow_depth` and `wetness` are
  the same shape. A path query must not be *worse*: it is CPU data (the `PathNetwork` the game
  handed in), so it can be **exact and current**, not sampled.
- **`docs/research/paths.md`** for the mask's own definitions: the coverage's edge fade
  (`edge_fade_m`, a third of the half width, at least 12 cm), the lateral offset as a fraction of the
  half width, the ruts' wheel lines at ±0.6 of the half width, and the per-texel grain. The query must
  answer in the same terms the shader draws in, or a game's footstep at a path's edge disagrees with
  the picture.
- **Pouyat et al. 2020** (corpus `10.1007_978-3-030-45216-2`), used by `paths.md`: a route is
  "bared soil", "compacted", "dries out soil", and traffic concentrates on it. A compacted road is
  *faster* and *harder* than the turf beside it — the physical reason a game wants the classification.
- **RTR4 §15.4.3** (Green 2007's distance field): the coverage the shader draws from the distance is
  a `smoothstep` over the fade. The query's coverage must use the same curve, so `surface()` and the
  picture agree at the edge.

## Model adopted

1. **A query on `Environment`, in the crate's established shape.** `surface_at(position)` answers a
   small `Copy` struct, `PathSurface`:

   ```rust
   pub struct PathSurface {
       pub kind: PathKind,     // Earth, Gravel, Stone, Snow
       pub coverage: f32,      // 1 well inside the route, 0 outside its edge, the shader's smoothstep
       pub half_width_m: f32,  // the route's own half width
       pub rut_depth_m: f32,   // the route's ruts (0 without)
       pub rut: f32,           // 0-1: how deep in a wheel line this point is (the shader's Gaussian)
   }
   ```

   `Some(..)` on or near any route (within `coverage > 0`), `None` elsewhere or with no network.
   A second query answers the distance question: `nearest_path(position)` returns
   `Option<PathHit { surface: PathSurface, distance_m: f32, point: Vec3 }>` — the distance to the
   route's centre line? No: to its *edge*, signed, so a game can place a prop 3 m off a road.

2. **Exact, from the network, not from the mask.** The mask is 0.175-1 m a texel and is thrown away
   after rasterising; the `PathNetwork` is the game's own data and is exact. The query walks each
   `Path`'s segments (the same `route_distance` the rasteriser uses, so the mask and the query cannot
   disagree), which for a village's few dozen routes is a few hundred segment tests — cheap enough for
   a few queries a frame and honest about the picture. A game querying thousands of points a frame
   can ask for the cheaper `on_path(position) -> bool` (distance below the half width).

3. **Current, not sampled.** `PathNetwork` is read directly, so a route laid this frame answers this
   frame (no readback interval). The doc says so, and says the opposite of the snapshot queries so a
   game knows which freshness it has.

4. **The same coverage curve as the shader**: `smoothstep(0, -fade, distance)` with
   `fade = edge_fade_m(half_width)` — one `PathMask::coverage` shared by both (moved out of
   `#[cfg(test)]` for this). The rut is the shader's Gaussian about ±0.6 of the half width with the
   0.10 sigma, and the kind is the nearest route's.

5. **What a game does with it** (documented, not implemented here — sound and speed are the game's):
   map `kind` to a footstep set and a movement speed multiplier (a gravel road 1.1×, a mud lane
   0.85×), use `coverage` to blend the two at a verge, `rut` to deepen a print or a dust puff, and
   `nearest_path().distance_m` to keep a building off a road.

## Parameters

| Name | Value | Basis |
|---|---|---|
| `PathSurface::coverage` | the shader's `smoothstep(0, -edge_fade_m(half), distance)` | `paths.md`; the picture and the query must agree |
| Edge fade | `edge_fade_m(half)` — a third of the half width, at least 0.12 m | `paths.md` Deviation 3 |
| Rut offset / sigma | 0.6 of the half width, 0.10 | `paths.md` Deviation 6b |
| `nearest_path` | the nearest route's segment distance | the rasteriser's own `route_distance` |
| Freshness | the current `PathNetwork` (no readback) | the network is CPU data |
| Cost | one segment test per route segment, per query | [INFERENCE] a village's ~40 routes is ~200 tests |

## GPU mapping

None: the queries are CPU and read the game's own `PathNetwork`. The shader keeps its mask; the two
share the coverage function and the lateral/rut constants so they cannot drift.

## Validation targets

1. **The query matches the picture (CPU).** For a route and a grid of points around it, the query's
   coverage equals the shader's formula on the same distance to 1e-6, and the rut's value equals the
   shader's Gaussian on the same lateral offset. One test, one shared function.
2. **Inside, edge, outside.** At a 5 m-wide road: coverage 1 at its centre and 1.5 m out; 0 at 3 m
   (beyond the fade); the edge itself (distance 0) coverage 0, the fade beginning there.
3. **The nearest route wins.** Two crossing routes of different kinds: a point nearer the lane reports
   the lane's kind and half width; a point nearer the road reports the road's.
4. **Distance to the edge, signed.** `nearest_path` 4 m from a 2.5 m road's centre line reports
   `distance_m == 1.5` and `point` on the centre line; inside, it is negative.
5. **Current.** A network inserted this frame answers this frame (no readback wait), and removing it
   makes the queries answer `None`.
6. **Cost.** 1000 queries against a 40-route network under 1 ms on one thread.
7. **Visual.** A capture with a small prop placed by `nearest_path` exactly 3 m off a road's edge,
   to show the query and the drawn verge agree.

## What great looks like

A game asks at a walker's foot and gets "gravel, coverage 1, ruts 3 cm, in a wheel line" on the road,
"earth, coverage 0.4" at its verge, and `None` a metre into the meadow — the answer changing exactly
where the picture's verge thins out. An AI asks where the nearest road is and gets the distance to its
edge, not to its centre line, so a barn placed 3 m clear looks 3 m clear. Nothing about the answer
depends on the camera, the readback cadence or the mask's resolution.

## Deviations from this plan

None yet: the note precedes the implementation. Recorded here as they are found.

## Pending index

None: Gregory 2018, Pouyat et al. 2020 and Green 2007 via RTR4 are in the corpus.
