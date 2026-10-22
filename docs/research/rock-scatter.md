# Rocks on the ground: a scatter of boulders and stones, not a painted texture

The crate's ground is an empty carpet. Every stone in it is *painted*: the scree's stones are
domes in the terrain material's colour and normal (`scree.md`), the cliff's blocks are cells of
a network in its fragment shader (`rock-blocks.md`). Nothing on the ground has a **silhouette**
or casts a **shadow**, and those two are the strongest cues a ground has for scale and for
being solid. At eye level a meadow, a hillside and a talus apron read as a textured sheet
(`/tmp/i42/before/hills_eye.png`, `hills_ground.png`, `mtn_foot.png`).

This note adds the missing population: rocks as **meshes**, placed on the terrain by its own
slope and sediment, sized by the size–frequency law measured on real rock fields, shaped like
the blocks a cliff sheds, seated on the surface the GPU actually draws, and weathered by the
crate's own weather.

## Sources

- **Golombek, M. P., Trussell, A. R., Williams, N., Charalambous, C., Abarca, H., Warner, N. H.,
  Deahn, M., Trautman, M., Crocco, R., Grant, J. A., Hauber, E., Deen, R. (2021)**, *Rock
  size-frequency distributions at the InSight landing site, Mars*, Earth and Space Science 8,
  DOI `10.1029/2021EA001959` (the PDF from the Caltech authors repository,
  `authors.library.caltech.edu/records/d1t32-akr32`, read with `pdftotext -layout`, 1143
  lines). Used:
  - The model: "The exponential model equations are of the form: **Fk(D) = k exp[−q(k) D]**,
    where Fk (D) is the cumulative fractional area (CFA) covered by rocks of diameter D or
    larger, k is the fraction of the total area covered by all rocks, and an exponential q(k)
    that governs how abruptly the fraction of the total area covered by rocks decreases with
    increasing diameter (Golombek & Rapp 1997), which is approximated by **q(k) = 1.79 +
    0.152/k**. These distributions form a family of non-crossing curves that flatten out at
    small rock diameters."
  - Why not a power law: power laws "invariably overestimate the number (or area) covered by
    large and small rocks", and the exponential models "are generally similar to Rosin Rammler
    and Weibull distributions" — the crate's own Weibull for the cliff's block sizes
    (`block-sizes.md`) is the same family, so the rocks on the ground and the blocks in the
    cliff come from one law.
  - The physics behind it: "ubiquitous flaws or joints will lead to exponentially fewer blocks
    with increasing size during weathering and transport".
  - The abundances: "the cumulative fractional area (CFA) covered by rocks is **∼1–2%** away
    from craters with obvious rocks in their ejecta", rocky ejecta "fall on **4–5%** exponential
    model curves", and the model "is well represented by exponential model curves for CFAs of
    1–5%" — a plain and a rocky patch of one landscape, in numbers.
  - The height: rocks measured were taller "than the usual hemisphere of **one half the
    diameter**" — the usual assumption is a rock as tall as half its width.
  - Golombek & Rapp 1997 (JGR 102(E2), DOI `10.1029/96JE03319`), the paper that introduced the
    model and fitted it to Earth analogue sites, was **not obtained** (Wiley 403, no open copy);
    its model is used as the 2021 paper restates it.
- **Verdian, J. P., Sklar, L. S., Riebe, C. S., Moore, J. R. (2021)**, *Sediment size on talus
  slopes correlates with fracture spacing on bedrock cliffs*, Earth Surface Dynamics 9:1073,
  DOI `10.5194/esurf-9-1073-2021` (open access; read in full in iteration 34, `scree.md`).
  Used here for the **shape** of the pieces a cliff sheds: "talus produced from the
  metasediment has **b : a and c : a ratios of 0.57 and 0.26**, respectively, on average … the
  granodiorite, which has corresponding ratios of **0.64 and 0.35** … andesite particles are
  more slab-like than basalt on average, with a lower mean c : a ratio of 0.26 compared to
  0.36", and the theory it tests (Domokos et al. 2015): "c : a ratios should have universal
  values of **0.67 and 0.45**, respectively, for particles produced by mechanical weathering".
  So a fallen rock is a *slab or a block*, not a sphere: its short axis is a third to a half of
  its long one. And its sizes: "median b-axis diameters ranging from 10 to 575 mm", with
  a-axes "as large as 5450 mm".
