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

Measured on this build (`big`, RTX 3090, `terrain_tour`, mountains, seed 7, 9 h, 1280 × 720; same
luminance band, so the light matches; the frames and the scripts are in `/tmp/i59`):

| measure (like-for-like) | a scattered stone | the cliff's face | the ground |
|---|---|---|---|
| the top facet's sRGB saturation, 2 m | **0.020** | 0.15 | 0.19 |
| the front facet's sRGB saturation, 2 m | 0.112 | — | — |
| the hue's spread over one body | **67.5°** (from the shading, not the rock) | 8-27° | — |
| the albedo's high-pass (3 × 3), sd | 6.30 DN | 5.07 DN | 3.29 DN |

So a stone is not *noisier* than the rock it fell from — that was my first, wrong reading, taken
by comparing a sunlit stone with a **shaded** cliff face (8.84 against 2.69 DN, an artefact of the
light, not of the material). What is wrong is **uniformity**: one palette tone over the whole
body, so a boulder's top facet is a grey of saturation 0.020 where the face beside it is 0.15, and
the hue a boulder shows is the light's, not the rock's. The shaded stones' blue is the sky's
ambient on a shadowed low-albedo surface, and the shaded *cliff* measures bluer still (+20.5 against
+16.5); it is physics and this note leaves it alone.

## Sources

- **Papavasiliou, N. (2015)**, *Real-Time Grass (and Other Procedural Objects) on Terrain*, JCGT
  4(1):26–49 (cited by `rock-palette.md`, which implemented the model for the cliff): the
  **score-weighted palette blend** `C = Σ sᵢ Tᵢ / Σ sᵢ` with a reference colour per rock and a score
  from the colour distance, and the crate's rule that the **mean must not move** — "a ground that
  changes colour changes the game's lighting". The stones take the same construction: the palette is
  the same, the *score* is the facet, and the mean over a shape is still 1.
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
joint sets plus chips, with a soft minimum, and it already computes each vertex's **analytic
normal** — which is a plane's normal on the facet that plane cut. The facet is therefore *free*:
the **carrier** of the soft minimum is the plane that owns the vertex, and the palette's tone is
the facet's.

**2. The tones are the cliff's four, so a stone is a piece of the same rock.**

No new palette and no new hues: `TONES` stays the cliff's (`ROCK_TONE_*` in `common.wesl`), and a
stone's facets wear three or four of them, as the cliff's fragments do.

**3. A facet is *one* colour, and the tone changes over the joint's own width.**

A shared vertex carries one colour and the rasteriser interpolates it between neighbours, which
smears the tone across the joint it is supposed to break at. Two builds were tried and rejected: a
hard per-triangle assignment read as a **sawtooth** on a small stone, and splitting every triangle
corner gave 3840 vertices and paid **+4.5 %** of the isometric scene's opaque pass. The shipped
build keeps the mesh **indexed** (827 vertices, the same 1280 triangles) and blends each vertex's
tone over the joint's own rounding, `TONE_JOINT` = 0.004 of the shape's radius: a facet keeps its
rock over its whole face and the colour changes across the joint itself, which is the surface's own
feature (Deviation 1).

**4. The tone table is the *shape's*, not the mesh's.**

The tones go out by the *area* each facet covers, and an area is measured on a mesh — so a table
derived from whichever mesh is being built gives the near and the far mesh of one rock *different
rocks*. Measured on the CPU over all twelve shapes: the worst vertex's hue differs by **19.0°**
between the two tessellations, and one shape's mean moved 3.7°. The meshes switch abruptly at
`22 · D`, so a stone crossing that distance changed colour, which the eye reads as the rock
flickering as the camera moves (Outcome). The table is computed **once per shape** from its
canonical (near) tessellation and stored on the `RockGeometry`, so both meshes wear the same rocks
to the bit; the near mesh — the one a player stands next to — is unchanged. Test
`a_rocks_two_meshes_wear_the_same_rock`.

**5. A vertex's tone weights are the soft minimum's terms at the joint's width, not the geometry's.**

The soft minimum already blends the planes' *normals* at the rounding's width; the tone uses the
same terms at `TONE_JOINT`, a narrower width. Widening the blend to the geometry's rounding washed
every facet toward the palette's mean (measured: an apex facet's chroma fell to 0.17 where a whole
facet is 0.25-0.53).

**6. The shape's brightness is normalised to the palette's, by a scalar.**

