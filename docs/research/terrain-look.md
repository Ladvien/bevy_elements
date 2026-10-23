# Terrain look: a tint and a saturation a game sets on the owned terrain's bare ground

Issue #35 (Sanctuary, an isometric siege game on the crate's owned `Terrain` at `0f5f71c`): the game lights its world with five palettes and grades its own materials per palette — at night every role colour is desaturated 40 % and cooled, and the field outside the walls must read as a navy veil (a translucent overlay, navy at 0.62 alpha) where nothing lights it. On the crate's terrain the meadow's green shows through the veil as olive, and the game cannot grade the ground: the terrain's fragment shader overwrites `pbr_input.material.base_color` with its own albedo (`render.wesl`), and the material is made inside the crate with no public handle or setting that reaches it. `TerrainMaterialParams` carries LOD, cloud shadows, water, cover, ripples, trails and ground textures, but no colour grade.

## Sources

- **Issue #35** (`https://github.com/Ladvien/bevy_elements/issues/35`, read 2026-10-23): the requirement, the proposed `TerrainLook { tint: Vec3, saturation: f32 }`, the edge cases (identity bit-for-bit, a uniform write and never a re-specialisation, saturation outside 0..1, the ground textures graded too, `TerrainRender::External` out of scope) and the five acceptance criteria quoted under Validation targets.
- **Akenine-Möller et al. (2018)**, *Real-Time Rendering*, 4th ed., §8.2.3 *Color Grading* (corpus `book_real_time_rendering_4th_akenine`, lines 8437-8459): grading is "creative manipulation of image colors"; "the scene-referred approach has since been gaining traction … due to its higher visual quality", illustrated by Uncharted 4's multiplication by a saturated cyan applied before the tone map against after it (Figure 8.16); Hable's overview of the common operations. A grade of the material's *albedo* is one step earlier than scene-referred grading: the light reflects off the graded colour, so lighting, fog, the atmosphere and the weather on top of it stay physical.
- **`bevy_flora` 0.46 `wet_on`** and this crate's `surface_cover.wesl` `wet_canopy` (`docs/research/wet-ground.md`): the saturation operator already in the family, `grey + (c − grey) · s` with `grey` the Rec. 709 luminance `0.2126 R + 0.7152 G + 0.0722 B` of linear RGB — an interpolation toward the colour's own grey for `s` < 1 and an extrapolation away from it for `s` > 1, the luminance kept either way. ITU-R BT.709's luma coefficients are the ones the crate already uses for linear luminance.
- Repository: `src/terrain/render.wesl` (the albedo built from the layers, the textures, the turf, the scree and the rock, then `wet_canopy` and `weather_surface`), `src/terrain/material.rs:44-79,460-530` (`TerrainMaterialParams`, `update_materials`: a material is re-prepared only when its parameters change, and its pipeline key is `TerrainExtensionKey`, the texture set's presence), `src/surface_cover.wesl` (`wet_canopy`, `canopy_share`), `AGENTS.md` (the register-pressure trap: a value carried across `rock_material` costs the whole pass).

## Model adopted

1. **What is graded: the bare ground's albedo**, after everything that makes it (the layers, the detail textures, the turf, the scree's stones, the rock's material and its sky occlusion) and before the ground cover (the wet canopy, wetness, puddles, snow), so standing water and snow keep the crate's colours and a grade never tints the snow.
2. **The operator.** With `s` = `max(saturation, 0)` and `t` = `tint`:
   $$\mathbf c' = \max\big(\mathrm{grey}(\mathbf c) + (\mathbf c - \mathrm{grey}(\mathbf c))\,s,\ 0\big) \odot \mathbf t.$$
   Saturation first, then the tint: a palette that desaturates and then cools (Sanctuary's night: −40 % and cooled) keeps the tint's colour, which a tint applied first and then desaturated would wash out. At `s` = 0 the ground is its own luminance times the tint; `s` > 1 oversaturates, clamped at zero so no channel goes negative. The tint is not clamped above 1: it is the game's grade.
3. **The canopy is read before the grade.** The wet grass's share (`canopy_share`, the albedo's excess green) is taken from the ungraded colour, so a desaturated or blue-graded meadow still beads the rain as grass (an excess green of a grey ground would be 0 and the meadow would take the porous sheen).
4. **The operator is branchless; the default is the identity within a float's rounding.** A branch on the uniform that skipped the default (to keep it bit-exact) cost the 4K opaque pass about 6 % on every frame, rain or shine (the dry meadow's opaque pass over its early prepass: base 54.6-57.5, the branch 57.3-61.4, branchless 56.3-57.7; `AGENTS.md`: the terrain's cost is its register allocation, not its arithmetic). Branchless, the default computes `grey + (c − grey) · 1` times 1, off from `c` by at most an ulp: the captures at the default match the base within 1 level, as the GPU's own noise does.
5. **A uniform, not a key.** `TerrainMaterialParams` gains `look: Vec4` (tint, saturation); `update_materials` re-prepares the material when it changes, as it does for the ripple phases every frame in rain. The pipeline key (`TerrainExtensionKey`) is untouched, so a look written every frame compiles nothing.
6. **The scattered rocks follow the ground.** `RocksPlugin`'s boulders and stones are the crate's own debris on the terrain, drawn with one shared `WeatheredMaterial` whose per-rock tones are vertex colours; a grade that left them out would leave warm tan stones on a cool graded ground (`/tmp/i48/after/mtn_graded.png`). Their material's uniform gains the same `look` lane, written from `TerrainLook` when it changes, and the weathered shader applies the same operator after the vertex colours and the stone grain and before the weather. A `WeatheredMaterial` a game makes keeps the identity (its own colours are the game's to grade).
7. **Out of scope:** `TerrainRender::External` / `GroundMaterial`, whose base colour is the game's own `StandardMaterial` and which the game can grade itself; a game's own `WeatheredMaterial`s.

