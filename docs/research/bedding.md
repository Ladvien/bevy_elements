# Beds that stack: a distant cliff is a succession, not a stripe pattern

The crate's beds are smeared by their own warp and every bed is identical, so a massif at a
distance averages to a flat blue-grey haze. A cliff is a **succession**: beds of different
thicknesses and different values stacking up, and the succession is what a distant cliff shows.

## Sources

- **Talling, P. J. (2001)**, *On the frequency distribution of turbidite thickness*, Journal of
  Sedimentary Research 71(3), DOI `10.1046/j.1365-3091.2001.00423.x` (the record and its
  abstract were read; **the PDF was not obtained**). Used: "the nature of this thickness
  distribution is currently uncertain, with log-normal or negative-exponential frequency
  distributions and power-law cumulative frequency distributions having been proposed by
  different authors", and the result — "turbidite bed thickness and sand-interval thickness
  within each bed have a frequency distribution comprising **the sum of a series of log-normal
  frequency distributions**". So a bed's thickness is a **draw from a distribution**, not a
  constant, and the distribution is heavy-tailed: a few beds are much thicker than the rest.
- **Longhitano, S. G., Nemec, W. (2005)**, *Statistical analysis of bed-thickness variation in a
  Tortonian succession of biocalcarenitic tidal dunes, Amantea Basin, Calabria, southern
  Italy*, Sedimentary Geology 178(1–4), DOI `10.1016/j.sedgeo.2005.05.006` (the record was
  read; the PDF was not obtained). Named as the treatment of *bed-thickness variation* — the
  variation the crate's identical beds do not have.
- **Bádenas, B., Aurell, M., Rodríguez-Tovar, F. J., Pardo-Igúzquiza, E. (2003)**, *Sequence
  stratigraphy and bedding rhythms of an outer ramp limestone succession (Late Kimmeridgian,
  Northeast Spain)*, Palaeogeography, Palaeoclimatology, Palaeoecology 198, DOI
  `10.1016/s0037-0738(03)00099-x` (the record was read; the PDF was not obtained). Named as
  the treatment of **bedding rhythms** — the *rhythm* of a succession, which is the pattern a
  distant cliff shows.
- **Lowey, G. W. (1992)**, *Variation in bed thickness in a turbidite succession, Dezadeash
  Formation (Jurassic–Cretaceous), Yukon, Canada: evidence of thinning-upward and
  thickening-upward cycles*, Sedimentary Geology 150(1–2), DOI
  `10.1016/s0037-0738(92)90021-i` (the record was read; the PDF was not obtained). Named for
  the **cycles** — a succession has a *rhythm* in its thicknesses, and not only a random draw.
- **Hill, S., Baker, D. (2012)**, *Rock-Solid Shading: Image Stability Without Sacrificing
  Detail*, SIGGRAPH *Advances* in Real-Time Rendering in Games course (corpus
  `s2/s2012-advances-pdf-slides-3.md`). Used: the anti-aliasing principle — **estimate the
  variance and widen the filter** rather than let a sub-pixel feature alias (Toksvig's
  σ² = (1 − |Nₐ|)/|Nₐ|, and LEAN mapping's direct variance estimation), and the point that a
  detail map must not alias at distance. For the *albedo* the crate's own `detail_keep` is the
  instrument: a band whose thickness is below the pixel's footprint must be **cancelled**, not
  averaged.
- **The crate's own `rock-material.md`**: the beds, the joints and the grain, and the two
  sentences that frame this change — "The bed's coordinate is a *tilted* height, so the beds cut
  across the terrain at an angle, and the warp tilts them further" (the *warp* is right; its
  amplitude is not), and "a bed is a *band* of rock with an edge, not a gradient" (the edge is
  right; the band is not).
- **Akenine-Möller, T., Haines, M., Hoffman, N., Shirley, P. (2019)**, *Real-Time Rendering,
  4th ed.* (corpus `bo/book_real_time_rendering_4th_akenine.md`), on the cellular texture and
  on Andersson's Frostbite terrain, where the materials are combined from "terrain type,
  altitude, slope" — the crate's own layer masks.

## The measurement that started this

The `mountains` preset from 120 m, over a rock-only 200 × 200 patch on the near massif:

| | the value |
|---|---|
| the mean luminance | 142.18 |
| the standard deviation | 9.39 |
| the max−min | 52.66 |
| the mean saturation | 8.42 |

The spread is the *terrain's own shading* — the hills' forms — and not the rock's material.
At 300 % the rock is a flat blue-grey wash with a soft, formless mottling: no bands, no layers,
no edges. The green turf and the ochre soil read clearly at the same distance, so it is not
the resolution that is failing; the rock has no structure to resolve.

## Model adopted

**1. The warp must undulate the beds, not displace them.**

The bed's coordinate is `(dot(p, TILT) + warp · 3.5) / 4.5`, and the warp is the ground's
45 m-scale field in [−1, 1]. So the warp displaces the bed coordinate by **up to 0.78 of a
bed** — most of a bed's own thickness. A 4.5 m grid distorted by a 45 m-scale field is not a
set of parallel bands; it is a set of *blotches*, and the mask then picks a random half of the
smeared field. That is the mottling.

The fix is the *amplitude*: the warp's displacement drops from 0.78 of a bed to about a
**fifth** (0.9 m), so the beds dip and undulate — which is what the warp was for — while
remaining bands.