- **Galin, E., Guérin, E., Peytavie, A., Cordonnier, G., Cani, M.-P., Benes, B., Gain, J.
  (2019)**, *A Review of Digital Terrain Modeling*, Computer Graphics Forum 38(2), DOI
  `10.1111/cgf.13657` (in the corpus, read). Used:
  - Where rocks lie: thermal erosion "correctly creates accretion cones of fallen rocks, sand,
    and granular materials in both screes and accretion areas", and "the talus angle varies
    within **30–45°** degrees interval for earth forms" — rocks rest below the angle of repose
    and not on the cliff above it.
  - Why the scatter matters: "fallen rocks do not only provide a fundamental visual improvement
    when represented at their rest state at the bottom of cliffs"; and the pipeline: "a bare
    earth terrain is usually the first part of the authoring pipeline to be subsequently
    augmented with props that represent rocks, trees, plants and buildings".
  - Their shape over time: spheroidal weathering, where "the local curvature and accessibility
    of the surface indicates its exposure to the environment and causes this to be eroded
    faster. **Positive curvature erodes edges and creases**" — an old rock on a meadow is a
    rounded block, a fresh one on a talus is angular.
- **van Muijden, J. (2017)**, *GPU-Based Procedural Placement in Horizon Zero Dawn*, GDC 2017,
  Guerrilla Games (slides PDF from `guerrilla-games.com/read/gpu-based-procedural-placement-in-horizon-zero-dawn`,
  read with `pdftotext`). How a shipping open world does it:
  - "We use procedural placement for all nature! 500+ asset types, 100.000+ objects in scene,
    **~250µs avg busy load on GPU**" — the budget a placement system can be held to.
  - The pipeline: a **density map** per layer evaluated from world data, then a **dither-based
    discretizing step** whose pattern follows "Even spread thresholds, Maximize 2D distance,
    Uniform 2D distance w, **Scale to w = footprint**", then per sample "1. Range Test 2.
    Threshold Test 3. Position generate 4. Normal construction". The crate's version of that is a
    jittered lattice whose cell is the size class's footprint, a threshold test against the
    local density, and a range test from the camera.
  - Their world data includes "**Variance_RockColor**" and "Variance_Lichen_Density" maps at
    1 m: the rocks' colour varies over the world, and is a placed quantity, not a constant.