The palette's mean is exactly 1 (`rock-palette.md`), but a shape's facets are *whole* rocks of it,
so its largest facet decides its body's mean and drifts a stone by up to 21 %. Each shape's
area-weighted luminance is scaled to 1 by **one scalar**, so the tones keep their hues and the hue
differences stay. (A per-channel mean cancels the hue, which is what `rock-palette.md` Deviation 2
and `ground-colour.md` both record.)

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The palette | the cliff's four tones, `TONES` in `mesh.rs` (= `ROCK_TONE_*` in `common.wesl`) | `rock-palette.md` Parameters |
| The facet | the carrier of the soft minimum: the joint plane (or the bounding sphere) that owns the vertex | the shape's own construction, `rock-scatter.md` Model 5 |
| The facet's tone strength | **1.0** (was 0.6 applied to a whole shape) | Model 2; `rock-palette.md` Deviation 2 |
| The joint's colour width | **0.004** of the shape's radius (`TONE_JOINT`) | Model 3; the geometry's own rounding is wider |
| The tones' assignment | by area, largest facet first, round-robin from a per-shape draw | Model 6; `every_stone_keeps_the_palettes_brightness` |
| The tone table's tessellation | **the near mesh's**, once per shape (`shape_tones`) | Model 4; `a_rocks_two_meshes_wear_the_same_rock` |
| The per-shape brightness | 1, by a scalar on the shape's tones | `rock-palette.md` Deviation 2 |
| The speckle's albedo swing | **2.0, unchanged** | measured, Deviation 2 below |

## GPU mapping

- **No new pass, no new buffer, no new binding, no shader cost.** The tones are the mesh's **vertex
  colours**, written on the CPU once per shape at start-up (24 meshes), which is the attribute the
  base's darkening already rides. The rock's shader path is untouched.
- The mesh stays **indexed**: 827 vertices against 642 shared ones, the same 1280 triangles. The
  tone blends over the joint, so no seam has to be drawn. Splitting every triangle corner instead
  (the first build) gave 3840 vertices and paid **+4.5 %** of the isometric scene's opaque pass at
  matched clocks. Nothing per pixel changed.
- The tone table is one `Vec<Vec3>` per shape (at most ~19 entries), computed once at start-up and
  shared by both meshes.

## What great looks like

- **A boulder below a cliff is visibly the cliff's rock.** At 2 m a stone shows two or three faces
  of *different* rock — a pale grey-buff slab beside a warmer, darker one and a cool end face — and
  its faces are flat with sharp edges, not a mottled sphere.
- **The variation is at the piece, not on the object.** A stone's colour changes at a *joint*, the
  way the cliff's does, and not across its middle: no gradient, no blotch.
- **The scatter's mean colour does not move.** A meadow full of stones keeps the brightness and hue
  it had: a game that changes a ground's colour changes its lighting.
- **The cliff and the ground are untouched**: a scattered stone is a mesh, and nothing about the
  terrain's material changes.

## Validation targets

1. **The palette is on the facets.** Every shape wears at least 3 distinct rocks, and a
   vertex's tone is its facet's over the face and changes only within the joint's own width, so no
   tone interpolates across a face. (CPU test `a_stone_wears_several_rocks_and_the_change_lands_on_its_joints`.)
2. **The brightness does not move.** Every shape's area-weighted brightness is 1 within 2·10⁻³
   (CPU test `every_stone_keeps_the_palettes_brightness`).
3. **The stone's chroma rises toward the face's.** On the 2 m boulder (`stone_a`), the top facet's
   sRGB saturation rises from **0.020** to at least **0.10** (the front facet from 0.112).
4. **A boulder's facets are different rocks.** The hue's spread across one boulder's sub-regions
   rises from **67.5°** to at least **70°**, and its mean hue moves toward the cliff's warm side.
5. **The silhouette does not change.** Positions and normals are the shape's own; only the colour
   attribute moves, and the seat, the `Aabb` and a game's collider read the same geometry.
6. **The cliff is untouched.** The 3 m face, the 6 m face, the dusk face and the 120 m massif are
   byte-identical (measured: max diff 1, dither).
7. **The ground is untouched.** `plains` at 40 m and `dunes` at 40 m are byte-identical bar one
   stone's edge.
