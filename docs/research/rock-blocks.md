# The cliff is made of blocks: a rock face that breaks along its joints

The crate's rock has a material — bedding, joints, a grain — and the material is nearly
invisible: it contributes an sd of **1.72 DN** to a face at 3 m, and at 200 m the massif is
**one colour** (a max−min of 1.00 DN over a 300 × 200 patch). The reason is structural: every
feature is a *multiplier on the albedo of one continuous surface*. A cliff is not a texture. It
is a pile of **blocks**.

## Sources

- **Zhang, S., He, M., Gu, J., Cui, Z., Wang, J., Zhong, L., Meng, Q., Wang, H. (2020)**, *Rock
  Mass Classification for Columnar Jointed Basalt: A Case Study of Baihetan Hydropower
  Station*, Geofluids 2020:6679317, DOI `10.1155/2020/6679317` (the record and its abstract
  were read; **the PDF was not obtained** — the publisher returns 403). Used: "an obvious
  characteristic of columnar jointed rock is that it is **discontinuous in geometry while
  continuous in mechanics**". A rock mass is a *set of blocks* held in one piece: the
  discontinuity in geometry is the blocks, and the mechanics are the continuity. That is the
  distinction the crate's rock has never had.
- **La Pointe, P. R., Hudson, J. A. (1985)**, *Characterization and interpretation of rock mass
  joint patterns*, SPE Formation Evaluation 1(1), DOI `10.1130/SPE199-P1` (the record was
  read; the PDF was not obtained). Named here as the standard treatment of *joint sets* — the
  families of parallel planes along which a rock mass breaks, and the reason a cliff's blocks
  are **tabular and aligned** rather than a random heap.
