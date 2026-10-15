# bevy_elements improvement loop — log

One improvement per iteration: captured, inspected, picked, researched, implemented, gated, shipped.
Iterations are appended newest-last.

## Iteration 1 — 2026-10-14 — the ground-fog volume darkened the sky it could not light

**Commit:** (this iteration) · **Sibling repos:** none touched.

### What changed, and why it ranked first

`FogPlugin`'s camera-following ground-fog `FogVolume` now fades out where Bevy cannot light it, and
carries the scene's *total* light where it can.

Bevy lights a `FogVolume` from the `DirectionalLight`s alone, and under cloud those carry only the
beam `T̄` — at full cover the sun's illuminance is literally 0 (instrumented: `beam = 0.0000`). The
cloud-scattered diffuse light reaches the camera's `AmbientLight` and never the volume, and Bevy's
`VolumetricFog.ambient_*` term cannot be the missing light (it is added once per ray, with no
density texture and no exposure — Bevy's own docs and this crate's note say so, which is why the
camera sets `ambient_intensity = 0`). So a thick cloud plus a short visibility made the volume
*remove* the sky and return nothing: a blizzard at noon rendered as a dark hole.

Adopted: `volume.density_factor = T̄ / (T̄ + 0.25 (1 − T̄))` and
`volume.light_intensity *= (T̄ + 0.25 (1 − T̄)) / T̄`, clamped 1–100. The two are exact reciprocals, so
the volume's radiance over its opacity is the scene's total light at every cover: it keeps its
appearance wherever it is lit and simply fades out of the frame where Bevy cannot light it, leaving
the sky and the distance fog to draw the whiteout. Ranked first because it is a **physically wrong
result visible in a shipping game's worst weather** (rank 1), not polish: an unlit volume that
occludes is an energy violation, and it is the most visible of the candidates at any hour.

Runner-up: the harness had no deterministic frame-sequence capture, which every later iteration
needs. It was built anyway as part of step 2 (below), but the fog defect outranked it as the
iteration's subject.

### Inspection findings

Captured before the change (`/tmp/i1/before/`, `/tmp/i1/ctl/`) and after (`/tmp/i1/after/`,
`/tmp/i1/fixed/`): a time-of-day sweep (05:30, 12:00, 20:12, 23:30) and all ten presets at noon,
1280 × 720, via `weather_tour`. The vision reviewer's crops are in `/tmp/i1/rev/`, `/tmp/i1/rev3/`,
`/tmp/i1/rev4/`.

Two reviews ran (both through the vision model; see "Harness" below):

1. **All ten presets at noon** — ranked Blizzard worst: "sky renders as a pitch-black nighttime
   celestial sphere" at `(20, 22, 26)` with a green, unwhitened ground; Snow's ground never whitens;
   DustStorm keeps a Rayleigh-blue zenith. It also flagged a bright horizon gap under Overcast
   (measured below as pre-existing) and particle-box edges.
2. **Blizzard and Fog, before vs after** (the crops that carry this iteration) — verdict "decisive
   improvement" for both. Blizzard: "the scene now unambiguously reads as daytime overcast rather
   than an anomalous midnight snowstorm under a 56.6° noon sun."

Measured, not just judged (luminance at x=1000, like-for-like captures with and without the fix):

| Preset | visibility | before | after | note |
|---|---|---|---|---|
| Blizzard | 200 m | 59, 43 | 135, 153 | the defect: 4× darker than Clear, now brighter than Snow |
| Fog | 300 m | 179, **98** | 215, **146** | the `y=200` dip is the volume's dark band |
| Snow | 1500 m | 137, 146 | 154, 162 | lit anyway: small change |
| Thunderstorm | 1500 m | 119, 135 | 135, 152 | |
| Rain | 3000 m | 145, 153 | 151, 160 | |
| Overcast | 20 km | 153, 161 | 154, 161 | |
| Clear | 40 km | 190, 201 | 190, 201 | **unchanged**, as designed |
| DustStorm | 800 m | 115, 141 | 115, 141 | **unchanged**: its cloud is thin, so the beam is 1 |

Decisive control: with the volume's density forced to `0` and nothing else changed, Blizzard's noon
zenith read `(122, 124, 129)` against `(46, 47, 50)` with the volume — proving the volume, not the
clouds or the exposure, was eating the sky.

### Verification

- `cargo test` — 209 lib + all suites pass (2 new unit tests: the fade-out and the reciprocity).
- GPU tests `cargo test --release --tests -- --ignored --test-threads=1` — pass.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` — clean.
- `cargo build --lib --target wasm32-unknown-unknown` — clean.
- `cargo build --examples --all-features` — clean.
- Visual: before/after crops at the same zoom, reviewed by the vision model (above). No perf change:
  the volume is the same size and march; only two scalars per frame differ.

### Harness

Two problems in step 2 were fixed, because every later iteration needs them:

1. **No frame-sequence capture.** `examples/common/mod.rs` now takes `ELEMENTS_SHOT_DIR`,
   `ELEMENTS_SHOT_EVERY` and `ELEMENTS_SHOT_FROM`; it waits for the frames to be written before
   exiting and captures until killed when there is no `ELEMENTS_FRAMES`. With `ELEMENTS_FRAMES` the
   run is a fixed 1/60 s step, so two runs simulate the same time (verified: identical simulated
   state; GPU output is not bit-deterministic, which the docs now say).
2. **Vision subagents fail on OpenRouter's `/responses` path.** A subagent that reads an image with
   a tool and then takes another turn gets `400 "Requests ending with a model turn are not
   supported"` from Google — reproduced and narrowed by replaying the exact request body: it is the
   replayed `reasoning` items. `PI_OPENROUTER_RESPONSES=0` (Chat Completions) accepts the same
   history; `/tmp/i1/vision.sh` runs a review that way. The in-process `modelRoles.vision` route is
   unusable for tool-using image review until this is fixed upstream.

