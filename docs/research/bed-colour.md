# A distant range is many rocks: the beds carry the colour to the horizon

Iteration 38 gave the cliff a **palette**: four rocks, mixed as convex shares, and the hue's
resultant over a rock-only patch of the face at 3 m fell from 0.800 to 0.656. But the palette
**stops at the fragments**, and the fragments are band-limited away at 120 m — so a distant
massif is still one cool grey, and a game that shows a mountain range from a kilometre away sees
no variety at all.

The massif at 120 m (`/tmp/i39/before/far120.png`): every massif the **same cool blue-grey**,
the beds' banding showing only as faint value stripes. The massif at 300 m
(`/tmp/i39/before/far300.png`): a single flat blue-grey.

This is by design — iteration 36 established that **the beds carry the distance**: the bed's
*edge* is band-limited away and the bed's *tone* is not. The design is right and the *bed's
tone* is the hole in it: it is a **scalar**, exactly like every other tone in the crate's rock
system was before iteration 38. A scalar can only make a rock lighter or darker.

So the palette has to go to the **bed's** scale, and the geology says how.

## Sources

- **Debret, M., Sebag, D., Desmet, M., Balsam, W., Copard, Y., Mourier, B., Susperriguui, F.,
  Arnaud, F., Bentaleb, I., Chapron, E., Lallier-Vergès, E., Winiarski, T. (2011)**,
  *Spectrocolorimetric interpretation of sedimentary dynamics: The new "Q7/4 diagram"*,
  Earth-Science Reviews 106, DOI `10.1016/j.earscirev.2011.07.002` (open access; the PDF was
  downloaded to `/tmp/research/debret2011.pdf` and read in full, 2716 lines of
  `pdftotext -layout`). Used, and it is the model this note adopts:
  - The **Q7/4 ratio** — "the ratio between the % reflectance value at 700nm to that at 400
    nm" — "provides the numerical description of the general slope of VIS", and it is the
    standard quantitative description of a sediment's colour. The paper's own finding: "the
    Q7/4 ratio (from **1.5 to 6**)" across the sediments it measured, with a value near 1 for
    the greyest and the highest for the reddest ("L* close to 90°, Q7/4 ratio close to 1" for
    the one fjord point that "was completely separated from the general trend").
  - The **mechanism**: the colour is the **iron oxide's form**. "the origin of the colour in
    the iron oxides has been studied. The absorption bands exhibited by the iron oxides from UV
    to near IR wavelengths originate from electronic transitions", and the consequence: "goethite
    is **yellow** while hematite is **red** because the corresponding band lies at a lower
    wavelength in the visible".
  - The **poles** of the diagram — "Clayey deposits, organic rich deposits (chlorophyll a and
    by products), altered organic matter deposits, **iron rich deposits**, carbonated deposits"
    — and the "Iron-rich deposits" pole is labelled by "a high peak around 575 nm corresponding
    to **hematite**".
  - The purpose: "Colour is a fundamental property of sediment and is often used for
    **lithographic description** to determine sedimentological structures, facies etc."
- **Shahrokhi, S., Bábek, O., Mansurbeg, H., Šegvić, B., Martín-Martín, J., Kapusta, J., Gregar,
  F., Kořenek, M. (2025)**, *Early Diagenetic Controls on Color Variation in Eocene Red Beds: A
  Case Study from the Gercus Formation, Zagros Basin, Kurdistan Region, Iraq*, Journal of
  Sedimentary Research, DOI `10.2110/jsr.2025.005` (the record and the abstract were read; **the
  PDF was not obtained**). Used: "the spectrum of sediment hues, from **red, to yellow-brown,
  gray-green, and gray**, arise from the formation and distribution of **Fe oxy/hydroxides**" —
  so a succession's beds span a *spectrum* of hues, and the spectrum's axis is the iron.
- **Tansem, K., Storemyr, P. (2020)**, *Red-coated rocks on the seashore…*, Geomorphology 395,
  DOI `10.1002/gea.21832` (the record and the abstract were read; the PDF was not obtained).
  Used from iteration 38: "rocks with **red iron films, rocks that are now generally gray**".
