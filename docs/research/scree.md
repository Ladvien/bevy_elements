# The scree is a scatter, not a band: stones of many sizes with turf between them

The crate's rock has a material (strata, fractures, a grain) and a place to put it, and the
scree at the turf's foot is the one place where the difference between a *texture* and a
*scatter* shows: a player expects to see **stones**, and the crate renders a **colour wash**.

## Sources

- **Verdian, J. P., Sklar, L. S., Riebe, C. S., Moore, J. R. (2021)**, *Sediment size on talus
  slopes correlates with fracture spacing on bedrock cliffs: implications for predicting initial
  sediment size distributions on hillslopes*, Earth Surface Dynamics 9:1073–1088, DOI
  `10.5194/esurf-9-1073-2021` (open access; the PDF was read in full, 914 lines of
  `pdftotext -layout`). Used:
  - "The detachment of rock fragments from fractured bedrock on hillslopes creates sediment with
    an initial size distribution that sets the upper limits on particle size for all subsequent
    stages in the evolution of sediment in landscapes."
  - The measurement: at 15 talus–cliff pairs "median b-axis diameters ranging from **10 to
    575 mm**", with the finest clasts "as small as **2 mm**" — so a real talus spans nearly
    **three orders of magnitude** in size, and its *median* moves by more than 10-fold between
    sites with the lithology.
  - "Median fracture spacing and particle size vary by more than 10-fold and correlate strongly
    with lithology… Fracture spacing and talus size distributions are also closely [related]…
    fracture spacing at most sites." — the stone's size comes from the *cliff it fell from*, so
    a scree's stones are not one size but the cliff's own size distribution.
  - The consequence for a landscape: "where weathering residence times are negligible, the
    latent size distribution can be used to predict the initial size distribution", and
    elsewhere "the size distribution is erased by weathering in the critical zone" — which is
    the *turf*: the soil and the grass that grow over a scree break it down.
- **Dorsey, J. O'B., Edelman, A. S., Jensen, H. W., Legakis, J., Pedersen, H. K. (1999)**,
  *Modeling and rendering of weathered stone*, ACM SIGGRAPH 1999, DOI `10.1145/311535.311560`.
  The record was read; **the PDF was not obtained** (no open copy from any provider). It is
  named here as the canonical treatment of a weathered stone surface, and its *approach* —
  model the stone's weathering (its fracture network, its surface's loss) and render the
  *result* — is the one the crate already follows for the rock's own material
  (`rock-material.md` Model 2).
- **The crate's own `rock-material.md`**: the strata, the fractures and the grain; and the
  sentence that is the key to this change — "A Worley cell network cannot make them — it
  bounds *cells*, so a face reads as a giraffe's hide". The same network used the *other* way
  round, as the **stones** rather than as the **cracks**, is the missing piece.
- **The crate's own `ground-colour.md`**: the palette's *shares* and the *clump* — a patch of
  ground is not one colour, and the same idea at a smaller scale is a scree.
- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`): the Overlay blend of a
  detail value onto a computed mask, which keeps 0 and 1 and is the identity at 0.5 — the
  crate's `overlay`, and the reason a *mask* can be broken up by a *detail* without moving
  where a layer is fully on or off.
- **Bevy 0.20's source**: `bevy_pbr-0.20.0/src/render/pbr_functions.wesl` — the surface's
  normal is perturbed by a *gradient* and the light does the rest, which is how a stone gets
  its own shading without a mesh.

## Model adopted

**1. The scree is a scatter of stones, and a stone is a *cell* with a radius.**

The crate's `worley3` returns the distance to the nearest feature point; the feature point
*is* a stone's centre, and the distance is the stone's radius. A cell network is the right
structure for a scatter — and the crate's own note says why it is the *wrong* structure for
fractures (a cell network bounds cells, so a face reads as a giraffe's hide). The two uses are
mirror images: the fractures want the cell's *boundaries*; the scree wants the cell's
*interior*.

So the scree needs a **2D** cell network, on the surface, in the *turf's* plane — the same
frame the turf's grain uses, because the scree lies on the near-horizontal surfaces where the
plane's projection is the surface. (The crate's `worley3` is 3D; the scree needs a 2D one, and
`noise.wesl` has `perlin2_d` and the hash it needs.)

**2. The stones have *many* sizes, and the sizes follow a power law.**

Verdian et al.: the clasts run from 2 mm to 575 mm, and the median moves by more than 10-fold
between sites. A scatter of one size reads as a *cobble pavement* or as a *gravel bed*; a
scatter of many sizes reads as scree.

The way to get many sizes from one cell network is to **nest** it: a coarse network's cells
hold a finer network's cells, and a stone's size is its cell's size times a per-cell random
factor, drawn from a distribution with a heavy tail. A cell's random factor is free — the
feature point's hash has three components, and one of them is spare.

**3. The stones are lit, not just tinted.**

A stone that is a different *colour* from the ground around it reads as a stain; a stone that
catches the light on one side and shades on the other reads as a *thing*. So each stone must
have its own **relief**: a normal perturbation from the distance field's gradient, which the
crate's fragment shader already applies (`n = normalize(n - (grad - dot(grad, n) * n))`).

The distance field's gradient points *away* from the stone's centre, so the stone's cap is
domed: the gradient tilts the normal outward, and the light rakes the stone's edge. That is
the same trick the crate's rock uses for its cracks ("the fractures give it the *step* a crack
has: the ridge's gradient turns the crack from a drawn line into a groove the sun rakes"), and
it is what makes a scatter of stones read as stones.

**4. The turf shows *between* the stones, and the stones thin out down the band.**

The scree is not a solid sheet of stone: it is stones of many sizes with turf between them.
So the scatter's *coverage* is below 1 — there are gaps — and the gaps are the turf, at the
turf's own colour. And the whole scatter lives in the band where the turf thins, thinning out
as the turf takes over.

**5. The scatter is band-limited by the pixel's footprint, like everything else in the crate.**

A stone is 10 cm across; a pixel at 700 m is a metre. A scatter that is not band-limited
aliases into a corduroy, and the crate has already paid that lesson twice (the rock's
fractures "band-limited by the cell, the cracks survived as a fine corduroy over the whole
massif at 700 m"). So the scatter's *presence* falls with the footprint, and where it is gone
the scree is the rock's own tone — the scree fades into the cliff, as a real one does.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The stone network's cell | 0.28 m | the finest talus clast's scale, and the crate's `ROCK_GRAIN_FINE_M` |
| The size spread | a per-cell factor with a heavy tail, ×0.35 to ×2.2 | Verdian et al.: 2 mm to 575 mm, a median moving 10-fold |
| The scree band | 22°–38° of slope | the crate's existing `scree` mask |
| The band-limit | the cell's size, like the fractures' width | `rock-material.md`'s corduroy lesson |
| The stone's relief | the distance field's gradient, scaled | the crate's crack relief |
| The turk's colour | the ground's own albedo | the scree is *on* the ground |

## GPU mapping

- **Inside the terrain's own fragment shader**, where the scree's colour tint already is
  (`src/terrain/render.wesl`). **No new pass, no new buffer, no new binding.**
- The cost is a 2D cell lookup in the scree band only, and the band is `scree > 0.01`, so the
  meadow and the cliff pay nothing. The crate's hottest pass is the mountains' opaque pass
  (0.138 ms on the baseline build), and this must not move it materially.

## What great looks like

- **Stones you can count.** At 2 m the scree is a scatter of individual stones with edges,
  each catching the sun on one side and shading on the other, with the turf showing between
  them.
- **Many sizes.** A big stone next to a fist-sized one next to gravel, not one size repeated.
- **The band thins into the turf.** The scatter's coverage falls with the slope, so the stones
  are dense at the cliff's foot and sparse in the meadow, and the transition is a *scatter*
  thinning out, not a colour fading.
- **The cliff's foot is not a line.** The scatter breaks the boundary, so the grass and the
  stone interleave instead of meeting at a contour.
- **The distance is calm.** At 700 m the scree is a tone, not a corduroy, and the mountain is
  not a wash of noise.

## Validation targets

1. **The stones are discrete.** At 2 m the scree's local contrast has *peaks* — a histogram
   with a second mode — and not a monotone gradient. Measured: the number of local maxima of
   the luminance along a cut across the scree.
2. **The sizes vary.** The measured stone sizes span at least a 4:1 range.
3. **The turf shows between them.** The scree's coverage is below 1: the grass's green is
   present in the scree band, not only outside it.
4. **The band thins.** The scatter's coverage falls with the slope, from about 0.6 at the
   cliff's foot to 0 in the meadow.
5. **The distance is calm.** At 700 m the scree has no visible corduroy: the luminance's
   high-frequency energy is no higher than the rock's own.
6. **The turf is untouched.** The meadow's mean and standard deviation are within 0.5 DN of
   the baseline's.
7. **The cost.** The mountains' opaque pass is within 10 % of the baseline's.

## Deviations from this plan

1. **The band is 16°–40°, not the rock's foot's 26°–34°.** The plan put the scree where the
   soil's band dies and the rock's ramp begins. Measured on the capture, that band is *nine
   degrees wide*, and on a long slope it traced the hillside's break as a **contour line** —
   the one thing a scree is not. The band is now the soil's own band up to the rock's ramp
   (16°–40°), which is an apron tens of metres deep, and the weight falls towards both ends.
2. **The stones' relief is a *slope*, not a height, and it is dimensionless.** The plan gave
   the dome a height in cell units. A gradient that scaled with the cell made the *smallest*
   stones' rims tilt by 85° and they rendered as black crescents; a gradient that scaled with
   the cell's reciprocal made the fine network's stones 2.6× steeper than the coarse ones. The
   dome is now a fixed *shape*: a big stone is a big dome and a small one a small dome, and
   both catch the light the same way.
3. **The network is nested, not single.** The plan had one network with a per-cell size
   factor. One network at one size reads as *pebbles on grass*: the stones are too sparse and
   the turf too dominant. Two nested networks — a coarse one for the big stones and a fine
   one for the small ones, the fine ones only where the coarse ones do not cover — read as a
   scree, and it is the nested tail that a real size distribution has (Verdian et al. 2021:
   the finest talus clasts are 2 mm against a median of 10 to 575 mm).
4. **The ground between the stones is a greyed soil, not the crate's SOIL tone.** At full
   SOIL it read as warm bare earth — a dirt road tracing the hillside. The matrix is now the
   SOIL tone mixed 62 % toward the rock, which is what a scree's matrix is: the weathered rock
   between the stones.
5. **The scree's soil replaces the turf's colour in the band.** The plan had the turf showing
   between the stones, and the turf's *green* showed through the whole band, so the scree read
   as a lawn with pebbles on it. A scree has no grass in it: the band's ground is the matrix,
   and the turf comes back at the band's edges and in the layer variation's patches.
6. **The cell is 0.45 m, not 0.28 m.** At 0.28 m the stones were 0.10 m across — pebbles. The
   note's own measurement (a median b-axis of 10 to 575 mm) puts a scree's typical stone well
   above that.
7. **The contact shadow is the stone's own occlusion, not a cast shadow.** The ground's
   fragment shader has no access to the sun's direction, so a *cast* shadow is not available
   without a new uniform. What is available — and what the capture wanted — is the stone's own
   occlusion of the matrix it lies on, which is the darkening of the ground within a stone's
   radius of its rim. It is what makes the stones sit *in* the dirt rather than on it.
8. **Two cell lookups and one noise evaluation per pixel, not four and two.** The contact
   shadow is computed from the same cell the stone's coverage came from, and the stone's grain
   is the same for both scales, so it is evaluated once. The captures are identical to the
   last decimal.

## Pending index

- Verdian et al. 2021 (`10.5194_esurf-9-1073-2021`) was **downloaded and read** for this note
  (the open-access PDF from the publisher). **Not indexed** (the ONNX out-of-memory error in
  `corpus.md`).
- Dorsey et al. 1999 (`10.1145/311535.311560`): the record was read, the PDF was **not
  obtained**. Listed here so the gap is on the record.
- `distill_index` is skipped for the above.
