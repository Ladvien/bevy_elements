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
| The block network's cell | 1.5 m | iteration 35: a block a player can read |
| The warp's amplitude | 0.09 of the cell, as a *gradient* | iteration 35's note: a tenth breaks the shape, a third tears the cells |
| The blocks' size spread | a Weibull with a geometric standard deviation of 0.31 | Verdian et al. 2021: the fracture spacing's GSD, 0.214–0.551 across fifteen sites |
| The blocks' size range | a hand's width to a metre | Verdian et al. 2021: the spacings run 2 to 5000 mm, the site medians 10 to 390 mm |
| The blocks' flatness | the c : a ratio of 0.45 | Domokos et al. 2015's universal value, via Verdian et al. 2021 |
| The band-limit | the *smallest* block's own width | the crate's `detail_keep`, and the corduroy lesson |

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

*(filled in as the implementation settles; every changed default is recorded here with its
citation.)*

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
