# Wet ground: a porous surface darkens, a canopy deepens, and only a film is a mirror

In rain the crate's ground turns into a mirror. Every surface that `weather_surface` wets goes to roughness 0.1 at full wetness (`wetness.md`, Model adopted 3, the plan's "roughness lerp to 0.1"), so a meadow, a cliff and a dune all reflect the overcast dome like still water. Measured against the same views under the dry Overcast preset (`/tmp/i47/before`, rows below the horizon), rain makes every ground **brighter and greyer**, the opposite of what water does to a surface:

| region | overcast dry: luminance / median saturation | rain: luminance / median saturation |
|---|---|---|
| meadow, near (eye level) | 111.8 / 0.399 | 132.0 / 0.092 |
| meadow, 10–60 m | 116.6 / 0.397 | 187.3 / 0.032 |
| meadow, looking down | 108.0 / 0.396 | 113.7 / 0.150 |
| cliff | 102.9 / 0.067 | 140.6 / 0.031 |
| dunes, 10–60 m | 190.6 / 0.339 | 199.5 / 0.043 |

In the frames this reads as a choppy grey lake where the meadow was (`hills_rain`), hills that look snow-covered at dusk (`hills_rain_dusk`), a cliff of crumpled chrome foil (`mtn_rain`), and dunes silvered to white in the middle distance (`dunes_rain`). This note replaces the single roughness rule with Lagarde's porous rule, keeps the film for the surfaces that hold one, and gives the terrain's grass the canopy rule `bevy_flora` already uses for leaves.

## Sources

- **Lagarde, S. (2013)**, *Water drop 3b – Physically based wet surfaces* (v1.3), `https://seblagarde.wordpress.com/2013/04/14/water-drop-3b-physically-based-wet-surfaces/`, read in full on 2026-10-23. The porous rule, verbatim: `float factor = lerp(1, 0.2, (1 - Metalness) * Porosity); Diffuse *= lerp(1.0, factor, WetLevel); Gloss = lerp(1.0, Gloss, lerp(1, factor, 0.5 * WetLevel));` — "We then reuse this diffuse factor to get a blending weight between the material gloss and the water gloss. As water is smooth, we use 1.0. I use a factor of 0.5 on WetLevel to limit the specular boost." The film is a separate regime: "When porous material is water saturated or when we have a smooth or metal material, a thin layer of water accumulates on the top of the surface"; his summary maps wet (type 2) to the porous rule, "the thin layer of water which can be considered as drenched region (type 3)" to "a slight smoothing of the normal and a slight boost in specular", and puddles (type 4) to water. On saturation: wet albedo is a non-linear function of dry albedo, "with low albedos reduced more by wetting than high albedos. A consequence … is that wet surface color is more saturated than dry surface color" (his quote of Lu et al.); his takeaways: "The change due to water induce an increase saturation, a hue shift and a brightness decrease of the diffuse part and an increase smoothing of specular part" and "Only porous material are affected by rain".
- **Akenine-Möller, T., Haines, E., Hoffman, N. et al. (2018)**, *Real-Time Rendering*, 4th ed., §9.9 (corpus `book_real_time_rendering_4th_akenine`, line 10934): "The process by which many materials become darker when wet is the inverse of the liquid froth example. If the material is porous, water penetrates into spaces formerly filled with air. Dielectric materials have an index of refraction that is much closer to water than to air. This decrease in the relative index of refraction decreases the …" (the conversion cuts the sentence; it cites Jensen, Legakis & Dorsey 1999, *Rendering of Wet Materials*, their ref. [821]). The darkening is a subsurface effect of the *pores*, not a property of a surface film.
- **`bevy_flora` 0.46.0**, `src/shading/common.wesl:535-557` (`wet_on`, the sibling this crate drives through its `flora` feature): foliage `grey + (rgb − grey)·(1 + WET_LEAF_SATURATE·wet)` with `WET_LEAF_SATURATE` = 0.25 and its roughness untouched (`mix(rough, min(rough, 0.5), wet·(1 − leaf))`); bark `pow(rgb, 1 + 0.15 wet) · mix(1, 0.55, wet)` and roughness toward `WET_WOOD_ROUGHNESS` = 0.5.
- **Chen, L., Liu, C., Zhang, L., Zou, R., Zhang, Z. (2017)**, *Variation in Tree Species Ability to Capture and Retain Airborne Fine Particulate Matter (PM2.5)*, Scientific Reports (published 2017-06-05), DOI `10.1038/s41598-017-03360-1` (corpus), on rain washing particulate matter from leaves: "leaf surfaces features like wax layer, trichomes and other protrusions can result in different contact angle between water droplet and different leaf surfaces. These factors create different water-repellent per[formance]" (line 57). A blade's wax beads the water; it holds no film.
- **Tatarchuk, N. (2006)**, *Artist-directable real-time rain rendering in city environments* (corpus `s2006-advances-materials-5`): raindrops "generate ripples and splashes in the puddles" (line 309); §3.6.3 "We simulate raindrops splashing when hitting solid objects", the examples a rooftop ledge and a store awning (lines 360-368). Rings belong to water, splash sprites to hard surfaces; neither is put on vegetation.
- **Xiao, Q., McPherson, E. G., Simpson, J., Ustin, S. (1998)**, *Rainfall Interception by Sacramento's Urban Forest*, Journal of Arboriculture 24(4), corpus `10.48044_jauf.1998.028` (lines 31-56): "Gross precipitation is either intercepted by canopy leaves, branches, and trunk, or it falls directly to the ground … Intercepted water is stored temporarily on canopy leaf and bark surfaces, eventually drips from leaf surfaces". A drop that meets a canopy is caught by a leaf; there is no free water surface for a ring.
- `docs/research/precipitation.md` ("Splashes", Parameters "Splash ring radius / lifetime" 0.15 m / 0.25 s from plan 2.2, Deviation 12 the ring's profile, Validation 13 "splash rings on the ground"), `src/precipitation/precip.wesl:110-121,246-277` (`ground_at` reads the terrain's surface plus water; every ground impact within 20 m writes a ring).
- Repository: `docs/research/wetness.md` (Model adopted 1–4, Parameters, Deviations 2, 10; its Validation 6 and 9 encode the 0.1 rule), `src/surface_cover.wesl:44-81,161-189` (`weather_surface`, `porosity`), `src/weathered/mirror.rs:11-24,119-150` (its CPU mirror), `src/terrain/render.wesl:556-570,661-689` (the layer weights `w` = rock, sand, soil, grass; the trench; the call), `src/terrain/ground.wesl:201-205`, `src/weathered/weathered.wesl:187`, `tests/gpu_weathered.rs:239-302`.
- Corpus searches (2026-10-23): `wet surfaces` 6 documents, `water film` 6, `Rendering of wet materials` 2 (the RTR4 bibliography and the 2006 course notes, both already used by `wetness.md`); `hs distill search "effect of dew or rain droplets on leaf and canopy reflectance in the visible"` returned no measurement of a wet canopy's visible reflectance (the hits are canopy greenness and water-content remote sensing).

