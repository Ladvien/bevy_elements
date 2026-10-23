# A cliff is cut by joint sets: partings, systematic joints and the cross joints between them

The cliff's blocks are the cells of a 3D cellular network (`rock_block_scale` in `src/terrain/common.wesl`), and its joints read as chicken wire. Iteration 43 drew the network's borders as thin cracks and could not make the face read as rock in five rounds (`docs/research/rock-joints.md`, `## Outcome`): whole cells read as crazy paving, cells grouped into blocks as camouflage. A cell network cannot give a block a straight edge. This note replaces the network with the structure a sedimentary cliff is actually cut by: **bedding partings, systematic joints confined between them, and cross joints confined between the systematic joints**.

## What the code does (the diagnosis)

1. **The joints are not the blocks' borders.** The first set's "joint" is the distance to the integer lattice cell's wall away from the feature point, so it draws lines through blocks and none where two blocks meet; it is up to 0.37 of a half-cell wide and widens with distance at full darkness (`soft = max(width, footprint / (cell · 0.5))`).
2. **The blocks are columns.** `0e58b68` divides the bedding normal's component by `ROCK_BLOCK_TABULAR` instead of multiplying it, so the cells are 2.5 m tall across the beds (`rock-joints.md` Deviation 5).
3. **The second set is a fence.** Its relief uses the unwandered, unsegmented plane, the same tilt on both sides, at a slope of 0.95 over 8–40 cm.
4. **Every piece is its own rock.** The palette is drawn per cell, so the near face is a patchwork of minerals, while one bed is one rock (iteration 43, rounds 3–4).
5. **The search is inexact.** `cell3`'s prune bound in the WESL is not clamped at zero, and the mirror's is a different bound (`rock-joints.md` `## Outcome`).

## Sources

- **Bai, T. (2000)**, *Formation and spacing of orthogonal cross joints: implications for subsurface fracture networks*, Stanford Rock Fracture Project vol. 11, F-1–F-18 (non-DOI; `https://stacks.stanford.edu/file/druid:zb158kp7289/RFP_2000_Bai.pdf`, read in full). "Cross joints extend across intervals between systematic joints, without cutting across the systematic joints"; orthogonal cross joints "abut the systematic joints at about 90° angles" and "typically form a 'ladder-like' pattern if viewed on a bedding surface"; they "are most commonly found between systematic joints with lesser spacing, whereas between pairs of systematic joints with greater spacings, orthogonal cross joints are less common" (after Engelder & Gross 1993); "systematic joints often are confined by layer boundaries with their height roughly equal to the thickness of the fractured layer" (after Narr & Suppe 1991); "ratios of S/T_f > 2 are extremely rare for competent lithologies"; the measured ratios are about 0.7–0.8 (Monterey Formation, after Gross 1993) and about 1.1 (Brallier Formation, after Ruf et al. 1998); the cross joints' critical spacing is about the systematic spacing S when S/T < 0.6 and about the layer thickness T when S/T > 1.8.
- **Bai, T., Pollard, D. D. (2000)**, *Fracture spacing in layered rocks: a new explanation based on the stress transition*, Journal of Structural Geology 22:43–57, DOI `10.1016/S0191-8141(99)00137-6` (abstract read; Elsevier): opening-mode fractures in layered sedimentary rocks "often are periodically distributed with spacings linearly related to the thickness of the fractured layer", with a critical spacing-to-thickness ratio of "approximately 1.0".
- **Ji, S., Rousseau, Y., Marcotte, D., Phillips, N. J. (2023)**, *The formation of orthogonal joint systems and cuboidal blocks: new insights gained from flat-lying limestone beds in the region of Havre-Saint-Pierre (Quebec, Canada)*, Journal of Rock Mechanics and Geotechnical Engineering, DOI `10.1016/j.jrmge.2023.03.012` (open access; abstract read, PDF downloaded): "the spacing of cross-joints is consistently larger than the spacing of systematic joints by a factor of 1.5 approximately"; spacing-to-bed-thickness ratios of 4.3 (systematic) and 6.4 (cross) in low-strain beds against about 1 in the strained strata across the river — strain sets the block size.
- **Palmström, A. (2015)**, *Joint characteristics*, RockMass.net (read for iteration 43, `rock-joints.md`): a joint's separation is "mainly less than a millimetre"; planar joints deviate by under 1 %; "continuous joints terminate at other joints".
- **Verdian, J. P. et al. (2021)**, Earth Surface Dynamics 9:1073, DOI `10.5194/esurf-9-1073-2021`: the latent block size is "set by the spacing and orientation of fractures, foliations, and bedding planes" (`block-sizes.md`).
- **Talling, P. J. (2001)**, bed thicknesses as sums of log-normal distributions (`bedding.md`): a succession mixes thin and thick beds.
- **Akenine-Möller, T. et al. (2018)**, *Real-Time Rendering*, 4th ed., ch. 6 (corpus `book_real_time_rendering_4th_akenine`): on a brick wall, "each brick face appears to be perfectly flat. This does not look right"; the shading normals are varied per brick; "the bricks should cast shadows onto the mortar". Scanned cliffs in shipping games are meshes (Fig. 19.29, Quixel Megascans); a terrain shader carries the structure procedurally.
- **Iteration 43** (`rock-joints.md`, `/tmp/i43/critique.md`): hairline traces anti-aliased by coverage removed the chicken wire; the bed's colour at every distance made the cliff read as a succession; cell networks read as paving or camouflage.