- **Olano, M., Baker, D. (2010)**, *LEAN Mapping: Linear Efficient Anti-aliased Normal
  Mapping*, ACM SIGGRAPH Symposium on Interactive 3D Graphics and Games (I3D 2010). Read through
  **Hill, S., Baker, D. (2012)**, *Rock-Solid Shading: Image Stability Without Sacrificing
  Detail*, SIGGRAPH *Advances* in Real-Time Rendering in Games course (corpus
  `s2012-advances-pdf-slides-3.md`, "Part 2: Mechanics of LEAN Mapping"). Used: a detail
  normal's *specular* is filtered by the **variance** of the normal, so a detail map does not
  alias at distance and the image is stable under resolution. The crate's rock has a detail
  normal (the grain's gradient) and no such filtering, and the crate's own `detail_keep`
  band-limit is the *albedo*'s answer to the same problem.
- **Akenine-Möller, T., Haines, M., Hoffman, N., Shirley, P. (2019)**, *Real-Time Rendering,
  4th ed.* (corpus `bo/book_real_time_rendering_4th_akenine.md`). Used: §6.3's procedural
  textures — "a cellular texture is formed by measuring distances from each location to a set of
  'feature points' scattered through space. Mapping the resulting closest distances in various
  ways, e.g., changing the color or shading normal, creates patterns that look like cells,
  flagstones, lizard skin, and other natural textures" — and §6.1's note on Andersson's
  Frostbite terrain, where "multiple textures are combined based on terrain type, altitude,
  slope" and "geometric models, such as bushes and **rocks**, are placed within the scene".
- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`), and the 2007 *Advances*
  slides (corpus `s2007-advances-pdf-slides-8-4-mb`, "Material masking… Red = slope-based cliff
  material"). Used: the *game* literature's answer to the same question — the terrain shader
  decides a material mask from the slope, and the rock's material is the crate's own. This
  iteration is not about the mask; it is about what the mask *reveals*.
- **The crate's own `rock-material.md`**: the beds, the joints and the grain, and the sentence
  that is the key to this change — "A Worley cell network cannot make them — it bounds *cells*,
  so a face reads as a giraffe's hide". The cells cannot make the *joints as drawn lines*, but
  the cells **are** the blocks, and a block is shaded rather than outlined.
- **The crate's own `scree.md`** (iteration 34): the scree is the same network used as the
  *stones*; the cliff's blocks are the same network used as the *pieces of the rock*.

## Model adopted

**1. The blocks are the cells of a 3D cellular network, in the rock's own frame.**

A block is a piece of the rock, so the network must be sampled in the **3D point**, not in the
ground plane: the crate's rock note is explicit that the rock's material is "the crate's 3D
noise at the 3D point, which has no projection at all, so it is the same field seen from every
direction and it stays put on the surface" — and a 2D network would stretch along a slope and
weave, exactly as the turf does.

**2. A block is shaded, not outlined.** The crate's note says a cell network's *boundaries*
read as a giraffe's hide — that is a statement about drawing the boundaries as *lines*. A block
is not a line: it is a **volume**, and it is shaded by its own tone and its own relief. So:

- **The block's tone** is its own: the cell's random factor, a per-block draw, so the cliff has
  blocks of many tones and the face is not one grey. This is the "two rock tones" the note asks
  for, and more: the beds carry the *large* tone and the blocks carry the *small* one.
- **The block's relief** comes from the distance field's gradient, exactly as the scree's stones
  do: the gradient points away from the block's centre, so it domes the block's face and the sun
  rakes its edge. The blocks' edges are the *joints*, and the light finds them.

**3. The joints are the cells' boundaries, and they are dark.** Where two cells meet, the rock
is broken, and a broken edge is a shadow: the joint darkens the rock beside it. That is the
crate's existing `ROCK_FRACTURE_DARKEN` applied at the *cell* scale rather than at the noise's
scale — and it is the one feature of a cliff a player reads as "solid" at a distance (the
note's own words).

**4. The block's scale is a metre or two, and it is band-limited.**

A block a player can read is a metre or two across, and the rock face in the capture is 2 m
across the frame at 3 m. So the cell is large — a metre and a half — and the *grain* and the
*fine fractures* stay inside each block as they are. The blocks are band-limited like
everything else in the crate: a block is a bigger feature than a crack, so it survives further,
but it must still be band-limited by the pixel's footprint or it aliases into a corduroy (the
crate has paid that lesson twice: the rock's fractures "band-limited by the cell, the cracks
survived as a fine corduroy over the whole massif at 700 m").

**5. At distance the blocks fall back to the beds, and the massif keeps its colour.**

A block is a small feature; at 200 m a pixel covers metres and the blocks are gone. What
remains must be the *beds* — a low-frequency band, which survives — and the massif's *large*
tone. So the blocks are a *near* feature that fades into the beds, and the massif from 200 m is
banded stone rather than one colour. The note's target #4 (a spread of at least 9 DN over the
massif from 700 m) is a target about the *beds*, not about the blocks: the beds are at a 4.5 m
spacing, and they are there.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The block network's cell | 1.5 m | a block a player can read; the note's "What great looks like" |
| The blocks' stretch along the bedding | 2.6 | La Pointe & Hudson 1985 on joint sets: the pieces are aligned, not a heap |
| The warp that breaks the cells' shape | 0.09 of the cell | a cell network's cells are convex and near-equal; a rock's are not |
| The block's tone spread | the cell's random factor, ±0.22 | a block is a piece of one bed, so the spread is narrow |
| The joint's width | 0.06 of the cell's half-boundary, ×0.30 to ×3.70 by the cell's draw | the joint is a line a fraction of a cell wide, and the draw is what varies the block's size |
| The joint's darkening | 0.68 of the rock's albedo | a joint is a shadow, and it is the one feature of a cliff a player reads as "solid" |
| The block's relief | the joint's own gradient, a slope of 0.85 | a block's face is *flat* and the step is at the joint |
| The band-limit | the joint's own width | the crate's `detail_keep`, and the corduroy lesson |

## GPU mapping

- **Inside the terrain's own fragment shader**, where the rock's material already is
  (`src/terrain/common.wesl`, `rock_material`). **No new pass, no new buffer, no new binding.**
- The cost is one 3D cell lookup in the rock's band only — the crate's hottest pass is the
  mountains' opaque pass (0.131 ms on the baseline build), and this must not move it materially.

## What great looks like

- **A cliff face at 6 m reads as rock**: blocks with their own tones, their edges catching the
  light, and dark joints between them — not a smooth wash with a speckle.
- **The mountain at 200 m has more than one colour**: the massifs separate into lighter and
  darker bands, and the scree below them is a third tone. The note's own target.
- **No weave**: the blocks are in the rock's 3D frame, so they do not stretch on a slope the way
  the turf does.
- **The distance is calm**: the blocks fade into the beds rather than aliasing into a corduroy.

## Validation targets

1. **The material is visible.** The rock's own high-pass (a 3 px blur subtracted) over a
   300 × 200 patch of a face at 3 m has an sd of at least **3 DN** (it is **1.72**).
2. **The blocks are discrete.** The block's coverage along a cut across a face crosses the
   half-way mark many times, where a smooth wash crosses it once.
3. **The blocks are aligned, not a random heap.** A cliff's blocks are tabular: the joint
   family's plane. *Measured:* the block network's cells are *elongated* along the bedding
   plane, so a face shows slabs and not gravel.
4. **The massif is not one colour.** Over a 300 × 200 patch of the massif from 200 m, the
   luminance spread is at least 9 DN (it is **1.00**).
5. **No weave on a slope.** The near band's fine structure has no fixed world direction, and the
   band at 2 m and at 6 m must not share a direction.
6. **The turf is untouched.** The `plains` preset at 6 m is byte-identical.
7. **The cost.** The mountains' opaque pass is within 10 % of the baseline's minimum.

## Deviations from this plan

1. **The joint is the cell's *boundary* — the second nearest feature point — and not the
   nearest point's distance.** The plan said the joint is where two cells meet, and took it
   from the nearest point's distance. That draws a *ring* round each point, and the face read
   as a **giraffe's hide** — the crate's own rock note's warning, reproduced exactly. The cell's
   boundary is the standard construction: the half-difference of the two nearest distances.
2. **The block's face is *flat*, not domed.** The plan gave each block a dome, like the scree's
   stones. A rock block is a *facet* and the step is at the joint, so the relief is the joint's
   own gradient and the block's face is level. A dome over each block read as a **pebble pressed
   into the face** — and a field of them as scales.
3. **The blocks are *one* network, not two nested.** The plan followed the scree and nested a
   coarse and a fine network. The fine network's pieces have to be masked by the coarse ones'
   *coverage*, and the joint is only 1 near a boundary, so the fine pieces showed almost
   everywhere and the face read as crumpled foil. A second network is the wrong instrument
   here: the blocks are *pieces of one rock*, and the cell network's own cells are already the
   pieces. The variation in the blocks' *size* comes from the joint's **width** instead, which
   is the cell's own draw — a rock's joints are not all the same, and a uniform width reads as
   a mosaic.
4. **The blocks are band-limited by the *joint's width*, not by the cell.** The cell is 1.5 m and
   a pixel at 90 m is a metre, so a cell-based band-limit let the blocks survive at 90 m and
   the massifs read as a **scaly, reptilian texture**. The joint is the finest thing the blocks
   draw, and it aliases long before the cell does — the crate's own rule for the fractures.
5. **The blocks are one network *squashed* along the bedding, not two networks.** The plan said
   the blocks are tabular; the implementation squashes the network's coordinates along the
   bedding plane's normal, so the cells stretch into slabs. That is the plan's own idea, and
   it works.
6. **The beds' second tone is a *value* change, not only a hue one.** The plan left the beds
   alone. They had to change: the tone was (1.06, 0.93, 0.76), whose luminance is 0.95, so
   the beds moved the luminance by 1.9 % and the massif from 90 m was one colour — a max−min
   of 1.00 DN, where the crate's own note asks for 9. A bed is a *darker* band (the constant's
   own comment says so), and the value is what carries at a distance. The tone is now
   (0.80, 0.73, 0.66) at a mix of 0.55, a 14 % albedo drop.
7. **The joint's width is the cell's own draw, and it is what sets the block's *size*.** The
   cells of a network are all the same size, so without this every block would be the same
   and the face would read as a cobblestone. A cell whose joint is wide is a *small* piece of
   rock and one whose joint is narrow is a big slab, and the cell's factor is used for the
   block's tone *and* for its joint's width, so a cell is consistently one piece.
8. **The network's coordinates are *warped* before the lookup.** A cell network's cells are
   convex polygons of nearly the same area, and a face of them reads as a mosaic — or, with the
   blocks' step, as *shingles*. A rock breaks into irregular fragments, and the warp is what
   makes the fragments irregular. The warp is the crate's own 3D noise at the 3D point, so it
   does not weave on a slope. At a tenth of the cell it breaks the blocks' *shape* and not
   their position; at a third of the cell the cells broke into a lace of amoeba-like shapes
   with holes, and the face read as camouflage.
9. **The cell's search is *pruned* by the distance to its nearest corner.** A feature point lies
   inside its cell, so a cell whose nearest corner is further than the *second* nearest
   distance so far cannot hold either of the two nearest points, and its hash — the expensive
   part — is not computed. The bound is exact, so the result is the same as the full search
   (the captures are identical to within the GPU's non-determinism), and the 27-cell lookup
   becomes about 12 hashes.
10. **The joint's edge is softened over the pixel's own footprint.** A joint is a line a
    fraction of a cell wide, and a hard edge on a line that narrow stair-steps at every
    distance. The width grows with the footprint, so the joint blurs into the rock rather than
    aliasing along it.
11. **The blocks are skipped entirely where they cannot resolve.** The band-limit is known
    before the network is looked up, so the far terrain pays nothing for the blocks: the
    mountains' opaque pass does not carry the cost over the whole massif, and at 90 m the
    frame is byte-identical to the baseline's.

## Pending index

- Zhang et al. 2020 (`10.1155/2020/6679317`): the record and the abstract were read; **the PDF
  was not obtained** (the publisher returns 403). Listed here so the gap is on the record.
- La Pointe & Hudson 1985 (`10.1130/SPE199-P1`): the record was read; the PDF was not
  obtained.
- Olano & Baker 2010 (LEAN mapping) was read through the 2012 *Advances* slides in the corpus.
- Hill & Baker 2012 (Rock-Solid Shading) is **in the corpus**
  (`s2/s2012-advances-pdf-slides-3.md`) and was read. **Not indexed.**
- `distill_index` is skipped for the above.