## Model adopted

1. **The porous rule (Lagarde 3b), for every surface `weather_surface` wets.** With porosity $\phi$ as before (`saturate((r − 0.5)/0.4)(1 − metallic)`, `wetness.md` Deviation 2) and $w$ = `min(max(wetness, puddle), 1)`, Lagarde's gloss blend toward water is $1 - \mathrm{lerp}(1, \text{factor}, 0.5\,w)$ with $\text{factor} = 1 - 0.8\,\phi$, i.e. $0.4\,w\,\phi$. With the crate's existing mapping of his gloss to $1 - r$ ([INFERENCE] in `wetness.md`, kept so porosity and gloss use one convention):
   $$r_p = r\,(1 - 0.4\,w\,\phi).$$
   A fully porous rough surface (r = 0.9–0.95) reaches **0.54–0.57** when soaked, a visible sheen, never a mirror. The albedo rule is unchanged ($\times(1 - 0.5\,w_e\,\phi)$, `wetness.md` Model 2).
2. **The film, where a film forms.** A smooth or metallic surface holds its water as a thin layer at once; a porous one only once it is saturated and more water arrives (the crate's "drenched" wetness above 1, which the rain streaks on walls reach, up to `DRENCHED_MAX` = 1.4). The film's weight is
   $$f = (1 - \phi)\,w + \phi\;\mathrm{saturate}\!\left(\frac{w_e - 1}{\texttt{DRENCHED\_MAX} - 1}\right),$$
   and the roughness is $\mathrm{mix}(r_p, \min(r_p, 0.1), f)$. For a smooth or metal surface ($\phi$ = 0) this is today's rule exactly; for the ground, whose wetness never exceeds 1, the film is zero and the porous rule alone applies; a drenched streak still reaches 0.1.
3. **The canopy (terrain only).** The grass layer is blades over soil; its wax beads the rain and holds no film (Chen et al.), and `bevy_flora` shades its wet leaves by **saturation alone**: $\mathbf c' = \mathrm{grey} + (\mathbf c - \mathrm{grey})(1 + 0.25\,w)$, the Rec. 709 luminance kept, the roughness untouched. The terrain knows its grass share: the layer weight $g$ = `w.w` less the trench's soil and the scree's stones and dirt ("a scree has no grass in it", `render.wesl`), $g\,(1 - \text{trench})(1 - \text{scree})$. Over that share the flora rule replaces the porous one: the albedo takes the saturation boost by $w\,g$, and `weather_surface` is passed the wetness $w\,(1 - g)$, so only the rest of the ground (soil, sand, rock, the trench, the scree) darkens and glosses. Puddles and snow are untouched: standing water and snow cover the blades whatever they are.
4. **What does not change.** Puddles (depth-driven, `wetness.md` Model 4), ripples, snow, the albedo rule, the wetness maps and every dry pixel. `GroundExtension` (`ground.wesl`) takes the shared porous rule but not the canopy rule: its base colour is the caller's and it has no grass share to read.
5. **Rain rings belong on water.** Today every rain impact on the ground within 20 m writes an expanding white ring (0.15 m, 0.25 s, `precipitation.md`), so with the ground no longer a mirror the meadow is littered with white circles (`/tmp/i47/r1/hills_storm.png`, dozens; `hills_rain_dusk.png`). A ring is the capillary wave of a drop on a free water surface (Tatarchuk: "ripples … in the puddles"); a drop on grass is caught by a leaf (Xiao et al.), on soil or sand it soaks in. So on a live terrain an impact rings only where the terrain holds standing water at least as deep as a puddle's start, `WetnessSettings::puddle_min_m` (2 mm, the depth at which the puddle mask, and its ripples, begin); a blocker's top face and the plane of a scene without a terrain, whose surface the kernel does not know, keep their rings as today.

## Parameters

| Name | Value | Basis |
|---|---|---|
| Porous gloss boost | 0.4 | Lagarde 3b: $0.5 \times (1 - 0.2)$, his "factor of 0.5 on WetLevel" times his full-porosity attenuation |
| Film roughness `WET_ROUGHNESS` | 0.1 | unchanged (`wetness.md`); now only for the film |
| Drenched range | 1 → `DRENCHED_MAX` 1.4 | unchanged (`wetness.md` Deviation 10) |
| Leaf saturation `WET_LEAF_SATURATE` | 0.25 | `bevy_flora` 0.46 `common.wesl:538` |
| Saturation luminance | Rec. 709 (0.2126, 0.7152, 0.0722) | `bevy_flora` `wet_on` |
| Canopy share | `w.w · (1 − trench) · (1 − scree)` | the terrain's own grass weight (`render.wesl:566-568`), less the trench and the scree band |
| Ring water depth | `WetnessSettings::puddle_min_m` (0.002 m) | the puddle mask's start (`wetness.md` Model 4); without the cover plugin its default |

## GPU mapping

- `surface_cover.wesl` `weather_surface`: the roughness line becomes the porous product and one `mix` with the film weight (a `saturate` and three multiply-adds). Same signature, same callers.
- `render.wesl`: the canopy share is one multiply of values already in registers; the saturation boost is a dot product and a multiply-add, only when `wet > 0`. No new binding, buffer, texture or uniform.
- `precip.wesl`: the ground hit's water depth is read from the `terrain_cells` the kernel already binds (bilinear, as `ground_at`), and the ring is written only when the ground was hit before any blocker and, on a live terrain, the water is at least `rain.y` deep; `rain.y` is the free lane of the existing `rain` uniform. No new binding.
- `weathered/mirror.rs`: the CPU mirror gets the same rule (its tests are the shading-number contract).
- Cost: a handful of ALU on the wet path; dry pixels unchanged. Measured in the bench like any shader change (4K, interleaved means).

## What great looks like

- **A meadow in rain** (eye level, 12 m, looking down): a deeper, richer green than the same meadow under a dry overcast, a little darker, the blades still blades; no sheet of sky reflection, no "lake", no white middle distance. The horizon fades by the rain's haze, not by a mirror.
- **A cliff in rain:** darker, more saturated stone with a soft sheen on the faces turned to the sky, the fragments and joints still reading as rock, not foil.
- **Rain on the meadow** (`hills_storm`, `hills_rain`): no white rings on the grass; the rain shows as streaks and haze. Rain on standing water (a puddle, a lake): rings, as before.
- **Sand in rain:** darker, browner sand; the middle distance stays sand-coloured.
- **At dusk toward the sun:** a broad glossy sheen on the wet ground where the sun's reflection falls (the porous gloss at r ≈ 0.55), not a field of white.
- **Puddles** (where the hydrology holds water): still level mirrors with ripples, now standing out from the duller wet ground around them.
- **Buildings** (`weathered_town`): walls darker with a sheen, rain streaks still glossy, the plastic and metal props as before.

## Validation targets

1. **Mirror, porous rule:** at $w$ = 1, $\phi$ = 1, roughness 0.9 → 0.54 and 0.95 → 0.57 (±1e-6); at $w$ = 0.5, $\phi$ = 0.5, $r$ = 0.7 the porous product is 0.63 and the film weight $(1 - 0.5) \cdot 0.5$ = 0.25, so 0.4975; albedo unchanged from today (× 0.5 at $w$ = 1, $\phi$ = 1).
2. **Mirror, film unchanged where it belongs:** $\phi$ = 0 (roughness 0.4, or metallic 1) at $w$ = 1 → 0.1 as today; a drenched porous surface ($w$ = 1.4) → 0.1 and albedo × 0.3 as today; never rougher than the dry surface; dry (all zero) bit-identical.
3. **GPU (`tests/gpu_weathered.rs`):** a fully wet weathered cube matches a plain cube of albedo × 0.5 at roughness **0.54** within 1 level, and is darker than the dry one.
4. **Captures, rain against the same views under dry Overcast** (`/tmp/i47`): in every ground band of the table above the rain's luminance is **at most** the overcast's and its median saturation **at least 0.9×** the overcast's (wet ground is darker and not greyer). The meadow's 10–60 m band, now 187 against 117, is the defect's own number.
4b. **The same, against the same rain unsoaked** (`*_rain_dry`: the Rain preset without `ELEMENTS_GROW_HOURS`, wetness ≈ 0.006 after 10 s). Recorded after the first captures: the Overcast control differs from Rain in deck, haze (20 km against 3 km visibility) and the veil of streaks, and the unsoaked rain cliff already measures 125.0 / 0.033 against Overcast's 102.9 / 0.067. Against this control the wet shading alone is measured: wet ground at most as bright, its median saturation at least 0.9× (meadow, near sand); the grazing bands of sand and rock may grey by the sheen Jensen et al. 1999 §4.2 measured (forward scattering 0.2 → 0.7 for sand, 0.1 → 0.8 for rock, `wetness.md` Sources).
5. **Unchanged elsewhere:** every dry capture is bit-identical (`hills_eye`, `hills_overcast`); snow (`hills_snow`) unchanged.
6. **Visual:** the What-great list, judged on `hills_rain`, `hills_rain_down`, `hills_rain_12`, `hills_rain_dusk`, `hills_drizzle`, `hills_storm`, `plains_rain_down`, `mtn_rain`, `scree_rain`, `dunes_rain` and `town_rain`.
7. **Rain rings:** in `hills_storm` (25 mm/h, no standing water in a 10 s capture) no ring on the meadow; in the `weathered_town` (no terrain) and on a blocker's roof the rings as before (`tests/gpu_precipitation.rs` `a_blocker_over_the_camera_holds_no_particles_and_collects_splashes` and the splash-rate test run on the plane and must pass unchanged).

## Deviations from this plan

1. **The wet roughness is Lagarde's porous rule, not the plan's lerp to 0.1** (`wetness.md` Model adopted 3, Parameters "Wet roughness", Validation 6 and 9). Reason: the measurements above; Lagarde 3b separates the porous gloss boost (limited, "a factor of 0.5 on WetLevel to limit the specular boost") from the film, and the 0.1 rule applied the film to everything. The film is kept for smooth and metal surfaces and for drenched porous ones.
2. **Lagarde's diffuse factor (× 0.2 at full porosity) is not adopted.** The crate's × 0.5 stays (`wetness.md` Model 2: the film physics gives 0.50–0.79 and flora's wood 0.55); only his gloss blend is taken, and it is derived from his factor so the 0.4 is his number.
3. **The grass canopy takes `bevy_flora`'s leaf rule, not a porosity.** No source measures a wet canopy's visible reflectance (corpus search above); the sibling's rule is the family's existing choice, and a terrain meadow that matches the wet trees on it is worth more than an invented porosity.
4. **Rain rings only on standing water** (`precipitation.md` Model "Splashes" and Validation 13: every ground impact within 20 m rings). Reason: Model adopted 5; Tatarchuk puts ripples in puddles and splashes on hard surfaces, Xiao et al. put a canopy's rain on its leaves. The plane of a scene without a terrain keeps the plan's rule, because the kernel does not know what it is.

## Pending index

- Lagarde 3b is a web page (no DOI); read from the URL, not ingested.
- `10.1038_s41598-017-03360-1` and `book_real_time_rendering_4th_akenine` are already in the corpus.