**2. The beds must stack: a bed's thickness is a draw, not a constant.**

Talling 2001: a bed's thickness follows a **log-normal** distribution, and a succession's
thicknesses are the *sum of a series* of them. A cliff of identical 4.5 m beds is a stripe
pattern; a cliff of beds whose thicknesses are drawn from a heavy-tailed distribution is a
*succession*, and that is what a distant cliff shows.

So the bed's **index** — which bed this is — must be found, and the bed's *thickness* must
come from a draw on that index. The index is available from the coordinate once the warp is
small: `floor(bed)` is the index, and the index is what a per-bed value is drawn from.

**3. A bed's own value comes from its index, so the beds differ from one another.**

Every bed is currently the same value (`ROCK_SECOND_TONE` mixed in at a fixed `ROCK_BED_MIX`),
so the cliff is one colour modulated by a mask. A succession's beds are *different rock* — a
sandstone, a shale, a limestone — and the cheap way to say that is a **per-bed tone** drawn
from the bed's index: a value per bed, and a second value for the beds that are *thin*, because
a thin bed is a different thing (a shale parting) from a thick one.

**4. The bands must not alias.**

A bed on a face 120 m away is a few pixels across. A stripe that is sub-pixel is a flat tone —
which is exactly what the massif is now. The crate's own `detail_keep` is the instrument: a bed
thinner than the pixel's footprint is **cancelled**, and what survives is the *variation
between* the beds — the succession's tone, which is a low-frequency feature and survives
everything.

So the bed's own edge is band-limited away with the distance, and the bed's *tone* is not. That
is the whole trick: at a distance the cliff shows its **succession** — bands of tone — and not
its stripes.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The bed plane's tilt | (0.32, 0.90, 0.28) | the crate's existing `ROCK_BED_TILT`; 35° from vertical |
| The bed's nominal spacing | 4.5 m | the crate's existing `ROCK_BED_M` |
| **The warp's amplitude** | **0.9 m** (0.2 of a bed, against the old 3.5 m) | the measurement: 0.78 of a bed smears the bands into blotches |
| **A bed's thickness** | a log-normal draw on the bed's index | Talling 2001: "the sum of a series of log-normal frequency distributions" |
| **A bed's tone** | a per-bed value from the index | the beds are *different rock*; the crate's own note: "a bed is a *band* of rock" |
| The bed's edge | band-limited by the footprint | the crate's `detail_keep`, and Toksvig's variance |

## GPU mapping

- **Inside the terrain's own fragment shader**, where the beds already are
  (`src/terrain/common.wesl`, `rock_material`). **No new pass, no new buffer, no new binding.**
- The cost is one noise evaluation for the per-bed value, and the crate's hottest pass is the
  mountains' opaque pass — which iteration 35 already put at **+28 %**. The beds' work must not
  add to that.

## What great looks like

- **A distant massif shows its succession**: bands of tone running across the rock, in layers
  that follow the bedding and not the hill's contours, with the hills' forms still reading
  through them.
- **The near face shows the beds as beds**: a stack of layers of different thicknesses and
  different values, with the grass's edge and the scree at the foot.
- **The banding is not a stripe pattern**: the beds' thicknesses vary, so the eye reads a
  *sequence* and not a wallpaper.
- **The distance is calm**: no aliasing, no corduroy, and the massifs' tones are the beds'
  tones and not a flat grey.

## Validation targets

1. **The massif is not one colour.** Over a rock-only 300 × 200 patch of the massif from 120 m,
   the luminance spread is at least **9 DN** (it is 6.9, and the *material's* own contribution
   is smaller still).
2. **The beds are bands and not blotches.** The bed's mask along a vertical cut crosses the
   half-way mark at least every 4 m, and the spacing's *coefficient of variation* is under 0.3
   (a smear has a huge one).
3. **The beds' thicknesses vary.** The thicknesses' spread is at least 2:1 between the tenth
   and the ninetieth percentile, and the distribution is heavy-tailed.
4. **The beds are tilted, not contoured.** A vertical cut across a hillside crosses beds at an
   angle to the contour, and the angle is the bed plane's (35° from vertical), not the hill's.
5. **The meadow is untouched.** The `plains` preset at 6 m is byte-identical.
6. **The distance is calm.** The high-frequency energy of the massif at 120 m does not rise: the
   bed's *edge* is cancelled and only its *tone* survives.
7. **The cost.** The mountains' opaque pass is within 10 % of iteration 35's (0.181 ms).

## Deviations from this plan

*(filled in as the implementation settles; every changed default is recorded here with its
citation.)*

## Pending index

- Talling 2001 (`10.1046/j.1365-3091.2001.00423.x`), Longhitano & Nemec 2005
  (`10.1016/j.sedgeo.2005.05.006`), Bádenas et al. 2003 (`10.1016/s0037-0738(03)00099-x`) and
  Lowey 1992 (`10.1016/s0037-0738(92)90021-i`): the records and their abstracts were read;
  **the PDFs were not obtained** (no open copy from any provider). Listed here so the gap is on
  the record.
- Hill & Baker 2012 (Rock-Solid Shading) is **in the corpus**
  (`s2/s2012-advances-pdf-slides-3.md`) and was read. **Not indexed.**
- `distill_index` is skipped for the above.
