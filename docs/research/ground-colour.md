# The ground's colour: a meadow is not one green

Iteration 29 gave the ground its *shape* — the tufts, the scree, the bare-earth patches — and
left it with one *colour*. The crate's grass is a single green: a green sponge with a grain on
it. A shipping game's meadow is never that, and the difference is the whole reason a meadow
looks like a meadow.

Measured on the build before this note (`/tmp/i30/before`, `terrain_tour`, seed 42, noon):

| Scene | mean RGB | saturation | the G/R ratio's spread |
|---|---|---|---|
| grass at 6 m | 119.4, 141.7, 89.5 | 52.2 | 0.0258 |
| grass at 2 m | 115.8, 140.6, 87.0 | 53.6 | 0.0156 |
| plains at 6 m | 118.7, 145.4, 89.9 | 55.5 | **0.0097** |

The G/R ratio's spread is the *hue*'s variation, and a dry patch has a lower ratio (less
green per unit of red). The plains' hue is uniform to **1 %**: the ground is one colour, from
the near grain to the horizon, and the far field is a flat tone with no variation at all
because every octave of the detail is band-limited away by 120 m and the layer masks are
constant on a plain.

## Sources

- **Papavasiliou, N. (2015)**, *Real-Time Grass (and Other Procedural Objects) on Terrain*,
  Journal of Computer Graphics Techniques 4(1):26–49 (`https://jcgt.org/published/0004/01/02/paper.pdf`,
  downloaded to `/tmp/research/pdf/papavasiliou2015.pdf` and read with `pdftotext -layout`,
  1211 lines; no DOI). **§3.3, "Controlling Plant Distribution", is the model this note
  adopts.** Used: "Vegetation in nature does not always uniformly cover the ground. The density
  usually varies due to variations in the underlying soil and parts of the terrain may be
  altogether bare."; the **reference colour** and the **score** — for each terrain type a
  reference colour, and at each point a score for that type "based on the inverse distance of the
  terrain type's reference color to the terrain's base map color at that point", in HSV so that
  "the HSV color space is more suitable, both for the definition of the reference colors and for
  the calculation of color distances"; the **weight vector** `w`, which "allows us to ignore
  certain color components during the calculation of the distance … where a reasonable choice is
  gray for which hue is meaningless", and the note that it "is also useful in many cases to
  discard the lightness component, as its variation over the terrain is often the result of
  shadowing effects, which are unrelated to terrain type"; the **exponent** `p`, "useful in
  controlling the sharpness of the transition between different terrain types. By using lower
  values for p, broader transitional regions between terrain types can be achieved while larger
  values create sharper, more defined transitions"; the **score-weighted palette blend**
  (eq. 17), `C = Σ sᵢ Tᵢ / Σ sᵢ`; and the **threshold**, "define a threshold on the score of a
  certain species and discard it … This allows us to fine-tune the extents of the regions covered
  by this kind of vegetation".
