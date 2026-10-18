# Cumulus that billow: the envelope model and value erosion

The fair-weather cumulus of `Clear` and `PartlyCloudy` (the weather type `b = 0.5`, a 1 800 m layer from 1 500 m) render as kilometre-wide pancakes: smooth domes with a fine sparkle of grain, stacked shelves on their flanks, and no turrets (iteration 17 review, `/tmp/i17/before/cu_sheet.png`, zooms `rib_a.png`, `rib_d.png`). Every outdoor frame under these presets shows them, so it is the largest beauty gap left in the sky; the stacked shelves are an artefact on top of it.

Cause, from the density of `clouds.md` (Model, "Density") and an experiment:

- **Inside a coverage blob the cloud is solid.** The coverage erosion `ρ₀ = remap(S′, 1 − r′, 1, 0, 1) · r′` keeps every positive `S′` once `r′` saturates at 1, and the weather map's soft edge (Δ = 0.12 of an fBm whose shortest wavelength is 2 km) saturates within ~100 m of a blob's edge. The 3D noise then only modulates density inside a solid mass instead of carving clouds out of it.
- **The cumulus height gradient is a flat slab.** `(0, 0.1625, 0.88, 0.98)` is constant from 16 % to 88 % of the height; the dome factor (the map's A channel, bilinear over 62.5 m texels) shortens it toward the blob edge, and each texel's kink of that bilinear field and the gradient's linear corners show as a terrace. Removing the dome scaling (experiment A, `/tmp/i17/expA/`, `cmpA.png`) removes the shelves and leaves a flat-topped wall: the shelves are the dome terraces.
- **The detail erosion is weak and coarse.** `0.2 · hf` with the 32³ detail tile at 1024 m (Worley cells 128–512 m) moves an edge by a fifth of the density ramp: grain, not billows.

## Sources

- **Schneider & Vos 2015**, *The Real-time Volumetric Cloudscapes of Horizon Zero Dawn* (corpus `s2015-advances-ppt-944-mb-pdf-5-mb-2`): billows are "packed, sometimes taking on a cauliflower shape" (ll. 473–481); Perlin noise dilated by inverted Worley "keep[s] the connectedness of Perlin noise but add[s] some billowy shapes" (l. 489); the cloud is the base shape eroded by the detail noise at its edges, with the Worley inverted toward the base for wisps (l. 661); a 128³ base and 32³ detail texture (ll. 555–569); curl noise distorts the detail (l. 601).
- **Schneider & Vos 2017**, *Nubis* (corpus `s2017-advances-pdf-slides-13mb`): `perlin-worley = remap(perlin, 1 − worley, 1, 0, 1)` (l. 393); "subtract the web like shapes of Worley noise from the low density regions of Perlin noise in order to introduce round shapes there" (l. 380).
- **Schneider 2022**, *Nubis, Evolved* (SIGGRAPH Advances; `SIGGRAPH2022-Advances-NubisEvolved-NoVideos.pdf`, text `/tmp/i17/pdf/nubis2022.txt`). The **envelope model**: a dimensional profile `bottom_gradient · top_gradient · edge_gradient` with `bottom = f²`, `top = (1 − f)^1.5` and an edge gradient over the last 35 m of the envelope; a noise composite blended from wispy (low) to billowy (high) by height around the cloud type (`noise_height_blend = remap(f, type + 0.1, type − 0.1)`); density `f · pow(saturate(noise − (1 − profile)), 0.27)` — a **value erosion** whose small exponent sharpens the silhouette.
- **Schneider 2023**, *Nubis³* (SIGGRAPH Advances; text `/tmp/i17/pdf/nubis2023.txt`): the same erosion `ValueErosion(profile, noise_composite)` for voxel clouds, sharpened by `pow(density, lerp(0.3, 0.6, …))`; billowy noise `lerp(noise.b, noise.a, profile^0.25)` (low to high frequency as the profile grows), wispy `lerp(noise.r, noise.g, profile)`; "Increasing Density = Layered Billows, Decreasing Density = Curly Layered Wisps".
- **Bauer 2019**, *Creating the Atmospheric World of Red Dead Redemption 2* (SIGGRAPH Advances, `slides_public_release.pptx`, text `/tmp/i17/pdf/rdr2.txt`): a 2D cloud map for placement, a height LUT per cloud type for the vertical shape, `density = smoothstep(shape + lut, …, map)`, then "cloud detail is carved out by utilizing a rescaling function, also known as linstep or inverse lerp" — the same value erosion.
- **Webanck et al. 2018**, *Procedural Cloudscapes*, CGF 37(2) (`10.1111/cgf.13373`, HAL `hal-01730789`, read): every type's shape is `s = max(φ − δ, 0)`, a low-frequency field minus a high-frequency one, the clamp "plays an important part in the visual appearance of the clouds as they are defined as the compact support of the density field"; cumulus base 0.8–1.5 km, range 0.1–5 km (Table 2); cumuliform clouds *shrink* (a value is subtracted) rather than fade as cover falls.
- **Goswami 2021**, *A survey of modeling, rendering and animation of clouds in computer graphics*, Vis. Comput. (`10.1007/s00371-020-01953-y`, downloaded): procedural noise eroding an envelope is the real-time family; physical simulation and metaball cumulus are offline.
- **Mol et al. 2023** (`10.1029/2022jd037894`, iteration 16): cloud sizes follow a power law; cloud edges are transparent over ≈ 25 m.
- **Toft, Bowles & Zimmermann 2016**, *Optimisations for Real-Time Volumetric Cloudscapes* (`10.48550/arxiv.1609.05344`, downloaded): fewer steps with jitter and TAA; the march here already does both.
- Not obtained: Plank 1969 and Neggers et al. 2003 (cumulus size statistics; no open copy), Wood & Field 2011 (publisher 404).

## Model adopted

The weather map, the coarse/fine march, the lighting and the stratus and cumulonimbus types are unchanged; the cumuliform weight `κ = clamp(4 b − 1, 0, 1)` (0 for stratus up to `b = 0.25`, 1 at cumulus `b = 0.5` and above) blends the new density in, so a stratus deck is exactly what it was.

1. **A rounded envelope instead of the slab.** The vertical profile of a cumuliform cloud is `P(f) = smoothstep(0, f_b, f) · (1 − smoothstep(s · f_t, s, f))`, `s` the blob's height scale from the dome factor `mix(MIN_HEIGHT_SCALE, 1, √dome)` as before: a flat, crisp base over the first `f_b` of the layer and a top that rounds off over the upper part of the blob's height — C¹ everywhere, so neither the gradient's corners nor the dome texels' kinks show as creases. The dome factor is read with the weather map's bilinear filter but enters only through this smooth function.
2. **Value erosion carves the clouds out of the coverage.** The dimensional profile is `D = P(f) · r′` (Nubis Evolved), and the density is `ρ = (saturate(N − (1 − D)) / D)^γ · D` — Nubis' `saturate(noise − (1 − profile))` normalised to reach `D` where the noise is 1, raised to `γ < 1` for a sharp silhouette. Where the profile is high (the core) almost every noise value survives; toward the top and the edges only the noise's peaks do, so the outline breaks into the noise's rounded cells.
3. **A billowy noise composite.** `N` blends a wispy and a billowy composite by height around the type (Nubis Evolved's `noise_height_blend`): wispy at the base (the Perlin-Worley base channel, as Schneider inverts the Worley toward the base), billowy above (inverted-Worley octaves of the base texture's G/B/A and the detail texture's R, low to high frequency as the profile rises, Nubis³). The detail texture's tile shrinks from 1024 m so its cells reach turret scale (≈ 100–250 m) and the finest billows (≈ 30–60 m); the base tile shrinks so the Perlin-Worley lobes are cumulus-sized (≈ 0.5–1 km) rather than 2 km.
4. **The coarse march samples the same envelope without the detail**, so it still finds every cloud the fine march would (the coarse test is `D · N_base > 0`).

## Parameters

Initial values; each change after tuning is recorded under Deviations.

| Name | Initial | Source |
|---|---|---|
| Cumuliform weight `κ` | `clamp(4 b − 1, 0, 1)` | design: stratus (`b ≤ 0.25`) untouched |
| Base rise `f_b` | 0.06 of the layer (≈ 110 m at 1 800 m) | flat bases, Nubis' `bottom_gradient` sharpened |
| Top rounding `f_t` | 0.45 of the blob's height | Nubis' `top_gradient (1 − f)^1.5`, C¹ form |
| Edge exponent `γ` | 0.4 | Nubis Evolved 0.27, Nubis³ 0.3–0.6 |
| Base tile | 8192 m → 4096 m | lobes ≈ 0.5–1 km (Webanck: cumulus range 0.1–5 km) |
| Detail tile | 1024 m → 384 m | Worley cells ≈ 48–190 m: turrets and billows |
| Wispy-to-billowy blend | around `f = 0.15` over ±0.1 | Nubis Evolved `remap(f, type ± 0.1)` |

## GPU mapping

No new pass, texture or binding. The fine sample already reads the base and detail textures; the composite reuses those taps (no new fetch). The coarse sample keeps one base tap. Expected cost: a few ALU per fine sample; the march's early exit may fire later inside sharper, emptier clouds, so the march is benchmarked against iteration 16 (`elements_clouds_march` 0.443–0.468 ms at High, hills, `PartlyCloudy`).

## What great looks like

Fair-weather cumulus as in a photograph from the ground on a June afternoon: individual clouds a few hundred metres to a kilometre or two across with flat grey bases at one height, bulging sunlit tops built of rounded turrets and smaller billows on them (cauliflower), crisp bright edges against blue sky where the sun grazes them, darker hollows between turrets, thin ragged wisps only near the base. No stacked shelves, no smooth plastic domes, no sparkle grain. Seen from below the base is flat; from a distance the tops read as cauliflower; at the horizon the field thins into separate puffs. Moving with the wind, the billows do not swim or pop.

## Validation targets

1. **No terraces (visual, zoomed).** The cumulus flanks of `terrain_tour` hills `PartlyCloudy` at 15.5 h, 12 h and 9 h (`/tmp/i17/cap.sh` views a, d, e) show no stacked shelves at 3× zoom.
2. **Billows (visual).** At those views the tops show rounded turrets at two scales and the silhouettes are crisp; the bases are flat.
3. **Stratus unchanged (CPU, `clouds::model`).** The stratus column optical depth (Validation 7 of `clouds.md`, 6.04) and every existing CPU density test hold, because `κ = 0` for stratus.
4. **The cloud still covers what the weather says (GPU).** The shadow-map statistics of `tests/gpu_clouds.rs` (Clear dark fraction 0.01–0.12; Overcast ≥ 0.9; the light factors) hold or are re-measured with the reason recorded; the map fraction is untouched by construction.
5. **The CPU mirror follows the shader (`clouds::model`).** The mirror's medium implements the same envelope and erosion; a new test pins the envelope's shape: zero below the base and above the scaled top, C¹ (no jump in slope), and `ρ ≤ D`.
6. **Performance.** `elements_clouds_march` at High, hills, `PartlyCloudy`, within 10 % of iteration 16's 0.443–0.468 ms, measured alternately against `main`.
7. **Motion.** A 6 s sequence with the clouds drifting shows no popping or swimming of the billows.

## Deviations from this plan

## Pending index

Downloaded by `hs paper download`: `10.48550_arxiv.1609.05344` (Toft 2016), `10.1007_s00371-020-01953-y` (cloud survey). Read from local copies not in home-still: Schneider 2022 and 2023 (Advances PDFs), Bauer 2019 (Advances PPTX), Webanck et al. 2018 (HAL; the Wiley copy failed). Not obtained: `10.1175/1520-0450(1969)008<0046:tsdocc>2.0.co;2` (Plank), `10.1175/1520-0469(2003)60<1060:ssoccp>2.0.co;2` (Neggers), `10.1175/2011jcli4056.1` (Wood & Field). `distill_index` not run (the MCP bridge is unavailable; the CLI pipeline indexes stored papers).
