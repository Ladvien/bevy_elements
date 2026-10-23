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
4. **Identity is exact.** The default (`tint` 1, `saturation` 1) skips the operator by a branch on the uniform, coherent across the draw, so the default renders as before: no `grey + (c − grey)` rounding.
5. **A uniform, not a key.** `TerrainMaterialParams` gains `look: Vec4` (tint, saturation); `update_materials` re-prepares the material when it changes, as it does for the ripple phases every frame in rain. The pipeline key (`TerrainExtensionKey`) is untouched, so a look written every frame compiles nothing.
6. **Out of scope:** `TerrainRender::External` / `GroundMaterial`, whose base colour is the game's own `StandardMaterial` and which the game can grade itself; the rock's geometry (`RocksPlugin`'s meshes, `WeatheredMaterial` with the game's colour).

## Parameters

| Name | Default | Range | Basis |
|---|---|---|---|
| `TerrainLook::tint` | `Vec3::ONE` | ≥ 0 per channel (linear multiplier) | issue #35 |
| `TerrainLook::saturation` | 1 | 0 grey, 1 unchanged, > 1 oversaturated; negative clamped to 0 | issue #35; the operator of `wet_canopy` |
| Luminance | Rec. 709 (0.2126, 0.7152, 0.0722), linear | — | the crate's and `bevy_flora`'s |

## GPU mapping

- `render.wesl`: `TerrainMaterialParams.look`; after the albedo is complete and `canopy_share` has read it, `if any(material.look != vec4(1.0))` applies the operator (a dot product, a multiply-add, a max, a multiply). The albedo is live at that point, so nothing new is carried across `rock_material` (`AGENTS.md`).
- `material.rs`: the field, packed from `Res<TerrainLook>` in `update_materials` (saturation clamped at 0 there); a changed look re-prepares the material, as any parameter change does.
- No binding, texture or shader def. Cost: a uniform branch on the default; four ALU when graded.

## What great looks like

- **The default is the crate's ground**, unchanged at every angle.
- **A night grade** (desaturated 40 %, cooled): the meadow a muted blue-grey green that sits under a navy overlay without reading olive; relief, texture detail and the turf's palette still legible; snow and puddles keep their own colours.
- **Saturation 0**: a monochrome ground with all its relief and detail, the snow-covered patches unchanged, wet ground still wet (the meadow keeps its canopy behaviour).
- **Tint 0.5**: the same ground at half the albedo, nothing else changed.
- **A palette blend** written every frame: a smooth glide, no hitch, no compile.

## Validation targets

The issue's acceptance criteria, each with how it is checked:

1. **Default identity** — "With `TerrainLook::default()`, a render … matches `0f5f71c` within 0 DN": captures of `terrain_tour` (`hills_eye`, `hills_down`, `hills_snow`, `hills_rain`, `dunes`) against the base build, and a CPU mirror test that the default operator is the identity bit for bit. On rock, compare structurally (`AGENTS.md`: the rock's cell networks flip on any change of the register schedule).
2. **Tint 0.5** — "a flat, sun-lit grass patch's linear albedo … is half of the default's, within 1 %": the CPU mirror, exact; the lit frame is checked for the expected darkening (the lit value is not half: the specular reflection, which the albedo does not scale, is a large share of a dark grass's radiance).
3. **Saturation 0** — "the bare ground's hue spread over a near patch is below 0.01, while a snow-covered patch's colour is unchanged": the mirror (zero chroma); captures of `hills_eye` at saturation 0 (chroma of the ground band) and of `hills_snow` at saturation 0 against the default (the snow band unchanged).
4. **No compilation** — "Writing a new `TerrainLook` every frame for 240 frames logs no pipeline or shader compilation after the first frame": a GPU test that writes a new look every frame and counts the render world's cached pipelines before and after.
5. **README** — the Terrain section names `TerrainLook`.

## Deviations from this plan

None from the issue's API. The order of the two operations (saturation, then tint) and the canopy read before the grade are this note's (Model adopted 2-3), with the reasons there.

## Pending index

- Nothing new to index: RTR4 is in the corpus; the issue is a web page.
