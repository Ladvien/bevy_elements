# Cloud shadow and illuminance at a point

A gameplay query for the cloud shadow at a world position, and the horizontal illuminance there: a stealth system, a solar panel, a plant, a creature that seeks shade, or an AI that sees farther in sunlight can ask whether a point is under a cloud's shadow and how much light reaches it. Until now the cloud shadow map was GPU-only; gameplay saw only the camera-wide mean `CloudShadows::camera_transmittance`.

## Sources

- Hillaire, S. (2016). *Physically based sky, atmosphere and cloud rendering in Frostbite*, §5.9 (not in the corpus; read for `docs/research/clouds.md`): the cloud shadow is a 2D transmittance texture over a flat world, sampled by projecting the receiver along the light. This crate's `cloud_shadow` kernel is that map (`docs/research/clouds.md`, "Cloud shadow map and the sun's illuminance").
- Kasten, F., Czeplak, G. (1980), *Solar Energy* 24:177, read second-hand as recorded in `docs/research/clouds.md` Sources: `GHI/GHI_clear = 1 − 0.75 N^3.4`, overcast floor 0.25. The crate's split `direct = T`, `diffuse = 0.25 (1 − T)` (`src/celestial/mod.rs`, `CloudLight`) is the point form of it.
- Corpus `s2021-advances-pdf-66-mb` (SIGGRAPH 2021 *Advances*): the direct light is "scaled by RGB light intensity (modulated by dynamic cloud shadow)" separately from the static sky visibility, i.e. the cloud shadow is a per-receiver multiplier of the beam, not a scene constant. Primary.
- Corpus `s2025-advances-materials-3` (SIGGRAPH 2025 *Advances*): sun shadow from a camera-centred shadow map, "combined with cloud shadows" per receiver; the cloud shadow is queried at the receiver's position, outside the map treated as unshadowed.

## Model adopted

1. The `cloud_shadow` kernel also writes every fourth texel's direct transmittance $T$ (texel `4k + 2` on each axis) into a 256² `f32` storage buffer: a point sample of the map every 31.25 m over the same 8 km square. Cloud shadows have structure of hundreds of metres (cumulus 0.3–2 km), so a 31 m grid resolves their edges to one cell.
2. The readback (4 Hz, `ElementsReadback`) copies it with the map's origin and the light direction it was generated for, captured at request time.
3. `Environment::cloud_shadow_at(P)` projects $P$ to the plane $y = 0$ along the generation direction $\hat l$ (elevation floored at 5.74°), exactly as the map's consumers do: $\mathbf q = P_{xz} - \hat l_{xz} P_y / \hat l_y$, and returns the bilinear $T$ there; outside the map $T = 1$ (as the material does). `None` before the first readback.
4. `Environment::illuminance_at(P)` is the clear-sky horizontal illuminance of the sun and the moon (USNO factor at the refracted altitude, `clear_sky_illuminance`) times the point form of Kasten & Czeplak, $0.25 + 0.75\,T(P)$, plus the 0.0005 lx floor: the same function the exposure meters with the camera mean, evaluated with the local $T$. Before the first readback $T$ is the cover-only beam $1 - c^{3.4}$.

Neither query sees terrain, buildings or Bevy's shadow maps: it is the cloud's shadow only.

## Parameters

| Name | Value | Source |
|---|---|---|
| Query grid | 256², 31.25 m cells, same square as the shadow map | 4 × the map's 7.8125 m texel; cloud structure ≥ 300 m (clouds.md) |
| Sample | texel `4k + 2`, centre at `origin + (4k + 2.5) · 7.8125 m` | point sample (no reduction pass) |
| Overcast floor | 0.25 | Kasten & Czeplak 1980 |
| Refresh | `ElementsReadback::interval` (4 Hz) | existing readback |

## GPU mapping

One extra `storage_buffer` binding in the shadow kernel's bind group (now 2 storage buffers, 1 storage texture in that stage) and one store per 16 texels inside the existing dispatch: no extra pass. Readback: 256 KB every 0.25 s (1 MB/s). Measured (`bench_elements`, High, 3840×2160, `PartlyCloudy`, `ELEMENTS_TERRAIN=none`, minimum over the diagnostic history, RTX 3090 at P2 1590–1725 MHz): `elements_clouds_shadow` 0.083 ms before, 0.087 ms after.

## Validation targets

Inventory: (a) the cloud shadows on the ground under `Clear` / `PartlyCloudy` cumulus; (b) probe markers in a new example `cloud_shadow_probes`, unlit, coloured from the query (warm yellow: `cloud_shadow_at > 0.5`; blue: shadowed), placed on the ground; (c) the shadow edge where a probe row crosses it.
- (a) At 1× from a camera 120 m up at −25° pitch, `PartlyCloudy` at 11:00: distinct darker patches of hundreds of metres on the ground, edges soft over a few tens of metres, no blockiness at 7.8 m texels. From a grazing view (6 m, −3°) the patches read as bands with depth.
- (b) At 1× every probe that stands in a ground shadow patch is blue and every probe in sunlight is yellow; mismatches only within one 31 m cell of an edge. From above and at grazing.
- (c) In a 2×/4× crop at an edge, the colour change of the probes falls on the ground's shadow edge within one probe spacing.
- Headless: `cloud_shadow_at` is `None` before a readback; on a synthetic snapshot it returns the stored value at a cell centre, 1 outside the map, and projects a raised point along the light; `illuminance_at` under `T = 1` equals the clear-sky value and under `T = 0` a quarter of it.
- GPU: under a partly cloudy sky (cover 0.4) the query at the grid's sample texels equals the shadow map's transmittance there (mean absolute difference < 0.02, with both lit and dark samples), and the darkest point's illuminance is a quarter of the brightest's.

## Deviations from this plan

None to defaults. The coarse grid is a point sample, not a box filter: a 31 m point sample of a field already smooth on 100 m scales; recorded so a later reduction pass has a reason to exist only if a test shows aliasing.

Validation changes made while building it: (1) the GPU target first compared the query with the camera mean under a uniform test slab; the shipped test compares it texel for texel with the shadow map under a real partly cloudy sky, which also checks the grid's placement (measured difference 0.0000 over 2 704 samples, 1 314 lit and 1 247 dark; illuminance 27 324–109 298 lx, ratio 0.250). (2) The example's marker spacing scales with the camera height (a third of it, at least 40 m): at the default 40 m a 560 m grid lay wholly inside or outside one cumulus shadow in every first capture. (3) The grazing target (a) is not met at 6–30 m: the nearest shadows of the `PartlyCloudy` sky over the scene's origin at 11:00 lie 300 m to 1 km out, where the aerial perspective adds about 95 DN (measured by tinting `t < 0.5` in `weathered.wesl` for one throwaway capture), so a shadow reads 8 % darker there; it is a property of the haze, not of the query, and is logged open in the loop log.

## Pending index

No new documents were added to the library; nothing to index. `hs status` failed in this shell ("No such device or address", no tty), so distill was not used; the corpus was searched by phrase (`cloud shadow`).