- **The crate's own notes**: `scree.md` (the scree's band, 16°–40° peaking at 26°, which the
  talus population follows so the painted stones and the meshes agree), `rock-palette.md` (the
  four tones in hue and saturation, mean exactly 1, which the rocks wear), `rock-material.md`
  (the band-limited 3D grain, `detail_scale_3d`, which the rocks' surface reuses),
  `terrain.md` (the CDLOD morph, which the rocks must sit on), `wetness.md` (the weathered
  material the rocks are drawn with).

## Model adopted

**1. Where rocks lie: a cover field `k(x)`.** `k` is Golombek's *total fractional area covered
by rocks*, evaluated per candidate from the terrain readback:

- **Talus**: `k = talus_cover · band(α)`, where `band` is the scree's own band
  (`smoothstep(16°, 26°, α) · (1 − smoothstep(31°, 40°, α))`), so the meshes lie exactly where
  the painted scree lies, at the rock's foot.
- **Meadow**: `k = meadow_floor + (meadow_cover − meadow_floor) · patches(x)`, where `patches`
  is a slow field (220 m) thresholded so about a quarter of the meadow is a rocky patch:
  Golombek's plain at 1–2% and rocky ground at 4–5% are one landscape's two states.
- **Nothing on the rock** (slope above 40°, the terrain material's rock layer): the talus angle
  is 30–45°, and a rock does not rest on a cliff.
- **Nothing on sand**: `k · (1 − smoothstep(0.15, 0.5, sediment))`; the material's sand starts
  at 0.5 m of sediment, and a dune buries its rocks.

**2. How big: Golombek's exponential law.** Rocks larger than `D` cover `k·exp(−q(k)·D)` of
the ground, `q(k) = 1.79 + 0.152/k`. The *number* of rocks per square metre with a diameter in
`[a, b)` is the area they cover divided by the area of one:

`n(a, b) = ∫ₐᵇ k·q·exp(−q·D) / (π·D²/4) dD`,

integrated numerically per size class. The meshes start at `D = 0.25 m`; below that the
scree's painted stones and the turf carry the ground. Four size classes, each an octave
(0.25–0.5, 0.5–1, 1–2, 2–3.6 m).

**3. Where exactly: a jittered lattice per class, thinned by `k`.** HZD's dither, done on the
CPU: each class has a lattice whose cell is its footprint (`w = footprint`); each cell holds one
candidate at a hashed position, with a hashed threshold `u`, and the candidate becomes a rock if
`u < n(a, b)·cell²` at its position (the expected count of the cell). The diameter is drawn
from the class's own distribution by inverse transform. The hash is of the cell's integer
coordinates, the class and the seed, so the same ground always carries the same rocks, and the
pattern does not depend on where the camera came from.

**4. How far: a range test, scaled by the rock's size.** A rock of diameter `D` is visible up
to `R = c·D` (`c` by quality: 60, 80, 100, 140 m per metre), so it vanishes at the same size on
screen whatever its size, and a pebble is not drawn 600 m away. With the count per area falling
as `exp(−q·D)/D²` and the area within `R` growing as `D²`, the total count is
`≈ 4c²·k·exp(−q·D_min)`: **bounded**, and dominated by the size the eye can see. The fade is
Bevy's `VisibilityRange` crossfade over the last 20 % of `R`, dithered, so no rock pops. Each
class keeps a ring of tiles around the camera; a tile is built when it enters the ring and
despawned when it leaves (with a margin, so a camera on a tile edge does not thrash).

**5. What shape: a jointed block, rounded by its age.** A rock is the intersection of a few
planes (the joints it broke along, `rock-blocks.md`) with a sphere, the intersection's edges
rounded by a soft minimum — spheroidal weathering erodes the edges first — with a small noise
on the surface, squashed to Verdian's axis ratios (`b : a` 0.55–0.85, `c : a` 0.30–0.55). Two
families: **angular** (a small rounding, the talus's fresh blocks) and **rounded** (a large
one, the meadow's old rocks). Twelve shapes, six per family, each in two levels of detail.
A rock lies on its largest face (the `c` axis up), tilted towards the slope's normal.

**6. How it sits: buried, on the surface the GPU draws.** A rock is buried by a quarter to a
third of its height — rocks are embedded in the regolith, and a rock resting on a point reads as
a prop dropped on the ground. Its height is the terrain *as the CDLOD patches draw it*: the
readback sampled on the lattice of the patch level that covers it, blended to the next level
by the same morph factor the vertex shader uses, so a rock 200 m away sits on the coarse patch
and not on the fine surface the readback holds. The seat is recomputed as the camera moves (the
morph follows the distance) and when the terrain's version changes.

**7. What it is made of: the cliff's rock, weathered by the crate's weather.** The rocks are
drawn with the crate's `WeatheredMaterial` (snow on the up faces, wetness and rain streaks,
cloud shadows), which gains a **stone grain**: the terrain's own band-limited 3D noise
(`detail_scale_3d`) at the world point, modulating the albedo and bending the normal, so a rock
has a surface at 2 m and is not a smooth plastic shape. The rock's albedo is the terrain's rock
albedo times a palette tone per shape (`rock-palette.md`'s four tones, mean exactly 1), and the
shape's vertex colour darkens its buried base (the contact) and mottles its faces.

**8. The data it needs.** The terrain readback carries the **sediment** as well as the surface
(one more `f32` per cell, written by the same kernel and only when the terrain changes), and
`TerrainSnapshot` publishes it.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| Smallest mesh rock `D_min` | 0.25 m | below it the scree's painted stones (`scree.md`, stones 0.1–0.7 m) |
| Largest `D_max` | 3.6 m | Verdian: a-axes "as large as 5450 mm"; b-axis 3.6 m is the class's top |
| `q(k)` | `1.79 + 0.152/k` | Golombek 2021 |
| Talus cover `talus_cover` | 0.30 | Deviation 1 |
| Meadow floor / rocky patch | 0.004 / 0.05 | Golombek's 1–2% plains and 4–5% rocky ground; Deviation 2 |
| Rocky patch field | 220 m, a quarter of the meadow | Deviation 2 |
| Talus band | scree's band, 16°–40° | `scree.md` |
| No rocks above | 40° | the material's rock layer; Galin's talus angle 30–45° |
| Sand cut | `1 − smoothstep(0.15, 0.5, sediment)` | the material's sand at 0.5 m |
| Visible range `c` | 60 / 80 / 100 / 140 m per m | Deviation 3 |
| Fade | last 20 % of `R`, dithered | Bevy `VisibilityRange` |
| Near LOD to | `22·D` | Deviation 4 |
| Axis ratios `b : a`, `c : a` | 0.55–0.85, 0.30–0.55 | Verdian 2021 (0.57–0.64, 0.26–0.36 measured; 0.67, 0.45 theory) |
| Burial | 22–35 % of the height | Deviation 5 |
| Tilt to the slope | 70 % of the way, ±8° jitter | Deviation 6 |
| Stone grain | 0.35 m, three octaves | `rock-material.md`'s grain (`ROCK_GRAIN_COARSE_M` 2.6 m is the cliff's; a rock is smaller) |

## GPU mapping

- No new kernel. The readback kernel (`terrain_readback` in `pack.wesl`) writes
  `(surface, sediment)` per cell instead of the surface alone; it runs only when the terrain's
  version moves.
- The rocks are ordinary mesh entities: Bevy batches every rock of one mesh asset into one
  instanced draw per pass (24 meshes: 12 shapes × 2 levels), draws their shadows into the sun's
  cascades, and fades them with `VisibilityRange`'s dither (`VISIBILITY_RANGE_DITHER`, which
  `weathered.wesl` already honours).
- The stone grain is three `perlin3_d` octaves in `weathered.wesl`, band-limited by the pixel's
  footprint, only for materials that ask for it (`grain_m > 0`), so every existing
  `WeatheredMaterial` is unchanged.
- CPU: the placement is per tile when a tile enters a ring; the seat update is a few lattice
  samples per rock when the camera has moved.

## What great looks like

- A hillside at eye level carries **rocks you could step over and boulders you would walk
  round**, sitting *in* the ground: buried, darker at the base, each with its own cast shadow.
- Many small, few large: a boulder is an event, a stone is texture — the exponential law.
- The rocks **cluster**: aprons of fresh angular blocks at the foot of every cliff, rocky
  patches on the meadow with clean grass between them. Never a grid, never an even sprinkle.
- The rocks are **the cliff's rock**: the same palette, the same grain; a boulder below a cliff
  is visibly a piece of it.
- Slabs and blocks, not potatoes: flat faces, rounded edges, the short axis a third of the long
  one.
- Nothing on the dunes, nothing on the cliffs, rocks in the stream bed.
- No pop: from a moving camera the scatter thins with the distance and no rock appears in a
  frame.
- In weather: snow caps on the tops, wet dark faces in rain, the cloud shadows over them.
- Reference: Horizon Zero Dawn's procedurally placed rock fields (GDC 2017), and real boulder
  fields — glacial erratics on an upland pasture, a talus apron below a crag.

## Validation targets

1. **Placement rules (CPU test)**: no rock where the slope is above 40° or the sediment above
   0.5 m; on a talus slope the rock count per area is at least 10× the meadow floor's.
2. **The size law (CPU test)**: over a large uniform patch, the measured cumulative fractional
   area of rocks with `D ≥ 0.5 m` and `D ≥ 1 m` is within 25 % of `k·exp(−q·D)`.
3. **Not a grid (CPU test)**: the Clark–Evans nearest-neighbour ratio of one class's rocks is
   between 0.85 and 1.6 (1 for a Poisson scatter, 2.15 for a hexagonal lattice).
4. **Deterministic (CPU test)**: a tile built twice holds the same rocks.
5. **Shapes (CPU test)**: every mesh's axis ratios are in the ranges above; its normals point
   out (positive signed volume); the angular family's edges are sharper than the rounded one's.
6. **Seat (CPU test)**: the seat height at level 0 equals the readback's bilinear surface; at a
   coarser level it equals the coarse lattice's interpolation, and at the morph's end both
   levels agree.
7. **At eye level** (`hills_eye`, `hills_eye_rev`, `hills_seed7`, 1.7 m, noon): rocks are
   visible, each with a cast shadow, none floating (no sky or turf visible under a rock's
   edge in a 4× crop), none a uniform colour.
8. **At the mountain foot** (`mtn_foot`, `mtn_elev`, `mtn_dusk`): an apron of angular blocks at
   the cliff's foot; at dusk the blocks rake the light.
9. **Elevated and aerial** (`hills_elev` 25 m, `hills_aerial` 250 m): clustered patches, no
   lattice, no visible ring where the scatter ends.
10. **Close-up** (`hills_ground`, 0.4 m): a rock's surface has grain; its silhouette is not
    visibly polygonal.
11. **Moving camera** (a 120-frame flight): no rock pops; the frame-to-frame change at the fade
    ring is dither, not a step.
12. **Weather** (Snow with 24 grown hours, Rain): snow on the rocks' tops, darker wet faces.
13. **Dunes**: no rock on the sand.
14. **Cost** (`bench_elements`, mountains, High, 1920 × 1080): the opaque, prepass and shadow
    passes together grow by at most **0.3 ms**; the main world's placement and seat update cost
    under 0.5 ms a frame in a flight.

## Deviations from this plan

1. **Talus cover 0.30.** Golombek's model is fitted to plains (1–5 %) and its curves flatten at
   high `k`; a talus is a clast-covered slope (Verdian), so its total cover is near 1, but most
   of it is below the meshes' 25 cm and is the scree's painted stones. With `k = 0.30`,
   `q = 2.30`, rocks above 25 cm cover `0.30·e^(−0.58)` = 17 % of the apron: a boulder field
   with stones between, as a talus looks.
2. **Meadow floor 0.4 %, rocky patches 5 %.** Golombek's plain is 1–2 %, but on a grass meadow
   the turf hides the small rocks the 1–2 % counts (they are below `D_min` and under the
   grass); the floor keeps the occasional erratic and the patch is the rocky ground's 4–5 %.
   The patches are a quarter of the meadow, an art-directed choice: rocky ground is the
   exception on a pasture.
3. **Visible range `c`.** A rock disappears at the same angular size: at High 1 m per 100 m is
   0.57°, about 10 pixels tall at 1080p and 60° of view. The count `≈ 4c²·k·exp(−q·D_min)` grows
   as `c²`, so the quality scales it.
4. **Near LOD to `22·D`.** The far mesh (80 triangles) is used where a rock is under about 25
   pixels across; the near mesh (1280 triangles) inside, where a silhouette's facets would show.
5. **Burial 22–35 % of the height.** No source measures burial; the value keeps the base under
   the surface across the LOD's error and reads as embedded, which a rock resting on a point
   does not.
6. **Tilt 70 % towards the slope's normal.** A rock on a slope rests on it, but not perfectly
   parallel; a full tilt reads as a decal.

## Pending index

- `golombek2021` (DOI `10.1029/2021EA001959`): the PDF is in `/tmp/i42`, not ingested into
  home-still (scribe has no free slots; the corpus copy would need the SciDB path).
- GDC 2017 van Muijden slides: a non-DOI source; the URL is recorded above.