### Visual review (`screenshot-reviewer`, after-crops, `/tmp/i5/review_a.md`)

The reviewer compared before/after at the same zoom levels and called the horizon finding
**resolved**: before, a 67 DN dip (sky L=227, shelf floor L=160, ground L=176) at `y = 85..104`
across the whole width; after, a monotone ramp (L=226 → 220 → 179 → 176) with **0 DN** of dip, on
both `Overcast` and `Rain`. It confirmed the ground now reads as snow-covered and matches the
weathered columns (`srgb(228, 229, 232)`), and that the 40 km plane shows no vertex swimming,
z-fighting or edge clipping against the atmosphere probe.

One **Noticeable** finding stands: the flat harness's snow ground is a uniform wash (std dev
< 0.4 DN, no relief). Cause measured, and it is a property of the scene rather than a regression:
the shipped snow path perturbs only `base_color`, `roughness` and `metallic` (`mirror.rs:149-151`)
and never the normal, so an up-facing plane lit only by the cloud-dimmed beam has nothing to vary
it — under Overcast the directional light's illuminance is near zero, so there are no cast
shadows either. On real terrain the same path follows the relief: `terrain_snow_g24` has a ground
std dev of 16 (mean 222) against the flat plane's 0, and the snow lies on the ridges. A snow
normal/albedo perturbation would be a new system, and so a research note first.

A second focused review (`/tmp/i5/review_b.md`) checked regressions and found **none**: Clear
ground identical (RMSE 3.9e-05, sampled (140,155,113) before and after) and PartlyCloudy unchanged
within dither (RMSE 7.56e-04, (132,146,112) → (132,146,113), cloud shadows matching), the 40 km
plane integrated to the horizon with no seam or step on any preset, DustStorm's ochre veil and
Fog's extinction uniform with no halos, dusk banding-free, night ground exactly (0,0,0) with
pin-sharp stars.

### Top 5 remaining items, ranked

1. **Blizzard's whiteout is not white yet.** Measured after the fix: sky 135–185 against snow 225,
   so the horizon line stays visible and a distant pillar reads with `ΔL ≈ 35 %` against the sky.
   The distance fog's in-scatter at maximum depth is brighter than the horizon sky. Rank 1: it is
   the same class of physically wrong result as this iteration's, in the same preset.
2. **A bright horizon gap / step under Overcast and Rain** (`y ≈ 270`, ~30 DN). Pre-existing
   (measured 36 DN before, 29 DN after this fix — it got slightly *smaller*), so not a regression;
   the reviewer found it in both passes. Distance-fog colour against the sky above the horizon.
3. **Snow's ground never whitens** under Snow or Blizzard in `weather_tour` (the ground-cover maps
   need terrain; the example's flat scene uses the plain ground material). Needs a decision:
   document it, or give the harness's ground plane the cover shading.
4. **DustStorm keeps a Rayleigh-blue zenith** with pristine white cumulus at 800 m visibility.
   The fog tint is only in the distance fog; the sky above is untouched.
5. **Particle-box edge** at 60 m under Rain/Thunderstorm: streaks stop abruptly against distant
   un-occluded geometry, and the draw has no far-particle haze.

Deferred/known, not ranked: the per-preset `docs/*.png` stills and `docs/tour.gif` were regenerated
this iteration only where the shipped build changed them.

## Iteration 3 — 2026-10-14 — the sky carries the weather's air (the whiteout's horizon line)

**Commit:** 816f473 · **Sibling repos:** none touched.

### What changed, and why it ranked first

The cloud composite now fogs **sky** pixels by the same air the distance fog gives every mesh:
`σ_air = σ_fog − 3.912/40 km`, read from the camera's own `DistanceFog` (`fog::sky_air_extinction`,
`CloudParams::air`, `composite.wesl`), multiplying the deck's radiance by `e^(−σ_air·t)` and adding
the fog colour times the same share. `t` is the path to the layer's entry, the conservative limit
(the veil beyond converges on the same colour, Validation 20).

Ranked first because it is the **rank-1 backlog item from iteration 2 and the top of iteration 1's
list**: a physically wrong result in the shipping crate's worst weather. The fog made the ground
converge on the fog colour but left the sky at the deck's own radiance, so a 200 m-visibility
blizzard showed a 40-level luminance step at the horizon — measured, and named "Blocker / inverted
luminance" by iteration 2's visual review.

Runner-up: snow's ground never whitens in `weather_tour` (backlog 3). It needs a terrain and the
ground-cover maps, so it is a harness change, not a renderer defect; the horizon step is visible in
every whiteout image in every scene.

### Inspection findings (iteration 2's captures, `/tmp/i2/`, and this iteration's `/tmp/i3/before/`)

Measured on the before build at the horizon (`x = 1000`): `tour_Blizzard.png` sky `(186, 193, 198)`
at `y = 269` against ground `(224, 228, 230)` at `y = 270`; `terrain_blizzard_air.png` sky
`(185, 188, 191)` at `y = 297` against terrain `(225, 228, 231)` at `y = 298`. `tour_Fog.png` had a
dark cloud band `(144, 147, 154)` between a bright fog base and the upper sky. Crops and the
before/after review are in `/tmp/i3/rev/`.

### Verification

