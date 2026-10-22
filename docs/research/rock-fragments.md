# The cliff is not a quilt: a rock face breaks into irregular fragments

The crate's blocks came out as a **quilt of identical lozenges** — a regular lattice of same-size
diamonds, unmistakably a pattern and not rock. The cause is a one-line regression: the commit
that replaced the block network's warp with the noise's own *gradient* left the line as
`let warp = vec3<f32>(0.0);`, so **the warp has been off since iteration 35** and the cells
have been a perfect lattice ever since. The commit's message describes the warp as "the best
version of the three"; the code does not have it.

The fix is not only to restore the warp. A lattice of *warped* cells is still a lattice of
*equally sized* cells, and the geology says a cliff's blocks are not equally sized: the
fracture spacing follows a **Weibull** distribution whose spread is wide.

## Sources

- **Verdian, J. P., Sklar, L. S., Riebe, C. S., Moore, J. R. (2021)**, *Sediment size on talus
  slopes correlates with fracture spacing on bedrock cliffs: implications for predicting initial
  sediment size distributions on hillslopes*, Earth Surface Dynamics 9:1073–1090, DOI
  `10.5194/esurf-9-1073-2021` (open access; the PDF was downloaded to
  `/tmp/research/verdian2021.pdf` and read in full, 914 lines of `pdftotext -layout`).
  Used:
  - The model: "fracture spacing distributions in rock commonly have shapes that follow one of
    these distributions (Gillespie et al., 1993)" — exponential, lognormal or power — and the
    result: "the **Weibull** form of the exponential distribution (Weibull, 1951) yielded the
    best fit to the data in most cases", for both the fracture spacing and the talus size.
  - The measurements: "Spacings between individual fractures on cliff faces range from **2 to
    5000 mm** across the suite of sites, with median spacings at individual sites ranging from
    **10 to 390 mm**", and the site's *spread*: the **geometric standard deviation** of the
    fracture spacing runs **0.214 to 0.551** across the fifteen sites (Table 2), a median of
    about **0.31**.
  - The *coupling*: "the close site-to-site correspondence between the spread in the b-axis
    diameter distribution and the spread in the fracture spacing distribution" — the spread of
    the blocks' sizes and the spread of the spacings are the same law.
  - The *shape* of the blocks: the talus particles are **rods, slabs and equi-sided blocks**,
    with the b : a and c : a ratios of a site's talus averaging **0.64 and 0.35** for the
    granodiorite and **0.57 and 0.26** for the metasediment, and the universal values for
    mechanically weathered blocks being **0.67 and 0.45** (Domokos et al. 2015). A block is
    **flat and elongated**, not a cube: the c : a ratio of about 0.45 is the c-axis against the
    longest axis.
  - The *mechanism*: the latent size distribution "is set by the **spacing and orientation** of
    fractures, foliations, and **bedding planes** (Sklar et al., 2017). These represent
    preexisting planes of weakness and determine the volume and shape of newly created
    sediment particles" — so a cliff's blocks are bounded by *three* families of planes, and
    the bedding is one of them.
  - The caveat: "the lower tails of the distributions follow a steeper trend than the rest of
    the data, as is the case for CB-3, GP-1, and EP-26, potentially reflecting an
    **undersampling of the smallest fractures**" — the smallest blocks are the ones a survey
    misses, so a real cliff has more small blocks than a measured one.
- **The crate's own `rock-blocks.md`** (iteration 35): the blocks, the joint and the warp, and
  the three sentences that frame this change — "The cells of a network are all the same size,
  so without this every block would be the same and the face would read as a cobblestone" (the
  *joint's width* was the answer, and it did not work: the **cells** are still the same size, so
  the face is a mosaic of equal cells), "a rock breaks into irregular fragments, and a warp is
  what makes the fragments irregular" (the warp was the answer, and it was never implemented),
  and "a cell whose joint is wide is a *small* piece of rock and one whose joint is narrow is a
  big slab" (the *size* was tied to the joint's width, so the block's size varied while its
  *cell* did not, which is why the variation read as a *width* and not as a *size*).
- **The crate's own `scree.md`** (iteration 34): the scree's stones, which carry the size
  spread that the cliff's blocks lack — "the nested tail that a real size distribution has
  (Verdian et al. 2021: 2 mm to 575 mm, a median moving 10-fold)".