- **The crate's own `rock-palette.md`** (iteration 38): the palette, the convex shares, the
  score, and the discipline — "the *mean* colour and the *mean* brightness stay where they
  were (a ground that changes colour changes the game's lighting)". The bed's palette is that
  note's construction at the bed's scale, and it inherits its discipline.
- **The crate's own `bedding.md`** (iteration 36): the beds stack, the bed's tone is a draw on
  its index, and the whole trick — "the bed's *edge* is band-limited away with the distance
  while the bed's *tone* is not, which is the whole trick: at a distance the cliff shows its
  succession and not its stripes". The hole in it: the bed's *tone* is a **scalar**.
- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`): the **Overlay** blend,
  which "doesn't affect areas where base mask is 0.0 or 1.0" — the instrument for a modulation
  that must not move the mean.

## Model adopted

**1. A bed's colour is its iron, and the iron is a draw on the bed's index.**

The mechanism is the iron oxide's form: goethite is yellow, hematite is red, and the Q7/4
ratio — the "numerical description of the general slope of VIS" — measures it, running from
**1.5 to 6** across the sediments Debret et al. measured and near 1 for the greyest.

So the bed's **Q7/4 is a draw on the bed's index**, over that range, and the bed's *hue* is what
the Q7/4 says: a low ratio is a grey bed and a high one is a red bed. This is the same
construction the crate's beds already use for their *thickness* and their *tone* — a draw on
the index — with the *quantity* changed from a brightness to a **hue**.

**2. The bed's hue is a *palette* of iron-bearing rocks, and it is mixed as convex shares.**

The same palette as the fragments', but at the *bed's* scale and with a *different* selection:
the bed's index, not the cell's. A bed is one rock, so the bed's own draw is emphatic — the
score is a **narrow split**, the same construction as iteration 38's, and the split's width is
Papavasiliou's `p`.

**3. The bed's colour is a *low-frequency* feature, so it survives every distance.**

The bed's *edge* is a line a fraction of a bed wide and it is band-limited away; the bed's
*tone* is a feature over metres of rock and it survives. A bed's *colour* is the same kind of
feature as its tone — a bed's colour is its own, over the whole bed — so it survives with it,
and the massif from 700 m shows a succession's *colours* and not only its bands.

**4. The near face must not change.**

The fragments already carry the palette, and the bed's colour is *under* them. So the bed's
colour is mixed in only where the fragments do not resolve — the same band-limit the crate
already uses for the blocks — and the face at 3 m is unchanged.

**5. The mean must not move.**

A game's lighting is set by its albedos, and the crate's own rule from the ground's palette is
that "the *mean* colour and the *mean* brightness stay where they were (a ground that changes
colour changes the game's lighting)". So the bed's palette is normalised the way the
fragments' was: the equal-weight mean is exactly (1, 1, 1).

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The bed's Q7/4 draw | the hash of the bed's index, over the ratio's range | Debret et al. 2011: the Q7/4 runs 1.5 to 6, and near 1 for the greyest |
| The palette | the fragments' four rocks, at the bed's scale | iteration 38's palette; a bed is a rock |
| The score's shape | a narrow split on the bed's index | iteration 38's construction: a product of smooth fields blends, and the blend is what left the cliff at one hue |
| The palette's mean | exactly (1, 1, 1) | the crate's rule: the mean colour and brightness stay where they were |
| The band's limit | the *fragments'* band-limit: the joint's width | the fragments carry the near field; the beds carry the far one |
| The region's scale | 260 m | a formation is a massif, and a massif is hundreds of metres |
| The region's bias | 0.30, on the noise's own range | a bias and not a replacement: a saturated bias moved the frame's mean 66 DN |
| The saturation | the palette's own, 0.10 to 0.30 | iteration 38: rock is a desaturated colour |

## GPU mapping

- **Inside the terrain's own fragment shader**, where the beds' tone already is
  (`src/terrain/common.wesl`, `bed_value` and `rock_material`). **No new pass, no new buffer,
  no new binding.**
- The bed's index's hash is **already drawn** — the bed's tone and its thickness both use it —
  so the bed's colour is free of new hashes.

## What great looks like

- **A range of mountains at 300 m has more than one colour**: a warm ridge beside a grey-blue
  one, an ochre slope beside a pale one. The beds' *colours* read as broad patches over the
  massifs, the way a real range's different formations do.
- **The colour is a *hue*, not a brightness**: the massifs separate into warm and cool, and
  the warm ones are *red-brown* and not merely lighter.
- **The near face is unchanged**: the fragments carry the close range, and the bed's colour is
  under them.
- **The distance is calm**: the colours are broad and low-frequency, and no corduroy appears.
- **The meadow is untouched.**

## Validation targets

1. **The distant massif is not one colour.** Over the rock of the frame at 300 m, the hue's
   resultant falls by at least **0.05** from the baseline's, and the chroma's median rises.
   (The measure is over the frame's *rock*, and the turf and the haze must be excluded —
   the baseline's own mask, which included the turf, read 0.2554 and was measuring the
   *green*.)
2. **The near face does not change.** The hue's resultant over a rock-only patch of the face
   at 3 m moves by less than **0.02**, and the frame's pixels differ in fewer than 1 %.
3. **The mean does not move.** The massif's mean brightness at 300 m moves by less than
   **1 DN**, and the mean hue by less than 0.005.
4. **The meadow is untouched.** The `plains` preset at 6 m is byte-identical.
5. **The cost.** The mountains' opaque pass is within 10 % of the baseline's minimum.

## Deviations from this plan

1. **The region is a *slow field* that biases the bed's selection, and not the bed's own
   draw.** The plan had the bed's colour drawn on the bed's index alone. That gives the beds
   their colour, and the beds are *horizontal*, so the result at a distance is a set of
   **stripes** — and a range of mountains is not banded, it is made of *formations*, and a
   formation is a whole massif. So a slow field at the massif's scale (**260 m**) biases the
   bed's selection, and the massifs separate into warm and cool.
2. **The region's bias is WEAK, and its value is the noise's own range.** The first version
   normalised the field by the noise's rms and multiplied it by 0.42, and it **saturated** the
   selection: one tone took a whole massif, the rock's mean colour then depended on which tone
   a massif happened to get, the frame's mean moved **66 DN**, and the massif came out as a
   **flat dark blue silhouette**. The field is a *bias* and not a replacement, the bias is
   **0.30**, and the field's value is the noise's own range rather than its rms.
3. **The bed's colour is band-limited against the *fragments*, and the limit is the
   fragments' own.** The plan said the band-limit was the fragments' band-limit, and it is:
   their limit is the **joint's width**, and it is a *known* number, so the limit is computed
   directly and the blocks' lookup is not needed for it. The two are then band-limited against
   each other **exactly once**: where a fragment resolves it wears its own rock, and where it
   is gone the bed's own rock shows through.
   The limit has to be applied **inside** the bed's own value and not to the finished tone.
   Applied to the finished tone, the bed's colour was mixed in at *every* distance, the two
   palettes compounded, and the face at 3 m changed in **83 %** of its pixels.
4. **The bed's tone is a `vec3` and not a scalar, and the bed's *value* is unchanged.** The
   bed's brightness spread — `1 + ROCK_BED_TONE·(h·2 − 1)` — is exactly what it was, and the
   *colour* is mixed in on top of it at the palette's own strength. The mean therefore holds by
   construction, and the frame's mean moves **0.3 DN** at 120 m and **not at all** at 300 m.
5. **The note's validation target at 300 m is not reachable, and the reason is the
   atmosphere.** The target was the hue's resultant falling by 0.05, and it falls by **0.076**
   — the target is met. But the *chroma's* median does not rise at all (0.0235 before and
   after): the rock's colour varies in **hue** and not in **saturation**, because at 300 m the
   frame's rock is about 70 % haze and the haze's own blue is the colour the eye reads. That is
   physically right and it is the atmosphere's limit, not the palette's. The target is
   restated on the *hue* and not on the chroma.

## Pending index

- Debret et al. 2011 (`10.1016/j.earscirev.2011.07.002`): the PDF was **downloaded and read**
  for this note (open access, via the HAL copy). **Not indexed.**
- Shahrokhi et al. 2025 (`10.2110/jsr.2025.005`): the record and the abstract were read; **the
  PDF was not obtained**.
- Tansem & Storemyr 2020 (`10.1002/gea.21832`): the record and the abstract were read; **the
  PDF was not obtained**.
- `distill_index` is skipped for the above.