Measured on the before/after captures at the horizon (`x = 1000`, mean of RGB), the step the
review called a blocker:

| scene | sky just above it | ground just below it | after |
|---|---|---|---|
| `tour_Blizzard` (200 m) | 192 | 225 | **227 / 225** — step gone |
| `tour_Fog` (300 m) | 191 (mid-sky band 148) | 216 | **219 / 216** — the dark band is gone |
| `tour_Snow` (1500 m) | 192 | 222 | **227 / 222** |
| `tour_DustStorm` (800 m) | 188 (chromatic clash) | 194 | **198 / 194**, the sky takes the ochre tint |
| `tour_Clear` (40 km) | — | — | **bit-identical, AE = 0** (also dawn, dusk, night) |
| `terrain_blizzard_g` | 175 | 177 | **227 / 227** |

The vision review's crops are in `/tmp/i3/rev/`; its verdict: "Fit to ship: horizon seam is
completely eliminated under low visibility while clear conditions remain bit-exact", with the
Blizzard horizon crop's standard deviation down from 4255.7 to 802.6 and `terrain_blizzard_g`'s
from 7565.6 to 186.6.

- `cargo test` — 211 lib + all suites pass (2 new unit tests: the air's arithmetic and monotonicity;
  the `Clear` preset pinned to `CLEAR_AIR_EXTINCTION`); `tour.gif` and the per-preset stills are
  regenerated from the new build and re-reviewed (no artifacts, all presets distinct).
- New GPU test `the_weather_air_makes_the_sky_the_fog_colour`: straight-up Blizzard, linear target,
  the frame's mean equals the fog colour to a half-float step (Δ 0.0026 of 6.91) with the air on and
  differs from it by half its luminance with the air suppressed (`CloudDebug::air = 0`).
- `the_far_veil_continues_the_marched_deck_at_full_cover` (Validation 20) now suppresses the air
  explicitly, because Deviation 29's fog saturated both sides of its 32 km boundary (ratio 1.000)
  and the test had gone blind; with the air off its measured ratios are 1.04–1.13 — the same values
  the build before Deviation 29 produced.
- `cargo clippy --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc
  --no-deps --all-features`, `cargo build --lib --target wasm32-unknown-unknown` — see the gate run
  below.

### Top 5 remaining items, ranked

1. **Snow's ground never whitens** under Snow or Blizzard in `weather_tour` (backlog 3 from
   iteration 1). The harness's flat ground uses the plain material; giving it the ground-cover
   shading (or moving the whiteout shots to `terrain_tour`) would let every later whiteout claim be
   judged on a scene that can actually be white.
2. **DustStorm keeps a Rayleigh-blue zenith** (backlog 4) with white cumulus at 800 m visibility:
   the fog tint is only in the distance fog; the sky above is untouched. The air mechanism added
   this iteration is the hook: the dust presets' *colour*, not just their extinction, is what is
   missing (`fog_hue` already tints the distance fog for dust).
3. **Particle-box edge** at 60 m under Rain/Thunderstorm (backlog 5): streaks stop abruptly against
   distant un-occluded geometry, and the draw has no far-particle haze.
