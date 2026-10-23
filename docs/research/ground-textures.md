# Ground textures: PBR detail maps for the terrain's sand, soil and grass

Underfoot, the terrain is a procedural noise: the meadow is a soft green felt (`/tmp/i45/before/hills_down.png`), the sand a beige blotch (`dunes_down.png`), and from 12 m the hills read as a low-poly game (`hills_12.png`). Nothing at the scale of a blade, a grain or a clod. This note adds **PBR detail textures** — albedo, height, normal, roughness and occlusion — to the terrain's ground layers, sampled without visible tiling and blended by height, while the crate's procedural layers keep the macro colour. The textures are data a game supplies; the crate's own default is neutral and renders exactly as before, and the examples use a set generated with golem.

## Sources

- **Andersson, J. (2007)**, *Terrain Rendering in Frostbite Using Procedural Shader Splatting*, SIGGRAPH 2007 course "Advanced Real-Time Rendering in 3D Graphics and Games" (corpus `10.1145_1281500.1281668`): "combine low-resolution unique color maps for low-frequency details with multiple tiled detail maps for high-frequency details that are blended in at certain distance to the camera"; materials masked procedurally by slope with detail added by an overlay blend; "having multiple detail mask textures together with all other textures quickly eat performance and texture samplers", so channels are reused.
- **Heitz, E., Neyret, F. (2018)**, *High-Performance By-Example Noise using a Histogram-Preserving Blending Operator*, Proc. ACM Comput. Graph. Interact. Tech. 1(2), DOI `10.1145/3233304` (HAL `hal-01824773`, read): partition the texture plane on a triangle grid and blend three randomly offset patches per pixel, "only 3 texture fetches"; linear blending of patches "introduces non-stationary contrast that reveals the grid pattern", which histogram preservation removes.
- **Mikkelsen, M. S. (2022)**, *Practical Real-Time Hex-Tiling*, JCGT 11(2):77-94 (`jcgt.org/published/0011/03/05`, read): hex-tiling without the histogram pre-processing — barycentric weights exponentiated (γ = 7), modulated per sample by a metric δ (luminance for colour, the slope's sine for normals, β = 0.6) and renormalised, then a Perlin S-curve with a contrast parameter `r`; "randomized rotation" of each hex tile, "a particularly useful feature" for patterns with orientation cues; normals are blended as height derivatives, since "the derivative is a linear operator"; sampling must use explicit gradients because of the per-tile offsets.
- **Akenine-Möller, T. et al. (2018)**, *Real-Time Rendering* 4th ed., ch. 6 (corpus `book_real_time_rendering_4th_akenine`): bump and normal mapping vary the shading normal per texel; a detail texture's roughness should differ from its neighbour's ("the mortar is supposed to be matte, whereas the bricks are glossy"); GPU trends make procedural texturing cheaper relative to memory.
- **The crate's own rules**: the ground's mean colour does not move (`ground-colour.md`); a detail is band-limited by the pixel's footprint on the surface (`detail.md`); the meadow's palette and the layer weights (`terrain.md`, `ground-colour.md`).
- **golem** (`generate_image`, FLUX.2 klein 9B on bmb): a test prompt gave a photographic top-down meadow at 1024² in about 20 s — tufts, bare soil, clover, dry stems, flat light (`~/trellis/output/images/i45_meadow_test.png`) — but with a radial composition (blades pointing out from a bare centre) that a repeated tile would show. golem has no texture or PBR-map tool in this session (`list_image_models`: FLUX.2 klein, dev, schnell, qwen, z-image, ernie, ideogram), so the maps are derived from the generated albedo.

## Model adopted

1. **Detail maps on top of the procedural layers** (Andersson). The terrain keeps its layer weights, its per-layer colours, the meadow's palette and the large fields; the texture adds what they lack: the high frequencies. Its albedo is stored as a **multiplier with mean 1** (the texture over its own blurred self), so the frame's mean colour does not move and a mip far enough away averages to exactly no change — the far terrain is untouched without a fade.
2. **One array texture, six layers** — for each of sand, soil and grass, one layer of RGB albedo detail and A height, and one of RG tangent-space normal (OpenGL convention, `+v` up), B roughness detail (mean 1) and A ambient occlusion. One texture and one sampler: Bevy's view group already binds up to about fifteen sampled textures on WebGPU, whose limit per stage is sixteen.
3. **A neutral default.** The crate inserts a 1×1×6 array whose texels decode to exactly no change (albedo and roughness ×1, a flat normal, equal heights, no occlusion). A game that supplies nothing renders as before; one that supplies a set gets textures. One shader path, two data sets.
4. **Hex-tiling** (Heitz & Neyret; Mikkelsen): the ground's world XZ, over the layer's tile size, on a triangle grid; three randomly offset and **rotated** samples, blended with Mikkelsen's exponentiated, luminance-modulated weights and his S-curve. The normal samples are rotated back and blended as height derivatives. Every fetch takes explicit gradients (`textureSampleGrad`), which also keeps the sampling legal in the shader's per-layer branches.
5. **Top-down projection.** The ground layers lie on slopes under about 40° (the rock takes over above), where a top-down projection stretches by at most 1/cos 40° = 1.3; the rock's faces are not part of this slice.
6. **Height blending.** The sampled heights reshape the layer weights, `w_i ∝ w_i · exp(k (h_i − ½))`, so grass grows in the soil's hollows and sand fills them, instead of a cross-fade; with equal heights the weights are unchanged, so the neutral default changes nothing.
7. **Normal, roughness and occlusion.** The detail normal adds its slope to the ground's relief (the material's `grad`); the roughness detail multiplies the layer's roughness; the occlusion multiplies the diffuse occlusion. Snow, wetness and puddles (`weather_surface`) act on the textured result as they do now.
8. **The procedural grain is shared.** The turf's procedural grain (`ground_detail`) is the default set's whole detail; a texture set carries a `grain` share (1 for the neutral set) so a photographic set does not double the near field with the felt.
9. **The texture set is a resource**, `GroundTextures { image, tile_m, grain }`, which a game inserts; the crate binds it to the terrain material. The examples load a set from `assets/ground/` (excluded from the published crate), generated with golem and packed by `tools/ground-textures/`, which takes either generated albedos or a standard PBR set (albedo, normal, roughness, occlusion, height) — a stylised set is a stylistic variant of the ground.
10. **The prep tool** makes an albedo tileable with a variance-preserving cross-fade against its half-period offset (Heitz & Neyret's insight: blended Gaussian-like patches keep their contrast when the blend preserves variance), removes the low frequencies (a generated photo's vignetting and composition), and derives height (band-passed luminance), normal (the height's slope), roughness (darker cavities rougher) and occlusion (the cavity depth) where a set does not supply them.

## Parameters

| Name | Value | Basis |
|---|---|---|
| tile size: sand, soil, grass | 1.5, 2.0, 2.0 m | a 1024² tile at 1.5-2 mm per texel, a grain to a tuft; at 1.7 m eye height the near footprint is 1-3 mm |
| hex tiles per texture tile | Mikkelsen's `2√3` grid scale | Mikkelsen 2022 |
| γ, β | 7, 0.6 | Mikkelsen 2022 |
| contrast `r` | 0.75 | Mikkelsen's Fig. 3: sharper than 0.5 without hex borders |
| height blend `k` | 8 | Deviation 2 |
| detail normal strength | from the texture | the set's own normal map |
| `grain` share, golem set | 0.3 | Deviation 3 |

## GPU mapping

- `TerrainExtension` gains `#[texture(107, dimension = "2d_array")]` and `#[sampler(108)]`; `TerrainMaterialParams` gains the three tile sizes and the grain share (re-prepared when the resource changes, as the material's other uniform values are).
- Per pixel, for each ground layer whose weight exceeds 0.01: one triangle-grid lookup and three hashes, then 3 × 2 fetches (albedo-height and normal-roughness-occlusion). Typically one or two layers are live, so 6-12 fetches; the rock (above 40°) samples none.
- Gradients for `textureSampleGrad` are the derivatives of the projected coordinate taken once in uniform control flow, rotated per tile.
- The neutral default is a 1×1×6 `Rgba8Unorm` array inserted under a constant handle (`uuid_handle`), so `GroundTextures::default()` is valid before any game code runs.

## What great looks like

- **Underfoot (1.7 m, looking down):** a meadow of blades and tufts with soil between them and a little litter; sand with grains, small ripples and scattered fragments; soil with crumbs and pebbles. Real relief in the light — the sun rakes the grains and blades — and no repetition in the frame.
- **At eye level and 12 m:** the meadow's colour field (the crate's palette, the large fields) is unchanged, with texture under it; transitions between grass, soil and sand follow the heights (grass in hollows, sand in cracks), not cross-fades.
- **From 80 m:** exactly the procedural terrain (the mips average to no change); no tiling pattern, no hex grid, no shimmer at mid distance.
- **Dawn, dusk, overcast:** the detail normal reads at grazing light and does not turn into noise; overcast keeps the micro-occlusion.
- **Weather:** wet, puddled and snow-covered ground act on the textured surface as before.
- **Without a texture set:** byte-identical to the baseline, up to the GPU's noise.

## Validation targets

1. **Neutral identity** (mirror): the neutral texel decodes to an albedo multiplier of exactly 1, a flat normal, roughness ×1 and occlusion 1; the height blend with equal heights returns the input weights; captures with the default set match the baseline (plains and hills within GPU noise).
2. **Hex weights** (mirror): the three weights are non-negative and sum to 1 everywhere; the vertex offsets are a pure function of the vertex, so two pixels of one hex agree.
3. **No visible tiling** (capture): `hills_down`, `plains_down`, `dunes_down` show no repeated feature at the tile period (an autocorrelation peak at the tile's offset below the baseline grain's), and no hex border.
4. **Captures** (`/tmp/i45/`): `hills_eye`, `hills_down`, `hills_12`, `hills_aerial`, `hills_dawn`, `hills_dusk`, `hills_overcast`, `hills_seed7`, `plains_eye`, `plains_down`, `dunes_eye`, `dunes_down`, `scree_down`, `talus_b` judged GREAT; `mtn_foot` unchanged on the rock.
5. **Cost:** the hills' opaque pass at eye level measured before and after, interleaved; the trade logged.
6. **Budget:** one texture and one sampler added to the terrain material; the wasm build passes.

## Deviations from this plan

1. **Detail maps, not absolute albedo.** Standard PBR sets carry an absolute albedo; this crate's colour design (the per-layer colours, the palette, the climate's dry and wet tints) owns the low frequencies, and the frame's mean colour must not move (`ground-colour.md`). So a set's albedo is normalised to mean 1 by the prep tool (Andersson's colour map + detail map split).
2. **Height blend exponent `k` = 8.** No source gives a number; the form `w · exp(k (h − ½))` is the common game practice of comparing layer heights (the "height-based blending" of splat maps), chosen because it is the identity for equal heights. 8 makes a 0.25 height difference a factor of 7.4, a clear but not hard edge.
3. **The procedural grain's share falls to 0.3 with a photographic set.** At 1 the felt noise lies over the blades and muddies them; at 0 the near field loses the crate's variation between tiles. 0.3 keeps the second without the first.
4. **One weight set for both texture layers.** Mikkelsen weights normals by their slope and colours by their luminance; here the weights are computed once from the albedo-height sample and reused for the normal-roughness-occlusion sample, so a feature's colour and its relief cannot come from different tiles.

## Pending index

- Mikkelsen 2022 (JCGT, non-DOI): read from the URL, not ingested (67 MB PDF).
- Heitz & Neyret 2018: read from HAL, not ingested (`hs paper download` found no copy).