Not obtained (Elsevier, no open copy): Narr & Suppe 1991 (JSG 13:1037, DOI `10.1016/0191-8141(91)90055-N`) and Gross 1993 (JSG 15:737, DOI `10.1016/0191-8141(93)90059-J`); their findings are cited through Bai 2000.

## Model adopted

1. **Mechanical layers.** Each bed (the existing stacked beds, `bed_value`) is split by **bedding partings** into layers; the bed's own boundaries are partings. Each bed draws a *bedding style* — its nominal layer thickness, log-uniform between `ROCK_LAYER_MIN_M` and `ROCK_LAYER_MAX_M` — so a succession mixes thin flaggy beds and massive ones (Talling). The number of layers is the bed's thickness over that, rounded, at least one; the internal partings are jittered by up to a quarter of a layer.
2. **Systematic joints (set A).** Planes perpendicular to the bedding with a fixed strike. In each layer their spacing is `S = r · T`: proportional to the layer's thickness (Bai & Pollard; Narr & Suppe via Bai), with the ratio `r` drawn per bed, log-uniform between `ROCK_SPACING_RATIO_MIN` and `ROCK_SPACING_RATIO_MAX` (Bai's measured 0.7–1.1 to Ji's low-strain 4.3, so the beds differ in blockiness the way lithologies and strains do). Each joint's position is jittered by up to 30 % of a spacing. Each layer has its own offset, so the joints **end at the partings** and are staggered from layer to layer.
3. **Cross joints (set B).** Planes perpendicular to both the bedding and set A. They live **between two systematic joints** and never cross one: each interval has its own offset, so the cross joints of neighbouring intervals do not line up — the ladder (Bai). Their spacing is `1.5 S` (Ji), jittered as set A's. An interval holds cross joints with a probability that falls with its width over the layer's thickness (Engelder & Gross via Bai), from 0.85 for an interval as wide as the layer to 0.3 for one three layers wide.
4. **A block** is a layer, an interval of set A and, where cross joints exist, an interval of set B. It wears its **bed's colour** (iteration 43, round 4) and its own **value**, a draw of ±`ROCK_BLOCK_TONE`; and its own **facet**, a random tilt of slope `ROCK_FACET_TILT` constant over the block (Real-Time Rendering: a perfectly flat face "does not look right").
5. **Traces.** Every joint and parting is a thin dark trace (`rock-joints.md`): half-width drawn per block between `ROCK_TRACE_MIN_M` and `ROCK_TRACE_MAX_M`, partings `ROCK_PARTING_WIDEN` times wider (a parting is a bedding surface, weathered back further than a joint), anti-aliased by coverage so a far trace fades instead of widening. The face rolls into each trace over two trace widths (the bevel), signed away from the crack on both sides, so the light finds the step and the downlight side shadows.
6. **The bed is the rock at every distance.** The bed's palette applies at every distance (no band-limit against the blocks); the blocks carry value only.
7. **Deleted:** the cell network (`cell3`, `Cell3`, `CELL_INSET`, its mirror), the block lattice (`ROCK_BLOCK_M`, `ROCK_BLOCK_TABULAR`, the warp), the formation-spaced second set (`ROCK_JOINT2_*`, `ROCK_FORMATION_M`), the per-block palette, the fine generation, and the band-limit gate on the bed's colour with its mirror and test.

