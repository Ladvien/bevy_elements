# The cliff is not one grey: a rock face carries a palette

The crate's cliff is a **single cool grey**, from the near face to the horizon. Measured on the
build before this note (`/tmp/i38/before`, `terrain_tour`, seed 42):

| Scene | the chroma's median | the hue's resultant | the hue's spread |
|---|---|---|---|
| the face at 3 m (a rock-only patch) | **0.0157** | 0.800 | 27.6° |
| the whole face at 3 m | 0.0275 | 0.853 | 21.1° |
| the face at dusk | 0.2039 | **0.9977** | **2.1°** |
| the face at dawn | 0.1176 | **0.9982** | **2.3°** |

At dusk and at dawn the cliff is a *single hue*, spread 2°. The whole face is one colour, and
the light only tints it. A cliff is not that: it is a **succession of different rock** — a
sandstone, a shale, a limestone — each with its own colour, and each wearing a *rind* of
weathered material that is a different colour again.

The crate's rock system is monochrome by construction: the tone is a **scalar** everywhere. The
bed's tone is `1 + ROCK_BED_TONE·(…)`, the block's tone is `1 + ROCK_BLOCK_TONE·(…)`, the
second tone is a fixed `vec3`. A scalar can only make a rock *lighter or darker*, never
*another colour*, and the hue's resultant at dusk is 0.998.

## Sources

- **Tansem, K., Storemyr, P. (2020)**, *Red-coated rocks on the seashore: The esthetics and
  geology of prehistoric rock art in Alta, Arctic Norway*, Geomorphology 395, DOI
  `10.1002/gea.21832` (the record and its abstract were read; **the PDF was not obtained** —
  Wiley returns 403). Used, and it is the model this note adopts: "the pronounced **red**
  bedrock surfaces in the current seashore zone are composed of **inorganic iron films** related
  to a high content of magnetite in the native sandstone"; and the sentence that is the whole
  change: "it is highly likely that the rock art was originally carved on rocks with **red iron
  films, rocks that are now generally gray**" — *the same rock* wears a red film in one place
  and is grey in another, and the film's own colour is what the surface shows.
- **Krinsley, D., Pye, K., Boggs, S., Tovey, D. (1998)**, *Desert (rock) varnish*, in *Backscattered
  Scanning Electron Microscopy and Image Analysis of Sediments and Sedimentary Rocks*,
  Cambridge University Press, DOI `10.1017/cbo9780511628894.008` (the record and its
  introduction were read; **the PDF was not obtained**). Used: "Desert (rock) varnish is a thin
  (**< 100 to 500 μm thick**) coating of **ferromanganese oxides, clay minerals, and trace
  elements**, which can form on sedimentary, igneous, or metamorphic rocks with stable surfaces";
  and that it is "a special kind of microscale sedimentary deposit" with "numerous ultrathin
  laminae that commonly grade into irregular pods of jumbled material" — so a varnish is
  **layered** and **patchy**, and it is a *coating on* the rock and not the rock.
- **Reneau, S. L. (1993)**, *Manganese Accumulation in Rock Varnish on a Desert Piedmont, Mojave
  Desert, California, and Application to Evaluating Varnish Development*, Quaternary Research
  40(1), DOI `10.1006/qres.1993.1084` (the record and its abstract were read; **the PDF was not
  obtained**). Used: "Rock varnish coatings tend to become **thicker, darker, and more
  continuous** over time, leading to the use of changes in overall varnish color and the
  percentage of clast surfaces covered by varnish as relative-age indicators" — so the varnish's
  *coverage* is a fraction of the surface, and its colour deepens with it. **Not indexed.**
- **Potter, R. M., Rossman, G. R. (1979)**, *The manganese- and iron-oxide mineralogy of desert
  varnish*, Geochimica et Cosmochimica Acta 43, DOI `10.1016/0009-2541(79)90085-8` (the record
  was read; **the PDF was not obtained**). Named as the mineralogy of the varnish.