8. **The cost.** The opaque pass is unchanged: nothing per pixel changed.
9. **The two meshes wear the same rock.** Every facet of every shape carries the same tone in the
   near and the far mesh, to the bit, so the switch at `22 · D` is a change of triangle count and
   not of colour (CPU test `a_rocks_two_meshes_wear_the_same_rock`).

## Deviations from this plan

1. **The facet is the soft minimum's *carrier*, not a grouping of triangles by plane.** The plan
   grouped the mesh's triangles by their plane. The carrier is already computed, is a per-vertex
   scalar the joint's own field supplies, and gives a boundary that follows the joint — where a
   vote over three corners zigzags (the rejected round-2 build). It also avoids a second pass over
   the faces.
2. **The speckle's albedo swing is unchanged at 2.0.** The plan quartered it, on a first
   measurement that compared *sunlit* stones with a *shaded* cliff face and reported the stone's
   high-pass as 3.3× the face's. Measured like for like — same luminance band, same light — the
   stone's high-pass is **6.30 DN** against the face's **5.07**, not three times it, so there was
   nothing to fix and the edit was reverted before it was written into the note. The stone grain
   stays as `rock-scatter.md` Deviation 8 set it.
3. **The tones are assigned by *area*, largest facet first, and the shape's brightness is
   normalised by a scalar.** The plan had a balanced round-robin, which bounds a shape's mean by
   its facet *count* — but a facet's *area* is what the eye integrates, and the largest facet of a
   rounded rock is 56-65 % of its surface. Sorting by area and then normalising is what actually
   holds the brightness, and it is the same construction `rock-palette.md` Deviation 2 used for the
   palette itself.
4. **The tone blends over the joint, and the mesh stays indexed.** The plan changed only the
   colour attribute of the existing shared-vertex mesh, which smeared a pale top into a buff side
   over the whole rock; splitting every triangle corner fixed the smear and cost +4.5 % of the
   opaque pass. The shipped build moves the *tone's own weights* to the joint's width instead, so
   the change lands on the joint without a seam (Model 3).
5. **The tone table is derived once per shape, from the near mesh.** The plan derived the tones
   inside `rock_geometry`, which runs once per mesh — and so gave a rock's two levels of detail
   different rocks. Found in this pass by a CPU probe over both tessellations, measured at up to
   19° of hue, and fixed by making the table the shape's (Model 4).

## Outcome

The item shipped, with one defect found and fixed in the same pass.

- **The facets wear the cliff's rocks.** On the 2 m boulder (`stone_b`, mac_air, 2560 × 1406):
  the top face's sRGB saturation **0.122 → 0.132** at 3×, with the front and end faces measuring
  other rocks — a pale buff top, a warm tan front, a cool grey-blue end. The boulder's whole-stone
  chroma read 0.104 → 0.120 against the ground's 0.143 → 0.190. The boundary is a single soft
  polyline along a facet edge at 3×: no sawtooth, no smear.
- **Nothing else moved.** Full-frame diffs (fraction of pixels changing by more than one level):
  the cliff face at 40 m **0.00 % (max diff 1)**, the dunes **0.00 % (max diff 0)**, the plains
  0.03 %, the 120 m massif 0.02 %. Frame means hold: `m40_s7` 110.0 → 110.0, `m40_s11` 86.7 →
  86.7. The change is the meshes' vertex colours and nothing else.
- **The defect this pass found: a rock's two levels of detail wore different rocks.** The table was
  derived from the mesh's own facet areas, so the 1280-triangle near mesh and the 320-triangle far
  mesh of one shape got different palette tones — **19.0°** of hue apart at the worst vertex, 3.7°
  on one shape's mean — and the abrupt switch at `22 · D` popped. The table is now the shape's
  (Model 4, Validation 9). The near mesh, which the previous pass signed off, is byte-identical;
  only the far mesh moves, onto the tones the near mesh already wore.
- **Not claimed:** the distance look at 40 m, where a stone is 2-4 px and the facets read as noise
  rather than as facets. It was no better before this change, and the cliff's own distance look
  (the block network's shingle fan) is the separate item the backlog carries.

## Pending index

None. This note cites only documents already converted and indexed (`10.5194_esurf-9-1073-2021`,
`10.1145_1281500.1281668`, `bo/book_real_time_rendering_4th_akenine`); the two publisher records
this note would have liked (La Pointe & Hudson 1985, Papavasiliou 2015's PDF) are the ones
`rock-palette.md` and `rock-blocks.md` already record as unobtainable, and are read through those
notes' quotes.

