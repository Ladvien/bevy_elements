# A cliff's joints are hairline traces between flat blocks, not a web of ribbons

Every near cliff in the crate wears a web of flat grey ribbons crossing each other, with a
fence of straight parallel bands over it (`/tmp/i43/crop_face3.png`, `crop_face30.png`; the
same web is in iteration 40's showcase `docs/block_sizes.png`). A rock face that a player reads
as *jointed rock* is the opposite: flat block faces, each at its own angle, separated by thin
dark traces where the blocks part. This note finds why the crate draws ribbons and replaces the
joints with what the rock-mechanics description says a joint is.

## What the code does (the diagnosis)

`rock_block_scale` in `src/terrain/common.wesl`:

1. **The first set's "joint" is not the cell's border.** `half` is the distance from the point
   to the *integer lattice cell's* wall on the side away from the feature point. The cell
   network's border is where the nearest and the second-nearest feature points are equally far
   (Worley, below); the lattice wall is a different surface, and it follows the warped lattice
   planes — a curvy grid over the face.
2. **The joints are wide and get wider with distance.** The darkened band is up to
   `0.19 × (0.25 + 0.85 × 2) ≈ 0.37` of the half-cell (about 18 cm on a 0.95 m block), the
   relief bevel twice that, and `soft = max(width, footprint)` keeps a line at full darkness
   while widening it to the pixel — so at 30 m the face is more ribbon than rock.
3. **The relief is a steep band, the same on both sides.** `step` and `step2` tilt the face by
   a slope of 0.95 (43°) over the band, and `step2` tilts along the second set's normal on
   *both* sides of its plane, so each band shades as one flat grey ribbon.
4. **The second set's relief ignores its own wander and segments.** `step2` uses `to_plane` —
   the *unwandered, unsegmented* plane — while the darkening uses `to_plane_w`; so the relief
   draws long, straight, evenly spaced bands across the whole face: the wire fence iteration 40
   meant to break.
5. **The second set's planes are wavy.** They wander by 0.48 of their spacing over the warp's
   3.8 m field: an amplitude-to-length ratio near 13 %.

## Sources

- **Palmström, A. (2015)**, *Joint characteristics*, RockMass.net (`rockmass.net/files/observation_joints.pdf`,
  read with `pdftotext`; it restates ISRM 1978, Bieniawski 1984 and Milne et al. 1992). Used:
  - "A joint is a three-dimensional discontinuity composed of two matching surfaces called
    joint walls."
  - Separation: "the maximum distance between two joint walls. It is generally small, **mainly
    less than a millimetre**, except for filled joints, seams or shears", classified from very
    tight (< 0.1 mm) to very open (10–25 mm). So the open trace a face shows is a weathered
    mouth of millimetres to centimetres — a hairline at the block's scale, not a band.
  - Waviness, from Milne et al. 1992: "**Planar joints u < 1%**", planar to wavy 1–2 %, wavy
    above 2 %, `u` the amplitude over the length. A joint at the crate's block scale is a
    plane, not a wandering curve.
  - Termination: "Discontinuous joints terminate in massive rock … **Continuous joints
    terminate at other joints**." A cross joint ends where it meets the block's own joint.
  - Smoothness: "Rough: Large angular asperities can be seen. Some ridge and high side angle
    steps evident" — the block's face is a rough plane.
- **Worley, S. (1996)**, *A cellular texture basis function*, SIGGRAPH 1996, DOI
  `10.1145/237170.237267` (the PDF from `cedric.cnam.fr/~cubaud/PROCEDURAL/worley.pdf`, read).
  Used:
  - "since F2 ≥ F1 for all x, we can map the function F2(x) − F1(x) … This combination has a
    value of 0 where F1 = F2, **which occurs at the Voronoi boundaries**" — the border is
    where the two nearest feature points are equally far, not a lattice wall.
  - "If the F1 function returns a unique ID number to represent the closest feature point's
    identity, this number can be used to form values that are **constant over a Voronoi
    cell** … When combined with bumping based on F2 − F1, quite interesting surfaces can be
    easily generated" — the flagstone surface: a constant per cell, a border between.
  - "Bump mapping of the flagstone-like areas is particularly effective."
- **Lagae et al. (2010)**, *A survey of procedural noise functions*, CGF 29(6), DOI
  `10.1111/j.1467-8659.2010.01827.x` (corpus): Worley's basis is "good for creating textures
  such as flagstone-like tiled areas … **rock**".
- **The crate's own notes**: `rock-blocks.md` (the blocks, their tone and joint, the
  band-limit known before the lookup), `block-sizes.md` (the second joint set as a coordinate,
  its Weibull spacing per 46 m formation, Strauhal & Zangerl 2021 on non-persistence),
  `rock-fragments.md` (the unequal cells, the warp), `rock-cost.md` (a gate skips the
  evaluation, not the value; a performance change must not change the image).