- **Akenine-Möller, T., Haines, M., Hoffman, N., Shirley, P. (2019)**, *Real-Time Rendering,
  4th ed.* (corpus `bo/book_real_time_rendering_4th_akenine.md`). Used: §6.3's procedural
  textures — "a cellular texture is formed by measuring distances from each location to a set
  of 'feature points' scattered through space. Mapping the resulting closest distances in
  various ways, e.g., changing the color or shading normal, creates patterns that look like
  cells, flagstones, lizard skin, and other natural textures" — and the note that a cell
  network's cells are *the same size*, which is the property the crate's own rock note already
  called out ("a Worley cell network cannot make them — it bounds *cells*, so a face reads as a
  giraffe's hide").
- **Hill, S., Baker, D. (2012)**, *Rock-Solid Shading: Image Stability Without Sacrificing
  Detail*, SIGGRAPH *Advances* in Real-Time Rendering in Games course (corpus
  `s2/s2012-advances-pdf-slides-3.md`), on the anti-aliasing principle: **estimate the variance
  and widen the filter** rather than let a sub-pixel feature alias. For the *albedo* the crate's
  own `detail_keep` is the instrument: a feature below the pixel's footprint must be
  **cancelled**, not averaged. **In the corpus**, read, **not indexed**.

## Model adopted

**1. The warp is restored, and it is the noise's own *gradient*.**

The commit that disabled it wanted the gradient: one evaluation for a whole 3D vector field,
where three separate noise *values* would be three lattice walks, and a gradient is a better
field anyway because it is smooth and it *points* somewhere, so the cells' boundaries are
pushed along a consistent direction rather than at random. The crate's `perlin3_d` already
returns `(value, ∂x, ∂y, ∂z)`, so the gradient is **one** call and not three.

The warp must be *strong enough to break the cells' regularity* and *weak enough not to
tear them*. The crate's note says "at a tenth of the cell it breaks the blocks' *shape* and not
their *position*; at a third of the cell the cells broke into a lace of amoeba-like shapes with
holes, and the face read as camouflage". The value is 0.09 of the cell, and with the warp off
the value was never tested.

**2. A block's *size* is its own draw, and the draw is a Weibull.**

A cell network's cells are all the same size, and that is the defect: a face of equal cells is
a mosaic. The geology is explicit that the *blocks* are the *latent* blocks bounded by the
fractures, and their sizes follow the fracture spacing's distribution — a **Weibull** with a
**geometric standard deviation of about 0.31** and a site median from 10 to 390 mm.

A Weibull's draw is cheap: the Weibull is the exponential's generalisation, and the standard
inverse-transform sampling is a **logarithm and a power** of the uniform draw. But a
lognormal's is not much dearer, and the crate's own scree already uses a per-cell factor with
a heavy tail.

**3. The size comes from the cell's *boundary*, not from the cell.**

A cell network's *cells* are equal. To get unequal blocks the network must be **non-uniform**:
the feature points must be spaced unequally, so the cells are unequal, and the distance field
around them is the *blocks'* boundary. A Voronoi network's cells are equal because the feature
points are on a lattice; the crate's own `cell3` already puts each point at a **random offset
inside its cell**, which is a *little* irregularity, and the irregularity is what has to be
amplified.

**4. The blocks are *tabular*, and the tabularness is a third plane.**

The geology gives three families of weakness — fractures, foliations and **bedding planes** —
and the blocks' shape is set by all three. The crate's blocks are already tabular along the
bedding (the network is stretched by 1.7 along the bedding's normal), and the measured
`c : a` of about 0.45 says the block is **flat**: the third axis is 45 % of the longest.

**5. The size's variation must be *band-limited* with the distance.**

A block of many sizes is a *range* of scales, and the small ones alias long before the large
ones do. So the size's spread is a function of the pixel's footprint: at 3 m the face shows
blocks from a hand's width to a metre; at 90 m it shows only the big ones, and the small ones
must be **cancelled**, not averaged, or the face turns into a corduroy.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The block network's cell | 0.95 m | a block a player can read, and two blocks across a face at 3 m |
| The warp's amplitude | 0.30 m, as a *gradient* | the gradient's mean magnitude is 0.94, so 0.30 m is 0.20 of a cell |
| The warp's scale | 9 cells | a warp over one cell scrambles the lattice instead of bending it |
| The blocks' size spread | the feature points drawn towards their cell's centre, `CELL_INSET` 0 | a cell network's cells can only differ in size through the points' offsets |
| The joint's width's draw | a Weibull, shape 1.6, scale 0.85, clamped at 2.0 | Verdian et al. 2021: the fracture spacing's Weibull fit, and its undersampled tail |
| The blocks' size range | a hand's width to a metre | Verdian et al. 2021: the spacings run 2 to 5000 mm, the site medians 10 to 390 mm |
| The blocks' flatness | the c : a ratio of 0.45 | Domokos et al. 2015's universal value, via Verdian et al. 2021 |
| The band-limit | the *joint's* own width | the crate's `detail_keep`, and the corduroy lesson |
| The joint's darkening | 0.76 of the rock's albedo | a joint is a shadow, and a shadow is wider than the crack |
| The block's relief | a slope of 0.95 | the joint's own gradient, so the sun rakes each block's edge |

## GPU mapping

- **Inside the terrain's own fragment shader**, where the rock's material already is
  (`src/terrain/common.wesl`, `rock_blocks`). **No new pass, no new buffer, no new binding.**
- The warp costs **one** `perlin3_d` — the gradient comes free with the value.
- The size's spread costs **one** hash and **one** logarithm per cell, and the logarithm is
  only needed where the blocks resolve.

## What great looks like

- **A cliff face at 3 m reads as rock**: fragments of *many* sizes — a big slab, a smaller
  block, a chip — each with its own tone, its own facet, and a dark joint between them. The
  fragments are *not* the same size, and the face does not repeat.
- **No quilt**: the face has no *period*. A regular lattice reads as a pattern at every
  distance; rock never repeats.
- **The blocks are tabular**: flat and elongated along the bedding, and their c : a is about
  0.45, so the face shows *slabs* and not gravel.
- **The distance is calm**: at 90 m the small blocks are gone and the massif carries the
  beds' banding, and the frame does not turn into a corduroy.

## Validation targets

1. **The face has no period.** The blocks' *sizes* vary by a wide factor: the tenth-to-ninetieth
   percentile of the blocks' areas spans at least **4:1** (it is about **1.2:1** now — every
   block the same size).
2. **The fragments are irregular.** The face's **orientation coherence** falls: a lattice has
   one preferred direction for its facets, and rock does not. Measured against the crate's own
   *good* surfaces (the scree, the turf), the cliff's face must be in the same band.
3. **The blocks are tabular.** The network's cells are *bigger* along the bedding than across
   it (the existing test), and the c : a ratio is near 0.45.
4. **The distance is calm.** The frame at 90 m is byte-identical to the baseline's, or the
   high-pass is below 1 DN.
5. **The meadow is untouched.** The `plains` preset at 6 m is byte-identical.
6. **The cost.** The mountains' opaque pass is within 10 % of the baseline's minimum.

## Deviations from this plan

1. **The warp is the noise's own *gradient*, and the commit that asked for it never wrote it.**
   The plan inherited the warp from iteration 35, where the commit that replaced three noise
   *values* with the noise's *gradient* left the line as `let warp = vec3<f32>(0.0);` — the
   warp had been **off** since iteration 35, and the cells had been a perfect lattice ever
   since. The commit's message describes the warp as "the best version of the three"; the code
   does not have it. This is the *cause* of the quilt, and it is a one-line regression, not a
   tuning problem.
2. **The warp's *amplitude* is calibrated for the gradient, not for the value.** The note's
   0.09 was the amplitude for the noise's *value*, whose range is ±1. The gradient's mean
   magnitude is **0.94** and its max 2.37, so 0.09 displaced a cell's boundary by **0.06 of a
   cell** — a fifth of what it needed to be, and the cells stayed regular. The amplitude is
   now **0.30 m**, which is a mean displacement of **0.20 of a cell** and a max of 0.44.
3. **The warp's *scale* is many cells, not one.** A warp that varies over the length of a
   cell moves different points by different amounts and **scrambles** the lattice instead of
   bending it: the cell's index stopped being monotonic along a straight cut, and the joint was
   crossed **zero times in 30 m**. The warp now varies over **nine cells**, so the whole
   lattice bends coherently — which is what a rock's fracture set does.
4. **The size spread comes from the network's *feature points*, not from a second network.**
   The plan's second generation — a fine network at a third of the cell, the scree's own
   construction — was tried and **rejected**: a seam every few pixels turned the face into a
   **dark speckle**, and the material's own high-pass fell from 10.0 DN to 6.5 DN, *below* the
   baseline's 6.7. Made tone-only, it was a mottle of patches of a slightly different rock
   and not a cliff either.
   The spread comes instead from the **cells themselves**: the feature points are drawn
   *towards* their cell's centre (`CELL_INSET` in `noise.wesl`), so a point near its cell's
   corner leaves a big cell behind it and a point at the centre a small one. That is the only
   place a cell network's cells can differ in size, and it costs one multiply.
5. **The joint's width is a *Weibull* draw, and the width is in the cell's own units.** The
   plan left the width as a linear draw, `0.30 + 3.4 · factor`, which is uniform. The
   Weibull's inverse transform is `λ · (−ln(1 − u))^(1/k)`, with the **shape 1.6** — between
   the exponential (1) and the Rayleigh (2), with the longer tail a cliff's spacings have. The
   width's scale had to be cut from 1.5 to **0.85**: at 1.5 the widest joint was *half* the
   cell, which ate the block, and the joint's weight was a field and not a line.
6. **The block's cell is 0.95 m, not 1.5 m.** A block a player can read is a metre or two
   across, and the rock face in the capture is 2 m across the frame at 3 m, so a 1.5 m cell
   put two blocks across the frame. At 0.95 m the face reads as a *fracture network* — the
   blocks are countable and the seams are a crack, not a line.
7. **The joint's darkening is a *shadow at* the joint, and the joint's width is 0.19.** The
   first version of the change had the joints at 0.13, and the face read as a **giraffe's
   hide** — a network of thin dark lines with flat blocks between them, which is the crate's
   own rock note's warning about drawing a cell network's boundaries as *lines*. A joint is a
   shadow, and a shadow is *wider* than the crack: the darkening is 0.76 of the rock's
   albedo, and the relief's step is 0.95 so the sun rakes each block's edge.
8. **The CPU mirror's `fract` was the wrong one.** Bevy's `Vec3::fract` is `x − trunc(x)` and
   so is **negative** for a negative coordinate, while WGSL's is `x − floor(x)` and is always
   in [0, 1). The two disagreed about where every cell's wall was, and the joint's weight came
   out at **zero** on the negative half of the world. The mirror now has its own `wesl_fract`.
9. **The Weibull's draw is *clamped* at its 95th percentile.** A Weibull's tail is unbounded, and
   an unbounded joint's width eats the block: at the 99th percentile the width was 47 % of the
   cell's half-boundary, so a block in that cell was a sliver. The note says why the tail is
   bounded in the data: "the lower tails of the distributions follow a steeper trend than the
   rest of the data, as is the case for CB-3, GP-1, and EP-26, potentially reflecting an
   **undersampling of the smallest fractures**" — a survey misses the smallest, so the measured
   distribution has a bounded tail and the render does too.
10. **The two block tests measure the *cell* and not the *joint*.** The joint is a line a
   fraction of a cell wide, so two heights on the same vertical line are usually *both* off a
   joint and trivially agree — the test passed for a 2D network, and failed for a 3D one once
   the joint grew narrow. The tabular test measured the *joint's crossings*, which are the
   **warp's** long-range bending as much as the lattice's, and the warp does not know about
   the bedding. Both now measure the cell: the *extent* of the network's coordinates along
   the bedding and across it, and the *index* of the cell at two heights.

## Pending index

- Verdian et al. 2021 (`10.5194/esurf-9-1073-2021`) was **downloaded and read** for this note.
  **Not indexed** (the ONNX out-of-memory error in `corpus.md`).
- The three papers the search turned up on the *block size distribution* — **Yao, W.,
  Sharifzadeh, M., Yang, Z. (2020)**, *Assessment of block size distribution in fractured rock
  mass…*, DOI `10.1063/1.5143906`; **Azizi, A., Moomivand, H. (2021)**, *A New Approach to
  Represent Impact of Discontinuity Spacing and Rock Mass Description on the Median Fragment
  Size of Blasted Rocks…*, DOI `10.1007/s00603-020-02360-4`; and **Park, L. et al. (1993)**,
  *In-situ block size assessment from discontinuity spacing data*, International Journal of
  Rock Mechanics & Mining Sciences, DOI `10.1016/0148-9062(93)90982-j` — were **found by
  `paper_search` and their records read**, but **no PDF was obtained** (the publishers returned
  a landing page or a paywall). Named here as the treatments of the *block size distribution
  from the discontinuity spacing*, which is the model this note adopts; the numbers in the
  table are from Verdian et al., whose PDF *was* obtained. **Not indexed.**
- Hill & Baker 2012 (Rock-Solid Shading) is **in the corpus**
  (`s2/s2012-advances-pdf-slides-3.md`) and was read. **Not indexed.**
- `distill_index` is skipped for the above.