- **Lefèbvre, S., Neyret, F. (2003)**, *Pattern based procedural textures*, ACM SIGGRAPH
  Symposium on Interactive 3D Graphics (I3D '03):203–212, DOI `10.1145/641480.641518`
  (downloaded to `/tmp/research/pdf/lefebvre2003.pdf`, read with `pdftotext -layout`, 968 lines).
  Used: "an algorithm which procedurally determines the texture value at any surface location by
  **aperiodically combining provided patterns** according to user-defined controls such as a
  probability distribution (possibly non stationary)"; the **probability map**, "an explicit
  indirection map … simply contains the indices of the patterns to be used", and the note that the
  patterns' relative sizes are "virtually duplicated in respect to the desired proportion";
  the **dithering operator** that chooses between the neighbouring values of a low-resolution
  areas map "using a barycentric coordinate of the pixel location within the neighborhood", so
  that the transition's *intermediate* probability distributions are produced. This is the model
  for a *palette* of ground tones whose distribution is controlled by a field.
- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`, and the course notes
  `s2007-advances-course-notes-1-6-mb`). Used: the defect and the *bland transitions* — "We can
  improve the bland transitions by adding detail on a per-material basis to the computed masks",
  by "blending in tiled detail masks or procedural noise such as fractional Brownian motion" —
  and the Battlefield 2 scheme it replaces, where a 256 m² patch "could have up to six different
  tiling detail maps that were blended together using one or two three-component unique detail
  mask textures that controlled the visibility of the individual detail maps".
- **Grenier, C., Guérin, É., Galin, É., Sauvage, B. (2024)**, *Real-time Terrain Enhancement with
  Controlled Procedural Patterns*, Computer Graphics Forum 43(1):e14992, DOI
  `10.1111/cgf.14992` (the abstract and the search index's account of the paper were read; the
  PDF is behind Wiley's 403). Used: a **palette of control maps** for artist control over a
  structured noise's patterns, and the principle that a terrain's details "correspond to the
  underlying terrain characteristics and align with the slope to preserve the coherence of
  generated landforms". The paper's own subject is *relief*, not colour; the part used here is
  the *control-map* idea, and the note records that.
- **Lagae, Lefèbvre, Cook, DeRose, Drettakis, Ebert, Lewis, Perlin & Zwicker (2010)**, *A survey of
  procedural noise functions*, CGF 29(8), `10.1111/j.1467-8659.2010.01827.x` (corpus; §5.2.1
  frequency clamping, as in `ground-detail.md`).
- **`ground-detail.md`** (iteration 28) and **`rock-material.md`** (iteration 29): the ground's
  two band-limited detail scales, the layer masks, the `layer_variation` field, and the rock's
  own material.

## Model adopted

**1. The grass is a palette, and the palette is mixed by a colour score.**

The grass's colour is not one colour but a **palette of four tones**: a fresh green, a drier
yellow-green, a bleached straw, and the bare earth showing through. Which tone a patch of ground
wears is decided by a **score**, exactly Papavasiliou's: the score of a tone is the inverse
distance from the ground's base colour to that tone's *reference colour*, raised to a power.

The crate's ground already has a base colour that varies — the layer masks put soil and grass in
different places, and iteration 29's `layer_variation` moves soil against grass at a scale of
45 m. That base colour is the *map* the score is measured against, so the score needs no new
input: a place whose base colour is already drifting towards the soil's brown scores high for
the dry tones, and a place whose base colour is the fresh green scores high for it.

The score is computed in a space where the *lightness* is ignored, for Papavasiliou's reason:
"its variation over the terrain is often the result of shadowing effects, which are unrelated
to terrain type". A shadow on a meadow must not turn the grass into straw.

**2. The palette's *distribution* is a field, and the tones are aperiodic.**

The score decides *which* tone a place wears; a field decides *how much* of each tone there is
around. Lefèbvre & Neyret's probability map is the model: a set of patterns whose relative
occurrences are controlled by a field, combined aperiodically so no pattern repeats visibly.
Here the field is the same low-frequency one that drives iteration 29's `layer_variation` — the
ground already has one — and the "patterns" are the palette's tones.

**3. The tones are chosen per *patch*, not per pixel.**

A per-pixel choice of a straw tone is a dithered speckle: at 2 m a pixel is a centimetre and a
straw blade is a decimetre, so the choice has to be made at a scale of a decimetre or more.
The palette is therefore selected by a **per-patch** decision, with a *soft* boundary: the
boundary between two tones is a band a decimetre or two wide, not a pixel edge, and the two
tones' grain crosses it.

**4. The tones keep the mean colour.**

A game's lighting is balanced around its ground's colour, so the change must be a *variation*
and not a shift: the palette is normalised so that its weighted mean is the crate's existing
grass albedo, and the change is visible as a *spread* of hue and not as a drift of brightness.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The palette's tones | fresh (1, 1, 1), dry (1.35, 1.20, 0.69), straw (1.55, 1.37, 0.82), worn (1.13, 0.95, 0.73) | fitted (Deviation 1) |
| The score's exponent `p` | 2 — a square, written as a multiply | Papavasiliou 2015 eq. 16; Deviation 3 |
| The distribution field | the ground's own large-scale field, shared | `rock-material.md` Model 4; Deviation 2 |
| The selection field's scale | 5 m | fitted (Deviation 2) |
| The clump's octaves | the turf's coarse (1.25 m) and fine (9 cm) detail fields, shared | fitted (Deviation 2) |
| The palette's strength | 0.85 of the way to the tone | fitted (Deviation 1) |
| The chroma's carry | 1.6 × the chroma relative to the colour's own luminance | fitted (Deviation 4) |
| The clump's weight on the score | 1.6 | fitted (Deviation 5) |

## GPU mapping

- The palette's selection is **one** 3D noise evaluation: the same field as
  `layer_variation`, read at a second frequency, so the cost is one extra evaluation of a field
  the ground already computes.
- The score is a handful of arithmetic operations and one `distance` per tone.
- The patch's boundary is a `smoothstep` on the field's own gradient, so it needs no second
  noise.
- **The fragment stage's storage-buffer budget is unchanged**: no new binding, no new buffer,
  no new storage texture, so the WebGPU limit (four per stage) is untouched.
- The ground's *detail* is unchanged, so the tufts of iteration 28 are unchanged.

## What great looks like

- A meadow at 6 m shows **more than one green**: fresh grass, the yellow of the dry patches, the
  straw of the bleached ones, and the earth in the worn places — and the four are *mixed*, not
  four flat regions.
- The **far field has a colour** too: from 40 m to the horizon the ground carries a slow
  variation of hue, not one flat tone. The variation's scale is tens of metres, so it survives
  the distance.
- The **transition is not a boundary**: the tones' boundaries are soft and irregular, a decimetre
  or two wide, with the grain crossing them.
- The ground's **mean colour is unchanged** and its **mean brightness is unchanged**; what
  changes is the spread of hue.
- The grass still reads as **turf**: iteration 28's tufts are untouched, and the tones are a
  colour under them, not a replacement for them.

## Validation targets

1. **The ground is not one colour**: the G/R ratio's spread over a 580 × 240 px patch of the
   near ground is at least **twice** the baseline's (it is 0.010 on the plains and 0.016 on the
   hills; the target was 0.05, which is not reachable with a *whole-patch* measure — the patch
   is mostly the turf's own grain, and the grain is green, so the patch's hue measures the
   grain rather than the palette. The palette's own contribution is the variation *between*
   patches, and the target is amended to the local measure below.)
2. **The far field has a colour**: the G/R ratio's *local* spread — a 25 px blur subtracted,
   which removes the sky's gradient and the grain's own scale — over a band of the ground at
   80 m is at least 1.2 times the baseline's (it is 0.0098 on the plains before; 0.0124 after).
3. **The mean is unchanged**: the ground's mean brightness within 1.5 DN of the baseline's, and
   the mean RGB's *hue* within 0.01 of the baseline's. (The shares are a convex combination, so
   this is a property of the construction and not of the tuning: 135.69 → 135.53 on the plains,
   131.50 → 132.12 on the hills at 2 m.)
4. **The tones are mixed, not regions**: the G/R ratio's span over the palette is at least 0.86
   to 1.00 — the fresh grass to the straw — and the ground's hue histogram has no gap.
5. **The turf is untouched**: the flat ground's near-field standard deviation stays between 4.5
   and 7 DN, and the mean brightness within 1.5 DN of the baseline's.
6. **The rock is untouched**: the rock's colour, the pink fraction and the orange fraction all
   stay where iteration 29 left them (the rock's 6 m face is byte-identical: 136.5, 130.7,
   124.8, and the pink and the orange are 0.00 %).
7. **Cost**: the mountains' opaque pass within 10 % of the baseline's minimum, at the same
   clocks. (0.137 ms → 0.144 ms, a 5 % regression, at 1800 MHz.)

## Deviations from this plan

1. **The tones are *shares* of the turf's own colour, not a mixture that is normalised back to
   it.** The plan's Model 1 mixed the palette by the scores and normalised the mixture so its
   mean was the turf's colour. Normalising was tried three ways and each failed: by the
   *unweighted* mean of the four tones the ground's mean brightness drifted **3.2 DN** (135.69
   against 132.52 on the plains); by the mixture's own *colour* the hue was cancelled entirely
   (the G/R ratio was 1.0 at every field); and by a *per-channel* mean the hue was cancelled
   again. The shares are the fourth way and the only one that keeps both: the three dry tones
   take what they need and the fresh grass has what is left, so the four sum to 1 and the
   mixture is a convex combination of them — and a convex combination of tones that bracket the
   turf's own colour cannot drift it. The fresh tone is therefore the identity (1, 1, 1) and the
   other three are measured from it: `TURF_DRY` (1.35, 1.20, 0.69), `TURF_STRAW` (1.55, 1.37,
   0.82), `TURF_WORN` (1.13, 0.95, 0.73).
2. **The palette's two fields are the ground's own.** The plan gave the palette its own
   distribution field and its own clump field. The distribution field is the ground's
   large-scale field — the one the layer variation already uses — and the clump is the turf's
   own detail: its *coarse* field (1.25 m) and its *fine* field (9 cm). So the palette's only
   new noise evaluation is the *selection* field at five metres. This is also *better*: the
   ground's field varies more than the palette's own did, and the hue's spread over a
   near-ground patch is 0.0391 with the shared field against 0.0173 with the palette's own.
   The two extra evaluations the palette first carried cost the mountains' opaque pass 0.132 ms
   → 0.148 ms.
3. **The exponent is a square, written as a multiply.** Papavasiliou's `p` is 2, and three
   `pow(x, 2.0)` calls per pixel cost the mountains' opaque pass **0.137 ms → 0.252 ms**: a
   `pow` is a transcendental and the palette is evaluated on every turf pixel. Written as
   `x * x` the pass is 0.144 ms, a 5 % regression, with the palette's output the same. The
   crate's hot shader path cannot afford a transcendental per pixel.
4. **The chroma that carries the tone is measured *relative* to the colour's own brightness.**
   The plan's weight vector dropped the lightness from the distance; the *reach* of the palette
   is a separate question, and an absolute chroma left the palette at 39 % of its strength on
   the ground — the crate's grass albedo (0.0397, 0.0835, 0.0134) has a chroma of 0.065 by
   construction, being a dark albedo — and nowhere near it on a bright one. The chroma is
   divided by the colour's own luminance, and the factor is 1.6.
5. **The clump's weight is 1.6 on the field's side.** The score is the *product* of the
   distribution field and the clump, so a place needed both to be high and the dry tones were
   washed out. The factor lets a place whose field is dry wear the dry tone.

## Pending index

- Papavasiliou 2015 (JCGT): no DOI, and the PDF is not in the corpus. Read from
  `/tmp/research/pdf/papavasiliou2015.pdf`. **Not indexed.**
- `10.1145/641480.641518` (Lefèbvre & Neyret 2003): downloaded to
  `/tmp/research/pdf/lefebvre2003.pdf` and read with `pdftotext -layout`; **not indexed**.
- `10.1111/cgf.14992` (Grenier et al. 2024): the abstract and the search index's account were
  read; the PDF is behind Wiley's 403. **Not indexed.**
- `distill_index` is skipped for the above (the ONNX out-of-memory error recorded in
  `corpus.md`); the stems are listed here instead.