## Parameters

| Name | Value | Basis |
|---|---|---|
| `ROCK_LAYER_MIN_M`, `ROCK_LAYER_MAX_M` | 0.35, 2.2 m | thin-bedded to massive within a 4.5 m succession; log-uniform per bed (Talling's mix) |
| `ROCK_PARTING_JITTER` | 0.5 of a layer (±0.25) | partings are not evenly spaced |
| `ROCK_SPACING_RATIO_MIN`, `_MAX` | 0.8, 3.0 | Bai 0.7–1.1 measured, "> 2 extremely rare" for competent beds; Ji 4.3 at low strain; log-uniform per bed |
| `ROCK_JOINT_JITTER` | 0.6 of a spacing (±0.3) | spacings are periodic on average, not exactly |
| `ROCK_CROSS_RATIO` | 1.5 | Ji 2023 |
| cross presence | 0.85 → 0.3 as width/T goes 1 → 3 | Engelder & Gross via Bai: commoner between closely spaced joints |
| `ROCK_TRACE_MIN_M`, `_MAX_M` | 0.012, 0.030 m | iteration 43: an aperture under a millimetre, a shadowed mouth of centimetres |
| `ROCK_PARTING_WIDEN` | 1.5 | Deviation 3 |
| `ROCK_JOINT_DARKEN` | 0.40 | iteration 43 |
| `ROCK_BEVEL_SLOPE` | 0.6 over two trace widths | iteration 43 |
| `ROCK_FACET_TILT` | 0.25 | Deviation 4 |
| `ROCK_BLOCK_TONE` | 0.12 | Deviation 5 |
| `ROCK_BLOCK_BAND_M` | 0.334 m | the blocks' band-limit: the old `ROCK_BLOCK_M · ROCK_JOINT_WIDTH · 1.85`, unchanged so the far face is unchanged |

## GPU mapping

No new bindings or passes; the change is inside `rock_material`'s fragment code. The bed's structure (index, bottom, top) is computed once and shared by `bed_value` and the blocks. A block costs three 1D jittered partitions — the layer inside the bed, set A in the layer, set B in the interval — each at most three integer hashes and a comparison, plus three hashes for the block's draws. That replaces `cell3`'s pruned search over 27 cells (one `hash33` per cell visited), a `perlin3_d` for the warp and a `perlin3` for the segments; the rock should get cheaper. The band-limit still skips the whole block computation where the blocks cannot resolve.

## What great looks like

- **At 3 m:** straight-edged blocks, a hand to a metre or two across, parted by thin dark cracks; the faces tilt a little differently so the sun picks them out; one rock across the blocks, lighter and darker pieces of it; no tile outline, no camouflage, no paving.
- **At 12–30 m:** the beds read as bands of different rock, and each bed has its own way of breaking — thin flaggy slabs in one, massive blocks with widely spaced vertical joints in the next; the vertical joints are staggered from layer to layer and never line up into a brick bond across a whole bed.
- **On a slope that runs along the bedding** (a bedding surface): the ladder — long systematic joints with short cross joints between them, like a limestone pavement.
- **Grazing light** (dawn, dusk): the cracks are shadowed steps; under overcast they are still lines, softer.
- **Far:** the massif unchanged (the blocks band-limit away exactly where they did).

## Validation targets

1. **Partings** (mirror): every bed boundary is a parting; the layers' thicknesses span `p90/p10 ≥ 3` over a run of beds; a bed holds at least one layer.
2. **Systematic joints scale with the layer** (mirror): over many layers, spacing over thickness has a median between 0.8 and 3.0, and no layer's ratio exceeds `ROCK_SPACING_RATIO_MAX`.
3. **They end at the partings** (mirror): of the systematic joints that reach a parting from below, fewer than 10 % continue in line (within one trace width) above it.
4. **Cross joints abut** (mirror): along a line parallel to set A crossing a systematic joint, the cross joints' positions on the two sides differ; a cross joint never crosses a systematic one. Their median spacing over the systematic joints' is between 1.3 and 1.7. Cross joints are present in a larger share of the narrow intervals (width/T < 1.2) than of the wide ones (> 2.4).
5. **Thin traces** (mirror): along a long cut the traces darken less than 8 %; every block's edge is a trace; a trace's integral across the line is the same at every footprint.
6. **Captures** (`/tmp/i44/`): `face3`, `face12`, `face30`, `mtn_foot`, `dusk`, `dawn`, `overcast`, `side_ne`, `side_sw`, `talus_b`, `back`, `seed7` judged GREAT; `far120` within 1 % of its pixels and the same mean; `plains` byte-identical up to GPU noise; no `failed to process`.
7. **Cost:** the mountains' opaque pass no slower than the baseline (interleaved, minimum).

## Deviations from this plan

1. **The block model is replaced.** The cell network, the tabular lattice and the formation-spaced second set (`rock-blocks.md`, `rock-fragments.md`, `block-sizes.md`) are replaced by partings and two joint sets confined by them; their defaults go with them. Source: Bai 2000, Bai & Pollard 2000, Ji 2023, and iteration 43's five rounds.
2. **The palette moves to the bed.** `rock-palette.md` drew the palette per fragment and `bed-colour.md` gave the bed its own only where the fragments do not resolve; the bed's palette now holds at every distance and the blocks carry value only (iteration 43, rounds 3–4: per-piece hue read as a patio, per-block as camouflage; a bed is one rock).
3. **A parting is wider than a joint** (`ROCK_PARTING_WIDEN` 1.5). No source measures it; a bedding surface is a plane of weakness the weather opens back further, and the beds must read as courses at 12 m.
4. **The facet** (`ROCK_FACET_TILT` 0.25, about 14°). No source measures a face's tilt from block to block; Real-Time Rendering's rule is that a perfectly flat brick face looks wrong. 0.25 is iteration 43's 0.22–0.30, which read without turning the face into cobbles once the facet was per block.
5. **The block's value** (`ROCK_BLOCK_TONE` 0.22 → 0.12). Once the blocks wear their bed's colour, iteration 43's ±22 % read as a light-and-dark patchwork (round 4); a block of one bed is a lighter or darker piece of the same rock.
6. **The joint's darkening** (`ROCK_JOINT_DARKEN` 0.76 → 0.40) and **width** (4.5–18 cm of band → a 1.2–3.0 cm trace): iteration 43, Palmström.

## Pending index

- Bai 2000 (Stanford RFP, non-DOI): read from the URL above, not ingested.
- Ji et al. 2023: downloaded (`10.1016_j.jrmge.2023.03.012`), conversion pending.
- Bai & Pollard 2000, Narr & Suppe 1991, Gross 1993: Elsevier, not obtained; abstract or citation only.
