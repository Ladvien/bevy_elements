# Terrain textures: standard PBR maps set on the terrain

The terrain's ground layers take PBR detail textures since iteration 46 (`docs/research/ground-textures.md`), but a game cannot hand them over the way it hands any other material its maps. The set is one crate-wide resource (`GroundTextures`) holding one pre-packed KTX2 array that only an offline Python tool (`tools/ground-textures/prep.py`) can make, and that tool reads albedos only: a real PBR set's normal, roughness, occlusion and height maps are thrown away (ground-textures.md, Deviation 14). This note makes the textures **settable on the terrain**: a component on the `Terrain` entity that takes ordinary maps per layer — a base colour and, optionally, a normal, roughness, occlusion and height map, as a PBR set or a glTF material carries them — which the crate packs itself, at runtime, into the format the shader already reads. A set can also bring its own colour, so a stylised set is a stylistic variant of the ground.

## Sources

- **Unity, *Terrain Layers*** (Unity 6 manual, `docs.unity3d.com/Manual/class-TerrainLayer.html`, read 2026-10-23): the shipping precedent for a per-layer, settable terrain material. A Terrain Layer "holds Textures and other properties that the Terrain's Material uses": a **Diffuse** (the base colour), a **Normal Map** with a **Normal Scale** ("0 … has no effect on lighting", "2 … twice as pronounced"), a **Mask Map** (R metallic, G ambient occlusion, B height, A smoothness, with channel remapping), and tile **Size** and **Offset**. Without a normal map, "the Terrain uses the normals generated from the Terrain heightmap". HDRP takes up to eight layers in one pass.
- **Khronos, *glTF 2.0 Specification*** (`registry.khronos.org/glTF/specs/2.0/glTF-2.0.html`, §3.9, read): the conventions every PBR set follows. The base colour texture "MUST be encoded with the sRGB transfer function"; in the metallic-roughness texture "roughness values are sampled from the G channel" and "metalness values are sampled from the B channel"; "occlusion values are linearly sampled from the R channel"; the tangent-space normal texture uses "the convention +X is right and +Y is up. +Z points toward the viewer", and "MAY additionally contain a scalar scale value that linearly scales X and Y components of the normal vector".
- **Bevy 0.20** (vendored source): `StandardMaterial::flip_normal_map_y` for normal maps in the DirectX (`−Y`) convention (`bevy_pbr/src/pbr_material.rs:421`); `Image::get_color_at` decodes the 8- and 16-bit, float, one-, two- and four-channel formats and returns `Color::srgba` of the stored value for an sRGB format and `Color::linear_rgba` of it otherwise (`bevy_image/src/image.rs:1893`), so the stored value is recoverable whatever sRGB flag the game loaded a map with.
- **Kovesi, P. (2010)**, *Fast Almost-Gaussian Filtering*, DICTA 2010, DOI `10.1109/DICTA.2010.30` (author's copy, `peterkovesi.com/papers/FastGaussianSmoothing.pdf`, read): "Repeated filtering with averaging filters can be used to approximate Gaussian filtering … at a fixed cost per pixel independent of filter size"; "three repeated averagings achieve a passable approximation to a Gaussian and beyond four repeated averagings the approximation becomes very good". For `n` passes the ideal width is `w_ideal = √(12σ²/n + 1)`; with `w_l` the odd width below it and `w_u = w_l + 2`, `m = (12σ² − n w_l² − 4 n w_l − 3n) / (−4 w_l − 4)` passes of `w_l` (rounded) and `n − m` of `w_u` give σ; the passes' variances add, `σ² = (m w_l² + (n − m) w_u² − n) / 12`.
- **Wells, W. M. (1986)**, *Efficient synthesis of Gaussian filters by cascaded uniform filters*, IEEE TPAMI 8(2), DOI `10.1109/TPAMI.1986.4767776`: the origin of the cascade (catalog record; not downloaded, see Pending index).
- **The crate's own ground-textures note** (`docs/research/ground-textures.md`): the packed format (six layers: albedo detail and height; normal, roughness and occlusion details), the hex-tiling, the height blend, and the derivations from an albedo with their measured deviations — the detail as the albedo over its own blur (σ = n/12), de-lighting at 0.65 (Deviation 7), the relief as a mean tilt (Deviation 8), occlusion `1 − 0.2·cavity` over its mean (Deviation 9), mips that average the decoded values (Deviation 10), the neutral derived roughness (Deviation 17).
- **Andersson, J. (2007)**, *Terrain Rendering in Frostbite Using Procedural Shader Splatting* (corpus `10.1145_1281500.1281668`): low-frequency colour from the terrain's own colour, high-frequency detail from tiled maps — the split that lets a set be reduced to detail, and lets its colour be a separate choice.
- **ambientCG** (`ambientcg.com`, CC0 1.0): standard 1K sets with `Color`, `NormalGL`, `NormalDX`, `Roughness`, `AmbientOcclusion` and `Displacement` maps; `Grass004` (a fine grass sward) and `Ground037` (mossy earth with grass and fine debris) are the examples' standard set.

## Model adopted

1. **The textures are a component on the terrain.** `TerrainTextures { sand, soil, grass, grain }` on the `Terrain` entity: each ground layer an `Option<GroundLayerTextures>`, `None` leaving it procedural. Inserting, changing or removing it at runtime re-textures the ground; while a new set is being packed the previous one keeps rendering, so a swap never flashes to the procedural ground. The crate has one terrain (`VoxelGround` owns it too), so the component is that terrain's.
2. **A layer is a PBR set** (Unity's Terrain Layer, glTF's material): `GroundLayerTextures { albedo, normal, roughness, occlusion, height, tile_m, colour, normal_scale, flip_normal_map_y, relief_deg, contrast, tileable }`. The base colour is required, every other map optional. `GroundLayerTextures::new(albedo, tile_m)` is a de-lit, tileable PBR set whose colour is its own; `.photograph()` marks a photograph or a generated image (golem's): its lighting is de-lit (`contrast` 0.65), it is made tileable, and its colour is not taken (`colour` 0), because a photograph's mean colour is its light's and its camera's.
3. **Maps are read as their stored values, by the glTF channels.** The base colour is sRGB-encoded (glTF), so it is decoded to linear whatever sRGB flag the image carries; every other map is data. The normal is `2c − 1` in RG(B), +Y up (OpenGL, glTF) unless `flip_normal_map_y`; the roughness is the G channel and the occlusion the R channel (glTF), so one glTF metallic-roughness or packed occlusion-roughness-metallic image serves both, and a greyscale map serves either; the height is the R channel.
4. **Packed by the crate, off the frame.** The maps are read from the images' CPU data once they have loaded, resampled to a common `resolution` (a power of two, 1024 by default: each map is halved by 2 × 2 averages while it is at least twice that, then resampled bilinearly with wrap), and packed in a task on Bevy's `AsyncComputeTaskPool` into the six-layer `Rgba8Unorm` array with every mip level that the shader already samples (ground-textures.md, Model adopted 2). A layer without a set is packed neutral and its tile is zero, so it is not sampled.
5. **Derived maps are the prep tool's, in the crate.** A layer without a normal derives the height from the base colour's band-passed luminance, the normal from the height's slope at the mean tilt `relief_deg`, the occlusion from the cavities, and a neutral roughness, as `prep.py` did (ground-textures.md, Model adopted 10 and Deviations 7-9, 17). The Gaussian blurs are Kovesi's cascade of four box filters with wrap-around, so a σ of 85 texels costs what one of 1 does.
6. **Given maps become detail.** The crate keeps the ground's absolute values — its layers' roughness (which the weather lowers), its light — and the set brings their variation: the roughness map becomes a multiplier `r / mean(r)`, the occlusion `ao / mean(ao)` (both mean 1, so the ground's mean roughness and light do not move), the height is band-passed and normalised as a derived one is (the layer blend compares heights about ½), and the normal is used as authored, its XY scaled by `normal_scale` (glTF's and Unity's scale).
7. **The set's colour is a share.** The base colour is reduced to a detail with mean 1 (ground-textures.md, Model adopted 1); its mean linear colour replaces the layer's own base colour by `colour` (0 the crate's colour, 1 the set's). The replacement is made in the layer's base colour, not in the detail, so it holds at every distance — where the detail is fetched and where it has faded out — and the crate's palette and fields still vary it (their multipliers have mean 1, `ground-colour.md`). The colours are uniform lanes compiled only into the pipeline with a set (the `GROUND_TEXTURES` shader def); without one the shader keeps its constants, unchanged.
8. **One path.** `GroundTextures`, its KTX2 loader, `assets/ground/ground.ktx2` and `prep.py` are removed: the examples hand the golem albedos to `TerrainTextures` as photographs, and select a standard CC0 set or none with `ELEMENTS_GROUND_TEXTURES`.

## Parameters

| Name | Value | Basis |
|---|---|---|
| packed resolution | 1024 (power of two, 16-4096) | the iteration-46 set's 1024²; 1.5-2 mm texels at a 1.5-2 m tile |
| blur | four box passes, wrap | Kovesi 2010 ("very good" beyond four) |
| detail low-pass σ | resolution / 12 | ground-textures.md (prep.py) |
| height band | σ 1.2 to resolution / 24, normalised to ½ ± 4 std | ground-textures.md (prep.py) |
| cavity σ | resolution / 96; occlusion `1 − 0.2·cavity`, over its mean | ground-textures.md, Deviation 9 |
| tileable cross-fade band | 0.18 of the tile | ground-textures.md (prep.py) |
| photograph `contrast` / `colour` | 0.65 / 0 | ground-textures.md, Deviation 7; Model adopted 2 |
| PBR set `contrast` / `colour` | 1 / 1 | a PBR base colour is already de-lit (glTF's base colour) |
| `relief_deg` (derived normals) | 10° (examples: 10° soil, 14° grass) | ground-textures.md, Deviation 8 |
| `normal_scale` | 1 | glTF, Unity |

## GPU mapping

- The shader's sampling, the array's layout and its binding (`TerrainExtension::ground`, bindings 107-108) do not change.
- `TerrainMaterialParams` gains three `Vec4`s: the sand's, the soil's and the grass's base colour, computed on the CPU (`mix(crate colour, set mean, colour)`), read only under `GROUND_TEXTURES`: by the textured layer blend, the far ground's plain colour, the trench and the scree's soil. A WESL function per layer returns the lane under `@if(GROUND_TEXTURES)` and the constant otherwise, so the no-set pipeline is the same code.
- The packing runs on the CPU, once per change, in a task; the material binds the result when the task finishes (as `ground_inputs` waits for an image today). Its cost is measured, not budgeted per frame.
- Memory: the array as before, 1024² × 6 × 4 B × 4/3 = 33.6 MB on the GPU. The images a game loads stay its own; the crate keeps no copy beyond the packed array.

## What great looks like

- **The golem set through the new path** renders as it did through the KTX2 (ground-textures.md's GREAT captures): the same blades, crumbs and pebbles, the same colour field; differences only where the box cascade differs from a Gaussian, below what the eye finds side by side.
- **A standard CC0 set** reads as that material: its own grass and earth colours at every distance, with no ring where the detail stops being fetched; its relief lit from the correct side under a raking sun (toward and away from it, dawn and dusk) — no inverted bumps, no flat paste; no tile repetition, no hex seam; snow, wetness and puddles act on it as on the procedural ground.
- **A swap at runtime** lands without a flash to the procedural ground or a hitch in the frame.
- **Without a set**: the old procedural ground, unchanged.

## Validation targets

1. **The packer is the prep tool**: the golem albedos packed by the crate against `assets/ground/ground.ktx2` (prep.py's), level 0, every channel of every layer: the mean absolute difference and the 99th percentile reported; the visual comparison is target 4.
2. **Unit tests** (permanent, consumer-visible): the box cascade's σ within Kovesi's bound of the target and its mean preserved; every packed level's albedo and roughness and occlusion detail average to 1 (the ground's mean does not move); a constant base colour packs a neutral detail and reports its own colour; an OpenGL normal map and the same map in DirectX with `flip_normal_map_y` pack the same normals, and both match the normal derived from the same height's slope in sign; a layer left out is neutral and its tile zero; a 2048² map packs to 1024².
3. **Settable at runtime** (headless test with `Assets<Image>`): `TerrainTextures` inserted on a terrain packs an array; changed, it packs a new one while the old stays bound; removed, the ground is procedural.
4. **Captures** (`/tmp/i50/`): the baseline's `hills_eye`, `hills_down`, `hills_12`, `hills_aerial`, `hills_dawn`, `hills_dusk`, `hills_dusk_away`, `hills_overcast`, `hills_rain_down`, `hills_snow`, `hills_seed7`, `plains_eye`, `plains_down`, `dunes_down`, `scree_down`, `mtn_foot` with the golem set (matching the baseline) and with the CC0 set (GREAT on its own), and a runtime swap sequence.
5. **Cost**: the opaque pass with each set and without one, against the baseline, interleaved; the packing time per set.
6. **The wasm build passes** (the task pool runs the packing on the main thread there).

## Deviations from this plan

1. **Standard maps are reduced to detail.** A PBR set's roughness and occlusion are absolute; here they become multipliers with mean 1 (Model adopted 6), because the ground's roughness is the crate's (the weather lowers it; the meadow's wet canopy, `wet-ground.md`) and its mean light must not move (ground-textures.md, Deviation 9). The base colour's absolute value is kept only by its `colour` share.
2. **A box cascade, not a Gaussian.** `prep.py` used SciPy's Gaussian; four box passes approximate it (Kovesi), so the packed detail differs from the KTX2's slightly (Validation target 1).
3. **Bilinear resampling, not Lanczos.** `prep.py` resized with Lanczos; the crate halves by averaging and then resamples bilinearly. The examples' sources are already 1024², where neither applies.
4. **The examples' standard set leaves the sand procedural**, as the golem set does (ground-textures.md, Deviation 15): the sand waits for a ripple field.

## Pending index

- Wells 1986 (DOI 10.1109/TPAMI.1986.4767776): paywalled, no provider copy; the cascade's formulas are Kovesi's, read from his copy.
- Kovesi 2010: read from the author's URL, not ingested (`hs paper download` found no copy).
- glTF 2.0 and Unity's manual: specifications read from their URLs.
