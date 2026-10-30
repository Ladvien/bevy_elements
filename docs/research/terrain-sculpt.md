# SculptTerrain: height brushes on the live terrain, for an editor and for a game

bevy_emerge's issue #99 ("Sculpt terrain, and pieces sit on it") asks for raise, lower, smooth, flatten-to-height and noise brushes with a radius and a strength, a stroke that changes the heights under it and nowhere else, and placed pieces that follow the ground when it is raised ("any missing query is filed on `bevy_elements`"). The crate has no way to change a live terrain's heights except a bake or a test writing the layer buffer. A game needs the same for craters, trenches, digging and landscaping. This note adds a message, `SculptTerrain`, that edits bedrock on the GPU, the one place the height lives, and lets the existing readback carry the answer to `Environment::ground_height`.

## Sources

- Corpus search for method phrases (`terrain brush`, `sculpting brush`, `height brush`, `terrain authoring`) found nothing: the library holds simulation and rendering work, not editor tooling. Recorded, not searched further.
- Brush falloff, the common practice in terrain editors (Unity Terrain, Unreal Landscape, World Machine): a radial weight 1 inside a hardness fraction of the radius and a smooth fall to 0 at the radius. Adopted as a smoothstep, whose zero slope at both ends leaves no seam at the brush's rim. Not a corpus source; the choice is the conventional one and is recorded under Deviations.
- `docs/research/terrain.md` (layers, pack, readback), `docs/research/erosion.md` (bedrock vs sediment layering: a hand edit moves bedrock, sediment then relaxes by the thermal pass), `AGENTS.md` ("send requests as plain fields of the extracted resource, atomics only to report back").
- bevy_emerge #99 acceptance, read 2026-10-29: a raise stroke changes the heights under it and nowhere else.

## What great looks like

A raise stroke of 5 m, radius 40 m, leaves a smooth round hill whose rim blends into the ground with no step, ring or crease, in the drawn terrain, the rocks and the cover on it the same frame it is applied. Heights at distance >= radius are bit-identical to before. A flatten stroke leaves a level disc at the requested height that blends into its surroundings; a smooth stroke visibly relaxes a ridge without moving its mean height; noise adds a seeded, reproducible roughness. The ground query answers the new height the next frame, so a piece placed on it can follow.

## Model adopted

`SculptTerrain { centre_m: Vec2, radius_m: f32, hardness: f32, kind: SculptKind }`, a `Message`. Positions are metre-frame world XZ, as every query takes them. `SculptKind`:

| kind | per-cell change of bedrock, with `w` the brush weight at the cell |
|---|---|
| `Raise { amount_m }` | `+ amount_m · w` |
| `Lower { amount_m }` | `− amount_m · w` |
| `Smooth { blend }` | `+ blend · w · (mean of the 8 neighbours' surface − surface)` |
| `Flatten { height_m, blend }` | `+ blend · w · (height_m − surface)`, `height_m` in the metre frame |
| `Noise { amplitude_m, feature_m, seed }` | `+ amplitude_m · w · n`, `n` = 3-octave Perlin in `[−1, 1]` at the cell centre |

`w(d) = smoothstep(1, 0, (d/R − hardness) / (1 − hardness))` for `d < R`, 0 beyond; `hardness` in `[0, 1)`. Smooth and flatten read the *packed* surface from the last pack (`cells.x`), so the kernel has no read-after-write race; one stroke runs per frame and the pack follows in the same compute pass, so the next stroke sees it. Raise, lower and noise only add a delta, so the sum of a drag's strokes is exact whatever the frame order.

Bedrock moves, sediment stays: raising 5 m under 1 m of sand puts the sand on top of a 5 m hill; thermal erosion relaxes anything over the talus angle as usual.

## Parameters

- `radius_m`: `> 0`, at most the terrain extent. `hardness`: `[0, 1)`, default 0.35 in the editor-facing constructor docs. `amount_m`, `amplitude_m`: finite, `>= 0`. `blend`: `[0, 1]`. `feature_m`: `> 0`. An invalid message is dropped with a warning (a message has no error channel), `SculptTerrain::validate() -> Result<(), SculptError>` is public so an editor reports the field first.
- Cost: one dispatch over the grid (a strict subset runs; the kernel returns outside the radius), once per stroke frame, nothing on a frame without one.

## GPU mapping

`sculpt.wesl`, entry `terrain_sculpt`, `@workgroup_size(8, 8)`, bindings: uniform `SculptParams { grid: GridParams, area: vec4 (centre xz, radius, hardness), shape: vec4 (kind, a, b, c), misc: vec4 (seed, y0, 0, 0) }`, `bedrock: array<f32>` read_write, `cells: array<vec4<f32>>` read-only. Dispatched at the head of the terrain's compute pass, before the pack, so the pack, flow, bounds and readback of the same frame carry the edit. The request is the extracted `TerrainGpu::stroke` plus an epoch; the render world answers through an atomic (`brushed`), the main world sends the next stroke only when it has answered (the pipelined-rendering rule in `AGENTS.md`).

## Validation targets

- 1x, aerial: a 5 m raise of radius 60 m on a flat 512 m terrain: a round hill, no ring at the rim.
- Close-up, grazing: the rim shows no crease (the weight's slope is 0 at `d = R`).
- Test (GPU): raise leaves every cell with `d >= R` bit-identical and the centre cell raised by `amount_m` (hardness 0.5, centre inside the flat top); lower undoes it to within an f32 ulp; flatten drives the centre to `height_m`; smooth lowers a spike's peak and conserves the sum to 1e-3 relative; noise with one seed twice is identical, with two seeds differs; `Environment::ground_height` after the stroke reads the raised height.
- CPU: the weight's mirror `sculpt_weight` equals 1 inside the hardness, 0 at the radius and beyond, and is monotone in between.

## Deviations from this plan

1. The smoothstep falloff and the `hardness` plateau are the editors' convention, not a cited model (no corpus source). Recorded as a default, not a physical claim.
2. Bedrock, not the surface, is edited so sediment, water and cover keep their meaning; a removal deeper than the sediment cuts into bedrock directly rather than first removing sediment.

## Pending index

None: no new document was added to home-still.
