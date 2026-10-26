# A scattered stone is a piece of the cliff: the palette on its facets, not one tone on the whole rock

Iteration 57 made the scattered stones visible under an isometric camera, and its own backlog put
the stone's **colour** first: "the stones are a desaturated blue-grey against a green meadow". This
note is that item, measured before it is argued.

The scattered rocks are whole meshes (`src/rocks/mesh.rs`), and every vertex of one shape carries
**one** palette tone: `tone = ONE.lerp(TONES[shape % 4], 0.6)`. The palette is the cliff's
(`rock-palette.md`: basalt, sandstone, limestone, varnish, mean exactly 1), and the cliff mixes it
per **fragment** — the cell network's own draw decides which rock each piece of the face is
(`rock_blocks`). The mesh decided it per **rock**. So a boulder is 60 % of the way to *one* rock's
hue, whole, and the cliff it fell from is never one hue at that scale.

Measured on this build (`big`, RTX 3090, `terrain_tour`, mountains, seed 7, 9 h, 1280 × 720; the
frames and the scripts are listed under Evidence):

| measure | the cliff's face | a scattered stone |
|---|---|---|
| chroma, median (`max−min` of the linear triplet over `max`), 3 m | **0.150** | **0.020** |
| the albedo's high-pass (3 × 3), sd | 2.69 DN | **8.84 DN** |
| tones visible on one body | 2–4 per fragment | **1** |
| shaded side, `B − (R+G)/2` | +20.5 | +16.5 (both sky-lit; not a defect) |

So the stone is **greyer than the rock it fell from**, and *noisier* than it: the surface swings
twice as hard per pixel and still carries one colour. A 4× crop (`z10_stone_px.png`) reads as a
mottled grey potato. The measured blue of the *shaded* stones is the sky's ambient on a stone in
shadow, and the cliff's shadowed face measures the same — it is physics and this note leaves it
alone.

## Sources

- **Papavasiliou, N. (2015)**, *Real-Time Grass (and Other Procedural Objects) on Terrain*, JCGT
  4(1):26–49 (cited by `rock-palette.md`, which implemented the model for the cliff): the
  **score-weighted palette blend** `C = Σ sᵢ Tᵢ / Σ sᵢ` with a reference colour per rock and a score
  from the colour distance, and the crate's rule that the **mean must not move** — "a ground that
  changes colour changes the game's lighting". The stones take the same construction: the palette is
  the same, the *score* is the facet, and the mean over a shape is still exactly 1.
- **Akenine-Möller, T., Haines, M., Hoffman, N., Shirley, P. (2019)**, *Real-Time Rendering, 4th
  ed.* §6.3 (corpus `bo/book_real_time_rendering_4th_akenine`): "a cellular texture is formed by
  measuring distances from each location to a set of 'feature points' scattered through space …
  creates patterns that look like cells, **flagstones**, lizard skin, and other natural textures",
  and that its cells' colours are set per cell — a cellular partition is a *colouring* as much as a
  shape. The crate already has the partition on the cliff (`rock_block_scale`); the stone's
  partition is the **joint planes its shape was cut by**.
- **La Pointe, P. R., Hudson, J. A. (1985)**, *Characterization and interpretation of rock mass joint
  patterns*, SPE Formation Evaluation (cited by `rock-blocks.md`): a rock mass is *sets* of
  parallel planes, and its pieces are the **blocks between them** — tabular, aligned, and each a
  whole piece. A stone that is one piece of rock has *one* tone per piece only if it is one piece;
  a boulder is several pieces still held together, which is what a jointed outline shows.
- **Verdian, J. P. et al. (2021)**, *Sediment size on talus slopes correlates with fracture spacing
  on bedrock cliffs*, Earth Surface Dynamics 9:1073 (corpus `10.5194_esurf-9-1073-2021`): the talus's
  clasts are the cliff's **blocks**, cut by the cliff's fracture spacing — "the closest
  correspondence to the fracture spacing at most sites … weathering has not modified latent" the
  size distribution. A talus block is a piece of the cliff's rock, *including its fabric*, which is
  the whole premise of the visual link this note is after.
- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`): the **Overlay** blend, "a =
  base < 0.5 ? 2·base·value : 1 − 2·(1−base)(1−value)", which "doesn't affect areas where base mask
  is 0.0 or 1.0". Named because it is the instrument this note does **not** use: a modulation on the
  albedo moves the mean, and the crate's palette rule is that the mean does not move.
- **The crate's own `rock-palette.md`** (iteration 39): the palette's four tones, the equal-weight
  mean of exactly `(1, 1, 1)` (Deviation 2: an unnormalised palette dropped the face's mean luminance
  by 2.7 DN), the sharp split instead of a smooth product (Deviation 4: a field made the cliff
  "blotches — coloured regions with every fragment in a region one rock"), and the sentence this
  note is measured against: "a cliff's colour must vary from **fragment to fragment**".
- **The crate's own `rock-scatter.md`** (iterations 40–42): Model 5, the shapes are "a block cut by a
  few joint planes and bounded by a sphere", and its `What great looks like`: "The rocks are **the
  cliff's rock**: the same palette, the same grain; a boulder below a cliff is visibly a piece of
  it." Its Validation 7 asks that no rock be "a uniform colour" — the stones pass that test only
  across the scatter, not within one body, which is what this note fixes.
- **The crate's own `rock-material.md`** Deviation 8: the stone grain is "a **mottle** at 0.30 m and
  a **speckle** at an eighth of it", with "the speckle carrying the stronger albedo swing", because
  one fBm from the mottle's size down "read as clay". The measurements above say the speckle now
  carries *too much*: the stone's high-pass is 3.3× the face's.

## Model adopted

**1. A stone wears the palette per *facet*, and its facets are the joint planes it was cut by.**

The shape already has the partition: `rock_geometry` cuts an icosphere by three near-orthogonal
joint sets plus chips, with a soft minimum, and it already computes each vertex's **analytic normal**
— which is a plane's normal on the facet that plane cut. So the facet is *free*: the vertex's
strongest plane is its facet, and the palette's draw is `hash(shape, facet)`.

**2. The tones are the cliff's four, so a stone is a piece of the same rock.**

No new palette and no new hues: `TONES` stays the cliff's (`ROCK_TONE_*` in `common.wesl`), and a
stone's facets wear two to four of them, as the cliff's fragments do.

**3. The mean over a shape stays exactly 1, and so does the mean over the scatter.**

A per-facet draw would drift the mean by chance (one shape's facets could land on basalt twice). The
draw is therefore **balanced**: the shape's facets are assigned palette indices round-robin from a
seed per shape, so each shape's facets are as evenly spread over the four tones as its facet count
allows, and a test asserts the mean of every shape's tones is within 1 % of 1. The scatter's mean is
then the palette's mean, which is exactly 1 (`the_palette_is_the_rocks_mean`).

**4. The tone's strength is per *facet*, and it is the cliff's.**

`TONE_STRENGTH` was 0.6 because a whole rock at full tone "read as a painted blue-grey"
(`mesh.rs`). A facet is a fragment, and a fragment *is* one rock: it wears its tone at full
strength, and the shape does not, because its facets differ. So `TONE_STRENGTH` becomes 1 and the
"painted" risk is answered by the variety instead of by washing every tone out — which is exactly
the cliff's own construction.

**5. The speckle is a *mineral grain*, not static.**

The stone's high-pass is 8.84 DN against the cliff's 2.69. The speckle's albedo swing (2.0 per unit
of the field, at an 8th of the mottle's size) is what makes a stone read as noise rather than as
stone; it drops to a quarter of that, and the mottle — the feature a player reads at arm's length —
carries the surface. This is a *reduction*, not a new feature: it is the same path, tuned, with the
cliff's own high-pass as the target.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The palette | the cliff's four tones, `TONES` in `mesh.rs` (= `ROCK_TONE_*` in `common.wesl`) | `rock-palette.md` Parameters; this note Model 2 |
| The facet | the vertex's dominant analytic plane (the joint set or chip that cut it) | the shape's own construction, `rock-scatter.md` Model 5 |
| The facet's tone strength | **1.0** (was 0.6 per shape) | Model 4; `rock-palette.md` Deviation 2 for the mean |
| The tones' assignment | balanced round-robin per shape, seeded by the shape | Model 3; `the_palette_is_the_rocks_mean` |
| The per-shape mean | `(1, 1, 1)` within 1 % | `rock-palette.md`: the mean must not move |
| The speckle's albedo swing | **0.5** (was 2.0) | `rock-scatter.md` Deviation 8, measured against the cliff's high-pass |
| The mottle's albedo swing | 1.2, unchanged | `rock-scatter.md` Deviation 8 |
| The two tones' *blend* across a joint | none: a facet is one rock | `rock-palette.md` Deviation 4 (a blend makes blotches) |

## GPU mapping

- **No new pass, no new buffer, no new binding, no shader cost.** The tones are the mesh's **vertex
  colours**, written on the CPU once per shape at start-up (24 meshes) — the attribute the stone
  grain and the base's darkening already ride. The stone's GPU path is untouched.
- The speckle's swing is a constant in `weathered.wesl` (`STONE_SPECKLE_ALBEDO`): one edit, no new
  work per pixel.

## What great looks like

- **A boulder below a cliff is visibly the cliff's rock.** At 2 m a stone shows two or three faces of
  *different* rock — a darker slab beside a pale one — and its faces are flat with sharp edges, not
  a mottled sphere.
- **The variation is at the piece, not on the object.** A stone's colour changes at a *joint*, the
  way the cliff's does, and not across its middle: no gradient, no blotch, no gradient-with-noise.
- **The surface is stone, not static.** At arm's length the stone has a mottle the size of a hand and
  a grain you can see where the light rakes, and the frame's high-pass on a stone is the cliff's own
  order of magnitude, not three times it.
- **The scatter's mean colour does not move.** A meadow full of stones keeps the brightness and hue
  it had: a game that changes a ground's colour changes its lighting.
- **At 40 m and at dusk** the stones read as stones — a few degrees of hue apart from one another and
  from the ground — and never as blue pebbles on a green lawn.

## Validation targets

1. **The palette is on the facets.** Over the four families, the number of distinct tones within one
   stone's vertices is at least 2 for every shape with 3 or more facets, and 4 for the angular
   family. (CPU test.)
2. **The mean does not move.** Every shape's mean vertex tone is within 1 % of `(1, 1, 1)`, and the
   scatter's mean is the palette's (CPU test, replacing the whole-palette mean test).
3. **The stone's chroma rises toward the face's.** In the 2 m stone frame (`stone_a`), a stone
   face's median chroma rises from **0.020** to at least **0.08** (the cliff's face measures 0.150).
4. **The stone is not static.** A stone's high-pass (3 × 3, sd) falls from **8.84 DN** to at most
   **5 DN**, against the face's 2.69.
5. **The silhouette does not change.** The shape meshes' positions, normals and indices are
   identical to the baseline's (only the colour attribute moves), so the seat, the physics and the
   screenshots' silhouettes are untouched (CPU test / byte-identical geometry).
6. **The cliff is untouched.** The 3 m face frame and the 120 m massif are byte-identical.
7. **The ground is untouched.** `plains` at 40 m is byte-identical.
8. **The cost.** The opaque pass is within 1 % (a vertex-colour change and one constant).

## Deviations from this plan

1. **The facet is taken from the *analytic* normal, not from the triangles.** The plan was to group
   the mesh's triangles by their plane. The analytic normal is already the facet's plane on the
   facet's interior (it turns only inside the rounding's width at an edge), and it is what
   `rock_geometry` computes anyway, so no second pass over the faces is needed.
2. **The speckle's swing is quartered, and the mottle's is not.** The plan was to leave the grain
   alone and change only the colour. The measurement above (a stone's high-pass 3.3× the face's)
   says the grain is half of why a stone reads as a grey potato, and it is one constant on the same
   path this note already touches. The mottle, which is the feature the eye reads, is unchanged.
3. **The tones are assigned round-robin and not drawn independently.** A free draw drifts a shape's
   mean by up to 12 % (four facets, four tones), and the crate's rule is that the mean does not move.
   The round-robin from a per-shape seed keeps the *which-facet-gets-which* random while bounding the
   mean.

## Pending index

None. This note cites only documents already converted and indexed (`10.5194_esurf-9-1073-2021`,
`10.1145_1281500.1281668`, `bo/book_real_time_rendering_4th_akenine`); the two publisher records this
note would have liked (La Pointe & Hudson 1985, Papavasiliou 2015's PDF) are the ones
`rock-palette.md` and `rock-blocks.md` already record as unobtainable, and are read through those
notes' quotes.
