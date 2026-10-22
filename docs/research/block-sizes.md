# A cliff carries slabs and chips: the blocks' sizes come from three spacings

The crate's cliff is a **dense field of same-sized angular chips** — a shattered safety-glass
window, not a rock face. At 3 m the face is a uniform mosaic of pieces a hand's width across;
a real cliff carries a metre-wide slab, a fist-sized block and a chip *in the same wall*.

The measurement that started it (the crate's own CPU mirror, `/tmp/i40`): the block network's
**cells** already vary — the p10 is 0.25 m and the p90 1.31 m, a **5.24:1** spread. So the
cells are many-sized and the *rendering* of them is what flattens them: every cell is drawn
with the *same* joint width and the *same* relief, so a big cell and a small cell read as the
same kind of piece.

And the geology says why a cliff has more than one *kind* of piece.

## Sources

- **Verdian, J. P., Sklar, L. S., Riebe, C. S., Moore, J. R. (2021)**, *Sediment size on talus
  slopes correlates with fracture spacing on bedrock cliffs*, Earth Surface Dynamics 9:1073–1090,
  DOI `10.5194/esurf-9-1073-2021` (open access; the PDF was downloaded to
  `/tmp/research/verdian2021.pdf` and read in full, 914 lines of `pdftotext -layout`). Used, and
  it is the model this note adopts:
  - The **mechanism**: the initial size distribution "should depend on three factors: the size
    distribution of discrete blocks latent within bedrock, the weathering processes that modify
    rock in situ, and the weathering processes that detach and mobilize the sediment. Where
    effects of weathering on size are minimal, the initial size distribution should strongly
    reflect the size distribution of latent blocks (i.e., the **"latent size distribution"**),
    which is **set by the spacing and orientation of fractures, foliations, and bedding planes**
    (Sklar et al., 2017). These represent preexisting planes of weakness and determine the
    volume and shape of newly created sediment particles".
  - So a block is the **intersection of THREE families of planes**, and its size is the product
    of three spacings — and a product of three is a much heavier-tailed distribution than one.
  - The **spacings'** own distribution: the fracture spacing's is a **Weibull** ("the Weibull
    form of the exponential distribution … yielded the best fit to the data in most cases"),
    running "from 2 to 5000 mm" with the site medians "from 10 to 390 mm" and a **geometric
    standard deviation** of 0.214 to 0.551 across the fifteen sites.
  - The **coupling**: "the close site-to-site correspondence between the spread in the b-axis
    diameter distribution and the spread in the fracture spacing distribution" — the spread of
    the blocks' sizes and the spread of the spacings are the same law.
  - The **shape**: the blocks are rods, slabs and equi-sided blocks, with the b : a and c : a
    ratios of a site's talus averaging 0.64 and 0.35 for the granodiorite and 0.57 and 0.26 for
    the metasediment, and the universal values for mechanically weathered blocks being **0.67
    and 0.45** (Domokos et al. 2015). A block is **flat and elongated**, and not a cube.
  - The **caveat**: the lower tails "follow a steeper trend than the rest of the data …
    potentially reflecting an **undersampling of the smallest fractures**" — a survey misses the
    smallest, so a real cliff has *more* small pieces than a measured one.
- **Strauhal, T., Zangerl, C. (2021)**, *The Impact of Fracture Persistence and Intact Rock
  Bridge Failure on the In Situ Block Area Distribution*, Applied Sciences 11, DOI
  `10.3390/app11093973` (the record and the abstract were read; **the PDF was not obtained** —
  MDPI returns 403). Used: "The **in situ block size distribution** is an essential
  characteristic of fractured rock masses"; and the mechanism that makes the size distribution
  hard: "The block size distribution can be estimated rather easily for **fully persistent
  fractures**, but it is a challenge to determine this parameter when **non-persistent
  fractures** in a rock mass should be considered. In many approaches, the block size
  distribution is estimated by **assuming that the fractures are fully persistent**, resulting in
  an underestimation of the block sizes for many fracture geometries."
  So the crate's single, always-persistent cell network is exactly the *simplification* this
  paper names — and the simplification is what makes every piece the same kind of piece.
- **Macciotta, R., Gräpel, C., Skirrow, R. (2020)**, *Fragmented Rockfall Volume Distribution
  from Photogrammetry-Based Structural Mapping and Discrete Fracture Networks*, Applied Sciences
  10, DOI `10.3390/app10196977` (the record and the abstract were read; **the PDF was not
  obtained**). Used: "the **fragmented volume-relative frequency distribution** of rockfalls" is
  the input a fragmentation model needs, and "simple **discrete fracture network** realizations
  … can be used to reliably estimate rock fall block volumes" — the block volumes are the
  *network's* volumes, so the network is where the size distribution has to live.
- **The crate's own `rock-fragments.md`** (iteration 37): the blocks, the warp and the unequal
  cells, and the sentence that frames this change — "The size spread comes from the **cells
  themselves**: the network's feature points are drawn *towards* their cell's centre, so the
  cells are unequal: a point near its cell's corner leaves a big cell behind it and a point at
  the centre a small one. That is the only place a cell network's cells can differ in size, and
  it costs one multiply." The hole in it: the cells differ in size, but every cell is drawn the
  *same way*, so the *pieces* do not read as different kinds.
- **The crate's own `rock-blocks.md`** (iteration 35): the blocks, the joint and the note's own
  warning — "A Worley cell network cannot make them — it bounds *cells*, so a face reads as a
  giraffe's hide" — and the **joint sets** (La Pointe & Hudson 1985): "a rock breaks along
  *joint sets* — families of parallel planes — so its pieces are aligned, not a random heap".

## Model adopted

**1. A block is the intersection of *three* sets of planes, so the size distribution is a
*product* of three.**

The crate has **one** cell network, so one spacing, and the pieces are all the same kind. The
geology has three — fractures, foliations and **bedding planes** — and a block's size is the
product of their spacings. A product of three heavy-tailed draws has a *much* heavier tail than
one, and a heavier tail is exactly what "slabs and chips in the same wall" is.

So the blocks are cut by **two more sets of planes**, and each set's spacing is a draw. The
simplest and cheapest construction that does this: **the cell network is evaluated at a scale
that is itself a draw**, so a cell in a coarse region is a slab and a cell in a fine region is
a chip. That is the *product* in its cheapest form: the local cell size is the product of a
draw and the nominal cell.

**2. The two extra sets are the *bedding* and a *second joint set*, and they are not more
networks.**

Two more cell networks would cost two more lookups — the crate's hottest path. But a set of
planes is a *period*, and a period is a *coordinate scaling*. So the second and third sets are
applied by **stretching the same lattice's coordinates by different factors in different
directions**, which is what a joint set *is*: a family of parallel planes. The lattice
becomes a family of *planes* in three directions, and the cell is the box between them.

**3. The cell's own *size* is the product of three spacings, and it is what varies.**

With the lattice stretched in three directions, the cell is a *box* whose three extents are
the three spacings. A cell whose bedding extent is large and whose joint extent is small is a
**slab**; a cell that is small in all three is a **chip**. So the size spread is not a draw
applied to a uniform cell: it falls out of the *geometry*, which is what the geology says.

**4. The rendering must show the difference, and the joint's *width* is the lever.**

The cells are many-sized already; the rendering flattens them. The joint's width is the
cell's own draw, so a small cell's joint eats the cell and a big cell's joint is a hairline.
The joint's width must instead **scale with the cell's own size**, so a chip's joint is a chip's
width and a slab's joint is a slab's width — which is what a joint *is*: a feature of the
rock's own scale.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The lattice's stretch along the bedding | the crate's 1.7 | `rock-blocks.md`: the blocks are tabular (La Pointe & Hudson 1985 on joint sets) |
| The second set's stretch | 1.0 (a second, independent direction) | a joint set is a family of parallel planes |
| The cell's own size spread | the product of the three extents | Verdian et al. 2021: the latent size distribution is "set by the spacing and orientation of fractures, foliations, and bedding planes" |
| The joint's width | scaled with the cell's own size | a joint is a feature of the rock's own scale |
| The block's c : a | 0.45 | Domokos et al. 2015's universal value, via Verdian et al. 2021 |
| The band-limit | the *smallest* block's own joint | the crate's `detail_keep`, and the corduroy lesson |

## GPU mapping

- **Inside the terrain's own fragment shader**, where the blocks already are
  (`src/terrain/common.wesl`, `rock_blocks`). **No new pass, no new buffer, no new binding.**
- The three spacings are the *same* lattice, stretched — so the cost is the *same* one lookup,
  and the sizes come out of the geometry rather than out of extra lookups.

## What great looks like

- **A cliff face at 3 m reads as rock**: a **slab** with a fist-sized block beside it and a chip
  in the crack between them, not a uniform field of same-sized pieces. The eye can *count*
  the big ones and see the small ones between.
- **The pieces are of different *kinds***: the slabs are flat and elongated, the chips are
  equi-sided, and the face has both.
- **The distance is calm**: the small pieces are gone by 30 m and the slabs remain, and there
  is no corduroy at any distance.
- **The meadow is untouched.**

## Validation targets

1. **The pieces are of many sizes.** The cells' sizes along a cut: the p90/p10 ratio is at
   least **8:1** (it is **5.24:1** now), and the *rendered* pieces' ratio — measured on the
   image, over the joint's crossings — rises with it.
2. **The face is not a *field of equal pieces*.** Over a rock-only patch of the face at 3 m,
   the distribution of the pieces' widths has a p90/p10 of at least **4:1** (it is 2.5:1 now).
3. **The distance is calm.** The frame at 120 m is unchanged to four decimals, and the high-pass
   at 20 m does not rise by more than 1 DN.
4. **The meadow is untouched.** The `plains` preset at 6 m is byte-identical.
5. **The cost.** The mountains' opaque pass is within 10 % of the baseline's minimum.

## Deviations from this plan

*(filled in as the implementation settles; every changed default is recorded here with its
citation.)*

## Pending index

- Verdian et al. 2021 (`10.5194/esurf-9-1073-2021`): the PDF was **downloaded and read** for
  this note. **Not indexed.**
- Strauhal & Zangerl 2021 (`10.3390/app11093973`): the record and the abstract were read; **the
  PDF was not obtained** (MDPI returns 403).
- Macciotta et al. 2020 (`10.3390/app10196977`): the record and the abstract were read; **the
  PDF was not obtained**.
- The three papers on the block size distribution from the discontinuity spacing — **Yao, W.,
  Sharifzadeh, M., Yang, Z. (2020)**, DOI `10.1063/1.5143906`; **Azizi, A., Moomivand, H.
  (2021)**, DOI `10.1007/s00603-020-02360-4`; and **Park, L. et al. (1993)**, DOI
  `10.1016/0148-9062(93)90982-j` — were found by `paper_search` and their records read, but
  **no PDF was obtained**. Named as the treatments of the block size distribution from the
  discontinuity spacing, which is the model this note adopts.
- `distill_index` is skipped for the above.