## Parameters

| Name | Default | Range | Basis |
|---|---|---|---|
| `TerrainLook::tint` | `Vec3::ONE` | ≥ 0 per channel (linear multiplier) | issue #35 |
| `TerrainLook::saturation` | 1 | 0 grey, 1 unchanged, > 1 oversaturated; negative clamped to 0 | issue #35; the operator of `wet_canopy` |
| Luminance | Rec. 709 (0.2126, 0.7152, 0.0722), linear | — | the crate's and `bevy_flora`'s |

## GPU mapping

- `render.wesl`: `TerrainMaterialParams.look`; after the albedo is complete and `canopy_share` has read it, the operator is applied unconditionally (a dot product, a multiply-add, a max, a multiply). The albedo is live at that point, so nothing new is carried across `rock_material` (`AGENTS.md`).
- `material.rs`: the field, packed from `Res<TerrainLook>` in `update_materials` (saturation clamped at 0 there); a changed look re-prepares the material, as any parameter change does.
- `weathered`: `WeatheredExtension` gains a private `look: Vec4` in its `#[uniform(100)]` block (identity by default; its other uniform fields are private, so no caller builds it by struct literal), `weathered.wesl` grades the base colour with it; `rocks` writes `TerrainLook` into its shared material when the look changes.
- No binding, texture or shader def. Cost: four ALU, measured as no change to the pass (Model adopted 4).

## What great looks like

- **The default is the crate's ground**, unchanged at every angle.
- **A night grade** (desaturated 40 %, cooled): the meadow a muted blue-grey green that sits under a navy overlay without reading olive; relief, texture detail and the turf's palette still legible; snow and puddles keep their own colours.
- **Saturation 0**: a monochrome ground with all its relief and detail, the snow-covered patches unchanged, wet ground still wet (the meadow keeps its canopy behaviour).
- **Tint 0.5**: the same ground at half the albedo, nothing else changed.
- **A palette blend** written every frame: a smooth glide, no hitch, no compile.

## Validation targets

The issue's acceptance criteria, each with how it is checked:

1. **Default identity** — "With `TerrainLook::default()`, a render … matches `0f5f71c` within 0 DN": captures of `terrain_tour` (`hills_eye`, `hills_down`, `hills_snow`, `hills_rain`, `dunes`) against the base build within 1 level, the GPU's own run-to-run noise (Model adopted 4: the operator's rounding at the default is at most an ulp, below an 8-bit level), and a CPU mirror test that the default operator is the identity within 1e-6. On rock, compare structurally (`AGENTS.md`: the rock's cell networks flip on any change of the register schedule).
2. **Tint 0.5** — "a flat, sun-lit grass patch's linear albedo … is half of the default's, within 1 %": the CPU mirror, within 1e-6; the lit frame is checked for the expected darkening (the lit value is not half: the specular reflection, which the albedo does not scale, is a large share of a dark grass's radiance).
3. **Saturation 0** — "the bare ground's hue spread over a near patch is below 0.01, while a snow-covered patch's colour is unchanged": the mirror (zero chroma); captures of `hills_eye` at saturation 0 (chroma of the ground band) and of `hills_snow` at saturation 0 against the default (the snow band unchanged).
4. **No compilation** — "Writing a new `TerrainLook` every frame for 240 frames logs no pipeline or shader compilation after the first frame": a GPU test that writes a new look every frame and counts the render world's cached pipelines before and after.
5. **README** — the Terrain section names `TerrainLook`.

## Deviations from this plan

None from the issue's API. The order of the two operations (saturation, then tint) and the canopy read before the grade are this note's (Model adopted 2-3), with the reasons there. The scattered rocks are graded too (Model adopted 6), which the issue does not ask for: it lists the bare ground's rock among what a grade must reach, and the crate's own stones lying on that ground are part of it.

## Pending index

- Nothing new to index: RTR4 is in the corpus; the issue is a web page.