4. **A bright horizon gap / step under Overcast and Rain** (~30 DN at `y ≈ 270`, iteration 1's
   backlog 2): pre-existing before iteration 1 and only slightly reduced by it; the air added this
   iteration changes it (Overcast's air is 9.78×10⁻⁵ m⁻¹), so re-measure it against these captures.
5. **Cloud edge stair-stepping** (iteration 2's review 4): 1–3 px discrete steps along cloud
   silhouettes in `tour_Clear`, `tour_Fog` and `tour_DustStorm`; a filtering or noise-octave
   question, not an energy one.

## Iteration 4 — 2026-10-15 — snow lies on the ground the examples show

**Commit:** `177754b` · **Sibling repos:** none touched.

### What changed, and why it ranked first

The examples' flat ground plane was a bespoke `StandardMaterial` extension that sampled only the
cloud shadow map (`examples/common/ground.rs` + `ground.wesl`, since deleted), so under `Snow`
and `Blizzard` `weather_tour` rendered olive-drab ground where the boxes and columns beside it
carried `WeatheredMaterial` and whitened. Two layers, one fix.

1. **Material.** `test_scene` now gives its plane `WeatheredMaterial` (the crate's own), so the
   example ground darkens and glosses in rain, runs with streaks and whitens under snow exactly as
   a consumer's mesh does. The duplicate shader is gone.
2. **Plane extent.** 2 km → 40 km. A 2 km plane leaves a sliver of unlit below-horizon
   atmosphere between its edge (1 km away) and the horizon; `ground_haze` (clouds.md Deviation 28)
   paints the camera's distance fog over it, and the result is a dark shelf at the horizon.
3. **Capture.** `SurfaceWeathering` and the cover maps advance on *game time*, and `ELEMENTS_HOUR`
   holds the clock, so a still of the first frame integrates 0 h of the held preset: no material
   could ever whiten in a screenshot. `ELEMENTS_GROW_HOURS=n` (examples/common, `grow_weather`)
   advances the clock one hour a frame for `n` frames plus one arming frame, then rewinds by what it
   advanced. Both consumers record the clock on their first frame and integrate nothing from it, so
   the arming frame is needed; and a jump larger than 24 h is clamped by both, so the growth is
   split into one-hour chunks. The captured hour, sun and sky stay the example's own.

Why it ranked first: rank 1 on iterations 1 and 3's backlogs, and the reviewer's finding 3
("ground under `tour_Snow` fails to read as snow-covered"). It beats the runner-up (iteration 3's
backlog item 2, the bright horizon step under Overcast and Rain) because the horizon step is
*physically* the fogged ground against a bright deck and moves only 29 DN, while a snowing sky over
green grass is a wrong result the crate can fix.

### Inspection findings

Before/after pairs are in `/tmp/i4/before/`, `/tmp/i4/after/`, crops in `/tmp/i4/crops/` and
`/tmp/i4/crops_after/`. The reviewer's before findings: `rain_sky` streaks at Δ ≤ 8 RGB levels
(versus > 50 against ground); `snow_ground` olive-drab; `overcast_horizon` a 66 DN drop (sky
L=225) to a band floor of L=159, 16 DN below the ground it meets. Measured on the new build,
x=16 column, rows 264-280 (`magick -crop 1x17+16+264`):

| scene | before | after |
|---|---|---|
| `tour_Overcast` horizon shelf | 224→194→156 (valley) →173 | 224→220→203→194→…→173 (ramp) |
| `tour_PartlyCloudy` horizon | 219→182→155 (valley) →196 | 219→211→204→201→…→196 |
| `tour_Clear` horizon | 203→207→170 (step) | 203→209→193→…→170 |

The 66 DN valley under Overcast and PartlyCloudy is gone: after the fix the same column is a
monotone ramp (224→203→194→186→181→177→173) with no dip below the ground it meets. Clear's
30 DN step is unchanged (clear air has no fog to converge on, and the Clear preset's plane edge
was never the cause).

### Verification

- `cargo test`: 242 tests, **211 pass, 31 ignored** (GPU, not run), 0 fail.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: clean.
- `cargo build --lib --target wasm32-unknown-unknown`: clean.
- GPU tests, one suite per process: `clouds::` 10 pass, `terrain::` 8 pass, `cover::` 5 pass /
  **2 fail (pre-existing)**, `dust::` 6 pass, `voxel::` **1 fail (pre-existing)**, integration
  (`tests/gpu_*`) 28 pass. The two cover failures (`gpu_wetness_soaks_and_dries_on_the_formula`,
  probe `Some(0.992)` against an expected `< 0.1` after drying, and
  `gpu_surface_weathering_follows_the_map`) were reproduced on **pristine HEAD** with this work
  stashed — a wetness-tolerance defect in the cover map, not a regression. The voxel failure
  (`gpu_voxel_snow_covers_the_top`, `src/voxel/gpu_tests.rs:318`) reproduces 2/2 at pristine HEAD
  with swap 1 at 23.05 h and 22.80 h against `20.7 ± 2.0 h` — a ~+2.1 h bias past the window, with
  no simulation-side `src/` drift since the test was written (the only later edits are a type
  alias and a constant rename). Both are logged below as backlog item 2.
- Ground material, snow and growth, before → after: `tour_Snow` (155,167,141) → (228,229,232) at 6 h;
  `tour_Blizzard` (155,167,141) → (228,229,232); `terrain_snow_g` (143,162,132) → (228,230,232) at 24 h
  (`CoverStats::simulated_s` = 86 400 s exactly, `terrain_blizzard_g` same).
- Determinism: two `ELEMENTS_GROW_HOURS=6` runs give identical ground (228,230,232) and identical
  status-line hour pixels (158,158) — the warm-up is reproducible.
- Regression: `ELEMENTS_GROW_HOURS=0` is bit-identical to the build before this change. Clear
  ground (140,155,112) before and after; PartlyCloudy ground (132,146,112) before and after.
- Cost: the change is `examples/`-only plus one stale `src/terrain/render.wesl` comment; no shader,
  kernel, or uniform changed, so the cost tables in the notes stand.

### Top 5 remaining items, ranked

1. **Snow has no surface relief on a flat ground.** The shipped path (`mirror.rs:149-151`) moves
   `base_color`, `roughness` and `metallic` and never the normal, so on the harness's flat plane
   under Overcast (beam ≈ 0, no cast shadows) the snow is a uniform wash (std dev < 0.4 DN).
   Real terrain follows the relief (std dev 16 on `terrain_snow_g24`), so this is the flat plane's
   missing normal detail, not a bug. A snow normal perturbation needs a research note and corpus
   search first — `snow-cover.md` cites no crust source.
2. **Three GPU tests fail on pristine HEAD.** `gpu_wetness_soaks_and_dries_on_the_formula`
   (probe `Some(0.992)` vs expected `< 0.1` after drying at `src/cover/gpu_tests.rs:783`) and
   `gpu_surface_weathering_follows_the_map` (`:846`) both fail without this work — the wetness map
   reaches 0.99 after the test's drying window, so either the drying constant or the assertion's
   tolerance is wrong. `voxel::gpu_tests::gpu_voxel_snow_covers_the_top`
   (`src/voxel/gpu_tests.rs:318`) fails 2/2 with swap 1 at 22.8–23.1 h against `20.7 ± 2.0 h`: a
   consistent ~+2.1 h bias with 0.25 h run-to-run variance and no simulation-side `src/` drift
   since the test was written, so either the swap is genuinely late or the window is tighter than
   the readback-observed swap time allows. Together the only things blocking a clean
   `cargo test --release --lib -- --ignored`; a real defect in a shipped domain.
3. **The horizon step under Clear** (~30 DN at `y ≈ 270`): physically the fogged ground against a
   bright deck; the only remaining horizon discontinuity in the showcases, and the reviewer's
   before-review found it under every long-visibility preset.
4. **DustStorm keeps a Rayleigh-blue zenith** with white cumulus at 800 m visibility (backlog 4).
5. **Rain streaks wash out against the bright sky** (reviewer finding 2: Δ ≤ 8 levels against the
   overcast deck, > 50 against the ground). A contrast question in the precipitation draw.

## Iteration 5 — 2026-10-15 — the horizon shows no dark band past the edge of a finite ground

**Commit:** `c9131e6` (code + test + docs + stills). **Sibling repos:** none touched.

### What changed, and why it ranked first

The extra instruction for this run was a full aesthetic inspection: every domain the crate ships,
captured and reviewed by the vision model (`screenshot-reviewer`, Gemini Flash), then the single
most valuable defect fixed.

Below-horizon sky pixels with no mesh behind them — past the edge of quickstart's hills, the lake
basin, any finite terrain — drew Bevy's unlit bare-planet term. `ground_haze` (clouds.md
Deviation 28) fogs over the planet's own root, and from a low camera that root is *near* (23 m at
5° below the horizon from 2 m), so the transmittance is ~1 and nothing fogs: the pixel is the dark
planet under a lit, fogged scene. Measured on the lake at three columns: a 33 DN dark band at the
horizon (193 against fogged ground 226); reproduced headlessly in the new GPU test at 0.59× the
fog colour (3.835 against haze 6.553 on the linear target). The fix floors the fog distance at
the march range (`MAX_MARCH_M`, 32 km): a below-horizon pixel with no mesh shows ground the fog
has fully carried, because nothing nearer than the far field is visible beyond a finite terrain.
It beat the runner-up (precipitation and dust washing out against bright backgrounds) because it
is a physically-wrong rank-1 artifact isolated to one shader line with a regression test, while
the washout is a particle-compositing redesign that touches every precipitation golden.

### Inspection findings

Capture: `/tmp/i6` — 35 sequential runs over every example domain (`capture.sh`): the five
times of day, ten sequences (cloud build, rain, snow, lightning-lead, wind, lake ripples, flood
flow, dust, fog roll), two terrain seeds (42, 7), two camera heights (6 m and 120–400 m), erosion
before/after, voxel snow, weathered town (rain wide + wall, snow, fog). Reviews: eleven
Gemini Flash batches (`/tmp/i6/reviews/b1.md` … `b11.md`), crops in `/tmp/i6/crops/`.

- **Blocker, reclassified: the "dark horizon shelf" of iteration 4's backlog was the harness's
  own status line.** The reviewer's b6 finding (68 DN, rows 10–56 of `cover_rain_gnd`) is the
  semi-transparent debug band the harness burns into the top of every capture; b11 read it
  verbatim ("Clear (forced) … vis 40000 m") and confirmed no shelf exists in the sky below it
  (row-to-row ≤ 1–2 DN). Crop below row 56 when judging horizons; AGENTS now says so.
- **Precipitation and dust wash out against bright backgrounds** (the runner-up, now rank 1):
  rain streaks ΔDN ≤ 3 against a 227-DN overcast sky while Δ64–75 against the ground
  (`b3_*`); snow flakes Δ ≤ 5 against the sky, Δ36–44 against ground (`b4s_*`); 60 000 dust
  sprites move the frame by ≤ 4 DN over 14 frames — "the storm appears as static homogeneous
  fog" (`b5_dust_*`). Streak advection, flake motion and shelter occlusion are otherwise clean.
- **Snow has no normal-dependent shading under an overcast sky** (`b7_snow_seed07_aerial.png`):
  the 24 h warm-up saturates the terrain white but the aerial view measures 0.56 DN of standard
  dev across 840 k pixels — Bevy's ambient is flat over normals and the beam is zero. This is
  iteration 4's backlog item 1 seen from the air, on real terrain.
- **Time of day** (`b1_*`): the horizon is continuous at all four hours; the dawn gradient is
  smooth to ≤ 1 DN per row; stars are 1–2 px points; night ground sits at (4,4,5). Two
  capture-design facts: `day_night`'s camera (yaw 106.7°) never faces the sun (dawn azimuth 64°,
  sunset 303°), and the moon is set at 23:30 (−12.3°) — the showcase cannot show either disc.
  The night sky reading brighter than dusk (16.1% vs 12.5% mean) is the Ward/Larson auto
  exposure the note adopts (celestial.md, "Auto exposure"), not a defect.