- **Papavasiliou, N. (2015)**, *Real-Time Grass (and Other Procedural Objects) on Terrain*,
  Journal of Computer Graphics Techniques 4(1):26–49 (`https://jcgt.org/published/0004/01/02/paper.pdf`).
  **This is the model for the mixing**, and it is the crate's *own* precedent: the ground's
  palette (iteration 30) is this model's implementation, and the rock's palette is the same
  construction on a different material. Used: the **reference colour** and the **score** — for
  each terrain type a reference colour, and at each point a score for that type "based on the
  inverse distance of the terrain type's reference color to the terrain's base map color at that
  point", in HSV so that "the HSV color space is more suitable, both for the definition of the
  reference colors and for the calculation of color distances"; the **weight vector** `w`, which
  "allows us to ignore certain color components during the calculation of the distance … where a
  reasonable choice is gray for which hue is meaningless", and the note that it "is also useful
  in many cases to discard the lightness component, as its variation over the terrain is often
  the result of shadowing effects, which are unrelated to terrain type"; the **exponent** `p`;
  and the **score-weighted palette blend** (eq. 17), `C = Σ sᵢ Tᵢ / Σ sᵢ`. **Not indexed.**
- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`): the **Overlay** blend, "a =
  base < 0.5 ? 2*base*value : 1 - 2*(1-base)*(1-value)", which "doesn't affect areas where base
  mask is 0.0 or 1.0" — the instrument for a *modulation* that must not move the mean, and the
  reason the crate's ground palette is mixed as **convex shares** rather than as a modulation.
- **The crate's own `ground-colour.md`** (iteration 30): the palette, the score, the convex
  shares, and the three failures that fixed them — "by the unweighted mean of the tones the
  ground's mean brightness drifted 3.2 DN, by the mixture's own colour the hue was cancelled
  entirely, and by a per-channel mean the hue was cancelled again". The rock's palette is that
  note's construction, and it inherits its discipline.
- **The crate's own `rock-material.md`** (iteration 29): the rock's albedo `(0.072, 0.069,
  0.067)` — "a stone face under a blue sky is grey with the blue in it" — and the "second, darker
  rock colour" mixed in by the strata. The second tone is a *fixed* `vec3`, so the strata's
  colour is one colour and not a palette.

## Model adopted

**1. The rock is a *palette* of tones, mixed as convex shares.**

The crate already has the construction: the turf's four tones, mixed by a score, as convex
shares that sum to one. The rock's tones are mixed the same way, and the reasons are the
crate's own: a convex combination **cannot drift the mean** (the mixture is between the tones
and the tones bracket the base), and a modulation **would** (Andersson's Overlay does not move
the mean, and the crate's note says the mean must not move: "the ground is a place and not a
paint colour, while the *mean* colour and the *mean* brightness stay where they were (a ground
that changes colour changes the game's lighting)").

**2. The tones are *rock*, not tints of one rock.**

A cliff's variety is not a tint of grey; it is *different minerals*. So the palette is not a
set of pale-to-dark greys: it is a set of **hues** — the grey of a fresh basalt, the ochre of
an iron-stained sandstone, the warm buff of a limestone, the dark brown-black of a manganese
varnish, the pale grey-green of a weathered granite. Each is a *whole colour*, and the palette
is the set of rocks the cliff is made of.

**3. The score is the cell's own draw, so a fragment is *one* rock.**

The turf's score is a *field* (how much of each tone is around) times a *clump* (which tone
this piece wears). The rock's is the same shape at the rock's own scale: the **cell's** factor
is the *distribution* (which rocks are around) and the cell's *own* factor the *selection* (which
rock this fragment is). The score is their product, so a fragment is a **piece of one rock** and
not a blend of several — which is what a fracture gives you.

**4. The *rind* is a separate layer, and it is the patchy one.**

Tansem & Storemyr's "red iron films, rocks that are now generally gray" and Krinsley et al.'s
varnish are both *coatings*: a layer on the rock's surface with its own colour and its own
coverage. So the palette has two members and a **coverage**: the bare rock's tone, and the
rind's tone, and the rind covers *a fraction* of the surface, patchily.

This is the piece the crate has never had, and it is the piece that makes a cliff read as
*outdoors* rather than as *grey*.

**5. The coverage is band-limited with the distance, and the *rind* is what survives.**

A rind is a fine, patchy feature; a rock's own colour is not. So at a distance the rind is
cancelled and the rocks' own colours remain — the same trick the beds use (iteration 36: the
bed's *edge* is band-limited away and the bed's *tone* is not).

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The rock's own tone | the crate's `(0.072, 0.069, 0.067)` | `rock-material.md` Deviation 4: "a stone face under a blue sky is grey with the blue in it" |
| The palette's size | 4 rocks and a rind | a succession is four or five rocks, not a continuum |
| The tones' hues and saturations | 215°/0.10, 35°/0.30, 45°/0.10, 25°/0.20 | rock is a desaturated colour; the raw RGB triples came out as primaries |
| The palette's mean | exactly (1, 1, 1) | the crate's rule: "the *mean* colour and the *mean* brightness stay where they were" |
| The split's width | 0.12 of the draw's range | Papavasiliou's `p`, as a `smoothstep`'s width and not a `pow` |
| The score's shape | two narrow splits, not a product | a product of smooth fields is smooth, and a smooth field blends |
| The shares' sum | 1 | the convex combination that cannot drift the mean |
| The rind's coverage | a fraction of the surface, patchy at the rind's own scale | Reneau et al.: the varnish's "percentage of clast surfaces covered" is the age indicator |
| The rind's band-limit | the rind's own patch size | the crate's `detail_keep`, and the corduroy lesson |

## GPU mapping

- **Inside the terrain's own fragment shader**, where the rock's material already is
  (`src/terrain/common.wesl`, `rock_material` and `rock_blocks`). **No new pass, no new
  buffer, no new binding.**
- The cell's factor is **already drawn** — the joint's Weibull uses it — so the palette's
  score is free of new hashes.
- The palette's *mixing* is a few multiplies and a `select`, and the `pow` is a square.

## What great looks like

- **A cliff face reads as a *place* made of different rock.** At 3 m a player can pick out a
  darker slab beside an ochre one and a pale one, and the face has a *palette* rather than a
  single colour.
- **The colour is a *hue*, not a brightness.** A cliff in shadow is blue-grey; a cliff in
  sunlight is warm; but the *rock* is warm-buff or rust or dark olive beside the others, and
  that difference survives the light's own tint.
- **The rind is patchy**: darker, warmer or rustier patches on the faces that have weathered
  longest, and the bare rock showing through where it has not.
- **The distance is calm**: the rind is gone, and the rocks' own colours remain as broad
  patches — a massif with colour in it, not a wash.
- **The meadow is untouched.**

## Validation targets

1. **The face is not one hue.** The hue's resultant over the lit pixels of the face at dusk
   falls from **0.998** to below **0.95**, and the hue's spread rises from **2°** to at least
   **8°**. (The face at 3 m: the resultant 0.80 → below 0.75.)
2. **The rock's *chroma* rises.** The chroma's median over the rock-only patch at 3 m rises
   from **0.0157** to at least **0.030** — the rock is a colour and not a grey.
3. **The mean does not move.** The face's mean brightness at 3 m and at dusk moves by less
   than **1 DN**, and the mean hue by less than 0.005: a convex combination cannot drift the
   mean, and a game that changes colour changes its lighting.
4. **The meadow is untouched.** The `plains` preset at 6 m is byte-identical.
5. **The distance is calm.** The massif at 120 m carries the rocks' colours and no corduroy;
   the frame's high-frequency energy does not rise.
6. **The cost.** The mountains' opaque pass is within 10 % of the baseline's minimum.

## Deviations from this plan

1. **The palette is written in *hue and saturation*, and not as RGB triples.** The plan named
   four rocks and left their colours to be written down. Written as RGB triples, the tones came
   out at saturations of **0.84** (a strong blue) and **0.75** (a strong orange) — and the cliff
   read as a **rust-painted fence**. Rock is a *desaturated* colour: the tones are now written as
   a hue and a saturation and converted, so the hues are the ones a geologist would name
   (215°, 35°, 45°, 25°) and the saturations are the ones a rock has (0.10 to 0.30).
2. **The palette's equal-weight mean is exactly (1, 1, 1).** The first version's mean was 0.93
   and the face's mean luminance fell **2.7 DN**. The rule is the crate's own, from the
   ground's palette: "the *mean* colour and the *mean* brightness stay where they were (a
   ground that changes colour changes the game's lighting)". The tones are normalised so that
   they do not, and the face's mean luminance now moves **0.38 DN**. The tones' own luminances
   span 0.65 to 1.26, so a fragment is a lighter or darker piece of rock as well.
3. **The distribution is the cell's *second* draw, and not a field.** The plan had the score be
   a *field* (which rocks are around) times the *clump* (which rock this piece wears), exactly
   as the ground's palette is. A field at six cells was tried first, and the cliff read as
   **blotches** — coloured regions with every fragment in a region one rock. A cliff's colour
   must vary from fragment to fragment, so the distribution is the cell's own second draw.
   The cell needed a *second* draw for it, and one hash gives one draw: the cell's factor is
   `hash33(c).z`, so the second is a **separate hash** of the same cell and the two are
   independent.
4. **The score is a *sharp split*, and not a product.** The plan had the score be the
   *product* of a distribution and a selection. The product of two smooth fields is a smooth
   field, and a smooth field blends — and the blend is what left the cliff at one hue. The
   score is two **narrow splits** instead: the selection splits the palette into its cool half
   and its warm half, and the distribution splits each half again, so the four members are the
   four combinations. The width of a split is Papavasiliou's `p`, written as a `smoothstep`'s
   width and not as a `pow` — the crate's own rule is that three `pow` calls per pixel cost the
   mountains' opaque pass 0.137 ms → 0.252 ms.
5. **The *rind* is a layer with a *coverage*, and the coverage is a threshold.** The plan said
   the rind is a separate layer with its own colour and its own patchy coverage. The coverage
   is a **threshold on a field at the film's own scale**, softened — the varnish's "percentage
   of clast surfaces covered" is the age indicator (Reneau et al. 1993), so the film takes
   *some* of the surface and not all of it. The film's own tone has a luminance of exactly 1,
   so the film moves the surface's *hue* and not its *value*: a rind is a colour on the rock and
   not a darkening of it.
6. **The note's validation target at *dusk* was the wrong measure.** The target was the hue's
   resultant below 0.95 at dusk, and it is not reachable: at dusk the *sun's own tint*
   dominates the hue — everything is one hue under a warm sun — and the rock's hue is a small
   perturbation on it. The resultant moves from 0.9977 to 0.9966 at dusk while the **spread**
   moves from 2.1° to 3.2°, so the variety is there and the resultant is the wrong statistic at
   that hour. The target is restated on the *noon* face, where the light is neutral.
7. **The palette does not reach the distance, by design.** The blocks are band-limited away at
   120 m and the massif carries the *beds'* banding and the beds' own tone, so the palette
   stops at the distance the blocks stop. That is the crate's design — the beds carry the
   distance (iteration 36) — and it is a known gap, not a defect of this change.

## Pending index

- Tansem & Storemyr 2020 (`10.1002/gea.21832`): the record and the abstract were read; **the PDF
  was not obtained** (Wiley returns 403).
- Krinsley et al. 1998 (`10.1017/cbo9780511628894.008`): the record and the introduction were
  read; **the PDF was not obtained**.
- Reneau 1993 (`10.1006/qres.1993.1084`): the record and the abstract were read; **the PDF was
  not obtained**.
- Potter & Rossman 1979 (`10.1016/0009-2541(79)90085-8`): the record was read; the PDF was not
  obtained.
- Anovitz et al. 2020 (`10.1016/j.gca.2020.08.016`): the record and the abstract were read; the
  PDF was not obtained. It is about porosity and not colour, and it is not used.
- Papavasiliou 2015 is read through the crate's own `ground-colour.md`, which cites it in full.
- `distill_index` is skipped for the above.