## Model adopted

1. **The first set's border is the cell network's own.** `cell3` returns the second-nearest
   feature point too; the border distance is the exact distance to the bisector plane of the
   two nearest points, `dot(x − (P1 + P2)/2, (P1 − P2)/|P1 − P2|)` — zero on Worley's border,
   the true distance to it away from the cells' corners. One loop, one more comparison per
   hashed cell; the prune's bound becomes the *second* distance so the second point is exact.
2. **A joint is a hairline trace, anti-aliased by coverage.** Its half-width `w` is a weathered
   mouth of 1–3 cm, drawn per block (the existing Weibull draw, so the traces vary). Seen at a
   footprint `f` larger than the trace, its darkness scales by `w / f` — a line narrower than a
   pixel covers part of it — so a far trace *fades* instead of widening. The trace is darker
   than before (a shadowed aperture), but a few pixels wide at 3 m.
3. **Each block is a flat facet at its own angle.** Worley's constant-per-cell value is a
   *tilt*: a random gradient per piece, constant over it, so the face is a set of planes that
   catch the light differently, meeting at the traces. The traces hide the facets'
   discontinuity; a narrow bevel (two trace widths, signed so the face rolls *into* the crack)
   rounds the edge.
4. **The second set is planar, and it ends at the first set's joints.** Its planes wander by
   at most 2 % of a segment (Palmström's planar-to-wavy), and their presence is a draw *per
   block*: a block is either split by the cross joints into slabs, each slab its own facet, or
   whole. So a cross joint's trace runs from one of the block's borders to another and stops —
   "continuous joints terminate at other joints" — with no noise field for its segments.
5. **The wide relief bands go.** `ROCK_BLOCK_RELIEF`'s 0.95 slope over the band, `step2` on
   the unwandered plane, and the segment field are deleted; the facets and the narrow bevels
   carry the relief.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| Border | bisector of the two nearest feature points | Worley 1996 |
| Trace half-width | 1.2–3.0 cm, the block's Weibull draw | Palmström 2015 (mm separations, weathered at the face); Deviation 1 |
| Trace darkening | to 0.40 of the albedo at full coverage | Deviation 2 |
| Coverage | `min(1, w / f)` | the line's area in the pixel |
| Facet tilt | a slope of 0.22 (about 12°) per piece, random direction | Deviation 3 |
| Bevel | two trace widths, slope 0.6 into the crack | Deviation 3 |
| Second set wander | 0.04 of the spacing (2 % of a segment) | Palmström 2015 (Milne 1992): planar ≤ 2 % |
| Second set presence | half the blocks, a per-block draw | Palmström 2015 (termination) |
| First set warp | 0.08 m (was 0.30 m) | Palmström 2015 waviness; Deviation 4 |

## GPU mapping

- `cell3` (`noise.wesl`) keeps the nearest *and* the second-nearest point in its one loop and
  prunes by the second distance; the CPU mirror (`gpu/noise_mirror.rs`) does the same.
- `rock_block_scale` (`terrain/common.wesl`) computes the bisector distance, the trace with its
  coverage, the per-piece facet and the bevel; the second set loses its `perlin3` segment field
  (a per-block hash instead). Net: one noise evaluation fewer, a few more cells hashed.
- No new binding, uniform or pass.

## What great looks like

- At 3 m: a rock face of **blocks** — flat faces at different angles, some lit, some in their
  own shade — parted by **thin dark lines** that meet at corners and end at each other.
- At 12–30 m: the blocks still read as pieces; the lines thin to hairlines and fade, never
  thicken; no straight band crosses the face.
- At 120 m: the massif is its beds and its colour; nothing of the joints remains but a faint
  texture of facets that also fades.
- In raking light (dusk, dawn) the facets carry the face: the joints are where the light breaks.
- Reference: any jointed granite or sandstone crag; the flagstone surfaces of Worley's Figure 2.

## Validation targets

1. **The joints are lines** (CPU mirror): over a long cut, the trace darkens less than 8 % of
   the face (it was a third), and its crossings per metre are about two per block.
2. **The border is the cell's**: at the bisector of the two nearest points the border distance
   is zero, and it is the true distance away from the corners (CPU test against a brute-force
   search).
3. **A far trace fades**: at a footprint four times the trace's width, its darkening is a
   quarter or less.
4. **The second set is planar**: along a segment its trace's deviation from a straight line is
   under 2 % of the segment's length, and it never crosses a first-set border.
5. **Facets**: the shading normal is constant within a piece (away from its edges) and differs
   between pieces.
6. **Captures** (`face3`, `face12`, `face30`, `far120`, `dusk`, `dawn`, `overcast`, `mtn_foot`,
   `mtn_elev`, `seed7`, `talus_b`, `back`, `plains`): no ribbon and no fence at any distance;
   blocks with flat faces and dark hairline joints at 3–12 m; the far massif unchanged; the
   plains byte-identical.
7. **Cost**: the mountains' opaque pass no slower than the baseline (interleaved, minimum).

## Deviations from this plan

1. **The trace's width.** The old joint was 4.5–18 cm of darkening plus a bevel twice that; a
   joint's separation is "mainly less than a millimetre" (Palmström), so a cliff's dark trace is
   its weathered, shadowed mouth. 1.2–3.0 cm is a few pixels at 3 m and fades by coverage
   beyond.
2. **The darkening.** A thin trace must be darker than a wide band to read: 0.40 of the albedo
   at full coverage, against the old 0.76 over a band that was a fifth of the block.
3. **The facet and the bevel.** No source measures a face's tilt between blocks; 12° keeps the
   blocks distinct under raking light without the face reading as cobbles, and the bevel's
   slope of 0.6 over two trace widths rounds an edge a few centimetres, as weathering does.
4. **The warp.** Iteration 37's 0.30 m warp bent the cells' borders by about 8 % of their
   length; Palmström's planar joints are under 1–2 %. 0.08 m keeps the cells irregular through
   their unequal sizes (`CELL_INSET`) and keeps the joints planar.

Found during iteration 43's attempt, which did not ship (branch `attempt/iter-43-joints`, `## Outcome` below):

5. **The tabular stretch is inverted on `main`.** Deviation 10 of `rock-blocks.md` says the lattice is *stretched* along the bedding normal so the cells are slabs; `0e58b68` changed `p + n(p·n)(T − 1)` to `p − n(p·n)(1 − 1/T)`, which divides the normal component and makes the blocks `T · 0.95 m = 2.5 m` columns across the beds. The test that approved it, `the_blocks_are_tabular`, compared two directions that both lie in the bedding plane (`normal.cross(in_plane)` is perpendicular to the normal) and read a longer lattice vector as a longer cell. Counting blocks along real cuts, the shipped lattice crosses 0.55 blocks per metre across the beds against 1.42 along them; the attempt's rewritten test fails on it and passes on the multiply.
6. **Open and tight joints.** Most of a rock mass's discontinuities are closed. The attempt grouped the cells whose feature points share a lattice cell 2.6 cells across: a border between groups is an open joint (full trace and bevel), one inside a group a tight hairline (0.35 of the ink). It adds no lookup, since both cells are the search's own.
7. **The bed is the rock.** The bed's palette (`bed-colour.md`) applies at every distance and the blocks carry value only. This was the attempt's clearest win: at 12 m, at dusk and under overcast the cliff reads as a banded succession of different rock. A palette drawn per piece read as a patio, and drawn per block as camouflage.
8. **The facet belongs to the open block.** A tight joint creases a block's face (0.08) and does not make a new face. A facet per piece made the cliff a cobbled mosaic.

## Outcome

Iteration 43 stopped after five rounds, NOT YET at every round, and `main` keeps the baseline (`/tmp/i43/critique.md`, `docs/loop-log.md` iteration 43). The hairline traces fixed what was asked: the chicken wire is gone at every distance, and every trace lies where two pieces meet. But each round exposed the network underneath:

| Round | Change | The face read as |
|---|---|---|
| 1 | Hairline traces on the Voronoi border, facets, planar second set | crazy paving: convex tiles of one size, each a flat tone |
| 2 | Tabular stretch restored (Deviation 5) | a dry-stone wall; a patio where the slope parallels the bedding |
| 3 | Open and tight joints, palette per block (Deviation 6) | camouflage: hue jumping block to block |
| 4 | Bed colour at every distance (Deviation 7) | a bedded succession, but a cobble mosaic inside each bed |
| 5 | Facet per open block (Deviation 8) | camouflage blobs: a union of convex cells has a rounded, stair-stepped outline |

**Hypothesis for the next attempt.** A jointed face reads as rock when its *open* joints are persistent **plane sets** — bedding partings and one or two cross sets with Weibull spacing, the cross joints terminating at the partings and staggered between them — so the blocks are parallelepipeds with straight edges. A Voronoi network, whole (paving) or grouped (camouflage), cannot give a block a straight edge; it should carry only the tight hairline fractures. Keep from the attempt: the hairline trace with coverage anti-aliasing, the bed's colour at every distance, the tabular fix and the exact prune bound.

Two defects on `main` remain open and change the image when fixed, so they wait for that attempt: the inverted stretch (Deviation 5), and `cell3`'s prune bound, which in the WESL omits the clamp at zero (so on an axis where the neighbour spans the point the bound is inflated and a cell holding a nearer point can be pruned), while the CPU mirror uses a different, far-corner bound, so the mirror's tests do not check the shader's search.

## Pending index

- Palmström 2015 (RockMass.net, a non-DOI source): the URL is recorded above.
- Worley 1996 (DOI `10.1145/237170.237267`): the PDF is in `/tmp/i43`, not ingested (ACM 403
  through home-still).