- **Clouds** (`b2_*`): cumulus edges monotone, no ghosting between consecutive accumulation
  frames (Δ ≤ 1 DN), overcast deck uniform. The shadow-contrast question is unresolved (the
  crop missed the cloud mass at Clear's 0.05 cover).
- **Lightning, wind, water, materials, fog** (`b4`, `b5`, `b9`, `b10`, `b6`): bolt geometry
  clean, tree sway anchored and coherent, lake shoreline sub-pixel continuous with no shimmer,
  ripple motion structured, flood channel flush with no gaps, wall streaks localized with clean
  onset, pavilion dry contrast sharp, fog volume against geometry natural. Minor: mid-flash is
  dimmer than the first stroke because `STRIKE_LEAD=6` (+100 ms) outlives the ~43 ms decay
  (the doc says "inside the flash"); splash rings sparse; near-shore foam sub-pixel at nadir;
  the flood marker sits outside the capture's frustum; fresh erosion cuts facet; the status text
  clips at the window edge.
- **Verified physical, not defects:** thin snow lets grass through (`cover_snow_gnd` reports
  7.7 mm and melting); no puddles at 1.6 mm of water (the model's threshold is 2 mm); sequence
  frames 0–12 render before the pipelines are ready (skip them when cropping pairs).

### Verification

- New GPU test `clouds::gpu_tests::the_horizon_sky_is_the_fog_colour_under_a_deck` (6 m eye,
  1° up, `Snow`): above the horizon the veil is the fog colour (6.543 against haze 6.553); below
  it the planet was 0.59× the fog colour and is now the haze. Red before the fix, green after.
- Before/after captures (`/tmp/i6/shots` vs `/tmp/i6/after`, reviewed in `b11.md`): the lake's
  horizon band lifts 193 → 226 DN at x = 100, 1400, 1800 with meshed terrain bit-identical
  (Δ 0 DN); `weathered_town`'s 40 km plane compares AE 0; flood fringes converge into the
  fogged ridge; open sky Δ 1 DN. The controls `tod_noon` (AE 1.4e-8) and the terrain seed-42
  ground shot (AE 2.4e-5, dither) are unchanged.
- `cargo test`: 211 pass, 0 fail (32 ignored, GPU). `cargo clippy --all-targets --all-features
  -- -D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features`, and the
  wasm32 build: clean.
- GPU suites, one process per module: `clouds::` 11 pass in two parts (see below), `terrain::`
  8 pass, `dust::` 6 pass, integration (`tests/gpu_*`) 28 pass; `cover::` and `voxel::` fail
  exactly the three pre-existing tests logged under iteration 4's backlog item 2 — reproduced
  there first, unchanged here.
- New test-infrastructure fact: `clouds::gpu_tests` now has eleven tests and the eleventh
  sequential wgpu instance cannot get an adapter (`GPU_NOT_FOUND`); the suite passes as
  ten-plus-one. Recorded in AGENTS.

### Top 5 remaining items, ranked

1. **Precipitation and dust wash out against bright backgrounds.** Rain streaks Δ ≤ 3 against
   a 227-DN overcast sky, snow flakes Δ ≤ 5, dust sprites ≤ 4 DN over 14 frames, while each is
   strongly visible against dark ground. The particles carry a fixed radiance; physically a
   flake or streak is lit like any surface — about 0.8× the sky behind it, so it should read
   slightly dark against the deck and bright against the ground. A compositor change in the
   precipitation draw, with the note's deviation and contrast tests on both backgrounds.
2. **Snow has no normal-dependent shading under an overcast sky** (iteration 4's item 1, now
   measured from the air: 0.56 DN across 840 k pixels on saturated terrain). Bevy's ambient is
   flat over normals; an overcast dome is not. Needs a corpus search (overcast sky-luminance
   models) and a research note before any code.
3. **Three GPU tests fail on pristine HEAD, and the clouds suite needs two processes.**
   `cover::gpu_wetness_soaks_and_dries_on_the_formula` (probe 0.992 against < 0.1 after
   drying), `cover::gpu_surface_weathering_follows_the_map`, and
   `voxel::gpu_voxel_snow_covers_the_top` (swap 1 at 22.8–23.1 h against 20.7 ± 2.0, a +2.1 h
   bias with no simulation-side drift since it was written). Plus the adapter cap above.
4. **`day_night` never shows the sun or the moon.** The camera yaw (106.7°) points away from
   the dawn sun (64°) and the sunset sun (303°), and the 23:30 moon is set (−12.3°): the discs
   the domain exists to render cannot appear in the showcase. An `ELEMENTS_HEADING` default that
   tracks the sun at dawn/sunset and a moon-up hour would show them.
5. **The small cosmetic set.** `ELEMENTS_STRIKE_LEAD=6` lands +100 ms after the trigger, past
   the ~43 ms flash decay, so "inside the flash" is wrong in `storm`'s doc; splash rings are
   sparse on a 6 m roof; near-shore foam is sub-pixel at nadir (view-ray thickness); the flood
   marker is outside the capture frustum; fresh erosion cuts facet; the status line clips at
   the window's right edge.

## Iteration 6 — 2026-10-15 — a dust storm streams instead of sitting as static fog

**Commit:** `19db0a4` (code + note + CHANGELOG + the three dust stills). **Sibling repos:** none.

### What changed, and why it ranked first

The full-feature inspection continued (the extra instruction): the iteration-5 backlog's rank 1
was "precipitation and dust wash out against bright backgrounds". Re-measured properly this
iteration — full-band histograms instead of the single 240 × 50 patches the finding came from —
two thirds of it dissolves:

- **Rain reads** (`before6/rain_day.png`, sky band rows 120–260): 50 150 of 161 000 pixels
  (31 %) sit 25–75 DN below the 225 sky — streaks were visible all along; the earlier "Δ ≤ 3"
  was measured on a streak-free patch.
- **Snow reads, sparsely** (`before6/snow_day.png`): flake cores reach 190–204 against a 225
  sky (Δ 21–35) on 4 558 pixels; the earlier "Δ ≤ 5" sampled a faint edge.

The third member was real: **dust sprites were genuinely invisible.** They are drawn at the
plan's 2 mm grain size, which is 0.02–0.1 px anywhere in the 60 m box, so the draw's own
sub-pixel coverage rule (`draw.wesl`, Garg & Nayar 2007 Eqs. 21–22) fades their opacity to
≈ 0.01: 1 247 pixels of 849 920 reached 150–199 against a 224 background and none below 150,
and the reviewer measured ≤ 4 DN of movement over 14 frames — "the storm appears as static
homogeneous fog" (`b5_dust_*`). The example's stated behaviour ("dust sprites stream past the
camera with the wind") did not exist on screen. That is a visible, physically-wrong-at-the-pixel
rank-1 defect in a shipped showcase, and it beat the runner-up (snow's flat overcast shading,
which needs a whole lighting-model note) because it is one constant in one vertex path with a
direct visual oracle.

**The fix:** the draw diameter (`DUST_SPRITE_DRAW_M`, 0.35 m) and the sprite opacity
(`DUST_ALPHA`, 0.5, below a flake's 0.8 so a cluster is a veil rather than a puff) are now
separate from the grain size. `DUST_SIZE_M` (2 mm) remains the size the sprite *extinction*
accounts — Validation 8's 8.7 · 10⁻⁷ m⁻¹ is untouched, as are the counts, the motion, the fog
and every other physics number. `dust.md` Deviation 15 records the split with the measurements.

### Inspection findings

Crops in `/tmp/i6/crops/` (`b12_*`), review in `/tmp/i6/reviews/b12.md` (Gemini Flash,
`screenshot-reviewer`):

- **After the fix the storm reads as particulate:** 26 % of the frame moves more than 6 DN
  (219 924 px), cores reach 150–199 (55 316 px), the tan chromaticity is preserved
  (mean (221, 208, 189) → (216, 203, 183); R:G:B ratio 1.17 : 1.10 : 1.00 before and after),
  and there are no opaque blobs (min channel 181/165/143), no whiteout, no quad edges. The
  reviewer's verdict: *"Does the dust storm now READ as streaming particulate rather than
  static haze? Yes … No artifact introduced … Ship."*
- **Temporal check** (`b12_dust_seqA/B_2x.png`, frames 3 apart): mean difference 4.48 DN,
  clusters advect downwind by (−2…−6, −8…−15) px over 3 frames, edges fade gracefully at the
  box boundary — continuous drift, no popping.
- **Corrections to last iteration's findings** (recorded so the backlog stops carrying them:
  the rain and snow "washout" entries were single-patch sampling artifacts — full-band
  histograms above).

### Verification

- `cargo test`: 211 pass, 0 fail. `cargo clippy --all-targets --all-features -- -D warnings`,
  `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features`, wasm32 build: clean.
- GPU suites, one process each: `clouds::` 10 + 1 (two parts, the adapter cap), `terrain::` 8,
  `dust::` 6, integration 28 across 11 binaries; `cover::` 5/2 and `voxel::` 0/1 fail exactly
  the three pre-existing tests from iteration 4's backlog item 2 — no new failures.
- `precipitation` unit tests: 16 pass (the sprite extinction pin is unchanged).
- Before/after captures under identical env (`/tmp/i6/before6` vs `/tmp/i6/after6`) and the
  motion sequence (`/tmp/i6/seq6/dust`); the three README dust stills regenerated from the new
  build (600/600/4200 frames, the commands the README documents).

### Top 5 remaining items, ranked

1. **Snow has no normal-dependent shading under an overcast sky** (iteration 5's rank 2):
   saturated terrain reads 0.56 DN of standard deviation across 840 k pixels from the air, and
   the flat-ground case reads < 0.4. Bevy's ambient is flat over normals; an overcast dome is
   not. Needs a corpus search (overcast sky-luminance models) and a research note before code.
2. **Three GPU tests fail on pristine HEAD, and the clouds suite needs two processes**
   (unchanged): `cover::gpu_wetness_soaks_and_dries_on_the_formula`,
   `cover::gpu_surface_weathering_follows_the_map`, `voxel::gpu_voxel_snow_covers_the_top`
   (swap 1 at 22.8–23.1 h against 20.7 ± 2.0), plus the eleven-instance adapter cap.
3. **`day_night` never shows the sun or the moon** (unchanged): camera yaw 106.7° against the
   dawn sun at 64° and the sunset sun at 303°; the 23:30 moon is set at −12.3°. The discs the
   domain exists to render cannot appear in the showcase.
4. **The small cosmetic set** (unchanged): `STRIKE_LEAD=6` outlives the ~43 ms flash decay
   (the doc says "inside the flash"); splash rings sparse on a 6 m roof; near-shore foam
   sub-pixel at nadir; the flood marker sits outside the capture frustum; fresh erosion cuts
   facet; the status line clips at the window's right edge.
5. **Cloud shadow contrast under Clear is unverified** (carried from iteration 5): the crop
   missed the cloud mass, so neither confirmation nor denial. Capture with the shadows in frame
   (choose a heading where cumulus sit upwind) and measure the ground's shadow depressions.

## Iteration 7 — 2026-10-15 — snow relief reads: the overcast dome shades by orientation

**Commit:** `d24c704` (two shaders, the example harness, note, CHANGELOG). **Sibling repos:** none.

### What changed, and why it ranked first

The extra instruction again: inspect every feature with the vision model. The pick is iteration
6's rank 1 (carried from 4 and 5): **snow under an overcast sky is a physically wrong flat
result** — saturated terrain reads 0.56 DN of standard deviation across 840 000 pixels from the
air, because Bevy's ambient is uniform over normals, the beam is ~0 under cloud, and no term in
the stack varies with orientation. It beat the runner-up (`day_night` never framing the sun or
moon) because it is a wrong rendering of a shipped domain in every overcast showcase, while the
runner-up is a showcase's camera choice.

**The fix** (`src/weathered/weathered.wesl`, `src/terrain/render.wesl`, inlined at both call
sites): the lit colour is multiplied by `mix(1, shape(N_y), 1 − T_deck)` where `shape` is the
Moon & Spencer (1942) CIE overcast dome — `L_γ = L_z(1 + 2 sin γ)/3`, the horizon a third of the
zenith (Dáru & Kittler, ESIM 2002, eq. 1; the corpus held no dome model, so the source was
fetched) — integrated over the dome with the ground's bounce at the crate's own albedos into
top 1 / side 0.68 / bottom 0.5, then re-fitted as `1 − 0.35·(1 − N_y)^0.3` [INFERENCE] after the
review's *Adjust*: the pure linear dome is physically honest but gives ≤ 2 DN on 5–15° slopes
(14 pixels ≥ 3 DN on the aerial view), because `cos θ` is flat near vertical; the power curve
keeps the anchors (side 0.65, bottom 0.57) and puts the gradient where the demand is. The ramp
is the raw cloud-map transmittance over each ground point (`cloud_deck`), not
`camera_transmittance`, which is wall-clock-smoothed and varied 0.18–0.77 between identical
runs. Clear air evaluates `T_deck = 1` → factor exactly 1.

The showcase's boxes, columns and spheres were `StandardMaterial` and never reached the shader
(round-1 finding): they now take `WeatheredMaterial` through `ShowcaseProp`, adopted like
`GroundPlane`.

### Implementation findings worth keeping (all cost an hour each)

- **An additive ambient correction clamps to black.** The first form added
  `EnvBRDFApprox × ambient × (shade − 1)` after `apply_pbr_lighting`; sized against Bevy's
  interior ambient it overshoots through the cloud-shadow `ratio` and went negative (forced-
  overcast terrain rendered 0–1 DN). Multiplying the lit colour cannot go negative, and under a
  deck the beam inside it is ~0, so it shapes the fill alone (the beam's share at partial cover
  is the deviation's stated cost).
- **`AmbientLight` is ~0 in this crate** — the additive delta measured ≈0.004 linear where a
  flat ambient would be ~5; the fill arrives with the environment-map light. Any future ambient
  work must scale the lit colour or the env light, not `lights.ambient_color`.
- **A viz that writes a bare `vec4` where `FragmentOutput` is required fails the pipeline**
  ("Entry point fragment … invalid") and the capture then shows the *fog background* — three
  rounds of readings were taken off a failed shader. Viz must set `out.color` and `return out`.
- **The tonemap runs outside the material shader**, so early-returned debug values come back
  ACES+sRGB-encoded; calibrate with constants sentinels before reading them (`0.1/0.4/0.8` →
  `91/149/194`).
- **`ElementsGlobals::camera_transmittance` is smoothed on wall-clock time** — run-dependent
  (0.18–0.77 for identical commands), unusable as a deterministic ramp; the cloud map itself is
  state-derived and deterministic.

### Inspection findings

Reviews in `/tmp/i7/review_b13.md` (round 1, *Adjust*) and `/tmp/i7/review_b14.md` (round 2,
*Ship*), Gemini Flash `screenshot-reviewer`, captures in `/tmp/i7/{before,final,final2}`:

- Round 1: the wash was still flat (99.99 % of the aerial ≤ 1 DN), the showcase props unchanged
  — the reviewer read `examples/common/mod.rs` and found the wiring gap — and the controls
  passed (AE with 2 DN fuzz: 0).
- Round 2: aerial spread 2.35 → 3.28 DN with "clear topological definition and structural
  relief across terrain crests"; overcast ground 629 116 px ≥ 3 DN, 30 196 ≥ 7, peak 9 DN,
  "hillsides and cut banks read unambiguous orientation-dependent shading"; box sides −9 to
  −15 DN with top caps exactly preserved; controls still fuzz-0; no banding, no Mach bands, no
  cloud-map-edge seam.

### Verification

- `cargo test`: 12 suites ok (211 lib tests), 0 fail. `cargo clippy --all-targets
  --all-features -- -D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features`,
  wasm32: clean (clippy needed a `type` alias for the adoption query).
- GPU suites: `terrain::` 8, `dust::` 6, `clouds::` 10 + 1 (two parts), `tests/gpu_weathered`
  4, integration 11 binaries — all pass; `cover::` 5/2 and `voxel::` 0/1 fail exactly the three
  pre-existing tests from iteration 4's backlog item 2.
- Controls: `tour_clear` AE 0.026 %, `terrain_clear_gnd` AE 0.075 %, both PAE 1 DN and fuzz-2
  AE 0 — clear air renders as before within dither (the props' material fork).

### Top 5 remaining items, ranked

1. **Aerial snow relief is still only 3–6 DN** (round 2 passed it, but it is the weakest
   surface): from 300 m the terrain's slopes are gentle and even the fitted curve gives ≤ 7 DN
   there. The next step is not a steeper curve but occlusion: a sky-visibility/AO term (what
   actually makes overcast relief read in photographs), which needs a research note and the
   corpus first.
2. **Three GPU tests fail on pristine HEAD** (unchanged): `cover::gpu_wetness_soaks_and_dries_
   on_the_formula`, `cover::gpu_surface_weathering_follows_the_map`,
   `voxel::gpu_voxel_snow_covers_the_top` (+2.1 h swap bias), plus the eleven-instance adapter
   cap that splits the clouds suite.
3. **`day_night` never shows the sun or the moon** (unchanged): yaw 106.7° against dawn azimuth
   64° and sunset 303°; the 23:30 moon is set. The discs the domain exists for cannot appear.
4. **The small cosmetic set** (unchanged): `STRIKE_LEAD=6` outlives the ~43 ms flash decay;
   splash rings sparse; near-shore foam sub-pixel at nadir; the flood marker outside the
   capture frustum; erosion-cut faceting; the status line clipping at the window edge.
5. **Cloud shadow contrast under Clear is still unverified** (carried twice): iteration 5's
   crop missed the cloud mass; capture with cumulus upwind and measure the ground's
   depressions.
