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

## Iteration 8 — 2026-10-15 — day_night frames the sun and the moon

**Commit:** `79e5e36` (examples only: `day_night`, `common::FlyCamera`, README, CHANGELOG — no
`src/` change). **Sibling repos:** none.

### What changed, and why it ranked first

The extra instruction again: inspect every feature with the vision model. Two inspection batches
(`/tmp/i8/review_a.md`, `/tmp/i8/review_b.md`, Gemini Flash `screenshot-reviewer`) closed every
rank-1/2 candidate: batch B passed the whole post-dome showcase (sky gradient monotone ≤ 2 DN
per 20 px, horizon continuous with zero shelf, prop top-vs-side shading 17–20 DN, terrain relief
5–8 DN with sub-1-DN speckle, cloud temporal pair at AE 0, rain streaks advection-coherent at
+9 DN against sky and +48 against ground) — **ship-shape**. The open categories therefore fell
to rank 3, and batch A measured the winner with a **Defect** verdict: **`day_night` never shows
the sun or the moon**. Its camera was aimed at bearing 253° (WSW) at +6° pitch while the bodies
sat elsewhere the whole sweep — sunrise 54–65° (ENE, behind), noon 180° at 68° up, sunset 304°
(past the 292° frustum edge), and the waxing moon at 150–175° (south, never in frustum): the
one example whose stated purpose is the celestial discs captured none of them in seven hours.
It beat the runner-up (an occlusion term for the aerial snow relief) because the runner-up
refines a fix the same reviewer already passed this week, while this is a whole missing feature
of the showcase the loop's inspection checklist names explicitly.

**The fix**: an `aim_at_sky` system (`PostUpdate`) turns the camera at the sun while it is
above the horizon, at the moon once the sun is down and the moon is up, and back at the spawned
evening heading when neither is up (`SunState`'s directions; yaw from the same `atan2` the
`camera()` helper uses). While the clock runs the aim follows the bodies; paused
(`ELEMENTS_HOUR`) it aims once (`Local<bool>`), so a still is framed on the disc and the view
stays free to drag — holding the right button releases the aim, which resumes on release. A new
`FlyCamera::face(yaw, pitch)` writes the controller's private fields.

### Verification

- Reviewer round 2 over the same seven hours (`/tmp/i8/review_c.md`, verdict **Ship**): the sun
  disc (10 × 10 px, core 255, centred within 2 px of frame centre) at 04:30 (alt 1.6°), 05:30
  (11.2°), noon (68.3° — the horizon sensibly out of the bottom of frame) and 19:36 (1.3°); the
  crescent moon (4 × 8 px, 37 % illumination faithful, 104–129 DN over the twilight sky) at
  20:12 and 21:30; 23:30 falls back to the evening heading with both bodies set (moon −1.8°) —
  stars-only, near-black ground (7,4,2). Sky checks unchanged from round 1: row steps ≤ 1 DN at
  every pitch, stars 1–2 px, no banding, no status-band occlusion (>300 px clear).
- Gates: `cargo test` 12 suites ok; `cargo clippy --all-targets --all-features -- -D warnings`,
  `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features`, wasm32 clean. GPU (one process
  per suite): `clouds::` 10 + 1 in the documented two parts (the eleven-instance adapter cap
  fired once when the suite ran whole — `GPU_NOT_FOUND`, not a regression), `dust::` 6,
  `cover::` 5/2, `voxel::` 0/1 (the three pre-existing failures from iteration 4's backlog
  item 2), integration 11 binaries.

### Top 5 remaining items, ranked

1. **Occlusion for the aerial snow relief** (carried): the dome curve gives the aerial view
   3–8 DN; a sky-visibility/AO term is what photographs actually use. Needs a research note and
   a corpus-first search (sky occlusion course notes exist in the corpus: SIGGRAPH 2015 "Sky
   Occlusion", 2006 "horizon shadowing", 2021 "Horizon Occlusion").
2. **Three GPU tests fail on pristine HEAD** (unchanged): `cover::gpu_wetness_soaks_and_dries_
   on_the_formula`, `cover::gpu_surface_weathering_follows_the_map`,
   `voxel::gpu_voxel_snow_covers_the_top` (+2.1 h swap bias); plus the eleven-instance adapter
   cap that requires the clouds suite to run in two parts (in AGENTS).
3. **The small cosmetic set** (unchanged): `STRIKE_LEAD=6` outlives the ~43 ms flash decay
   (the doc says "inside the flash"); splash rings sparse on a 6 m roof; near-shore foam
   sub-pixel at nadir; the flood marker outside the capture frustum; erosion-cut faceting; the
   status line clipping at the window's right edge.
4. **Cloud shadow contrast under Clear is still unverified** (carried twice): iteration 5's
   crop missed the cloud mass; capture with cumulus upwind and measure the ground's shadow
   depressions.
5. **The moon's phase is the only moon feature exercised** (new): the aim fix frames the disc,
   and the reviewer confirmed the 37 % crescent renders faithfully — but no sweep has inspected
   the full moon (a month's phase sweep at a fixed hour) or the moon lighting terrain at
   different phases. Cheap to capture (`WorldClock` date), worth one batch.

## Iteration 9 — 2026-10-16 — cloud shadow and illuminance at a point

**Commits:** `c4f9340` (research note), `13f1092` (implementation, example, test, README), `cd77f34` + `da1a7ce` (0.5.1 release, changelog; the second makes `Environment`'s moon term optional so no new resource is required). **Sibling repos:** none. **Aspect:** clouds (iterations 7 and 8 were ground cover and sky).

**Shipped:** merged to `main` (`1e4c0f3`), tag `v0.5.1`, published at https://crates.io/crates/bevy_elements/0.5.1.

### What changed, and why it ranked first

Gameplay could ask for wind, water, snow and wetness at a point but not for light: the cloud shadow map was GPU-only and `CloudShadows::camera_transmittance` is one number for 70 m around the camera. A stealth system, a shade-seeking creature, a solar panel or a growth model needs the cloud shadow and the light **at a position**. New: `Environment::cloud_shadow_at(Vec3) -> Option<f32>` (direct transmittance toward the light, the position projected along the light as the ground material does, 1 outside the 8 km map, `None` before the first readback) and `Environment::illuminance_at(Vec3) -> f32` (clear-sky sun + moon × Kasten & Czeplak's `0.25 + 0.75·T`). The `cloud_shadow` kernel stores every fourth texel's `T` in a 256² buffer (31.25 m cells) inside the existing dispatch; the 4 Hz readback copies it with the origin and light direction captured at request time. Cost: `elements_clouds_shadow` 0.083 → 0.087 ms (min, High, 3840×2160, P2 1590–1725 MHz). New example `cloud_shadow_probes`: a 15×15 grid of unlit markers coloured by the query (yellow open, blue shadowed) with the centre's lux on screen and logged every 120 frames.

### Candidates scored (1–5)

| Candidate | Game hook | Visible | Correctness | Cheap | Sum | Ownership |
|---|---|---|---|---|---|---|
| **Cloud shadow + illuminance at a point** | 5 | 3 | 3 | 5 | **16** | ours |
| Occlusion term for the aerial snow relief (backlog 1) | 1 | 3 | 3 | 3 | 10 | ours |
| The three pre-existing failing GPU tests (backlog 2) | 2 | 1 | 4 | 3 | 10 | ours |
| Cloud shadow contrast under Clear verified (backlog 4) | 1 | 3 | 3 | 5 | 12 | ours (folded into this iteration's inventory) |
| Full-moon / moonlit terrain sweep (backlog 5) | 1 | 3 | 2 | 5 | 11 | ours |
| Save/restore of the weather state | 4 | 1 | 2 | 3 | 10 | ours |

### Shot list (Gemini Flash `screenshot-reviewer`, `/tmp/i9/r1.md`–`r7.md`)

| File | Inventory element | Verdict |
|---|---|---|
| `/tmp/i9/a_pc_above.png` (PartlyCloudy 11:00, 120 m, −25°) | (a) ground shadows, (b) probes | Defect (major): no shadow crossed the 560 m grid (0/225) — test-design defect, fixed by height-scaled spacing and a heading scan |
| `/tmp/i9/b_pc_grazing.png` (6 m, −3°) | (a), (b) | Defect: same cause (0/225) |
| `/tmp/i9/c_clear_above.png` (Clear) | (a), (b) | Pass |
| `/tmp/i9/d_pc_afternoon_high.png` (17:00, 400 m, −45°) | (a), (b) | Defect: grid wholly in one shadow (225/225, probes all blue, consistent) — no edge in view |
| `/tmp/i9/{a,d}_*_crop{2x,4x}.png` | (c) edges | Pass ×4 (no edge in crop) |
| `/tmp/i9/e_overcast.png` | (a), (b) under a deck | Pass: 225/225 blue, ground uniform |
| `/tmp/i9/seq_sheet.png` (6 frames, 1.5 s apart) | (b) temporal stability | Pass (r6): probes stable, lux readout smooth; r3's "blocker" was the 0/225 grid of round 1 |
| `/tmp/i9/c_clear_above_crop2x.png` | (c) | Pass |
| `/tmp/i9/f_pc_1500.png` (1500 m, −55°, 250 m spacing) | (a), (b) | Defect (major) → resolved by crops: the patch is faint at 2–3 km (haze); crops show the probe colour change on it |
| `/tmp/i9/g_pc_800.png` (800 m, −35°, 150 m) | (a), (b) | Defect (major) → resolved by crops: in-patch probes are partial blends (thin cloud), readout threshold 0.5 |
| `/tmp/i9/h_pc_1500_16h.png` (16:00) | (a), (b) | Pass: 220/225 blue, every visible probe blue |
| `/tmp/i9/i_pc_200_grazing.png` (40 m, −4°) | (a) bands with depth | Defect (major): no bands visible — see open item 1 |
| `/tmp/i9/{f,g}_*_crop{2x,4x}.png` | (c) | Pass ×4: probe colour change on the ground's gradient within one spacing; soft edges, no texel stepping |
| `/tmp/i9/j_pc_h300.png` (120 m, −25°, heading 270, 36/225) | (a), (b), (c) | **Pass**: patch 14 % darker, ~32 blue probes all inside it, grey probes on the edge |
| `/tmp/i9/k_pc_h315.png` (heading 315, 33/225) | (a), (b), (c) | **Pass**: diagonal soft edge, grey probes along the 150–158 DN contour |
| `/tmp/i9/l_pc_h315_grazing.png` (8 m, −4°) | (a) | Defect (major): no bands — open item 1 |
| `/tmp/i9/n_pc_grazing30.png` (30 m, −6°, 50/225) | (a), (b) | (b) Pass: blue probes form a coherent receding band; (a) Defect: ground only 4–5 % darker — open item 1 |
| `/tmp/i9/m_tour_pair.png` (weather_tour PartlyCloudy, base `b7529f9` vs this) | before/after | **Pass**: 0 px differ at 2 % fuzz; ground and sky identical |

### Defects found, fixed, re-shot

- Round 1 grid never met an edge (40 m spacing, 560 m): the example's spacing now scales with height (`ELEMENTS_SPACING`, default height/3 ≥ 40 m), the readout is logged, and a heading scan (0–315°, 120 m) found shadowed probes at 270° and 315° — re-shot as j/k, both **Pass** with edges.
- Grazing (a) investigated rather than assumed: a throwaway build tinted `t < 0.5` blue in `weathered.wesl` (reverted, rebuilt). The ground's `t < 0.5` pixels lie exactly where the blue probes are (rows 288–336 of n), and there the debug tint's red channel still reads ~95 DN: the aerial perspective adds ~95 DN at 0.3–1 km, so a ¼-light shadow reads 8 % darker. Query and render agree; the contrast loss is haze.

### Defects still open

1. **Grazing cloud shadows are washed out by the haze** (minor; not this change): from 6–40 m the nearest PartlyCloudy shadows over the origin are 0.3–1 km out, where inscatter adds ~95 DN; they read 4–8 % darker. The note's grazing target (a) is unmet; an example with a shadow nearer the camera, or a check that the haze level at 1 km under `PartlyCloudy` is right, is the follow-up.

### Verification

- New tests: `query::tests` (2, headless: `None` before a readback and the cover estimate for the light; a shadowed cell, its neighbour, a raised point projected along the light, outside the map, illuminance ratio 0.25) and `gpu_clouds::the_cloud_shadow_query_reads_the_shadow_map` (2 704 samples, mean |query − map| 0.0000, 1 314 lit / 1 247 dark, 27 324–109 298 lx).
- Gates: `cargo build --release`, `cargo test` (all suites ok), `cargo build --examples --all-features`, clippy `-D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc`, wasm32 — clean. GPU: integration binaries all ok (gpu_clouds 5, hydrology 9, imports 1, precipitation 6, water 2, weathered 4, wind 2); lib `terrain::`, `dust::` ok; `clouds::` 10 + `the_horizon_sky` 1 ok; `cover::` 5/2 and `voxel::` 0/1 — the three documented pre-existing failures.

### Top 5 remaining items, ranked

1. **Grazing cloud shadows vs haze** (new, open item 1): verify the `PartlyCloudy` aerial perspective at 1 km against the visibility the weather states (95 DN of inscatter at 1 km is a lot for 20+ km visibility); if it is right, frame a near shadow in an example.
2. **Three GPU tests fail on pristine HEAD** (unchanged): `cover::gpu_wetness_soaks_and_dries_on_the_formula`, `cover::gpu_surface_weathering_follows_the_map`, `voxel::gpu_voxel_snow_covers_the_top`.
3. **Occlusion for the aerial snow relief** (carried).
4. **The moon's phase sweep / moonlit terrain** (carried); `illuminance_at` now gives a number to check the moonlight against (full moon ≈ 0.1–0.3 lx).
5. **The small cosmetic set** (carried): `STRIKE_LEAD`, sparse roof splashes, nadir foam, flood marker, erosion faceting, status-line clipping.

## Iteration 10 — 2026-10-16 — gameplay queries stay fresh without a window

**Commits:** `3ad816d` (research note `docs/research/readback-pacing.md`), implementation + test + docs, 0.5.2 release, this log. **Sibling repos:** none.

**Shipped:** merged to `main` (`b6a690a`), tag `v0.5.2`, published at https://crates.io/crates/bevy_elements/0.5.2. **Aspect:** gameplay queries / GPU readback (iterations 8 and 9 were sky and clouds).

### What changed, and why it ranked first

Backlog item 2, the three GPU tests failing on pristine HEAD since iteration 4, turned out to be a consumer-visible bug, not test noise. Traced (temporary prints in `readback.rs`, reverted): in an app with no window every readback's request-to-delivery latency went 2, 4, 8 … frames and the last delivery of any field was 3.1 s into the run; 85 s later `Environment::wetness` answered 0.325 while the map read 0.0. Only presenting a swap chain makes the CPU wait for the GPU, so a headless app (a server, a simulation tool, every GPU test) queued frames without bound and every gameplay query froze on the state of its first seconds. A non-blocking `Poll` per frame only moved the freeze to 23 s: the queue, not the callbacks, was the cause. Windowed apps were fine (`cloud_shadow_probes` logged `snapshot_age` 250 ms, one interval, over 1500 frames).

**The fix**: `pace_without_window` (`src/gpu/readback.rs`, `Render`/`PostCleanup`, after `map_buffers`): a frame with no `ExtractedWindow` waits for its own GPU work (`RenderDevice::poll(PollType::wait_indefinitely())`), so the queue is bounded at one frame and each readback lands on the next frame. With a window it returns at once. No-op on WebGPU.

It ranked first over the haze item (backlog 1): measuring the haze showed the distance fog's colour already matches Bevy's horizon (fog ≈ 0.53 linear display vs horizon sky 0.6–0.7) and the 30 km `PartlyCloudy` extinction gives 12 % fog at 1 km, so the iteration-9 "95 DN of inscatter" was the auto exposure lifting sunlit ground beyond a cloud shadow, not a fog defect — no change there (left in the backlog as a capture item, below).

### Candidates scored (1–5)

| Candidate | Game hook | Visible | Correctness | Cheap | Sum | Ownership |
|---|---|---|---|---|---|---|
| **Readbacks freeze without a window (backlog 2's root cause)** | 5 | 1 | 5 | 5 | **16** | ours |
| Grazing cloud shadows vs haze (backlog 1) | 1 | 3 | 2 | 3 | 9 | ours (measured: no fog defect) |
| Occlusion for the aerial snow relief (backlog 3) | 1 | 3 | 3 | 3 | 10 | ours |
| Moon phase sweep / moonlit terrain (backlog 4) | 1 | 3 | 2 | 5 | 11 | ours (sky: last two iterations) |
| Mesh distance fog double-counts the 40 km clear-air floor that Bevy's aerial perspective also draws | 1 | 2 | 3 | 5 | 11 | ours (Bevy's ground extinction is ~2×10⁻⁵ m⁻¹ against the 9.8×10⁻⁵ floor: ≤ 2 % at 1 km) |
| Save/restore of the weather state | 4 | 1 | 2 | 3 | 10 | ours |

### Inventory and shot list (Gemini Flash `screenshot-reviewer`, `/tmp/i10/review1.md`)

The change creates and alters no visual element: the wait never runs in a windowed app. The inventory is therefore the windowed frame itself, checked before/after.

| File | Inventory element | Verdict |
|---|---|---|
| `/tmp/i10/pair_PartlyCloudy.png` (`base_tour_PartlyCloudy.png` vs `after_tour_PartlyCloudy.png`, `weather_tour` noon) | windowed frame unchanged | **Pass**: 1 px of 921 600 differs by 1 LSB (GPU noise); sky, clouds, horizon, ground, props identical |
| `/tmp/i10/pair_Overcast.png` (`base_tour_Overcast.png` vs `after_tour_Overcast.png`) | windowed frame unchanged | **Pass**: bit-identical (same SHA-256); no banding, shelf or black holes in either half |

### Defects found, fixed, re-shot

None visual. The functional defect is the change itself.

### Defects still open

None from this change.

### Verification

- New test `tests/gpu_readback.rs::the_queries_stay_fresh_without_a_window` (`Ultra`, 256² terrain, 100 ms interval, 1200 frames): oldest snapshot **683 ms before** the fix (fails the 200 ms limit), **83 ms after**.
- Gates: `cargo build --release`, `cargo test` (all suites ok), `cargo build --examples --all-features`, clippy `-D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc`, wasm32 — clean. GPU, one process per suite: integration binaries all ok (gpu_clouds 5, hydrology 9, imports 1, precipitation 6, readback 1, water 2, weathered 4, wind 2); lib `terrain::` 8, `cover::` **7/7**, `dust::` 6, `voxel::` **1/1**, `clouds::` 10 + `the_horizon_sky` 1. **The whole GPU suite is green for the first time since iteration 4.**
- Note for the loop: `.omp/skills/bevy-elements-build-loop/SKILL.md` §6.5 still lists the three tests as pre-existing failures; they now pass and any failure is a regression (the skill is user-authored, so it is not edited here).

### Top 5 remaining items, ranked

1. **Occlusion for the aerial snow relief** (carried).
2. **The moon's phase sweep / moonlit terrain** (carried); `illuminance_at` gives the number to check (full moon ≈ 0.1–0.3 lx).
3. **Save/restore of the weather state** (new to the top 5): a game hook for save games — `Weather`, `WorldClock`, the director's schedule and the cover/water layers.
4. **Grazing cloud-shadow capture** (re-scoped from iteration 9's item 1): the fog is right; frame a shadow within 300 m of a grazing camera so the shadow contrast can be judged without 1 km of air.
5. **The small cosmetic set** (carried): `STRIKE_LEAD`, sparse roof splashes, nadir foam, flood marker, erosion faceting, status-line clipping.

## Iteration 11 — 2026-10-16 — the weather can be saved and restored

**Commits:** `7a0a241` (research note `docs/research/weather-save.md`), implementation + test, 0.5.3 release, this log. **Sibling repos:** none.

**Shipped:** merged to `main` (`8f90456`), tag `v0.5.3`, published at https://crates.io/crates/bevy_elements/0.5.3. **Aspect:** climate and weather / save state (iterations 9 and 10 were clouds and GPU readback).

### What changed, and why it ranked first

Backlog item 3. A game could serialize `WorldClock`, `Climate`, `GeoLocation` and `Weather`, but not `WeatherDirector`: private fields, no `serde`, no `Reflect`. A reloaded save therefore started a fresh director, which arms at `Clear` on its first frame, drops a forced preset, cuts a blend in progress, and then walks a different Markov chain (each hour's step depends on the previous preset). `WeatherDirector` and its private `Blend`, `Transition` and `Request` now derive `Reflect`, `PartialEq` and (feature `serde`) `Serialize`/`Deserialize`. The scheduler's draws are a pure hash of `(seed, hour)`, so the chain position is the only state; no RNG cursor exists to save. The highest Game hook of the open items, a small CPU-only surface, no GPU cost. The GPU layers (snow cover, wetness, water, terrain) are out of scope and listed below.

### Candidates scored (1–5)

| Candidate | Game hook | Visible | Correctness | Cheap | Sum | Ownership |
|---|---|---|---|---|---|---|
| **Save/restore of the weather director (backlog 3)** | 5 | 1 | 4 | 5 | **15** | ours |
| Save/restore of the GPU layers (cover, water, terrain) | 5 | 1 | 3 | 1 | 10 | ours (needs whole-layer readback and upload) |
| Occlusion for the aerial snow relief (backlog 1) | 1 | 3 | 3 | 3 | 10 | ours |
| Moon phase sweep / moonlit terrain (backlog 2) | 1 | 3 | 2 | 5 | 11 | ours |
| Grazing cloud-shadow capture (backlog 4) | 1 | 3 | 1 | 5 | 10 | ours (capture only) |

### Inventory and shot list (Gemini Flash `screenshot-reviewer`, `/tmp/i11/review1.md`)

The change creates and alters no visual element (CPU derives only). The inventory is the windowed frame, before (`main`, `/tmp/base`) and after.

| File | Inventory element | Verdict |
|---|---|---|
| `/tmp/i11/pair_PartlyCloudy.png` (`base_`/`after_PartlyCloudy.png`, `weather_tour` noon) | windowed frame unchanged | **Pass**: 0 px differ at 2 % fuzz, max 2/255 |
| `/tmp/i11/pair_Thunderstorm.png` (`base_`/`after_Thunderstorm.png`) | windowed frame unchanged | **Pass**: 0.3 % px differ, all thin rain streaks (a 2×2 opening removes every one); mean signed delta ≈ 0, no sky/cloud/ground change |

### Defects found, fixed, re-shot

None.

### Defects still open

None from this change.

### Verification

- New `tests/save.rs` (`required-features = ["serde"]`, `serde_json` dev-dependency): a director saved through JSON after 72 game hours and loaded into a fresh app gives the same `Weather` as the uninterrupted run at all 720 updates of the next 72 h (with preset changes in that window); restoring only the clock diverges; a director saved one hour into a two-hour forced Thunderstorm blend restores forced, at the same blend, and reaches 25 mm/h. Before the fix the test does not compile (`WeatherDirector: serde::Serialize` not satisfied).
- Gates: `cargo build --release`, `cargo test` (all suites ok), `cargo test --features serde --test save` (2 ok), `cargo build --examples --all-features`, clippy `-D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc`, wasm32 — clean. GPU, one process per suite: gpu_clouds 5, hydrology 9, imports 1, precipitation 6, readback 1, water 2, weathered 4, wind 2; lib `terrain::` 8, `cover::` 7, `dust::` 6, `voxel::` 1, `clouds::` 10 + `the_horizon_sky` 1 — all ok.
- `hs status` failed in the loop's shell ("No such device or address", no TTY); research used the synced corpus (Gregory 2018 §16.4.5) and the existing climate-weather sources.

### Top 5 remaining items, ranked

1. **Save/restore of the GPU layers** (new): snow cover, wetness, water and terrain live in storage buffers; a save needs a whole-layer readback and a restore an upload through `Assets<ShaderBuffer>`.
2. **Occlusion for the aerial snow relief** (carried).
3. **The moon's phase sweep / moonlit terrain** (carried); check against `illuminance_at` (full moon ≈ 0.1–0.3 lx).
4. **Grazing cloud-shadow capture** (carried): frame a shadow within 300 m of a grazing camera.
5. **The small cosmetic set** (carried): `STRIKE_LEAD`, sparse roof splashes, nadir foam, flood marker, erosion faceting, status-line clipping.

## Blocked pass — 2026-10-17 — the Opus 5.5 reviewer cannot run

No iteration was started. The skill's preflight sent one image to the `screenshot-reviewer` agent (`openrouter/anthropic/claude-opus-5.5`) before any capture, and it failed in 0.6 s:

> 402 This request requires more credits, or fewer max_tokens. You requested up to 65536 tokens, but can only afford 42467. To increase, visit https://openrouter.ai/workspaces/default/keys/bd57b3406fa7580c26d459649a68144eccacb275026701a7bfaf041bef55a088 and adjust the key's monthly limit

Every iteration needs a GREAT verdict from this reviewer, so the pass stopped (skill Rules: a reviewer that cannot run is a blocker). Unblock by raising the OpenRouter key's monthly limit. 42 467 affordable output tokens at $20 per 1M is about $0.85 left, under one multi-image review.

State at the stop: `main` at `096719b`, every gate and GPU suite green (2026-10-17). home-still healthy (distill idle on CUDA, both scribes up). The backlog is unchanged from iteration 11, plus the crate-review fixes F1–F11 that landed between iterations (`12ad5f3`..`4cf0527`). F12 (bind groups re-created every frame) was not fixed and is not measured.

Second pass, same day: the reviewer preflight failed again in 0.4 s, `402 … You requested up to 65536 tokens, but can only afford 42177.` The key's limit has not been raised; the pass stopped before any capture.

Third pass, same day: preflight failed again, `402 … can only afford 42174.` Stopped before any capture.

## Iteration 12 — 2026-10-17 — twilight, the sunset exposure and the night sky

**Commits:** `5a96d37` (research note `docs/research/twilight.md`), `2559c3f` (implementation + `tests/gpu_twilight.rs`), `80f3772` (README, CHANGELOG, `celestial.md` Deviation 19, regenerated stills), this log. **Sibling repos:** none. **Not released** (the loop no longer bumps, tags or publishes). **Visual judgment:** by the loop agent itself (user decision 2026-10-17, after the Opus 5.5 reviewer hit OpenRouter's credit limit three passes in a row).

### What changed, and why it ranked first

The baseline sweep (29 captures, `/tmp/i12/before/`, findings in `/tmp/i12/critique.md`) found three linked defects in the sky, each measured on a linear `Rgba16Float` target:

1. **Every night sky was brown, ~40 DN.** Bevy's default `ClearColor` (sRGB 43, 44, 47) is written behind the sky before any exposure and Bevy's atmosphere draws over it with its transmittance: a constant linear (0.022, 0.0205, 0.0186) behind every sky pixel with every light off, on four dates.
2. **Twilight did not exist.** The sun's light was switched off at −2° (an unsourced M1 constant), so from −2° to −18° Bevy's atmosphere had no light; what showed was item 1.
3. **The low sun blew the frame out.** The illuminance-metered exposure left the sky toward the sun at 6–20× display peak from +12° to −2° (`dn_dusk.png` zenith 245 DN; `wt_sun_18.6` 252 DN).

Fixed: the sun lights the atmosphere to −18°, scaled below the horizon by `g(h)` fitted on Bevy's renderer to Hulburt's measured zenith twilight (`k` 0.44/° to −6°, then Hulburt's own 0.983/°); below the horizon its cloud factor is the dome's total so the composite's `sky_gain` is 1; the precipitation draw takes the sun only while its disc is up; the auto exposure is the larger of the illuminance target and a cap from the sky toward the sun (Patry 2021's highlight threshold, ceiling 3.2× display peak, the glow at a 27° sun); a camera that kept Bevy's default clear colour is cleared to the night sky's own glow (`E_floor/π` = 1.6×10⁻⁴ cd/m²) at its exposure.

It ranked first because the twilight sky had no light source at all, wrong for an hour of every evening in any game with a day cycle, and its fix also needed the exposure and the background. Runner-up: the flat, washed-out terrain shading on `hills` and `mountains`, a matter of degree rather than a missing light.

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **Twilight missing, brown night sky, low-sun blowout** | 1 (wrong sky for the time) | picked |
| 2 | Flat terrain shading (`tt_hills42_eye`, `tt_mtn_elev`) | 2 (beauty) | runner-up |
| 3 | Night thunderstorm rain reads as white static (`storm.png`) | 2 | needs a look at streak brightness under a flash |
| 4 | Clouds in twilight flat grey; moonlit cloud rims | 2 | cloud lighting below `SUN_HANDOVER_DEG` |
| 5 | Pillar edge steps through the cloud composite (`dn_dusk` 4× crop) | 1 (cosmetic) | composite against geometry |
| — | Backlog: GPU-layer save, aerial snow occlusion, moon phase sweep, grazing shadow capture, cosmetic set | | carried |

### Sources

Hulburt 1953, *JOSA* 43:113 (`10.1364/JOSA.43.000113`); Haber, Magnor & Seidel 2005, *ACM TOG* 24(4) (`10.1145/1095878.1095884`) — both fetched through SciDB and read from `pdftotext`, not yet ingested (S3 upload failure, note's `Pending index`); Bruneton & Neyret 2008 (`10.1111_j.1467-8659.2008.01245.x`); Hillaire 2020 (`10.1111_cgf.14050`); Patry 2021 (`s2021-advances-pdf-66-mb`); Jensen et al. 2001 (`10.1145_383259.383306`, through `hs distill search`); Bevy 0.20 source (`pbr_lighting.wesl` visible-sun factor, `volumetric_fog.wesl`, `ScatteringMedium::earth`'s ozone).

### Critique and verdicts (`/tmp/i12/critique.md`; before `/tmp/i12/before/`, after `/tmp/i12/after/`)

| Feature | Evidence | Verdict |
|---|---|---|
| Twilight sky, toward the sun (−2.2° … −10.9°) | `after/tw/wt_sun_19.{0,3,6,9}.png`, `cmp/sheet_wt_sun_19.3.png` | GREAT: violet sky over an orange band shrinking to a red afterglow under navy with stars |
| Twilight sky, away from the sun | `after/tw/wt_away_19.{0,3,6}.png` | GREAT: Earth's shadow under the pink Belt of Venus |
| Golden hour / sunrise exposure | `after/tw/wt_sun_18.6.png`, `wt_away_18.6.png`, `dn_dawn.png` | GREAT: no white field, orange cloud rims, silhouettes |
| Zenith (GPU) | `tests/gpu_twilight.rs` | 0.94×, 1.01×, 0.65× of Hulburt at −2.2°, −3.9°, −5.5°; blue-purple |
| Night background | `after/tw/wt_night_23.png`, `wt_*_21.2.png` | GREAT; cosmetic: the horizon reads (9,5,2) DN, the neutral glow reddened by the horizon transmittance, visible only stretched |
| Continuity | `/tmp/i12/lapse_after.png`, `lapse_after.gif` (19:24 → 22:36) | GREAT on round 3 (round 1: olive bloom then a snap to black from Bevy's multiple-scattering floor, fixed by the −6° tail; round 2: the same from `sky_gain` amplifying the night 250×, fixed by the dome-total cloud factor) |
| Clouds in twilight / moonlit | `after/tw/wt_sun_19.3.png`, `wt_sun_21.2.png` | not touched; NOT YET, next item |

Daytime regression check (MAE against before, same frames): every noon and terrain capture ≤ 0.002 except `wt_PartlyCloudy_noon` 0.086 and `wt_Fog_noon` 0.059 — deterministic (after-vs-after 0): a camera in a cloud shadow no longer opens up past the sunlit sky (0.4 stop darker, richer sky), kept and documented. `docs/storm.png` was not regenerated: its change is the flash moment (nondeterministic), not this iteration.

### Verification

- New tests: `tests/gpu_twilight.rs` (3 GPU tests; all three fail on `eaf51e0`: glow 21.9× peak, night floor 0.022, zenith ratio 0.066 against Hulburt's 0.18), `celestial` unit tests for the twilight factor, the disc factor, the glow cap's range and the sky gain once the sun has set.
- Gates: `cargo build --release`, `cargo test` (all ok, lib 221; `the_lightning_schedule_is_frozen` unchanged), flora/voxel feature tests, `cargo build --examples --all-features`, clippy `-D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc`, wasm32 — clean. GPU, one process per file/module: clouds 5, hydrology 9, imports 1, plugins 1, precipitation 6, readback 1, twilight 3, water 2, weathered 4, wind 2; lib `clouds` 10 + `the_horizon_sky` 1, `terrain` 8, `cover` 7, `dust` 6 — all ok.
- Performance: no GPU pass added; the sun's shadow cascades are now off below −0.5° (they were on to −2°). `bench_elements` at the default 4K/High and at 1080p could not allocate: another tenant (`VLLM::EngineCore` 15.9 GB + `hs-distill-server` 7.5 GB) held 23.5 of the card's 24 GB. Measured at `ELEMENTS_QUALITY=low ELEMENTS_TERRAIN=none ELEMENTS_PRESET=PartlyCloudy`, 1920×1080, GPU P2 at 1755–1800 MHz logged in the same shell: total min 0.571 ms before, 0.558 ms after; every pass's minimum within 0.006 ms.
- `lake` example: out of GPU memory in the same shared-memory state (environment, not a defect).

### Top 5 remaining items, ranked

1. **Clouds in twilight and moonlight**: post-sunset cumulus are flat neutral grey (no warm underside for the first degrees, no sky tint), and moonlit cloud edges form a light rim around a dark core against the now-dark sky (`pick_light` hands over at −4°; `cloud_ambient` tint).
2. **A view-dependent exposure cap**: the glow cap ignores where the camera looks, so a camera facing away from a low sun is up to ~2.5 stops darker than the illuminance alone (`docs/dawn.png`); meter the brightest sky in the frame (Patry's screen highlights).
3. **Flat terrain shading** (runner-up): `hills` reads uniform green at noon, `mountains` pale at 16 h.
4. **Night thunderstorm rain** (`storm.png`): flash-lit streaks read as white static over a black sky.
5. **Save/restore of the GPU layers** (carried) and the cosmetic set (`STRIKE_LEAD`, sparse roof splashes, nadir foam, flood marker, erosion faceting, status-line clipping, the composite's pillar edge step).

## Iteration 13 — 2026-10-17 — clouds take the colour of the light that fills them

**Commits:** `9ec2c62` (research note, `clouds.md` Deviation 30), `8de42a5` (implementation + `tests/gpu_cloud_ambient.rs`; the note's Deviation 30 and Validation 22 rewritten to the shipped model), `d3af2cf` (README, CHANGELOG, regenerated stills), this log. Merged through a pull request. **Sibling repos:** none. **Not released.**

### What changed, and why it ranked first

The clouds' ambient (the diffuse light on their shaded sides) was `vec3(ambient)`, a neutral grey scalar whatever the sky, so after sunset every cloud was grey cardboard against a violet or navy sky (civil twilight base (57, 57, 59) under a violet sky; nautical twilight (76, 77, 77) against navy (49, 52, 68)). Backlog item 1 of iteration 12. The ambient now takes the chromaticity of the diffuse light it stands for: the clear sky's irradiance from the camera's atmosphere environment map (16 Fibonacci directions over the upper hemisphere, divided by the cloud factor the sun's light carries so the sky above the layer is undimmed, plus the night-sky background) plus the share of the light's beam the cloud above the sample has scattered, $E_\odot \hat l_y (1 - e^{-\tau_l})$. Its luminance is unchanged.

Round 1 (sky colour alone) gave royal-blue night clouds in a brown-black sky: the cube held a moonlit atmosphere negligible next to the airglow floor; adding the floor fixed it. Round 2 passed the twilight frames but the GPU veil test failed (far-row tint 0.249 vs 0.126) and the horizon capture showed why: the noon `Overcast` deck turned periwinkle above a white band, because deep in a thick deck only the ambient is left. Round 3 adds the scattered sun, the physical reason behind Hillaire's "slightly blue" clouds and his artists' desaturation knob, which the model therefore does not need.

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **Grey cloud ambient against coloured twilight skies** | 1 (wrong colour for the light) | picked |
| 2 | View-independent exposure cap (anti-solar sky darker than its clouds at −8°) | 2 | runner-up, owns the nautical contrast |
| 3 | `tt_mtn_pc_aerial` 2-px dark line at y ≈ 298 | 1 (cosmetic) | |
| 4 | Overcast at −4° a featureless flat deck | 2 | |
| 5 | Rain at noon: a uniform near-sky wall | 2 | |

### Sources

Hillaire 2016 §5.5.1 (ambient from Frostbite's sky probe; desaturation because clouds came out slightly blue; single-colour ambient listed as the first thing to improve), Schneider & Vos 2015 (`s2015-advances-ppt-944-mb-pdf-5-mb-2`, ll. 1218–1240: ambient from the sky by time of day), Bevy 0.20 `atmosphere/environment.wesl` (the cube is `sample_sky_view_lut`, absolute radiance before exposure).

### Critique and verdicts (`/tmp/i13/critique.md`; before `/tmp/i13/before/`, after `/tmp/i13/after/`, horizon `/tmp/i13/hz/`)

| Feature | Evidence | Verdict |
|---|---|---|
| Civil twilight (−4.2°), PartlyCloudy and Overcast | `wt_*_sun_19.2`, `wt_Overcast_away_19.2` | GREAT: violet-grey base (62, 55, 62), rose-grey deck (136, 126, 135) |
| Nautical twilight (−8.1°) away from the sun | `wt_PartlyCloudy_away_19.6` | GREAT: mauve (81, 74, 83) from the glow that lights it; its brightness against the navy sky is the runner-up's |
| Golden hour (+1.8°) | `wt_PartlyCloudy_sun_18.6` | GREAT: warm taupe body (74, 59, 56) under the orange rim |
| Noon PartlyCloudy, Overcast, Snow, horizon | `r3_day.png`, `hz/sheet3.png` | unchanged neutral decks (round 2's blue overcast fixed) |
| Night (crescent moon) | `wt_*_21.2` | the night sky's hue (14, 11, 9) |
| Continuity | dusk lapse 19:00 → 21:48 (2026-06-15), 20 consecutive frames at 20:30 | monotone through the sun→moon handover, ≤ 1 DN a frame |

### Verification

- New test `tests/gpu_cloud_ambient.rs::the_deck_takes_the_colour_of_its_light`: dusk red/green 1.26 vs 1.02 neutral; noon blue/red 1.05× neutral, bounded to (1.0, 1.1) (the sky-colour-only build gave 1.27×); fails on `162c33b` (identical decks with and without the map). Validation 20 (veil) holds: far-row tint differences ≤ 0.036.
- Gates: `cargo build --release`, `cargo test` (0 failures), `cargo build --examples --all-features`, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32 — clean. GPU, one process per file/module: every `tests/gpu_*.rs` file and lib `clouds` (10), `terrain` (8), `cover` (7), `dust` (6), `voxel` (1), `the_horizon_sky` — all pass.
- Performance (`bench_elements`, `ELEMENTS_QUALITY=low ELEMENTS_TERRAIN=none ELEMENTS_PRESET=PartlyCloudy`, 1920×1080, P2 at 1740–1860 MHz logged in the same shell, two alternating rounds): `elements_clouds_march` min 0.190 / 0.184 ms before, 0.184 / 0.189 ms after; total 0.598 / 0.587 against 0.573 / 0.590 ms. No measurable cost. (The shared card still holds ~22 GB for other tenants; High/4K benches do not fit.)
- Stills: `docs/cloud_sunset.png`, `twilight.png`, `belt_of_venus.png`, `sunset.png` regenerated at their recovered poses (the base build reproduces the old stills to MAE ≤ 0.0008); `tour.gif` (noon) unchanged by this iteration.

### Next feature (user direction, 2026-10-17)

Asked to review the crate for the single feature most useful to game developers: **interactive ground trails** — feet, wheels and bodies pressing tracks into snow, sand and mud that new snowfall, wind and time erase. Every interaction the crate offers runs one way (environment → game: `Environment`, `WeatheredMaterial`, `PrecipitationBlocker` shelter); nothing a character does marks the world, and the terrain's 4 m cells cannot hold a footprint. Literature pulled for it: Sumner, O'Brien & Hodgins 1999 (arXiv 2302.08683), Alvarado et al. 2022 (`10.3389/frvir.2022.801856`), Su et al. 2023 (`10.1145/3610548.3618159`), Goswami 2024 survey (`10.1111/cgf.15059`), Hanák 2021 (MUNI thesis, surveys AC III, Batman, Rise of the Tomb Raider, HZD Frozen Wilds, TLOU II), Svensson 2019 (Umeå thesis), Barré-Brisebois 2014 (GDC), CRREL Report 90-9 (shallow-snow sinkage), Wong's Bekker table via NASA TM-20250006958, Tatarchuk 2006 POM (`s2006-advances-materials-10`).

### Top 5 remaining items, ranked

1. **Interactive ground trails** (the next iteration, above).
2. **A view-dependent exposure cap** (carried): meter the brightest sky in the frame.
3. **Flat terrain shading** (carried).
4. **Night thunderstorm rain** reads as white static (carried).
5. **Save/restore of the GPU layers** and the cosmetic set (the aerial's 2-px line at y ≈ 298, the composite's pillar edge step, `STRIKE_LEAD`, sparse roof splashes).

## Iteration 14 — 2026-10-17 — trails: footprints and ruts in snow, sand and mud

**Commits:** `9033ff1` (research note `docs/research/trails.md`), the implementation with `tests/gpu_trails.rs` (the note's Deviations 1–10 and measured validations), the README / CHANGELOG / stills / GIF, this log. Merged through a pull request. **Sibling repos:** none (`bevy_flora` owns no ground marks). **Not released.**

### What changed, and why it ranked first

Every interaction the crate offered ran from the environment to the game; nothing a character did marked the world, and the terrain's 4 m cells cannot hold a footprint. `TrailsPlugin` adds the missing direction: an `Imprint` presses a camera-following window of centimetre texels (High 1024² × 5 cm) to its Bekker–Wong sinkage (Wong's constants; CRREL's 500 kg/m³ compaction limit in snow; Alvarado's 0.30 m loose layer), with a rim of `2ν` of the displaced volume, sweeping the path between frames. The cover map's new snow fills a trench by what it gained, melt clamps it, the snow drift and sand flux diffuse it, sand and mud slump (Sumner's angles). The terrain material and `GroundExtension` draw it per pixel: parallax occlusion mapping with per-texel steps and a B-spline bisection, a soft self-shadow, an occlusion term, compacted-snow tint, soil under grass and water on wet rut floors.

Eighteen capture rounds. The defects found and fixed, in order: rims stacking into fins (a plastic rim target), ruts refilled by their own rims, a 14 cm parallax step against 5 cm texels, stair-stepped bilinear walls (B-spline refinement), on/off shadow blocks across sand ruts, mirror-white mud ruts (water only on floors), fluted wet walls (the normal from the B-spline, not a stored gradient), regular dark ticks on a rut's far wall (shadow samples spaced by each pixel's own reach), and grid-aligned slump steps (eight neighbours).

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **Interactive ground trails** | 3 (missing game ↔ world interaction) | picked (user direction, iteration 13) |
| 2 | View-dependent exposure cap | 2 | carried |
| 3 | Flat terrain shading | 2 | carried |
| 4 | Night thunderstorm rain as white static | 1 | carried |
| 5 | Save/restore of the GPU layers | 3 | carried |

### Sources

Sumner, O'Brien & Hodgins 1999; Alvarado et al. 2022; Su et al. 2023; Goswami 2024 (survey); Hanák 2021 (AC III, Batman, Rise of the Tomb Raider, HZD, TLOU II); Svensson 2019; Barré-Brisebois 2014; CRREL 90-9; NASA TM-20250006958 (Wong's Bekker constants); Tatarchuk 2006 (POM, `s2006-advances-materials-10`); Sigg & Hadwiger 2005 (B-spline filtering). Paywalled and not obtained: Onoue 2005, Zeng 2007, Dagenais 2016, Michels (GPU Pro 7), Festenberg 2011.

### Critique and verdicts (`/tmp/trails/critique.md`; rounds `/tmp/trails/r1`–`r18`, final `/tmp/trails/final/`)

| Feature | Evidence | Verdict |
|---|---|---|
| Snow prints and ruts, 10.5 h and 15.6 h | `final/snow.png`, `final/snow_low.png`, `r16/snow_top_crop.png` | GREAT: crisp pockets, blue-shadowed walls, sunlit lips |
| Dry sand at 18.3 h | `final/sand.png` | GREAT: slumped troughs with low rims; the shaded wall's silhouette steps per pixel (no AA, `r17/comb*.png` proves it is not the field) |
| Mud after rain | `final/mud.png`, `final/mud_wall.png` | GREAT: soil floors, wide low rims, no fluting or ticks |
| Dry sand at noon (62°) | `r16/sand_walk_zoom.png` | faint, as real 3.4 cm dishes are; recorded in Validation 7 |
| Snowfall (×120) | `r18/fill.png` | softens at the cover's 1.5 cm/h; a day to fill, not the plan's hour |
| Motion | `docs/trails.gif` (frames 1080–1500) | no swimming, no window seam |

### Verification

- `tests/gpu_trails.rs` (4): the press depth 23.65 cm for the model's 24.0 and a rim of 9.9 % for `2ν` = 10 %; a 20 m/s rut with no texel shallower than 24 cm; 5 cm of new snow shallows a trench 23.65 → 18.64 cm and leaves the flat at 0; a trail stays put as the camera moves 10 m and is gone after it leaves the window. Five unit tests in `trails::model` (sinkage, soil monotone in wetness, rock, window centre, oversized sweep).
- Gates: `cargo build --release`, `cargo test` (0 failures), `cargo build --examples --all-features`, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32, rustfmt on this iteration's files, `tests/save.rs` with `--features serde` (2). GPU, one process per file/module: every `tests/gpu_*.rs` file (trails 4, hydrology 9, precipitation 6, clouds 5, weathered 4, twilight 3, water 2, wind 2, cloud ambient, imports, plugins, readback) and lib `clouds` (10), `terrain` (8), `cover` (7), `dust` (6), `voxel` (1), `the_horizon_sky` — all pass.
- Performance (`bench_elements`, High, hills, `PartlyCloudy`, 1080p, P2 at 1.5–1.9 GHz logged in the same shell, two rounds each): `elements_trails` 0.119 ms with 16 wheels (0.187 ms before untouched texels skipped their ground lookups); `main_opaque_pass_3d` 0.054–0.058 ms off, 0.057–0.058 ms on.
- Stills `docs/trails_snow.png`, `trails_sand.png`, `trails_mud.png` and `docs/trails.gif` (new); `tour.gif` unchanged by this iteration.

### Top 5 remaining items, ranked

1. **A view-dependent exposure cap** (carried): meter the brightest sky in the frame.
2. **Flat terrain shading** (carried).
3. **Night thunderstorm rain** reads as white static (carried).
4. **Trails on `VoxelGround` and the wind's solid mask**: an imprint does not yet change the wind or the hydrology (the field is visual and erodes, it does not drain).
5. **Save/restore of the GPU layers** and the cosmetic set (the aerial's 2-px line, the composite's pillar edge step, `STRIKE_LEAD`, sparse roof splashes, POM silhouettes without AA).

## Iteration 15 — 2026-10-18 — waves: ripples and wakes from bodies in the water

**Commits:** `ec7027d` (research note `docs/research/waves.md`), the implementation with `tests/gpu_waves.rs` (the note's Deviations 1–10 and measured validations), the README / CHANGELOG / stills / GIF, this log. Merged through a pull request. **Sibling repos:** none (no sibling owns open water). **Not released.**

### What changed, and why it ranked first

The hydrology fills lakes and rivers, but nothing that moved through them disturbed them: a boat, a wader or a dropped stone passed through a glassy flow-mapped surface. Trails gave the ground an answer to what crosses it; the water had none. `WavesPlugin` adds it: a `WaterContact` ellipsoid presses the water with the pressure head of its occupied column (Colen & Kolomeisky's pressure-forced oscillator, so a body at rest leaves the water flat around it) plus Bernoulli's impact head while it enters; a linear capillary-gravity field in three camera-following cascades (iWave, a 13 × 13 kernel least-squares fitted to Airy's dispersion per cascade, Canabal's analysis) carries the waves, reflected by the shores, slowed by depth, advected by the current, broken at `ak` = 0.44 into foam. The water's two passes add its slopes and a lace of foam.

Eleven capture rounds. The defects found and fixed, in order: no wake (the example scaled the contact with the hull's mesh), tan hard-edged reflections on wave faces (reflections below the horizon sampled Bevy's ground disc; now held at the horizon), foam a featureless smear (now lace), a falling pebble making no rings (the entry head), a pebble's foam a notched rectangle (five-point binomial and a coverage cap), checkerboards at the forcing (the band split's 3 × 3 binomial).

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **Interactive water: ripples and wakes from bodies** | 3 (missing game ↔ world interaction) | picked: the water is the one simulated surface that ignored what crossed it |
| 2 | Save/restore of the GPU layers | 3 | carried |
| 3 | View-dependent exposure cap | 2 | carried |
| 4 | Flat terrain shading | 2 | carried |
| 5 | Night thunderstorm rain as white static | 1 | carried |

### Sources

Tessendorf 2004 (iWave; *Simulating Ocean Water* course notes); Canabal et al. 2016 (dispersion kernels); Ottosson 2011 (Frostbite's interactive water); Crest (`UpdateDynWaves.compute` and docs); Yuksel et al. 2007 (wave particles); Jeschke & Wojtan 2017 (breaking at `a` = 0.07λ); Schreck et al. 2019; Kass & Miller 1990; Colen & Kolomeisky 2021 (pressure-forced surface, eq. 8); Darmon et al. 2014 (Kelvin's 19.47° for all Froude numbers, the apparent narrowing); `s2006-advances-materials-5/6` (ToyShop's water); `eth-cgl-sim_anim-Sol11b`.

### Critique and verdicts (`/tmp/i15/critique.md`; rounds `/tmp/i15/r1`–`r11`, final `/tmp/i15/final/`)

| Feature | Evidence | Verdict |
|---|---|---|
| Hull wake at 16.5 h | `r11/sheet.png` (side, behind, front, over), `final/gif/` | GREAT: Kelvin's V with transverse crests, silvered by the horizon, lace foam behind the stern |
| Pebble rings | `final/peb_strip.png` | GREAT: short wavelets outrun long ones; the foam dissolves |
| Wader | `r11/wader.png` | GREAT: two short V wakes and flecks |
| Float | `r11/float.png` | right but subtle: 0.7 Hz rings are 3 m swells |
| Window seam | `r11/main_far.png` | none where the 25.6 m window ends |
| Motion | `docs/waves.gif` (frames 1500–1900) | no swimming as the camera rides with the hull |

### Verification

- `tests/gpu_waves.rs` (3): a body at rest holds the water down under it (−3.63 cm) and nowhere else (0.09 mm 3.5 m away), unchanged after the camera moves 3 m; a hull at 2 m/s leaves a wake whose edge stays within 19.1° (Kelvin 19.47°) with a 7.7 cm peak; waves do not cross onto dry ground (4.6 cm on the water, 0 on the bank). Four unit tests in `waves::model` (the kernels' dispersion within 3 % over 3–12 texels and stable, a floating sphere's head, a rotated ellipsoid's chord, a falling body striking only as it crosses).
- Gates: `cargo build --release`, `cargo test` (0 failures), `cargo build --examples --all-features`, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32, rustfmt on this iteration's files, `tests/save.rs` with `--features serde` (2). GPU, one process per file/module: every `tests/gpu_*.rs` file (waves 3, hydrology 9, precipitation 6, clouds 5, trails 4, weathered 4, twilight 3, water 2, wind 2, cloud ambient, imports, plugins, readback) and lib `clouds` (10), `terrain` (8), `cover` (7), `dust` (6), `voxel` (1), `the_horizon_sky` — all pass.
- Performance (`bench_elements`, High, lake, `ELEMENTS_CONTACTS=8`, `PartlyCloudy`, 1080p, P2 at 1.74–1.80 GHz logged in the same shell, two rounds on and off): force 0.037, step 0.217, publish 0.033 ms (0.29 ms; 0.23 ms on still water; Low 0.15 ms), above the note's 0.15 ms and logged as `waves.md` Deviation 10. The GPU was shared with a vLLM server during the runs.
- Stills `docs/waves_wake.png`, `docs/waves_rings.png` and `docs/waves.gif` (new).

### Top 5 remaining items, ranked

1. **The sky darkens toward black over a filled lake at 17.5 h** (found here; identical with `ELEMENTS_WAVES=off`, so not the waves).
2. **Buoyancy readback**: publish the wave height under each `WaterContact` so a float can ride the waves it and others make.
3. **Save/restore of the GPU layers** (carried).
4. **A view-dependent exposure cap** and **flat terrain shading** (carried).
5. **Wave cost**: step the coarse cascades at 30 Hz or skip windows with no body and no waves; refraction of what is under the water (the pebble draws where it is, not where it appears).

## Iteration 16 — 2026-10-18 — the sun under broken cloud: a steady sky under a cloud's shadow, a hidden disc

**Commits:** `3ea0d0f` (research note `docs/research/broken-cloud-light.md`), the implementation with the new GPU test, the README / CHANGELOG / still / tour GIF, this log. Merged through a pull request. **Sibling repos:** none. **Not released.**

### What changed, and why it ranked first

Iteration 15's top backlog item, "the sky darkens toward black over a filled lake at 17.5 h", reproduced on `main` and turned out to be general: the sun's and moon's lights carried the cloud transmittance of the 70 m around the camera, and Bevy lights its sky, aerial perspective and environment map with them, so every cumulus that shaded the camera took the whole sky with it — black in `waves` at 17.5 h (the camera's `T̄` reached 0 and `sky_gain` had nothing to scale), grey in `terrain_tour` under `PartlyCloudy` within seconds of a shadow arriving. A second defect of the same scene: the sun disc burned through a cumulus's dark base, because the march kept the last 1 % of the sky behind an opaque cloud and the disc is 10⁵ times the sky. Category 1 (visible, physically wrong, in the default `PartlyCloudy` of the flagship example); it beats the runner-up (buoyancy readback, category 3) because every game with fair-weather cumulus hit it.

The lights now carry the shadow map's mean over 8 km (Hillaire 2016 eq. 22's hemisphere mean; Kasten & Czeplak's mean over points), computed on the CPU from the 256² grid already read back for `Environment::cloud_shadow_at`; the crate's surfaces keep their per-pixel shadow relative to it. `CloudShadows::camera_transmittance` became `mean_transmittance` (migration note in the CHANGELOG); the camera kernel and its readback are deleted. An exhausted march is opaque (Schneider & Vos 2015). Bevy's `DirectionalLightTexture` would have shadowed every `StandardMaterial` too, but it needs clustered decals, which Metal and WebGPU lack; the `waves` example's bodies became `WeatheredMaterial` so they take the shadow.

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **The sky follows the camera's cloud shadow; the sun shines through cumulus** | 1 (visible artifact) | picked |
| 2 | Buoyancy readback for `WaterContact` | 3 | carried |
| 3 | Save/restore of the GPU layers | 3 | carried |
| 4 | View-dependent exposure cap; flat terrain shading | 2 | carried |
| 5 | Wave cost (coarse cascades at 30 Hz) | 5 | carried |

### Sources

Hillaire 2016 §4.1.2, §5.9, §5.9.2 (local text); Wenzel 2006 (`s2006-advances-materials-12`); Ubisoft Montréal 2025 (`s2025-advances-materials-3`); Patry 2021 (`s2021-advances-pdf-66-mb`); Schneider & Vos 2015 (`s2015-advances-ppt-944-mb-pdf-5-mb-2`); Mol & van Heerwaarden 2025 (`10.5194_acp-25-4419-2025`, downloaded); Mol et al. 2023 (`10.1029_2022jd037894`, downloaded); Kasten & Czeplak 1980 (via `celestial.md`); Bevy 0.20 source (atmosphere light loop, light render layers, clustered-decal light textures). Not obtained: Perez et al. 1993.

### Critique and verdicts (`/tmp/i16/critique.md`; before `/tmp/i16/before/`, after `/tmp/i16/after/`, `/tmp/i16/cmp/`, sequences `/tmp/i16/seq_*`)

| Feature | Evidence | Verdict |
|---|---|---|
| Sky under a cloud's shadow | `after/long_cmp.png` (waves 17.5 h, frames 600–1800), `seq_cmp.png` + `seq.py` (sky 0.9 % range vs 26.6 %), `docs/cloud_shadow_sky.png` | GREAT |
| Sun behind a cumulus | `after/dn_dot_cmp.png` (4× zoom: disc gone, silver lining kept) | GREAT |
| Props in the shadow | `after/props.png` (waves bodies darken with the water) | GREAT with `WeatheredMaterial`; plain `StandardMaterial` sees the mean (documented) |
| No regression | `cmp/all.png` (Overcast, Thunderstorm, night, dusk, Clear), `day_night` 16.5/18.0 h within 0.5 %, `tour_sheet.png` | GREAT |

### Verification

- `tests/gpu_clouds.rs` `a_cloud_over_the_camera_leaves_the_lights_alone` (new): beam 0.5666 in the shade vs 0.5752 in the sun, ambient 11 842 vs 11 607 lx; on `7dd1ef0` it fails (0.0222 vs 0.8591). `the_shadow_map_and_the_sun_factor_follow_the_clouds` now also checks the beam against the map's mean (Clear 0.9696 vs 0.9691). `clouds::model` `an_opaque_column_hides_what_is_behind_it` replaces the early-exit test.
- Gates: `cargo build --release`, `cargo test` (0 failures), `cargo build --examples --all-features`, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32, rustfmt on the changed hunks, `tests/save.rs` with `--features serde` (2). GPU, one process per file/module: every `tests/gpu_*.rs` file and lib `clouds` (10), `terrain` (8), `cover` (7), `dust` (6), `voxel` (1), `the_horizon_sky` — all pass; `gpu_hydrology`'s two bake tests timed out (240 s wall deadline) while a vLLM server held the GPU at 95 % and passed on a rerun with the GPU idle.
- Performance (`bench_elements`, High, hills, `PartlyCloudy`, 1080p, four rounds alternating `7dd1ef0` and this tree, P2 1740–1800 MHz logged in the same shell): `elements_clouds_shadow` 0.066–0.079 → 0.059–0.065 ms, march 0.436–0.466 → 0.443–0.468 ms, composite 0.037–0.038 → 0.042 ms (unchanged code); net ≈ −0.003 ms. CPU mean of the readback grid 47 µs at 4 Hz.
- `docs/tour.gif` regenerated; `docs/cloud_shadow_sky.png` new.

### Top 5 remaining items, ranked

1. **The anti-solar sky at sunset reads dark grey** (`/tmp/i16/cmp/pair_w1_19.6.png`: sun +1.3°, `PartlyCloudy`, looking east; same before and after): the glow-capped exposure leaves the Belt of Venus side nearly black between orange clouds.
2. **Ribbed cumulus faces**: the large cumulus in `terrain_tour` shows horizontal stair-step shelves on its sunlit face (`/tmp/i16/seq_after/`).
3. **Buoyancy readback**: publish the wave height under each `WaterContact` so a float can ride the waves (carried).
4. **Save/restore of the GPU layers** (carried).
5. **A view-dependent exposure cap**, **flat terrain shading**, and **wave cost** (carried).

## Iteration 17 — 2026-10-18 — cumulus that billow

**Commits:** `6930852` (research note `docs/research/cumulus-shape.md`), the implementation with the mirror and tests, the README / CHANGELOG / still / tour GIF, this log. Merged through a pull request. **Sibling repos:** none. **Not released.**

### What changed, and why it ranked first

The review of `terrain_tour` under `PartlyCloudy` from several angles (`/tmp/i17/before/cu_sheet.png`) showed every fair-weather cumulus as a kilometre-wide smooth pancake with stacked shelves on its flank — iteration 16's "ribbed cumulus faces" turned out to be a symptom of a bigger beauty gap visible in every outdoor frame under `Clear` and `PartlyCloudy`. Experiment A (no dome scaling, `/tmp/i17/expA`) proved the shelves are the dome terraces; the slab and the weak, coarse detail erosion made the pancakes. It beats the runner-up (the dark anti-solar sky at sunset, one hour of the day) because it is in every daytime frame.

Cumuliform clouds are now carved by Nubis Evolved's envelope model: a flat base as a factor on the density, a C¹ rounded top following the blob, convective cells from the base noise (closing as the cover reaches 1), and a value erosion `√saturate((N − (1 − D)) / D)` of a billowy Worley composite on retuned tiles (4096 m base, 640 m detail). The detail octaves fade to their mean as the march's step outgrows them. The CPU mirror carries the same model; stratus is unchanged.

Eleven capture rounds. Found and fixed in order: a blob stayed one mass (cell mask, core profile), clouds translucent grey-blue (full density in cores), billows hanging under the base at sunset (base as a density factor), rim grain at 3× (edge exponent ½, detail tile 640 m, a detail band-limit), a Blizzard deck opened into holes (gaps close with `cover⁴`), the coarse march's `pow` cost (a linear sign test).

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **Cumulus as pancakes with shelves** | 2 (+1: terraces) | picked |
| 2 | Anti-solar sky at sunset dark grey | 1 | carried |
| 3 | Buoyancy readback | 3 | carried |
| 4 | Save/restore of the GPU layers | 3 | carried |
| 5 | View-dependent exposure cap; flat terrain shading; wave cost | 2/5 | carried |

### Sources

Schneider & Vos 2015 (`s2015-advances-ppt-944-mb-pdf-5-mb-2`); Schneider & Vos 2017 (`s2017-advances-pdf-slides-13mb`); Schneider 2022 *Nubis, Evolved* and 2023 *Nubis³* (Advances PDFs, downloaded); Bauer 2019 *RDR2* (Advances PPTX, downloaded); Webanck et al. 2018 *Procedural Cloudscapes* (`10.1111/cgf.13373`, HAL); Goswami 2021 cloud survey (`10.1007_s00371-020-01953-y`, downloaded); Toft et al. 2016 (`10.48550_arxiv.1609.05344`, downloaded); Mol et al. 2023. Not obtained: Plank 1969, Neggers et al. 2003, Wood & Field 2011.

### Critique and verdicts (`/tmp/i17/critique.md`; rounds `/tmp/i17/r1`–`r11`, `b2_r7`–`r10`; final `/tmp/i17/final`, `final_b2`)

| Feature | Evidence | Verdict |
|---|---|---|
| Cumulus shape (9 h toward the sun, 15.5 h away, overhead, horizon, aerial) | `final/strip.png`, `final_b2/sheet.png`, `docs/cumulus.png`, `r7/zgc.png` | GREAT: cauliflower turrets, separate thermals, crisp silhouettes, no shelves |
| Sunset undersides | `final_b2/sunset.png`, `b2_r9/zh3c.png` | GREAT; rim sparkle on the horizon row at 4× (half-resolution march), cosmetic |
| Stratus, Overcast, Thunderstorm, Blizzard | `final_b2/sheet.png`, `the_weather_air_makes_the_sky_the_fog_colour` | unchanged |
| Motion | `seq_sheet.png`, frame differences 0.35–0.37 with no spikes over 6 s | GREAT |

### Verification

- `clouds::model`: `the_cumulus_envelope_is_smooth_and_the_erosion_bounded` (C¹ envelope, plane base, monotone bounded erosion), `the_coarse_test_never_misses_a_cloud` (four types, three detail weights), `detail_noise_only_removes_stratus_density`, `the_cumulus_shadow_covers_what_the_weather_says` (shade 0.048 / 0.377 / 0.818 against the map's 0.052 / 0.399 / 0.799); the stratus column still τ 6.04. `tests/gpu_clouds.rs`: the Clear shade fraction in the 8 km window is 0.006 (floor now 0.001, Deviation 9); the camera test picks 164 m blocks ≤ 150 m apart and allows 5 % (measured 3.4 %).
- Gates: `cargo build --release`, `cargo test` (0 failures), `cargo build --examples --all-features`, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32, rustfmt on the changed files, `tests/save.rs` with `--features serde` (2). GPU, one process per file/module: every `tests/gpu_*.rs` file (clouds 6, hydrology 9, precipitation 6, trails 4, weathered 4, twilight 3, waves 3, water 2, wind 2, cloud ambient, imports, plugins, readback) and lib `clouds` (10), `terrain` (8), `cover` (7), `dust` (6), `voxel` (1), `the_horizon_sky` — all pass. `a_flash_lights_the_cloud_at_dusk_and_not_at_noon` now allows 5 % at noon (measured 3.4 %: the cumulonimbus's flat base sits where the flash fires; `cumulus-shape.md` Deviation 10).
- Performance (`bench_elements`, High, hills, `PartlyCloudy`, 1080p, alternating runs against `2791780`, P2 at 1725–1800 MHz logged in the same shell, the GPU otherwise idle): `elements_clouds_march` 0.437–0.474 ms (seven runs) → 0.489–0.492 ms (three runs of the final shader): +6 %, within the note's 10 %; `elements_clouds_shadow` 0.058–0.079 → 0.069–0.072 ms. An `EMPTY_LIMIT` of 6 gained nothing; `pow` → `sqrt` in the erosion and a linear coarse test took the march from 0.521–0.527 to 0.489–0.492.
- Stills `docs/cumulus.png` (new), `docs/tour.gif` regenerated.

### Top 5 remaining items, ranked

1. **Cumulus lighting**: from above (`final_b2/aerial.png`) the tops are tan in the haze, and backlit or front-lit bodies are low-contrast cream-grey rather than brilliant white with darker bases; the multiple-scattering and ambient balance of the march, now that the shapes are right.
2. **The anti-solar sky at sunset reads dark grey** (carried).
3. **Rim sparkle** on distant lit cloud edges at sunset (half-resolution march; a full-resolution edge pass or a bilateral upsample).
4. **Buoyancy readback** and **save/restore of the GPU layers** (carried).
5. **A view-dependent exposure cap**, **flat terrain shading**, **wave cost** (carried).

## Iteration 18 — 2026-10-18 — clouds that are white: multiple scattering by diffusion

**Commits:** `d3d0f8d` (research note `docs/research/cloud-lighting.md`), the implementation with the mirror, the Monte Carlo table and tests, the README / CHANGELOG / still / tour GIF, this log. Merged through a pull request. **Sibling repos:** none. **Not released.**

### What changed, and why it ranked first

Iteration 17's top item. With the shapes right, the lighting was the beauty gap in every outdoor frame: cumulus were translucent grey-blue from the ground, khaki from 3.8 km and black underneath at sunset (`/tmp/i18/before/sheet.png`). A Monte Carlo reference (`docs/research/cloud-lighting-mc.py`: homogeneous conservative slabs, the march's dual-lobe phase, 2×10⁶ photons per case) put numbers on it: three Wrenninge octaves gave 0.16–0.3× of a thick top and nearly nothing of a base. The best octave sum the rule `a ≤ b` allows still had 3× the error of a diffusion term.

The octaves are replaced by single scattering plus photon diffusion between the lit face and the cloud behind, `α √μ_l / π · (τ_b + z) / (τ_l + τ_b + 2z)`, with Jensen 2001's extrapolation `z = 2/(3(1 − g))` (the free fit recovers 3.39 against 3.33) and α = 0.68; `τ_b` is one extra base-shape sample 300 m behind each sample. Two more defects surfaced while prototyping and were fixed: descending rays took their aerial perspective along a path that met the ground (khaki from above; now Bruneton's reversed ray), and the clouds' ambient still carried the sunlight the diffusion now carries, turning the noon overcast cream (the ambient is now the sky's colour alone). The far veil had cloud behind a deck's base and drew a ridge at the 32 km boundary; it has none now.

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **Cloud lighting: grey/khaki cumulus, black bases** | 2 | picked |
| 2 | Anti-solar sky at sunset dark grey | 1 | carried |
| 3 | Rim sparkle on lit cloud edges at sunset | 2 | carried |
| 4 | Buoyancy readback; save/restore of the GPU layers | 3 | carried |
| 5 | View-dependent exposure cap; flat terrain shading; wave cost | 2/5 | carried |

### Sources

Hillaire 2016 *Physically based sky, atmosphere and cloud rendering in Frostbite* (§5.8); Hillaire 2020 SIGGRAPH PBS course slides (Unreal's octaves, albedo ≈ 1 needs multiple scattering); Bouthors et al. 2008 (HAL `inria-00333007`: orders 1–30 carry 30–100 % of the reflectance); Jensen et al. 2001 (`10.1145_383259.383319`, the extrapolated boundary `φ − 2AD ∂φ/∂n = 0`); Kallweit et al. 2017 (`10.1145_3130800.3130880`, downloaded). Not obtained: Lacis & Hansen 1974, Meador & Weaver 1980, Wrenninge 2013.

### Critique and verdicts (`/tmp/i18/critique.md`; rounds `/tmp/i18/p1`, `p2`, `r1`–`r3`)

| Feature | Evidence | Verdict |
|---|---|---|
| Cumulus from the ground (15.5 h, 9 h, 11 h) | `r3/cmp_front.png`, `r3/strip.png`, `r3/cmp_horizon.png` | GREAT: white tops, grey bases, crisp against the blue |
| Cumulus from 3.8 km | `r3/cmp_aerial.png` | GREAT: white over the haze (was khaki) |
| Sunset undersides | `r3/z_sunset.png`, `r3/z_rim.png` | GREAT: orange-lit, volume reads; rim speckle unchanged (carried) |
| Noon overcast | `r1/overcast.png` (cream, NOT YET) → `r3/overcast.png` | GREAT: neutral (226, 225, 222), brighter than (193, 195, 197) |
| Storm, night, overhead | `r3/cmp_storm.png`, `r3/cmp_night.png`, `r3/cmp_e.png` | pass: storm unchanged under its fog, moonlit clouds keep their form |

### Verification

- `clouds::model`: `the_slab_matches_monte_carlo` (96 cases, mean squared log error 0.067 ≤ 0.08; thick tops 0.57–1.12×, bases 0.45–1.19×, `cloud-lighting.md` Deviation 2), `the_integrator_converges_on_the_slab` (Q 64 within 5 % of Q 1024), `the_source_bound_is_the_phase_peak_and_the_diffuse_albedo` (2.7285), the phase integral and values, the stress bound, the white furnace.
- GPU: `the_far_veil_continues_the_marched_deck_at_full_cover` (near rows ratio 0.95–0.97, tint ≤ 0.016, chroma ≤ 0.031; far rows now 0.08 / 0.15, measured 0.062 / 0.122, a smooth drift with distance, Deviation 4); `the_deck_takes_the_colour_of_its_light` (noon blue/red 1.12× the neutral deck, bound now < 1.2; dusk red/green 1.26 against 1.02).
- Gates: `cargo build --release`, `cargo test`, `cargo build --examples --all-features`, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32, rustfmt on the changed files, `tests/save.rs` with `--features serde`, every GPU file and lib module (`/tmp/i13/gpu_suite.sh`).
- Performance (`bench_elements`, High, hills, `PartlyCloudy`, 1080p, alternating runs against `8b0620c`, P2 at 1740–1800 MHz logged in the same shell, the GPU otherwise idle; one run at P5 discarded): `elements_clouds_march` 0.480–0.524 → 0.513–0.539 ms (+3–7 %, within 10 %); `elements_clouds_shadow` unchanged (0.069–0.072 → 0.068–0.073).
- Stills `docs/cloud_lighting.png` (new), `docs/tour.gif` regenerated.

### Top 5 remaining items, ranked

1. **Open issues #7–#15** (filed by a consumer integrating the crate): orthographic cameras and a follow anchor, a world-units scale, opt-outs on `ElementsCamera`, `CelestialPlugin` without its own lights, a public `ElementsGlobals` handle, `bevy_flora` 0.46, the `isomesh` feature for many volumes, serializing `SurfaceWeathering`, a snow season on `Climate`.
2. **The anti-solar sky at sunset reads dark grey** (carried).
3. **Rim sparkle** on distant lit cloud edges at sunset (carried).
4. **The aerial haze floor** seen from 3.8 km is a flat featureless grey (`r3/aerial.png`).
5. **Buoyancy readback**, **save/restore of the GPU layers**, **a view-dependent exposure cap**, **flat terrain shading**, **wave cost** (carried).

## Iteration 19 — 2026-10-18 — a focus for the camera-relative domains, and orthographic cameras (issue #7)

**Commits:** `4de4ed2` (research note `docs/research/view-anchor.md`), the implementation with tests, the `isometric` example, the README / CHANGELOG / still / GIF, this log. Merged through a pull request; closes #7. **Sibling repos:** none. **Not released.**

### What changed, and why it ranked first

The user asked to start from the open issues. Nine were filed today by a consumer (Sanctuary) integrating the crate; #7 ranked first because it blocks the adoption of five domains at once under the camera every isometric and strategy game uses, and #8 (units) touches the same boundaries after it. Under an orthographic camera 1500 m back from what it shows, every camera-relative domain centred 1500 m away and the cloud march built its rays from one eye point.

`ElementsFocus::Entity` / `Point` on the camera now centres the domains (the view carries the focus and the eye separately; the terrain's level of detail stays on the eye, which its vertex shader morphs by). The cloud passes build rays from the view matrices for both projections. The capture review of a new `isometric` example then found five things Bevy measures from the camera's translation that an isometric camera breaks (Deviations 1–5 of the note): the distance fog, the shadow cascades, precipitation drawn at a telephoto's pixel coverage and a box smaller than the view, the ground-fog volume crossed at one slant, and ray origins inside a low deck. Each is adapted only for an orthographic followed camera; a perspective camera is unchanged.

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **#7 orthographic cameras and a focus** | 3 (adoption) | picked |
| 2 | #8 world-units scale | 3 | next |
| 3 | #9 opt-outs on `ElementsCamera`, #10 `CelestialPlugin` without lights | 3 | queued |
| 4 | #14 serialize `SurfaceWeathering`, #15 a snow season | 3 | queued |
| 5 | #11 public globals handle, #12 bevy_flora 0.46, #13 isomesh volumes | 3 | queued |

### Sources

Marschner & Shirley, *Fundamentals of Computer Graphics* §4.3.1 (corpus); *Real-Time Rendering* 4th ed. §4.7.1 (corpus); Puig-Centelles, Ripollés & Chover 2009 (`10.1007/s00371-009-0366-9`) and Puig-Centelles et al. 2011 (`10.4018/jcicg.2011070102`), both fetched from SciDB and uploaded to the home-still inbox; Tatarchuk 2006 (corpus `s2006-advances-materials-6`); Losasso & Hoppe 2004 (corpus `10.1145_1015706.1015799`); Bevy 0.20's own orthographic test, cascade split and atmosphere rays.

### Critique and verdicts (`/tmp/i19/critique.md`; rounds `/tmp/i19/r1`–`r10`, sequences `seq`, `seq2`)

| Feature | Evidence | Verdict |
|---|---|---|
| Clear / PartlyCloudy / Overcast under the isometric camera | `r10/sheet.png` | GREAT: crisp town, sun shadows, soft overcast |
| Rain, Thunderstorm, Snow | `r10/z_rain.png`, `r10/sheet.png` | GREAT: streaks and flakes over the whole view, no box edge; the pale wet ground is pre-existing (same in the perspective town) |
| Fog | `r10/Fog.png` (was white `r7`, a hard line `r8`) | GREAT: a depth fade from the bottom edge |
| Motion | `seq2`, frame differences 0.032–0.051 | GREAT: the box follows the walker, no respawn bursts |
| Perspective unchanged | `fogcheck/cmp.png`, the GPU suite | unchanged |

### Verification

- New tests: `plugin::the_view_centres_on_the_focus` (point, entity, entity without a transform, removed), `clouds::model::orthographic_rays_are_parallel_and_perspective_rays_share_the_eye` (the ray mirror: parallel rays on the focus plane, the camera's plane without a focus), `lightning::visual::an_orthographic_ribbon_keeps_two_pixels_at_every_distance`, `fog::an_orthographic_view_fogs_from_the_bottom_of_its_ground`, `tests/lightning.rs` `strikes_and_thunder_centre_on_the_focus`, `tests/gpu_precipitation.rs` `the_domains_centre_on_the_focus_of_an_orthographic_camera` (rain box, wind window, fog volume centred on a focus 1500 m from the camera; the box follows a 20 m pan with its count), `tests/gpu_cloud_ambient.rs` `an_orthographic_camera_sees_the_deck_a_perspective_one_does` (luminance 1.04×).
- Gates: `cargo build --release`, `cargo test`, `cargo build --examples --all-features`, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32, rustfmt on the changed files, `tests/save.rs` with `--features serde`, every GPU file and lib module.
- Performance (`bench_elements`, High, hills, `PartlyCloudy`, alternating against `8670d02`, P2 at 1530–1740 MHz with the GPU shared by a vLLM job): `elements_clouds_march` 0.524–0.530 → 0.468–0.529 ms, resolve and composite unchanged; no regression.
- Stills `docs/isometric.png`, GIF `docs/isometric.gif` (new).

### Top 5 remaining items, ranked

1. **Open issues #8–#15**, #8 (a world-units scale) first.
2. **Wet surfaces reflect a clear sky under overcast**: `WeatheredMaterial` and the ground reflect Bevy's environment map, which has no clouds, so wet ground under Rain reads as a pale sheen (`r10/town_rain.png`, perspective and orthographic alike).
3. **The anti-solar sky at sunset reads dark grey** (carried).
4. **Rim sparkle** on lit cloud edges at sunset (carried).
5. **Bevy's own sky and aerial perspective are perspective-only** (`uv_to_ray_direction`): under an orthographic camera looking at the horizon the sky is a perspective fan; the crate's clouds are right, Bevy's atmosphere is not.

## Iteration 20 — 2026-10-18 — a game keeps its own look and lights (issues #9, #10, #11)

**Commits:** `ad45762` (research note `docs/research/integration.md`), the implementation with tests, `ELEMENTS_LOOK=game` in the `isometric` example, the README / CHANGELOG / still / GIF, this log. Merged through a pull request; closes #9, #10, #11. **Sibling repos:** none. **Not released.**

### What changed, and why it ranked first

Of the open issues after #7, #8 (a units scale) turned out to touch ten render shaders, 31 metre constants and every transform-reading domain: an iteration of its own, next. #9, #10 and #11 share one theme every art-directed game hits (the crate took over the camera's look, the lights and the globals), so they went first, together. `ElementsCamera` gained five dressing flags (`atmosphere`, `distance_fog`, `volumetric_fog`, `ambient`, `msaa_off`), each inserting and driving only what is on, removing only what the crate put there, and flipped at runtime by a change system; `DepthPrepass` follows the plugins that read it. `CelestialSettings { lights, sky }` keeps `SunState` without spawning or touching lights (`update_sky` computes, `drive_lights` writes). `ELEMENTS_GLOBALS_BUFFER` is public. The capture review then found the game look's rain lit by the crate's 50 klx sun; the particles now take the `Sun`-marked light (the crate's or the game's) and the camera's own ambient, and the cloud shadows follow a game's `Sun`-marked light.

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **#9, #10, #11: the game's look, lights and globals** | 3 (adoption) | picked |
| 2 | #8 world-units scale | 3 | next (wide: ten shaders, every domain) |
| 3 | #14 serialize `SurfaceWeathering`, #15 a snow season | 3 | queued |
| 4 | #12 bevy_flora 0.46, #13 isomesh many volumes | 3 | queued |
| 5 | Wet surfaces reflect a clear sky under overcast | 2 | carried |

### Sources

Patry 2021 (corpus `s2021-advances-pdf-66-mb`: physically based lighting that "can deviate from physical correctness" with artist knobs per weather state); Schneider & Vos 2015 (corpus: "art direct-able" as a requirement); Tatarchuk 2006 (corpus: artist-directable rain as separate pieces); Bevy 0.20's atmosphere extraction, environment-map observer and `Camera3d`'s required `Exposure`.

### Critique and verdicts (`/tmp/i20/critique.md`; rounds `/tmp/i20/r1`, `r2`, `seq`)

| Feature | Evidence | Verdict |
|---|---|---|
| Game look: key light, palette, no fog or sky | `r2/sheet.png` | GREAT |
| Rain and snow on the game look | `r1/sheet.png` (stark white, NOT YET) → `r2/z_rain.png` | GREAT: in the palette's range, walls wet |
| Cloud shadows along the game's light | `seq_sheet.png` | GREAT |
| Default look | `r1` top row, `tests/gpu_dressing.rs` | unchanged |

### Verification

- New tests: `tests/gpu_dressing.rs` — `the_default_dressing_is_unchanged` (the component set, one atmosphere, two lights, the moon disc and stars), `a_bare_camera_without_lights_keeps_the_games_look` (only `DepthPrepass` inserted, no atmosphere, lights, moon or stars; the game's fog, ambient, MSAA and `Sun`-marked light keep their values; `SunState` equal to the lit run's), `flags_flip_at_runtime`; `tests/gpu_weathered.rs` `a_game_material_reads_the_globals_buffer` (a material extension defined in the test binds `ELEMENTS_GLOBALS_BUFFER` and draws the pinned wetness and snow cover to ±1 DN).
- Found by a test: `Camera3d` requires `Exposure`, so the old `insert_if_new(Exposure::SUNLIGHT)` never applied and an `exposure` flag would be meaningless (Deviation 1).
- Gates: build, `cargo test`, examples, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32, rustfmt on the changed files, `tests/save.rs`, every GPU file and lib module.
- Performance: no shader or kernel change on the default path (the cloud view query gains an `Msaa` filter; the precipitation light is a few CPU lines); not re-benchmarked.
- Still `docs/game_look.png`, GIF `docs/game_look.gif` (new).

### Top 5 remaining items, ranked

1. **#8 world-units scale** (ten render shaders, the metre constants, every transform boundary).
2. **#14 serialize `SurfaceWeathering`** and **#15 a snow season on `Climate`**.
3. **#12 bevy_flora 0.46**, **#13 isomesh with many volumes and a game-owned ground**.
4. **Wet surfaces reflect a clear sky under overcast** (carried).
5. **The anti-solar sky at sunset**, **rim sparkle**, **Bevy's perspective-only sky under an orthographic camera** (carried).

## Iteration 21 — 2026-10-18 — the snowpack survives a save, and a snow season (issues #14, #15)

**Commits:** `175fc8b` (research note `docs/research/snow-season.md`), the implementation with tests, `weathered_town`'s November calendar and save/load, the README / CHANGELOG / still / GIF, this log. Merged through a pull request; closes #14, #15. **Sibling repos:** none. **Not released.**

### What changed, and why it ranked here

#8 (a units scale) was ranked first but touches every render shader and transform boundary; #14 and #15 share the snowpack and are what a game with a calendar and saves hits first (Sanctuary's lying snow vanished on load, and a cold autumn night snowed before its "snow from 14 November"). `Climate::snow_season: Option<SnowSeason>` gates the snow presets out of season and carries `Weather::snow_possible` into every reader of the rain–snow split, the GPU kernels included (`snow_fraction_at(t, possible)`), ramping over the hour outside each boundary. `SurfaceWeathering` serializes; a restored pack integrates only when the loaded clock is within an hour of the save, and with a terrain re-seeds the cover maps through a one-off `cover_seed` kernel (point pack × each cell's slope and shelter exposure), dropping a readback in flight across it.

### Candidates ranked

| # | Candidate | Category | Note |
|---|---|---|---|
| 1 | **#14 + #15: the snowpack in a save, a snow season** | 3 (adoption) | picked |
| 2 | #8 world-units scale | 3 | next (wide) |
| 3 | #12 bevy_flora 0.46, #13 isomesh many volumes | 3 | queued |
| 4 | Wet ground reflects the sun under overcast | 2 | carried; seen in this iteration's 11:00 captures |

### Sources

Jennings et al. 2018 (`10.1038/s41467-018-03629-7`, read from the PDF: the 50 % rain–snow threshold averages 1.0 °C and spans −0.4 to 2.4 °C, the transition 2.6–4.6 °C wide, so a physical ramp cannot keep snow out of a near-freezing fortnight: the season is an authored override); Patry 2021 (art knobs per weather state); the repo's climate-weather, snow-cover and wetness notes. Brown & Mote 2009 not retrievable (Pending index).

### Critique and verdicts (`/tmp/i21/critique.md`)

| Feature | Evidence | Verdict |
|---|---|---|
| Daily stills with auto exposure at 21600× | round 1 | FAIL (blown out; added `ELEMENTS_EV100`) |
| 11:00 stills | round 2 | NOT YET (pre-existing wet-ground sun glare in frame; tofu labels) |
| 15:30 stills: 9 Nov differs, 14 Nov identical | `docs/snow_season.png` | GREAT |
| Daily flip-book, flakes vs rain on 2 Nov | `docs/snow_season.gif` | GREAT |
| Save at 15 Nov, load on a fresh app | `/tmp/i21/save/pair.png` | GREAT |

### Verification

- New tests: `tests/snow_season.rs` (no Snow or Blizzard and no lying snow 1–13 November in the cool-temperate climate with seed 1, the same precipitation as without the season, all of it rain; 40 days from 14 November identical to no season; a −3 °C Rain hour is rain outside and snow inside; forced Snow snows outside; the boundary ramps over 23:00–24:00), `tests/save.rs` (`a_restored_snowpack_continues_the_same_snow` 720 updates identical; `a_snowpack_loaded_a_day_later_neither_melts_nor_grows`, against a carried copy that integrates the day; `files_from_before_the_snow_season_load`), `climate::model` `a_snow_season_wraps_the_year_and_ramps_outside_it`, `cover::gpu_tests::gpu_a_restored_snowpack_seeds_the_maps` (open cells at 0.1 m and 20 mm SWE, wetness 0.6, the 100 cells under the roof bare and dry).
- Found by a test: the first seed request was an `Arc<AtomicU64>` bump, which the render world saw before the frame's parameters were extracted; the render frame in flight ran the seed with the previous, empty pack and marked it done (the map stayed bare, `seeded` caught up). The request is now a plain extracted field beside `params.seed`; atomics only report back. Recorded in `AGENTS.md`.
- Gates: build, `cargo test`, examples, clippy `-D warnings`, `cargo doc` `-D warnings`, wasm32, rustfmt on the changed files, `tests/save.rs`, every GPU file and lib module.
- Performance: one multiply per cell in three kernels; the seed is one 8×8 dispatch per load. Not re-benchmarked.

### Top 5 remaining items, ranked

1. **#8 world-units scale**.
2. **#12 bevy_flora 0.46**, **#13 isomesh with many volumes and a game-owned ground**.
3. **Wet ground reflects the sun under overcast** (Bevy's environment map is the clear sky; visible in every wet noon capture).
4. **The anti-solar sky at sunset**, **rim sparkle** (carried).
5. **Bevy's perspective-only sky under an orthographic camera** (carried).

## Iteration 22 — 2026-10-18 — the `flora` feature on bevy_flora 0.46 (issue #12)

**Commits:** the dependency bump with README / CHANGELOG / still, this log. Merged through a pull request; closes #12. **Sibling repos:** none (bevy_flora 0.46.0 is published). **Not released.**

A game on bevy_flora 0.46 built two copies with the `flora` feature (pinned to 0.30), and `ElementsFloraPlugin` wrote resources its trees did not read. 0.46 pins Bevy 0.20, so the bump keeps pin parity. Its `FloraWind`, `GustFront`, `FloraSeason` and `FloraWetness` keep the fields this crate writes; `GustFront` speed and width are now metres under `FloraUnits` (bevy_flora's `units.rs`: fields that size the simulation in metres, positions in world units), which is what `WindGust` carries, so `src/flora.rs` needs no change. No research note: no system changed, and bevy_flora's own units documentation is the source.

### Verification

- Builds and `cargo clippy --all-targets --all-features -D warnings`, `cargo doc`, the examples and the wasm32 library with `flora`; `flora::tests` (2) pass.
- Visual (`/tmp/i22`): `flora_wind` under forced Rain at 15.8 m/s: autumn colour, wet bark; the amplified difference between frames a quarter-second apart outlines every leaf and branch of the near tree (the crowns sway and flutter: the wind reaches the trees). The pale discs at the horizon are the scene's white spheres in fog, as in the 0.30 still. `docs/flora_wind.png` re-captured. GREAT for what this changes (the wiring); the scene itself is unchanged.

### Top 5 remaining items, ranked

1. **#8 world-units scale** (a `FloraUnits`-style `ElementsUnits`; then `ElementsFloraPlugin` maps it onto `FloraUnits`).
2. **#13 isomesh with many volumes and a game-owned ground**.
3. **Wet ground reflects the sun under overcast**.
4. **The anti-solar sky at sunset**, **rim sparkle** (carried).
5. **Bevy's perspective-only sky under an orthographic camera** (carried).

## Iteration 23 — 2026-10-18 — snow on many voxel volumes without a terrain (issue #13)

**Commits:** `3b95667` (research note `docs/research/voxel-volumes.md`), the implementation with tests, the `voxel_walls` example, README / CHANGELOG / still / GIF, this log. Merged through a pull request; closes #13. **Sibling repos:** none. **Not released.**

### What changed

`VoxelSnow` puts `VoxelGround`'s snow slab on any number of volumes with no terrain: one column per voxel in the volume's field space, the same scan, hold, edit rescans and field swaps (the state is shared; `terrain: Option<Entity>` tells the two apart), and a depth per column from the ground-cover map at the column's world position where a crate terrain lies under it, else the point snowpack, times the top's shed-slope exposure, in field units under a moved, yawed and uniformly scaled volume. `VoxelPlugin` runs it even without the terrain, hydrology and cover plugins.

Two crate defects surfaced in the capture review, both at cliffs that the smooth voxel hill never had: `SnowyField` blended a top across a step and floated the slab's underside off the lower columns (now the nearest column across a step of more than a voxel), and the slope exposure read a merlon beside a walk as a 63° slope and shed the walk's snow (now a neighbour more than a voxel away in height is a wall).

The issue's "builds against 0.0.10" is declined with evidence (Deviation 1): bevy_isomesh 0.0.10 is built on Bevy 0.19 and lacks `replace_field` and `IsomeshSystems`, and bevy_isomesh 0.0.11 pins isomesh 0.0.11 exactly, so a widened core requirement could never resolve to 0.0.10. The consumer moves its path checkout to the published 0.0.11.

### Sources

This crate's `voxel-snow.md`, `snow-cover.md` (Fearing 2000's shed angles) and `snow-season.md` (the point pack); bevy_isomesh 0.0.10 (docs.rs) and 0.0.11 (registry) sources; isomesh 0.0.10 and 0.0.11 diffed for every item used.

### Critique and verdicts (`/tmp/i23/critique.md`)

| Round | Finding | Verdict |
|---|---|---|
| 1 | Base seam, blobby merlons, black wedges | FAIL |
| 2 | Wedges are geometry: the slab across a cliff (crate) and an air sheet under each merlon (example SDF) | NOT YET |
| 3 | Slab sides shaded as stone (painted per vertex); bare notch in each crenel (exposure across a cliff, crate) | NOT YET |
| 4 | `docs/voxel_walls.png`, `docs/voxel_walls.gif` | GREAT |

### Verification

- New tests: `voxel::tests` `two_snow_volumes_get_slabs_and_no_terrain` (two volumes, one moved, turned 0.3 rad and scaled by 2, no `Terrain`, 0.6 m and 0.3 m field-unit slabs), `a_snow_volume_samples_the_cover_map_at_its_world_position`, `a_tilted_snow_volume_gets_no_slab`, `a_volume_with_both_components_is_a_voxel_ground`, `removing_voxel_snow_restores_the_field`; `voxel::model` `snow_columns_are_one_per_voxel_until_the_cap`, `a_top_sheds_by_its_slope` (with the cliff case), `only_an_upright_evenly_scaled_volume_takes_snow`; `voxel::field` `the_slab_sits_on_both_sides_of_a_cliff`.
- Gates: build, `cargo test`, examples, clippy `-D warnings`, `cargo doc`, wasm32 with `isomesh`, rustfmt, `tests/save.rs`, the voxel GPU tests.

### Top 5 remaining items, ranked

1. **#8 world-units scale**.
2. **Wet ground reflects the sun under overcast**.
3. **Bright snow scenes**: the illuminance-metered exposure renders a sunlit snowfield and the stone on it pale (`voxel_walls`).
4. **The anti-solar sky at sunset**, **rim sparkle** (carried).
5. **Bevy's perspective-only sky under an orthographic camera** (carried).

## Iteration 24 — 2026-10-18 — a world-units scale (issue #8)

**Commits:** `0717357` (research note `docs/research/units.md`), `9628f11` (the core: `ElementsUnits`, the metre-frame view, the atmosphere scale), the boundary conversions with tests, the examples' `ELEMENTS_UNITS`, README / CHANGELOG / AGENTS / still, this log. Merged through a pull request; closes #8. **Sibling repos:** none. **Not released.**

### What changed, and why it ranked first

The last open issue, and the one that blocked a foot-unit game outright (every metre size 3.28× too small, the sky's aerial perspective 3.28× too strong). `ElementsUnits { units_per_metre }` keeps the crate simulating in metres in a frame of world ÷ u and converts at the boundaries only: the followed view (one place every camera-relative domain reads), each `Transform` read (`to_metres`), what the crate writes into Bevy's world (the spawned `Atmosphere`'s scale, `DistanceFog`, the ground-fog volume, cascades, the bolt and its light at intensity × u², terrain and water patches and bounds), the public positions, and the 13 shaders that take world positions from Bevy (a `units` factor in each pass's own uniform; `ElementsGlobals` 80 → 96 B). Two parallel slices did the ~120 CPU and 13 shader boundaries against a written contract (`/tmp/i24/contract.md`, from an inventory of both sides).

### Sources

Gregory 2018 (MKS simulations, the world unit the game's choice; corpus), Lagarde & de Rousiers 2014 §4.1–4.2 (homogeneous distance units, `E = I/d²`, 1 unit = 1 m; the author's PDF), Bevy 0.20's `world_to_atmosphere` (the entity's scale), bevy_flora's `FloraUnits` rule.

### Critique and verdicts (`/tmp/i24/critique.md`)

| Scene | Metres vs feet | Verdict |
|---|---|---|
| Hills, partly cloudy | RMSE 0.26 %, ×20 diff: random pollen only | GREAT |
| Hills, rain | RMSE 1.0 %, random drops only | GREAT |
| Cumulus at 16:00 | RMSE 0.33 %, same clouds and haze | GREAT |
| Scripted strike | Same flash glow; bolt hidden by rain on main too | GREAT for the change |

### Verification

- `tests/gpu_units.rs` `a_world_in_feet_answers_as_the_same_world_in_metres` (hills, wind, overcast; heights within 2.3e-5 m, wind within 0.12 %, the same cloud shadows); `tests/lightning.rs` `a_strike_in_a_world_of_feet_is_a_thousand_metres_off`; `precipitation::tests::the_rain_box_is_sixty_metres_in_a_world_of_feet`; unit tests in fog, celestial, clouds (view ray), query (terrain height in feet).
- The full GPU suite at the default 1.0 (every converted shader is a no-op there) and all gates.
- Found during capture: `common::camera` bundling a `Projection` panicked `storm`; the planes are scaled by a system instead.

### Top 5 remaining items, ranked

1. **Wet ground reflects the sun under overcast**.
2. **Bright snow scenes** (the illuminance-metered exposure on a sunlit snowfield).
3. **A bolt hidden by heavy rain at 1.5 km** in `storm` (the scene or the bolt's brightness against the rain veil).
4. **The anti-solar sky at sunset**, **rim sparkle** (carried).
5. **Bevy's perspective-only sky under an orthographic camera** (carried).

## Iteration 25 — 2026-10-18 — the clouds in the environment map (sky light and reflections under cloud)

**Commits:** `e561f09` (research note `docs/research/sky-light.md`), `cac69fa` (fix: the crate's atmosphere planet centre, iteration 24's regression), the sky probe with its tests, docs and images, this log. Merged through a pull request. **Sibling repos:** none. **Not released.** The GPU was taken through `gpu-tenant claim coding --who bevy_elements@big` (olmOCR held 15.5 GB; the examples ran out of memory without the lease).

### First: a regression from iteration 24

The baseline's `waves` under Clear showed a beige dome of radial streaks instead of a blue sky, reflected in the pond, and the partly cloudy town an orange glow. Bisected to `9628f11`: the crate's `Atmosphere` got `Transform::from_scale(u)`, and Bevy reads that entity's `GlobalTransform` as the planet's centre (it places a default one `R` below the origin, which the `Transform` then overwrote). Fixed with `plugin::planet_transform` and a test that fails on the old transform; AGENTS.md trap; `units.md` Deviation 1. The iteration-24 GPU suite and feet/metre captures had passed because they never looked at this sky.

### Candidates (no open issues)

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | Glossy surfaces mirror a clear sky under cloud; overcast water brighter than its sky `(230,238,240)` under `(226,226,226)` | 1 (physically wrong) | **this iteration** |
| 2 | Bright snow scenes under the illuminance-metered exposure | 2 | carried |
| 3 | A bolt hidden by heavy rain in `storm` | 2 | carried |
| 4 | Anti-solar sunset sky, rim sparkle | 2 | carried |
| 5 | Bevy's perspective-only sky under an orthographic camera | 4 | carried |

Rank 1 beat rank 2 because it is wrong in every cloudy preset on every glossy surface (water, puddles, wet ground, `WeatheredMaterial`), where the exposure item touches one scene class. It is the backlog's "wet ground reflects the sun under overcast": the cause was not the sun but the uniform cloud `AmbientLight`, whose specular term Bevy shades with the same boost as its diffuse (2.2× the sky in water at grazing), plus a clear-sky environment map.

### What changed

- **The sky probe.** `march.wesl`'s second entry point marches the clouds into a cube of the atmosphere cube's size (128²) around the followed camera, composes it with Bevy's cube as the screen composite composes the sky (`common.wesl::sky_through_clouds`, now shared), and the camera's `GeneratedEnvironmentMapLight` is handed the composed cube once every texel has been marched (`SkyProbe`, an `Arc<AtomicU32>` handshake). One texel of every 4×4 block per face a frame at High, half the screen's steps, blended into a ping-pong history. The ray march itself became `integrate()`, shared by the screen and the cube; the screen march now computes the sky irradiance once per ray instead of twice.
- **The cloud ambient is retired on the probe camera** (`celestial::update_ambient`): the cube's dome is that light, shaded over normals by Bevy's irradiance map (top : wall : underside 1 : 0.67 : 0.37).
- **The Moon & Spencer shade factor became a relief cue** (`surface_cover.wesl::overcast_relief`, terrain and weathered): the first build darkened walls twice (a metal box's side 86 DN); removing it outright flattened the overcast relief from the air; dividing out the dome's own cosine keeps the −6.5 % relief cue without the double shading (`snow-cover.md` Deviation 16).

### Sources

Hillaire 2016 (Frostbite; clouds in every view including the environment map, eq. 22), Lagarde & de Rousiers 2014 §4.9 (the distant light probe refreshed with the weather; clear 8000, overcast 2000 cd/m²), Epic's *Sky Lights* documentation (Real Time Capture, 128² cube, 9-frame time slicing, PS4 costs), Bauer 2019 via imgeself 2020 (RDR2's per-frame sky cube with clouds), Karis 2013 (split sum, `/tmp/i25/pdf/karis2013.pdf`), Kittler & Darula 2014 (`10.2478/brj-2014-0012`, downloaded: Kähler's 2:1 and 3:1 overcast gradations, `L_Z/E`), Schneider & Vos 2015, Patry 2021, Kasten & Czeplak 1980, Bevy 0.20 source.

### Critique and verdicts (`/tmp/i25/critique.md`)

| Scene | Before → after | Verdict |
|---|---|---|
| Pond, PartlyCloudy | uniform blue water → cumulus and blue gaps in the water; temporal d2/d1 1.39 (motion, not flicker) | GREAT |
| Pond, Overcast / storm | water brighter than its sky → `(194,209,211)` at grazing under `(226,226,226)` | GREAT |
| Town, Overcast / Rain | white veil, metal box glowing on every face → colours read, metal mirrors sky on top (194) and ground on its sides (98) | GREAT |
| Terrain, Overcast (6 m, 120 m) | one washed tone → saturated, haze layers; relief cue kept (`cmp/relief2.png`) | GREAT |
| Snow (6 h grown) | snow brighter than its sky → `(217,222,227)` under `(224,227,230)` | GREAT |
| Dusk, night | no blowout, no black frame | GREAT |
| Clear | unchanged (probe ≡ atmosphere cube: 0.0000) | GREAT |

### Verification

- `tests/gpu_sky_probe.rs`: without cloud the probe is the atmosphere cube (worst relative difference 0.0000); overcast noon E/model 0.76 (±35 %), `L_Z/E` 0.343, wall 0.67 and underside 0.37 of a top through Bevy's irradiance map, ambient 0; partly cloudy cloud share above 10° 0.373 at cover 0.40, Bevy's cube 0.000.
- `plugin::tests::the_crate_atmosphere_centres_the_planet_below_the_origin` (fails on iteration 24's transform).
- `gpu_clouds::the_shadow_map_and_the_sun_factor_follow_the_clouds` and `a_cloud_over_the_camera_leaves_the_lights_alone` lost their ambient assertions (the ambient is 0 on the probe camera; the model's share stays covered by `celestial::tests::the_ambient_carries_the_diffuse_share_and_nothing_at_night`).
- Full GPU suite, cargo test (276), clippy, doc, wasm, save: pass.
- Bench (`bench_elements`, 3840 × 2160, PartlyCloudy, P2 1680–1770 MHz, minimum): `elements_clouds_sky_probe` 0.047 / 0.062 / 0.155 ms (Low / High / Ultra); first build 0.189 ms at High. Screen march 0.883 → 0.869 ms; clouds total 1.418 → 1.474 ms at High.

### Top 5 remaining items, ranked

1. **The overcast dome is flat** (zenith 1.08 × the horizon; Kähler and CIE 2:1–3:1): Eddington's emergent radiance `(2 + 3μ)/5` in `cloud_scatter`'s diffusion term, checked against the fogged horizon (`clouds.md` Deviation 29). `sky-light.md` Deviation 7.
2. **Bright snow scenes** (the illuminance-metered exposure on a sunlit snowfield); the overcast is now metered 0.4 EV above what the dome lights (`sky-light.md` Deviation 8).
3. **The water's body colour under a grey sky** (the storm pond's teal mid-distance `(160,189,197)`, pre-existing).
4. **A bolt hidden by heavy rain at 1.5 km** in `storm`.
5. **The anti-solar sky at sunset**, **rim sparkle**, **Bevy's perspective-only sky under an orthographic camera** (carried).

## Iteration 26 — 2026-10-18 — a look for rain and snow under far-off cameras (issues #25, #23, #27)

**Commits:** `0f8ead5` (research note `docs/research/precipitation-look.md`), the implementation with its tests, the examples, docs and images, this log. Merged through a pull request that closes #25, #23 and #27. **Sibling repos:** none. **Not released.** The GPU was taken through `gpu-tenant claim coding --who bevy_elements@big` (olmOCR's vLLM held 15.2 GB and distill 7.5 GB).

### Candidates (open issues first)

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | #25: rain under an orthographic camera reads as sleet (streaks 63° from vertical at 35° pitch), snow as identical dots | 2 (looks wrong to the consumer) | **this iteration** |
| 2 | #23: `SurfaceWeathering` has no `PartialEq`, so a game's save struct cannot derive it | 3 (API) | **this iteration** (small) |
| 3 | #27: no forecast of the scheduler's coming presets | 3 (API) | **this iteration** (small) |
| 4 | #26: trails on a game's own ground without the crate's `Terrain` | 3 (feature) | next iteration (a larger change: the trail window without terrain layers) |
| 5 | The overcast dome is flat | 2 | carried |

#25 beat #26 because every strategy or isometric consumer sees it in every wet preset; #23 and #27 are a few lines each with clear acceptance tests and rode along.

### What changed

- **`PrecipitationLook`** (`src/precipitation/look.rs`): fall and wind scales on the rain streak, a tint and opacity, a log-normal flake size (salt 5, Box–Muller, z clamped ±2) and a minimum flake width, packed into two `vec4`s of the draw's `DrawParams`; draw-only, defaults the identity. `readable()` (fall ×4, wind ×0.5, tint `(0.85, 0.9, 1.0, 0.7)`, σ 0.5, 1.5 px). CPU mirrors of the streak and the size factor carry the acceptance tests.
- **`WeatherDirector::forecast`**: from the director's `last_hour` and `scheduled` (a fresh chain starts Clear at the clock's hour, a clock jump restarts it, as `run_weather` does), catches the chain up to the clock's hour and runs `schedule` on for the requested hours, without touching the director.
- **`SurfaceWeathering: PartialEq`**: manual, every field but the transient `restored`; `tests/save.rs` now asserts whole-struct equality after a serde round trip.
- **Examples:** `ELEMENTS_PRECIPITATION=readable` in the harness, `ELEMENTS_PITCH` in `isometric`; the game look brightens its streaks against its own key light.

### Sources

Tatarchuk 2006 (corpus `s2006-advances-materials-6`: artist-set rain direction and speed, random particle size), Lagarde 2012 (*Remember Me*: rain should not match reality in bright scenes; slant by art, not wind), Wang & Wade 2004, Garg & Nayar 2007, Tiira et al. 2016 (`10.5194/amt-9-4825-2016`, downloaded: exponential snow PSD), Langer et al. 2004, `view-anchor.md` Deviation 3.

### Critique and verdicts (`/tmp/i26/critique.md`)

| Round | Scene | Finding | Verdict |
|---|---|---|---|
| 1 | Thunderstorm, tint α 1.3 | an opaque white wall (153 DN vs 140) | FAIL |
| 1 | Snow | a few 4–5× soft blobs (log-normal tail) | FAIL |
| 2 | game look, α 0.7 | rain within ~8 DN of the sunlit grass, nearly invisible | NOT YET |
| 3 | brighter crate tint | crate-lit town greyed into a veil; reverted, the game sets its own tint | NOT YET |
| 4 | Rain / game Rain / Snow / game Snow / dusk at 35° | upright 15° streaks that read as rain; flakes of many sizes; town legible | GREAT |
| 4 | Defaults | iso Rain and Snow mean colour within 0.02 DN of before; same dashes and dots | GREAT |

### Verification

- `look::tests`: streak angle 63.4° physical → 15.3° readable at 35° pitch; flake CV ≥ 0.3 with the mean within 5 %; the default is the identity. `weather::tests::the_forecast_is_the_weather_that_comes` (fresh chain and mid-chain under a forced Thunderstorm, director unchanged). `tests/save.rs` (5).
- Full GPU suite, cargo test, clippy, doc, wasm, save: pass.
- Bench (`bench_elements`, High, 3840 × 2160, Thunderstorm, P2 1695–1755 MHz, minimum, two runs each): `elements_precip_draw` 0.122 / 0.126 ms before, 0.123 / 0.115 ms readable.

### Top 5 remaining items, ranked

1. **#26: trails on a game's own ground** without the crate's `Terrain`.
2. **The isometric town at night is black** (22 h, `Rain`: mean 1 DN, max 8, under both looks).
3. **A 25 mm/h storm hides the whole town under an orthographic camera** (both looks; the storm's coverage and the linear fog from the nearest ground).
4. **The overcast dome is flat** (Eddington's emergent radiance in `cloud_scatter`), carried.
5. **Bright snow scenes under the metered exposure**, **the water's body colour under a grey sky**, **a bolt hidden by heavy rain**, carried.

## Iteration 27 — 2026-10-18 — trails on a game's own ground (issue #26)

**Commits:** `c81ac03` (research note `docs/research/trails-game-ground.md`), the implementation with its tests, the examples, docs and images, this log. Merged through a pull request that closes #26. **Sibling repos:** none (`TrailGround` takes `bevy_flora`'s `FloraGround` shape by convention, not by dependency). **Not released.** The GPU was taken through `gpu-tenant claim coding --who bevy_elements@big`; olmOCR's vLLM reloaded during the lease and left 0.9 GB, so the llama-swap models were evicted again as a grant does (logged on the board).

### Candidates (open issues first)

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | #26: trails need the crate's `Terrain`; a game with its own ground (Sanctuary) gets none, no query, and the materials fade every trail under a camera that stands back from its focus | 3 (missing feature) + 1 (a wrong result: the fade by the camera) | **this iteration** |
| 2 | The isometric town at night is black | 1 | carried |
| 3 | A 25 mm/h storm hides the town under an orthographic camera | 2 | carried |
| 4 | The overcast dome is flat | 2 | carried |
| 5 | Bright snow scenes under the metered exposure | 2 | carried |

#26 beat the black night because it is the only open issue, a consumer's blocked gameplay decision (prints at dawn show where flankers went), and it carried a real bug: every trail under the isometric camera was faded out.

### What changed

- **`TrailGround`** (world XZ → world height, soil or `with_sand()`): the kernels take a uniform ground from `SurfaceWeathering` (snow depth and density, wetness), the contact is tested on the CPU against the game's height, and the terrain and cover bindings hold placeholders; `TrailGpu` owns the handles it binds, so the render world no longer needs `TerrainGpu` or `CoverGpu`.
- **`WeatheredExtension::with_trails()`**: the same parallax march as `GroundExtension` (shared `trails::surface`), the window, its centre, the beam's share and the snow depth from two new `vec4`s in `ElementsGlobals` (96 → 128 B, rewritten in place, so the moving centre re-prepares nothing), the field at bindings 104/105, no storage buffer.
- **The window centre is published** (`TrailState::centre`) and every material fades by it, not by the eye: the fix for the focus.
- **`Environment::trail_depth`**: a fifth kernel (`trail_readback`, its own layout and pass) writes the deepest point per 2 × 2 texels with the window origin in a header; `TrailSnapshot` places it.
- **`TrailSettings::texel_m`**, **`SurfaceWeathering::with_snowpack`** (replacing the crate's test-only constructor).
- Examples: the `isometric` walker presses prints into the town's own ground (8 cm texels, `ELEMENTS_SNOWPACK`, `ELEMENTS_TRAIL_TEXEL`); `trails` with `ELEMENTS_TERRAIN=mesh` (a game's hilly mesh, no terrain); `bench_elements` with `ELEMENTS_TERRAIN=game`.

### Sources

Barré-Brisebois 2014 (GDC, *Deformable Snow Rendering in Batman: Arkham Origins*: "we don't have terrain", snow surfaces a game marks, replenish while snowing; slides read as text), Tran 2018 (Kodeco, a world-space trail map projected onto the ground material), Gawron 2018 and `roymasad/Unreal-Snow-Deformation` (surfaces opt in), Hanák 2021 (RotTR's window, TLOU2's parallax prints on any mesh), Tatarchuk 2006 (`s2006-advances-materials-10`), Alvarado et al. 2022 (`10.48550_arxiv.2209.10215`), CRREL 90-9, `bevy_flora` 0.46 `FloraGround`. `distill_search` through the MCP bridge returned 404; `hs distill search` worked.

### Critique and verdicts (`/tmp/i27/critique.md`)

| Scene | Evidence | Verdict |
|---|---|---|
| Game mesh, snow, 2–20 m | `r2/ms3.png` | GREAT: crisp pockets, sky-blue far walls, sunlit lips; equal to the terrain path |
| Game mesh, sand 18.3 h / mud | `r2/sd3.png`, `r2/z_mud.png` | GREAT (trail); the example's mud art is a plain wet colour |
| Isometric, 17.3 h, against the light | `r3/e1.png`, `r3/e3.png`, `final/iso_eve.png` | GREAT: the walker's whole loop a dotted ellipse of shadowed prints at 1× |
| Isometric, dawn behind the camera / noon | `r1/z6_both.png`, `r3/n1b.png` | faint, by physics (down-sun, tonemapper shoulder); Deviation 6 |
| Terrain path | `r2/terrain_snow.png` vs `before/trails_snow.png`; `gpu_trails` 4/4 | unchanged |

Found and fixed in review: mesh mode forcing Clear before the harness grew the snow; the weathered snow bared everywhere when a game pins `snow_cover` without a depth (`gpu_weathered` caught it).

### Verification

- `tests/gpu_trail_ground.rs` (4): 20 m walk, `trail_depth` 0.0700 m at all 39 points, `Some(0)` beside, `None` outside, no `Terrain`; contact on a 10 % slope (0.0690 m on the ground, 0 from 0.3 m above); 5 cm more snow 6.90 → 1.90 cm; a weathered plane draws the print (129, 166, 214) against (234, 231, 231) only `with_trails()`. `trails::tests` (readback decode, a moved window refused).
- Full GPU suite, cargo test, clippy, doc, wasm, save: pass.
- Bench (`bench_elements`, High, 1080p, P2 1755–1800 MHz, minimum): `elements_trails` hills 0.119–0.121 ms (before 0.119), game ground 0.112–0.116 ms; `elements_trails_readback` 0.024–0.026 ms on its 4 Hz ticks; `main_opaque_pass_3d` with the ruts filling much of the frame on a weathered plane 0.413 ms against 0.226–0.236 ms off.

### Top 5 remaining items, ranked

1. **The isometric town at night is black** (22 h, `Rain`: mean 1 DN), carried.
2. **A 25 mm/h storm hides the whole town under an orthographic camera**, carried.
3. **Bright snow scenes under the metered exposure**: snow at 236 DN sits in the tonemapper's shoulder, which also hides prints at noon (`trails-game-ground.md` Deviation 6), and house shadows on snow read a saturated sky blue.
4. **The overcast dome is flat** (Eddington's emergent radiance in `cloud_scatter`), carried.
5. **A compacted-ground material stage for `WeatheredMaterial`** (Barré-Brisebois's two stages: a trench floor through grass shows soil on the terrain, the game's colour on a mesh), and **the water's body colour under a grey sky**, carried.

## Iteration 28 — 2026-10-20 — the ground has surface detail at a metre

**Commits:** `7342d28` (research note `docs/research/ground-detail.md`), the implementation with
its tests, docs and changelog, this log. **Sibling repos:** none. **Not released.** The GPU was
taken through `gpu-tenant claim coding --who bevy_elements@big`. The pass began with a cloud
iteration (the overcast dome's gradation); the invoking prompt redirected it to the ground, and
that work was reverted, diagnosed and recorded under "Deferred" below.

### Candidates (open issues first: none open)

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | The ground is a flat wash at every eye height: near-patch standard deviation **1.07 DN** of 255 at 6 m, 1.64 at 2 m, 1.81 on sand | 2 (beauty gap) | **this iteration** |
| 2 | The isometric town at night is black (22 h, Rain: mean 1 DN) | 1 | carried |
| 3 | A 25 mm/h storm hides the town under an orthographic camera | 2 | carried |
| 4 | The overcast dome is flat (1.08:1 against CIE's 2:1–3:1) | 1 | attempted, deferred (below) |
| 5 | Bright snow scenes under the metered exposure | 2 | carried |

Rank 1 beat the black night because it is in every daylight frame of every outdoor game, at the
distance a player spends the game looking at; the night is one preset at one hour.

### What changed

- **Two band-limited octave scales** in `terrain::common::ground_detail`: a coarse 1.25 m Perlin
  sum (4 octaves, ±20 %) for the mid-distance, a fine 9 cm sum (3 octaves, decorrelated seed,
  ±85 %) for the near field. Each octave is cancelled once the pixel footprint cannot resolve it
  (Lagae et al. 2010 §5.2.1, frequency clamping), replacing the old single 0.8 m⁻¹ band faded by
  a 40–120 m distance constant. `detail_keep` and `detail_scale` are the pieces.
- **`noise::perlin2_d`**: 2D gradient noise in XZ with its analytic derivative. The detail cannot
  use the 3D noise at the surface point: a 3D slice on a tilted surface is not band-limited and
  streaked the cliff faces with diagonal bands (recorded as Deviation 1).
- **The detail fades with the surface's upness** (`smoothstep(0.55, 0.9, n.y)`) as well as the
  footprint, and its footprint is the XZ span divided by the upness: the detail is a function of
  XZ, so a slope's compressed projection must not under-filter it (Deviations 2 and 3).
- Both `TerrainExtension` and `GroundExtension` use it, so a game's own ground gets the same
  surface.

### Sources

Lagae et al. 2010, *A survey of procedural noise functions* (corpus
`10.1111_j.1467-8659.2010.01827.x`): §5.2.1 frequency clamping, §5.2's "a 3D noise sampled at a 2D
surface will not result in a band-limited texture", Cook & DeRose's finding that Perlin is weakly
band-limited. Andersson 2007 (corpus `10.1145_1281500.1281668`): the defect ("lack of detail
either up close, in a distance, or at high angles") and the texture-based fix this note replaces
with procedural bands. Perlin 1985 (via Lagae §2), Musgrave et al. 1989, Galin et al. 2019 (via
`terrain.md`), `terrain.md` Model 2 and Deviation 19 (the replaced constants).

### Critique and verdicts (`/tmp/i28g/critique.md`)

| Round | Finding | Verdict |
|---|---|---|
| 1 | 3.47 DN near-field (target 4); the 15–40 m band softer than both sides; grain organic | NOT YET |
| 2a | black speckles: the clump remap drove the albedo below zero | FAIL |
| 2c | sand a halftone grid, rock a woven fabric at 4× (single Worley lattice) | FAIL |
| 2d–2e | diagonal streaks on the cliff faces: a 3D noise slice is not band-limited on a slope | FAIL |
| 2f | streaks gone by the upness gate; grass fell to 1.98 DN | NOT YET |
| 2h | **sd 4.26** grass, 5.86 dunes, 5.55 plains, 6.51 grazing; means within 1 DN; no streak, no speckle, no flicker | **GREAT** |

### Verification

- `terrain::model::mirror::tests::the_detail_band_limit_keeps_resolved_octaves_and_cancels_the_rest`:
  kept whole at half the feature, zero at the feature, monotone and continuous across the falloff.
- Distance invariance: far-ground mean 161.019 → 161.017 DN (`hills_6m_noon`), 153.681 → 153.681
  (`hills_aerial`); snow covers the detail (`trails` `ELEMENTS_TERRAIN=mesh` under Snow).
- Temporal: consecutive static frames RMSE 0.013 % of range.
- Full gate: `cargo test` 251, clippy, doc, wasm, the GPU integration suite (one process per file
  and all 19 files), the lib GPU modules clouds/terrain/cover/dust (32) and `the_horizon_sky`
  alone: all pass.
- Bench (`bench_elements`, High, 1920 × 1080, hills, minimum): **0.046 ms** at P2 1710 MHz,
  against the baseline build's **0.058 ms** at P2 1620 MHz in the same session — the new detail
  is *cheaper*, because the old expression evaluated `fbm3` and `perlin3_d` at three octaves each
  and a 2D derivative chain per octave costs less. No regression to justify.
- `nvidia-smi` logged in the same shell: P2, 1680 MHz idle; 1710 during the new run, 1620 during
  the old.

### Deferred: the overcast dome's gradation (attempted first, reverted)

The first half of this pass built Eddington's emergent intensity `(2 + 3μ)/4` for the deck's
radiance (unit hemispherical mean, so the irradiance, exposure and `illuminance_at` are
unchanged). The CPU mirror confirmed the deck grades 3.36:1 and the Monte Carlo slab error
improved 0.0674 → 0.0669; the GPU composed cube read 1.08 → 1.34 (target 1.8–3.2) with
`L_Z/E` 0.344 → 0.383 (inside the CIE 0.382–0.409 band) and `E/model` 0.76 → 0.77. **Blocker,
diagnosed:** the deck grades correctly but the *composite* dilutes it, because
`sky_through_clouds` adds the clear-sky cube at `sky_gain ≈ 126×` under a deck and at grazing
that constant (~1900 cd/m²) flattens the measured ratio. The next attempt must fix the grazing
band's compositing before the gradation can be measured. Files: `/tmp/i28/findings-cloud.md`.

### Top 5 remaining items, ranked

1. **The isometric town at night is black** (22 h, Rain: mean 1 DN), carried three iterations.
2. **The overcast dome is flat**, with its blocker now diagnosed precisely (the `sky_gain` leak at
   grazing); carries its Eddington implementation and measurements in `/tmp/i28`.
3. **A 25 mm/h storm hides the whole town under an orthographic camera**, carried.
4. **Rock has no material detail of its own**: the ground's grain is gated off the steep faces,
   which are left with only the mask's colour and the terrain's geometry. Strata, cracks and a
   rock-specific bump are the natural next piece, and `terrain.md` has no note for it.
5. **The ground's large-scale variation** (patches of longer and shorter grass, bare earth,
   trampled areas) — the masks vary only with slope and sediment, so a plain is one colour at
   200 m. A low-frequency, contrast-only modulation driven by the same noise would fix it.

## Iteration 29 — 2026-10-20 — rock has a material, and the ground has a place in it

**Focus:** the ground, the earth, the mountains — the stuff that's stood on.

**Commits:** `578b9e1` (the research note `docs/research/rock-material.md`), `11b1490` (the
implementation), `71794da` (the split: the turf off the faces, the rock's material), `c0871aa`
(the rock's colour, the relief and the scree), `ebc5c9b` (the docs, the changelog and the
showcase images), this log. **Sibling repos:** none. **Not released.**

### Candidates

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | The rock faces were a woven herringbone (coherence 0.71 at a fixed 43.5° world direction) and the rock had no material of its own (one flat colour, saturation 15.6 against the grass's 54.8) | 1 | **this iteration** |
| 2 | The plain at 6 m was one uniform green — the layer masks vary only with slope and sediment | 2 | **this iteration** (the same change) |
| 3 | The slope band's soil drew a red-brown contour line round every rock | 1 | **this iteration** (the same change) |
| 4 | The isometric town at night is black (22 h, Rain: mean 1 DN) | 1 | carried |
| 5 | The overcast dome is flat (1.08:1 against CIE's 2:1–3:1), with its blocker diagnosed in iteration 28 | 1 | carried |

Rank 1 beat rank 4 because it is in every outdoor frame of the game the crate ships in — a
player looks at rock whenever the terrain is steep, and the mountains *are* the domain the focus
names. Rank 2 and 3 turned out to be the same defect family (the layer masks) and were taken
with it.

### What changed

- **The ground's turf and the rock's material are two different noises.** The turf — grass, soil
  and sand — is the crate's 2D noise in the world XZ plane, which is the *projection* of the
  surface onto that plane, and it is right on the near-horizontal surfaces where the plane's
  projection is the surface. It must not reach a slope: the plane's lattice is stretched along
  the slope and shows through as a weave at a fixed world direction. So the turf is gated off
  (`turf_gate`, closed at 37°, open at 23°) and the rock's ramp opens at 30°, and the rock's
  material is the crate's **3D noise at the 3D point**, which has no projection at all.
- **The rock has a material:** beds on a *tilted* plane, a second warmer bed tone, a fracture
  network of ridged noise (a Worley cell network bounds cells and read as a giraffe's hide), the
  rock's own grain, and a relief so the light does work on the face.
- **The ground has a place in it:** a low-frequency field takes soil from the grass, so a plain
  gets patches of bare earth. Andersson's Overlay detail cannot do this — it "does not affect
  areas where the base procedural mask is 0.0 or 1.0", and a plain's grass mask is 1 everywhere.
- **The slope band's soil is broken into patches and tapers into the rock**, so the warm brown
  that ringed every grass patch is gone.

### Sources

- **Lagae, Lefèbvre, Cook, DeRose, Drettakis, Ebert, Lewis, Perlin & Zwicker (2010)**, *A survey
  of procedural noise functions*, CGF 29(8), `10.1111/j.1467-8659.2010.01827.x` (corpus; §3.2.1
  wavelet noise, §5.2.1 frequency clamping, §5 on sampling a noise on a surface).
- **Cook & DeRose (2005)**, *Wavelet noise*, ACM TOG 24(3), `10.1145/1073204.1073264` — read
  through the survey's account; the PDF at `graphics.pixar.com` returned HTML and the ACM copy
  is paywalled. **Not indexed.**
- **Lagae, Vangorp, Lenaerts & Dutré (2010)**, *Procedural isotropic stochastic textures by
  example*, Computers & Graphics 34(3), `10.1016/j.cag.2010.05.004` — downloaded to
  `/tmp/research/pdf/lagae2010.pdf` (9.5 MB) and read with `pdftotext -layout` (715 lines).
  **Not indexed.**
- **Weiß, Bayer & Westermann (2020)**, *Triplanar displacement mapping for terrain rendering*,
  Eurographics 2020, `10.2312/egs.20201016` — the preprint downloaded to
  `/tmp/research/pdf/weiss2020.pdf` and read with `pdftotext -layout` (295 lines). **Not
  indexed.**
- **Andersson (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`) — the Overlay blend and its
  "does not affect areas where the base procedural mask is 0.0 or 1.0".
- **Söderlund (2015)**, *Procedural Modeling of Rocks*, Uppsala University MSc thesis — the record
  and the abstract were read; the PDF did not download. **Not obtained.**

### The critique and the verdicts (`/tmp/i29/critique.md`)

| Round | Finding | Verdict |
|---|---|---|
| 1 | The surface-frame projection put a herringbone moiré over every face (the normal comes from the terrain's 2 m cell gradient, so the frame rotates per pixel and the noise's lattice swims in it); the Worley fractures were a giraffe's hide; the rock was one flat colour; the plain was one colour | FAIL |
| 2 | The 3D noise at the 3D point removed the weave, but the beds on the world's height made the massif a topographic map, and a fine corduroy came back at 700 m | NOT YET |
| 3 | The tilted beds and the split fixed the corduroy, but the rock was too light, the pink patches covered 16.75 % of the 2 m face, and the grass had an orange rim | NOT YET |
| 4 | The rock's grain was too contrasty (sandpaper), the saturation 7.6, and the scree was missing | NOT YET |
| 5 | The grain's contrast down, the relief up, the rock's hue in, the scree in: the face is stone, the massif is grey stone with scree, the plain has patches, the pink and the orange are gone | **GREAT** (the scree and the grass's own colour are NOT YET — see below) |

| Feature | Verdict |
|---|---|
| The rock's face (the weave) | **GREAT** |
| The mountain from 700 m | **GREAT** |
| The plain at 6 m | **GREAT** |
| The hills' ground at 6 m | **GREAT** |
| The scree at the grass's foot | NOT YET (a graded band, not a scatter) |
| The grass's own colour | NOT YET (a pure green with no dryness — not this change's business) |

**The change as a whole: GREAT.** The two blockers — the weave and the rock with no material —
are fixed, the colour defects are gone, and the cost went *down*.

### Measurements

- **The weave.** The structure tensor's coherence of the near band, and its direction:

  | | 2 m | 6 m |
  |---|---|---|
  | before | 0.709 at 43.3° | 0.655 at 45.2° |
  | after | 0.580 at 33.8° | 0.675 at 33.0° |

  The before's two bands shared a *world* direction (43.3° and 45.2°) — the signature of a
  lattice. The after's do not (33.8° and 33.0°), and the remaining anisotropy is the terrain's
  own relief: a 9 px high-pass of the face keeps the coherence at 0.676, and the *baseline* has
  the same at 0.625, because the terrain's 2 m cells' normals are anisotropic on a slope. The
  note's target of a coherence below 0.45 was therefore wrong — it asked the whole band to be
  isotropic when the terrain's relief cannot be — and is amended in the note with the
  measurement that replaces it.
- **The colour defects.** Pink (a warm brown mixed with a cool grey) and orange (a warm brown
  ring round the grass): 16.75 % and 9.67 % of the mountain's 2 m and 6 m faces before, **0.00 %
  and 0.00 %** after; the orange rim from 1415 pixels to none; the hills' pink from 0.32 % to
  0.04 %.
- **The plain.** The 60 px block standard deviation 2.83 → 3.34 DN, the mean within 0.02 DN.
- **The flat ground is unchanged.** The near-field standard deviation 6.11 → 5.86 DN, the mean
  133.66 → 133.31 DN.
- **Cost.** `bench_elements`, High, 1920 × 1080, `mountains`, the minimum over the
  diagnostic history, two runs of each build alternating in one session, the GPU at
  1695–1725 MHz: **`main_opaque_pass_3d` 0.136 ms on the baseline build, 0.127 ms on this
  one** — the mask's fBm went from four octaves to one and that pays for the rock's material.
  The GPU was contended for part of the session (vLLM at 94 %); the llama-swap models were
  evicted with the `gpu-tenant` lease's own recipe and the numbers are from the clean runs.
- **The full gate.** `cargo test` 252, clippy, doc, wasm, the GPU integration suite (22 files),
  the lib GPU modules clouds (10), terrain (8), cover (8) and dust (6), and `the_horizon_sky`
  alone: all pass.

### The next five items, ranked

1. **The scree is a graded band, not a scatter** — the grass meets the stone in a smooth wash
   where it should be stones of many sizes with turf between them. The idea and the place are
   right; the scatter is not there. The natural next piece of work on this material, and the
   rock's grain needs a *cell* structure for it (a Worley network of *stones*, not of walls).
2. **The grass has no dryness.** A pure green (RGB 120, 147, 92, saturation 55) with no dry
   grass, no litter and no yellowing at the edges. It is the crate's grass albedo and this
   change did not touch it, but against the rock's grey it reads as a colour change rather than
   as a place, and a meadow at a stone's foot wants some dryness in it.
3. **The isometric town at night is black** (22 h, Rain: mean 1 DN), carried three iterations.
4. **The overcast dome is flat**, with its blocker diagnosed in iteration 28 (the `sky_gain` leak
   at grazing); it carries its Eddington implementation and its measurements in `/tmp/i28`.
5. **A 25 mm/h storm hides the whole town under an orthographic camera**, carried. The rain's
   *readable* look is also less legible than the physical one (measured in the last iteration:
   the readable streaks carry 32 DN of contrast against the grass where the physical ones carry
   44), which is the defect the town capture keeps showing.

## Iteration 30 — 2026-10-20 — the ground has more than one colour: a meadow is not one green

**Commits:** `7197ba7` (the research note `docs/research/ground-colour.md`), `2251ce9` (the
palette), `1e6d030` (the shared fields), `a5c89f4` (the convex shares), `7775900` (the clump's
weight), `548e3c7` (the broken edge), `811adc6` (the square, not a `pow`), `6a9b3b9` (the docs,
the changelog and the showcase image), this log. **Sibling repos:** none. **Not released.**

### Candidates

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | The ground is **one colour** over the whole frame: the hue's spread over a near-ground patch is 0.010 on the plains and 0.016 on the hills | 2 (a beauty gap) | **this iteration** |
| 2 | The grass/rock boundary is a blurred paint edge | 1 | **this iteration** (the same change, at the boundary) |
| 3 | The scree is a graded band, not a scatter | 2 | carried |
| 4 | The isometric town at night is black (22 h, Rain: mean 1 DN) | 1 | carried |
| 5 | The overcast dome is flat (1.08:1 against CIE's 2:1–3:1) | 1 | carried |

Rank 1 beat the carried items because it is in *every* outdoor frame of the game: a third-person
camera looks at the ground from one to twenty metres most of the time. The crate's ground
already had the *shape* of a meadow — its silhouette, its tufts, its patches of bare earth — and
none of the *colour*.

### What changed

- **The turf is a palette of four tones** — fresh grass, the yellow of a dry patch, the straw
  of a bleached one, the earth of a worn place — mixed by a **score**, exactly Papavasiliou's
  model: a low-frequency field decides how much of each tone is *around* and a finer one
  decides which tone this piece of ground *wears*, and the score is their product.
- **The four tones are mixed as *shares* that sum to one.** The mixture is a convex combination
  of them, so the ground's mean colour and brightness cannot drift. Three other normalisations
  were tried and each failed (the unweighted mean of the tones drifted the mean brightness
  3.2 DN; normalising by the mixture's own colour cancelled the hue entirely; a per-channel
  mean cancelled it again).
- **The palette's two fields are the ground's own** — the large-scale field the layer variation
  already uses, and the turf's coarse and fine detail fields — so the palette's only new noise
  evaluation is its selection field at five metres. It is also a *better* field than the
  palette's own: the ground's varies more, and the hue's spread over a near-ground patch is
  0.0391 with the shared field against 0.0173 with the palette's own.

### Sources

- **Papavasiliou, N. (2015)**, *Real-Time Grass (and Other Procedural Objects) on Terrain*,
  JCGT 4(1):26–49 (`https://jcgt.org/published/0004/01/02/paper.pdf`, downloaded to
  `/tmp/research/pdf/papavasiliou2015.pdf` and read with `pdftotext -layout`, 1211 lines; no DOI).
  **§3.3, "Controlling Plant Distribution", is the model this iteration adopts**: "Vegetation in
  nature does not always uniformly cover the ground. The density usually varies due to
  variations in the underlying soil and parts of the terrain may be altogether bare."; the
  **reference colour** and the **score** — for each terrain type a reference colour, and at each
  point a score for that type "based on the inverse distance of the terrain type's reference
  color to the terrain's base map color at that point", in HSV; the **weight vector** `w`, which
  "allows us to ignore certain color components during the calculation of the distance … where a
  reasonable choice is gray for which hue is meaningless", and the note that it "is also useful
  in many cases to discard the lightness component, as its variation over the terrain is often
  the result of shadowing effects, which are unrelated to terrain type"; the **exponent** `p`,
  "useful in controlling the sharpness of the transition between different terrain types. By
  using lower values for p, broader transitional regions between terrain types can be achieved
  while larger values create sharper, more defined transitions"; and the **threshold**. **Not
  indexed.**
- **Lefèbvre, S., Neyret, F. (2003)**, *Pattern based procedural textures*, ACM SIGGRAPH
  Symposium on Interactive 3D Graphics (I3D '03):203–212, DOI `10.1145/641480.641518`
  (downloaded to `/tmp/research/pdf/lefebvre2003.pdf` and read with `pdftotext -layout`, 968
  lines). Used: "an algorithm which procedurally determines the texture value at any surface
  location by **aperiodically combining provided patterns** according to user-defined controls
  such as a probability distribution (possibly non stationary)"; the **probability map**, "an
  explicit indirection map … simply contains the indices of the patterns to be used"; and the
  **dithering operator** for the areas map's interpolation, which this iteration replaces with a
  smooth boundary so the transition is not a speckle. **Not indexed.**
- **Andersson, J. (2007)**, *Terrain rendering in Frostbite using procedural shader splatting*,
  SIGGRAPH 2007 *Advances* ch. 5 (corpus `10.1145_1281500.1281668`): the "bland transitions" the
  crate inherited and the per-material detail that fixes them.
- **Grenier, C., Guérin, É., Galin, É., Sauvage, B. (2024)**, *Real-time Terrain Enhancement with
  Controlled Procedural Patterns*, CGF 43(1):e14992, DOI `10.1111/cgf.14992` (the abstract and
  the search index's account; the PDF is behind Wiley's 403): the **palette of control maps** for
  artist control over a structured noise's patterns, and the principle that a terrain's details
  "correspond to the underlying terrain characteristics". **Not indexed.**
- **Lagae et al. (2010)**, *A survey of procedural noise functions*, CGF 29(8),
  `10.1111/j.1467-8659.2010.01827.x` (corpus), as in `ground-detail.md`.
- **`ground-detail.md`** (iteration 28) and **`rock-material.md`** (iteration 29): the ground's
  two detail scales, the layer masks, the layer variation, and the rock's own material.

### The critique and the verdicts (`/tmp/i30/critique.md`)

| Round | Finding | Verdict |
|---|---|---|
| 1 | The carry term used the base colour's *absolute* chroma, and a dark albedo has a small one by construction, so the palette reached the ground at 39 % of its strength and the hue's spread *fell*; the near field had no variation; the far field was a haze | NOT YET |
| 2 | The hue's spread was four times better with the shared fields, but the mean drifted 3.2 DN and three `pow` calls cost 84 % of the opaque pass | NOT YET |
| 3 | The convex shares hold the mean, the square is a multiply, and the clump's coarse field breaks the patch's edge: the plains is a meadow | **GREAT** |

| Feature | Verdict |
|---|---|
| The plain at 6 m | **GREAT** |
| The hills' ground at 6 m and 2 m | **GREAT** |
| A game's own ground (`GroundExtension`) | **GREAT** |
| The rock (unchanged, byte-identical) | **GREAT** |
| The dry patch's own *texture* | NOT YET (a tint on green blades, not straw-coloured blades) |
| The patches' texture against the green grass's | NOT YET (a dry patch is coarser) |

**The change as a whole: GREAT.** The blocker — a ground that is one colour — is fixed, the mean
does not move, the rock is untouched, and the cost is 5 %.

### Measurements

- **The ground is not one colour**: the G/R ratio's spread over a 580 × 240 px patch of the near
  ground — the plains 0.0097 → **0.0226**, the hills at 6 m 0.0258 → **0.0339**, the hills at 2 m
  0.0156 → 0.0169, the dunes 0.0047 → 0.0096. The far field's *local* spread (a 25 px blur
  subtracted) 0.0098 → 0.0124.
- **The mean does not move**: the plains' mean brightness 135.69 → 135.55, the hills at 2 m
  131.50 → 132.14, and the mean hue within 0.002. The shares are a convex combination, so this
  is a property of the construction and not of the tuning.
- **The palette's own span**: the G/R ratio runs from 1.00 (the fresh tone, the identity) to 0.86
  (the straw).
- **The rock is byte-identical**: the 6 m face reads 136.5, 130.7, 124.8 before and after, and
  the pink and the orange are 0.00 % in both.
- **The turf is untouched**: the flat ground's near-field standard deviation 6.11 → 5.86 DN.
- **Cost.** `bench_elements`, High, 1920 × 1080, `mountains`, the minimum over the diagnostic
  history, one build at a time with the GPU quiet before each run and the clocks logged in the
  same shell, at 1800 MHz: **`main_opaque_pass_3d` 0.137 ms on the baseline build, 0.144 ms on
  this one** — a 5 % regression. The palette's three `pow` calls were 84 % of it (0.252 ms), and
  writing the square as a multiply brought it back.
- **The full gate.** `cargo test` 253, clippy, doc, wasm, the GPU integration suite (22 files),
  the lib GPU modules clouds (10), terrain (8), cover (8) and dust (6), and `the_horizon_sky`
  alone: all pass.

### The WESL traps this iteration cost an hour on

All three make the surface render as a **dark navy with no shader error at all**, and all three
are now in `AGENTS.md`:

- A name used from another WESL module must be in the **import list**. An unimported name makes
  naga emit a broken shader *silently*.
- **`pow(x, p)` with `p` in a `let`** does the same; a literal exponent works.
- A local that **shadows** another local in the same scope does the same.

### The next five items, ranked

1. **The dry patch's own texture.** The palette gives the ground a *hue*; a dry patch needs a
   *texture* too — the blades inside it should take the patch's hue rather than the turf's, and
   a dry patch is coarser than green grass. The colour half is in this crate and is the natural
   next step; the geometry half (a grass mesh) belongs to `bevy_flora`.
2. **The scree is a graded band, not a scatter** — stones of many sizes with turf between them.
   The idea and the place are right; the scatter is not there. The rock's grain needs a *cell*
   structure for it (a Worley network of *stones*, not of walls).
3. **The isometric town at night is black** (22 h, Rain: mean 1 DN), carried three iterations.
4. **The overcast dome is flat**, with its blocker diagnosed in iteration 28 (the `sky_gain` leak
   at grazing); it carries its Eddington implementation and its measurements in `/tmp/i28`.
5. **A 25 mm/h storm hides the whole town under an orthographic camera**, carried. The rain's
   *readable* look is also less legible than the physical one (measured in iteration 28: the
   readable streaks carry 32 DN of contrast against the grass where the physical ones carry 44),
   which is the defect the town capture keeps showing.

## Iteration 31 — 2026-10-20 — the night sky lights the ground: a moonless night is not a black screen

**Commits:** `2e89e8e` (the research note `docs/research/night-sky-light.md`), `8c896cb` (the
implementation), `6d316c3` (the docs, the changelog and the showcase image), this log. **Sibling
repos:** none. **Not released.**

### Candidates

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | A **moonless night is a black screen**: the isometric town at 20 h renders at 1 DN | 1 (physically wrong) | **this iteration** |
| 2 | The town's ground is a flat olive-brown at 18 h | 2 | carried (the example's own material) |
| 3 | The scree is a graded band, not a scatter | 2 | carried |
| 4 | The dry patch's own texture (a tint on green blades) | 2 | carried |
| 5 | The overcast dome is flat | 1 | carried (blocker diagnosed in iteration 28) |

Rank 1 beat the carried items because it is the only one that is *class 1* — a physically
wrong result rather than a beauty gap — and because it makes the game unplayable at night: the
player cannot see their character, their ground, or their world. It has also been carried three
iterations, so it is the most persistent defect in the crate's backlog.

### What changed

- **The night sky's radiance is part of the ambient.** The airglow, zodiacal light and starlight
  that lie *behind* the atmosphere are the crate's `ILLUMINANCE_FLOOR_LUX = 0.0005 lx`, and the
  crate used it for the exposure meter's floor and the background's clear colour — but the
  **ambient that illuminates surfaces** came from `cloud_ambient`, which is zero when neither
  the sun nor the moon is up. So a moonless night had no ambient at all: the sky was lit and the
  world on it was not.
- **The sky probe's cube carries it too.** A camera *with* a probe has its ambient set to 0 —
  the light comes from the cube — and the probe's cube is *display-referred*, so the radiance is
  converted with the camera's exposure, as the fog's haze is.
- **The colour is the *scotopic* one** — a cool blue-green, from Jensen et al. 2001's blue shift
  and the rod's curve, whose peak is at 507 nm.
- **The cloud does not dim it**: the night sky is behind the atmosphere, and the floor is
  *added* to the bodies' light, not multiplied with it.

### Sources

- **Jensen, H. W., Durand, F., Stark, M. M., Premože, S., Dorsey, J., Shirley, P. (2001)**,
  *A Physically-Based Night Sky Model*, ACM SIGGRAPH 2001, DOI `10.1145/383259.383306` (**in the
  corpus**, stem `10.1145_383259.383306`, read in full for this note). Used: Figure 2, the
  sources of natural illumination at night (zodiacal $1.2\times10^{-7}$, integrated starlight
  $3.0\times10^{-8}$, airglow $5.1\times10^{-8}$, galactic $9.1\times10^{-9}$, cosmic
  $9.1\times10^{-10}$ W/m², the full moon $2.1\times10^{-3}$); §6, the four sources of diffuse
  night light and the sentence that is the reason for this note — "These effects are especially
  important on moonless nights, when **a small change in illumination can determine whether an
  object in the scene is visible or invisible**"; §6.2, "Airglow is the principal source of
  light in the night sky on moonless nights"; §7, "**The sky is never completely black**" and the
  blue shift. **Not indexed** (the ONNX out-of-memory error in `corpus.md`).
- **The crate's own `twilight.md`**: the *What great looks like* the note had already set — "a
  moonless sky is near-black with stars and **the horizon's silhouettes still readable against
  it**" — and its item 8, the visual target of "silhouettes against a moonless sky", which the
  *sky* delivered and the *ground* did not.
- **The crate's own `celestial.md`**: Jensen's numbers as the crate uses them, and the
  non-lunar night illumination's sum.

### The critique and the verdicts (`/tmp/i31/critique.md`)

| Round | Finding | Verdict |
|---|---|---|
| 1 | The fix was in the *ambient*, which is right for a camera without a probe and invisible for one with it: the town has a probe, so it was still black (0.907 → 0.9085 DN). And the cube is display-referred: adding the raw radiance put a value 10⁵ times too small in it (the mean went 0.91 → 2.15 DN, the houses appeared and the ground did not) | NOT YET |
| 2 | The night sky's radiance in the ambient *and* in the probe's cube, display-referred: the town is a night scene, the day is byte-identical, and the cost is nil | **GREAT** |

| Feature | Verdict |
|---|---|
| A moonless night, isometric | **GREAT** |
| A moonless night, `day_night` (a followed camera with a probe) | **GREAT** |
| The twilight's continuity | **GREAT** |
| The day | **GREAT** (byte-identical) |
| The cost | **GREAT** (0.8 %) |
| The night sky's *colour* | NOT YET (brown; pre-existing, and the next item) |
| The town's ground | NOT YET (the example's own flat plane) |

**The change as a whole: GREAT.** The blocker — a moonless night is a black screen, and the
player cannot see their character — is fixed, on both the probe's path and the ambient's path,
the day is untouched, and the cost is nil.

### Measurements

- **A moonless night is not black**: the isometric town in `Clear`, `ELEMENTS_ZOOM=60
  ELEMENTS_PITCH=35`, 2026-04-15 — 20 h **0.907 → 24.763 DN** (27×) and 22 h **0.739 → 22.443
  DN** (30×), with the blue/red ratio rising from 0.992 to 1.059.
- **Both paths**: the `day_night` example (a followed camera, with a probe) at 22 h
  **3.409 → 14.230 DN**, and the `terrain_tour` camera (no probe) at 22 h 20.9 → 29.2 DN.
- **The day is untouched**: the town at 12 h reads 148.208 DN before and after, and at 18 h
  108.565 DN before and after — byte-identical.
- **The twilight has no step**: the terrain's mean luminance at 17, 18, 19, 19.5, 20, 21 and
  22 h is 159.5, 150.7, 118.0, 80.9, 65.8, 22.7 and 29.2 — the sun's fall and the night sky's
  floor meet, and the night *rises* as the twilight ends, as it must.
- **The moon still wins**: a moonlit night is four times brighter than the moonless one on the
  same frames, against Jensen's Figure 2: the full moon is 8700 times the moonless sources.
- **Cost.** `bench_elements`, High, 1920 × 1080, `mountains`, the minimum over the diagnostic
  history, the two builds run back to back in one session with the clocks logged throughout:
  **`main_opaque_pass_3d` 0.251 ms on the baseline build, 0.253 ms on this one** — 0.8 %, and the
  fix is on the CPU. (The GPU was contended for part of the session; the llama-swap models were
  evicted with the `gpu-tenant` lease's own recipe and the numbers are from the clean runs, at
  the same clocks for both builds.)
- **The full gate.** `cargo test` 253, clippy, doc, wasm, the GPU integration suite (22 files),
  the lib GPU modules clouds (10), terrain (8), cover (8) and dust (6), and `the_horizon_sky`
  alone: all pass.

### A test that asserted the bug

`celestial::tests::the_ambient_carries_the_diffuse_share_and_nothing_at_night` asserted
`brightness == 0.0` for a moonless night, with the comment "a night with no moon stays dark
whatever the cloud". That was the crate's *intent*, written down and tested — and it is what
made the night a black screen. The test now asserts the correction, and it is named
`..._and_the_night_sky_at_night`.

### The next five items, ranked

1. **The night sky is brown.** The `day_night` example's sky at 22 h reads R/G = 1.542, and
   `twilight.md`'s *What great looks like* says the night should be "**Nothing brown**". It is
   pre-existing (the baseline reads 1.599) and unchanged by this iteration, but it is the same
   night and it is the crate's own note that says it is wrong.
2. **A game at night has nothing to light with.** A moonless night is *shadowless* — physics —
   but a game wants a torch, a street lamp, a window, and the crate gives a game nothing to
   place them with: no `Environment` query for the local light, no hook for a game's own lights.
   That is an API gap, and it is in the crate's class-3 list.
3. **The dry patch's own texture.** The palette gives the ground a *hue*; a dry patch needs a
   *texture* too — the blades inside it should take the patch's hue rather than the turf's, and
   a dry patch is coarser than green grass. The colour half is in this crate; the geometry half
   (a grass mesh) belongs to `bevy_flora`.
4. **The scree is a graded band, not a scatter** — stones of many sizes with turf between them.
   The idea and the place are right; the scatter is not there.
5. **The overcast dome is flat**, with its blocker diagnosed in iteration 28 (the `sky_gain` leak
   at grazing); it carries its Eddington implementation and its measurements in `/tmp/i28`.

## Iteration 32 — 2026-10-20 — the night sky is blue, not brown: what is behind the atmosphere

**Commits:** `4d79c3f` (the research note `docs/research/night-sky-colour.md`), `b19fd3d` (the
implementation), `870461a` (the test), `9dec61b` (the docs, the changelog and the showcase
image), this log. **Sibling repos:** none. **Not released.**

### Candidates

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | The **night sky is sepia**: at 22 h with no moon its red/green ratio is 1.52, and the crate's own `twilight.md` says "Nothing brown" | 1 (physically wrong) | **this iteration** |
| 2 | A game at night has nothing to light with (no `Environment` query, no hook for a game's own lights) | 3 (API) | carried |
| 3 | The dry patch's own texture (a tint on green blades) | 2 | carried |
| 4 | The scree is a graded band, not a scatter | 2 | carried |
| 5 | The overcast dome is flat | 1 | carried (blocker diagnosed in iteration 28) |

Rank 1 beat the carried items because it is *class 1* — a physically wrong result, and the
crate's **own research note** says the night must be blue, so the crate contradicts itself in the
one domain where it is most specific. It is also in every night frame of the game.

### What changed

- **The clear colour carries the *scotopic* colour divided by the atmosphere's transmittance.**
  The clear colour is what lies *behind* the atmosphere, and the atmosphere is drawn over it with
  its transmittance — measured at (0.904, 0.640, 0.353) at the zenith, which is Rayleigh
  extinction and is *warm*. A neutral clear colour behind a warm-extinguishing atmosphere is a
  brown sky; the fix is to make the clear colour blue enough that what comes through is the
  cool blue-black the rod's eye expects.

### Sources

- **Jensen, H. W., Durand, F., Stark, M. M., Premože, S., Dorsey, J., Shirley, P. (2001)**,
  *A Physically-Based Night Sky Model*, ACM SIGGRAPH 2001, DOI `10.1145/383259.383306` (**in the
  corpus**, stem `10.1145_383259.383306`, read in full). Used: §6.2, "Airglow is the principal
  source of light in the night sky on moonless nights"; §6, the four sources of diffuse night
  light and the sentence that is the reason — "These effects are especially important on
  moonless nights, when a small change in illumination can determine whether an object in the
  scene is visible or invisible"; §7, "**The sky is never completely black**" and the blue shift.
  **Not indexed** (the ONNX out-of-memory error in `corpus.md`).
- **The crate's own `twilight.md`**: the *What great looks like* — "a moonless sky is near-black
  with stars and the horizon's silhouettes still readable against it… **Nothing brown**" — and its
  Model 1, which puts the night sky's radiance behind the atmosphere and lets the transmittance
  do the rest.
- **The crate's own `celestial.md`** and `night-sky-light.md`: the night sky's radiance and the
  *scotopic* colour that iteration 31 put in the *ambient*.
- **The CIE 1951 scotopic luminous efficiency function**, through Jensen §7's blue shift: the
  rod's curve peaks at 507 nm and falls away on both sides, so the airglow's green (557.7 nm) and
  red (630 nm) lines are read as a cool blue-green.

### The critique and the verdicts (`/tmp/i32/critique.md`)

| Round | Finding | Verdict |
|---|---|---|
| 1 | The scotopic colour put *directly* into the clear colour left the sky brown (the zenith's red/green 1.411 → 1.415): the clear colour is behind the atmosphere, and the atmosphere's transmittance is warm, so a blue clear colour comes through less blue. The division is the whole fix | NOT YET |
| 2 | The scotopic colour divided by the measured transmittance: the zenith's red/green 0.783 and blue/green 1.197, the horizon still warmer, the probe's path bluer too, the day and the twilight byte-identical, and the cost nil | **GREAT** |

| Feature | Verdict |
|---|---|
| The night sky's colour | **GREAT** |
| The sky's gradient toward the horizon | **GREAT** |
| The sky probe's path | **GREAT** |
| The day and the twilight | **GREAT** (byte-identical) |
| The cost | **GREAT** (0 %) |
| The zenith's faint warm cast at the very top | NOT YET (cosmetic; the status band's glow) |
| A moonless night's shadows | NOT YET (physics; a game needs its own lights) |

**The change as a whole: GREAT.** The blocker — a sepia night sky, which the crate's own note
says is wrong — is fixed on both paths, the day is untouched, and the cost is nil.

### Measurements

- **The sky is blue.** The `day_night` example at 22 h (2026-07-14, `Clear`, the sun at −17.5° and
  the moon at −14.2°, so neither is up), the zenith's red/green **1.411 → 0.783** and its
  blue/green **0.551 → 1.197**; 30° up 1.742 → 0.993; the low sky 2.119 → 1.626, so the
  horizon is still warmer than the zenith, as the longer path says.
- **Both paths.** The `weather_tour`'s upper sky, which comes from the sky probe's cube rather
  than the clear colour, goes from blue/green 1.179 to **1.266**.
- **The day and the twilight are byte-identical**: the dusk's sky 209.5, 208.5, 212.0 and the
  dawn's 223.8, 206.7, 202.6, before and after.
- **Cost.** `bench_elements`, High, 1920 × 1080, `mountains`, the minimum over the diagnostic
  history, the two builds run back to back in one session with the clocks logged throughout:
  **`main_opaque_pass_3d` 0.147 ms on the baseline build, 0.147 ms on this one** — the fix is one
  `vec3` written per camera per frame.
- **The full gate.** `cargo test` 254, clippy, doc, wasm, the GPU integration suite (22 files),
  the lib GPU modules clouds (10), terrain (8), cover (8) and dust (6), and `the_horizon_sky`
  alone: all pass. (Two cloud tests failed on a first run and passed on the second: the GPU was
  at 100 % with another tenant's model resident, and the `gpu-tenant` lease's eviction recipe
  cleared it. The same two failed on the *baseline* build on that run.)

### The next five items, ranked

1. **A game at night has nothing to light with.** A moonless night is *shadowless* — physics —
   but a game wants a torch, a street lamp, a window, and the crate gives a game nothing to
   place them with: no `Environment` query for the local light, no hook for a game's own lights.
   That is an API gap, and it is the crate's class-3 list.
2. **The dry patch's own texture.** The palette gives the ground a *hue*; a dry patch needs a
   *texture* too — the blades inside it should take the patch's hue rather than the turf's, and
   a dry patch is coarser than green grass. The colour half is in this crate; the geometry half
   (a grass mesh) belongs to `bevy_flora`.
3. **The scree is a graded band, not a scatter** — stones of many sizes with turf between them.
   The idea and the place are right; the scatter is not there.
4. **The overcast dome is flat**, with its blocker diagnosed in iteration 28 (the `sky_gain` leak
   at grazing); it carries its Eddington implementation and its measurements in `/tmp/i28`.
5. **A 25 mm/h storm hides the whole town under an orthographic camera**, carried. The rain's
   *readable* look is also less legible than the physical one (measured in iteration 28: the
   readable streaks carry 32 DN of contrast against the grass where the physical ones carry
   44), which is the defect the town capture keeps showing.

## Iteration 33 — 2026-10-20 — a game lights its own night: the exposure, and what a light has to be to read

**Commits:** `4fb14ea` (the research note `docs/research/game-lights.md`), `e9d4583` (the
implementation with its tests), `6539053` (the `isometric` example), `a430941` (the README, the
changelog and the note's Deviations), `c9d0d37` (the lights off in daylight), this log.
**Sibling repos:** none. **Not released.**

### Candidates

| Rank | Candidate | Class | Decision |
|---|---|---|---|
| 1 | **A game at night has nothing to light with**: the crate meters an exposure and gives a game no way to balance its own light against it | 3 (API) | **this iteration** |
| 2 | The scree is a graded band, not a scatter (stones of many sizes with turf between them) | 2 | carried |
| 3 | The dry patch's own texture (a tint on green blades) | 2 | carried |
| 4 | The overcast dome is flat (`sky_gain` leaks at grazing, diagnosed in iteration 28) | 1 | carried |
| 5 | A 25 mm/h storm hides the whole town under an orthographic camera | 2 | carried |

Rank 1 beat the carried items because it is the one that makes a *game* of the crate work: the
crate simulates a day and a night, a sun and a moon, a sky and a metered exposure, and a game
that puts a character in that world at night has **nothing to light them with**. It is also the
crate's own class-3 list, and the crate's `night-sky-light.md` already names it: "a game at
night wants its own lights (a torch, a street lamp, a window), and the crate gives a game
nothing to place them with". The carried items are a beauty gain on a detail at the boundary
of two surfaces, against an API gap that stops a game shipping its night.

### What changed

- **`ElementsExposure`**, a resource holding the camera's EV100 and its exposure factor,
  refreshed every frame. The celestial plugin writes the *followed* camera's exposure there,
  whether the crate metered it or the game drove it (`ElementsCamera::auto_exposure` off).
- **`Environment::exposure()`, `display_value(luminance)`, `luminance_for_display(display)`,
  `light_intensity(luminance, distance, albedo)`, `light_illuminance(intensity, distance)` and
  `night_light_colour()`**, and the same functions in the new `light` module, flat from the
  crate's root.
- **The lightning's own `display_value` is now that module's**, so there is one path for the
  feature and no duplicate constant.

**The shape of it.** A scene-linear value of **0.18 is 18 % grey, the middle of the picture**,
and the crate's metering is built so that an 18 % grey surface reads at 0.18 *at every hour* —
because the exposure is applied inside Bevy's PBR shader and the tonemapper after it, so the
number the crate names is the tonemapper's *input*. The power a light needs scales
*inversely* with the exposure: the same lamp wants **62 253 lm at noon and 2.8 lm at
midnight** for the same picture, so a game that hard-codes a lamp's lumens is wrong by five
orders of magnitude between the two, and a game that asks the crate is never wrong.

### Sources

- **Patry, J. (2021)**, *Real-time samurai cinema*, SIGGRAPH *Advances* (corpus
  `s2021-advances-pdf-66-mb`, the "Tonemapping | Exposure" section, lines 2990–3060, and the
  low-light vision section, lines 3855–3900). Used: "Lights are authored using **physical
  units**"; "we also switched to a **primarily illuminance-based exposure system**, which takes
  albedo out of the equation altogether", with luminance used "only if the highlight luminance
  exceeds a threshold" — "fire particles, bright specular highlights"; the *feedback loop* they
  had before ("the feedback loop between luminance-based exposure and albedo… resulted in
  albedo values that were too dark"), which the crate does not have, and which is why **a game's
  own light does not move the camera's exposure** — the stable behaviour, and the reason the
  gap is not an exposure bug; and the low-light section's **Purkinje shift** ("low-light vision
  is not 'black & white'… a color shift introduced by the rods as they take over"), which is
  why a game's light at night should be the crate's *scotopic* colour and not a warm one.
  **Not indexed** (the ONNX out-of-memory error in `corpus.md`).
- **Bevy 0.20's source** (`$R` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`):
  `$R/bevy_camera-0.20.0/src/camera.rs:275` — `Exposure::exposure()` is `2^(−ev100) / 1.2`;
  `$R/bevy_pbr-0.20.0/src/render/light.rs:574` and `:711` — a light's power is **luminous power
  in lumens**, mapped to `Φ / 4π` for a point light *and* for a spot light ("we choose to use
  the same 4\*PI divisor in both cases so that toggling between point light and spot light
  keeps lit areas lit equally"), where Filament would use `π` for a spot; `$R/bevy_pbr-0.20.0/
  src/render/pbr_lighting.wesl` — `getDistanceAttenuation` (`1/d²`) and the spot's
  `spot_scale`/`spot_offset` (the cone's attenuation is 1 inside the inner angle and 0 at the
  outer); `$R/bevy_pbr-0.20.0/src/render/pbr_functions.wesl:850` — the exposure is applied
  *inside* the PBR shader, before the tonemapper; `$R/bevy_light-0.20.0/src/spot_light.rs:152` —
  Bevy's own calibration, "1,000,000 lumens is a very large 'cinema light' capable of
  registering brightly at Bevy's default 'very overcast day' exposure level".
- **The crate's own `celestial.md`** — the metering model (`ev100_for`, `exposure_for_adaptation`,
  `scene_illuminance`) — and **`night-sky-light.md`** — the illuminance floor and the *scotopic*
  colour.
- **Jensen et al. 2001** (through the crate's own `night-sky-colour.md`) — the blue shift and
  the scotopic curve.

### The critique and the verdicts (`/tmp/i33/critique.md`)

| Round | Finding | Verdict |
|---|---|---|
| 1 | The API was right and the *example* was not: the lamp's pool was a 1 m blob, the lamp's head blew out to 255 DN (it was lit by its own light 0.15 m away), and the paving's 10 m slab cut the pool with a hard edge | NOT YET |
| 2 | A `SpotLight` for the hand-held light, the head *emissive* rather than lit, the lamp at the square's centre and the paving at 14 m: the frame's contrast **2.44 → 13.97 DN**, the head not blown, the day untouched | NOT YET |
| 3 | The head's radiance as a scene-linear value (8.0, a step above the pool's 0.30) and the lamp's light off in daylight: the contrast **14.18 DN**, the day's maximum 255 → 232 DN, the noon capture's mean within 0.5 DN of the baseline's | **GREAT** |

| Feature | Verdict |
|---|---|
| The exposure a game reads | **GREAT** |
| The light's power, in lumens | **GREAT** |
| The night colour | **GREAT** |
| The town's night, at full frame | **GREAT** (2.44 → 14.18 DN) |
| The town's night, at 350 % | **GREAT** (the head, the falloff on the walls, the four shadows) |
| The day | **GREAT** (within 0.5 DN of the baseline) |
| The cost | **GREAT** (no GPU work at all) |
| The lamp's head is a cube | NOT YET (cosmetic; the example's set dressing) |
| The roofs' near-black tops | NOT YET (physics; the lamp is under the roof line) |

**The change as a whole: GREAT.** The API gap is closed in one path, the units are checked
against Bevy's own calibration, the promise is proved in a test that runs outside the crate, and
the demonstration in the example takes the town's night from a flat wash to a lit place.

### Measurements

- **The night reads.** The isometric town at 22 h (2026-04-15, `Clear`, zoom 45 m, pitch 35°):
  the frame's standard deviation **2.44 → 14.18 DN**, its maximum 49.0 → 176.7 DN, and the
  four houses' shadows now read.
- **The same light reads the same at every hour.** The scene-linear value the pool reads at is
  **0.06000 at 12 h, 22 h and 2 h** — to five figures — while the *picture* differs, because
  the tonemapper compresses the daylight scene, which is right: a torch does not read at noon.
- **The day is untouched.** The noon capture's mean is (143.12, 151.39, 130.84) against the
  baseline's (142.62, 150.86, 130.20) — 0.5 DN in each channel, within the GPU's
  non-determinism — and the standard deviation within 0.03.
- **The metering, measured through the crate's own `ev100_for`:** the moonless night
  (0.0005 lx) is EV100 −4.75 and the noon sun (10 000 lx) EV100 9.75; an 18 % grey surface reads
  at 0.18 at both.
- **Cost.** `bench_elements`, High, 1920 × 1080, the minimum over the diagnostic history, the
  two builds run **back to back in one session** so the card's load is the same for both:
  round 1 (the card idle) the baseline's total **1.901 ms** and this one's **1.925 ms**; round
  2 (the card loaded by another tenant) the baseline's **11.294 ms** and this one's
  **11.188 ms**. Every pass is within the noise (`elements_clouds_march` 0.561 → 0.549 and
  1.369 → 1.343; `elements_wind` 0.166 → 0.155 and 0.395 → 0.366; `main_opaque_pass_3d` 0.052
  → 0.052 and 0.207 → 0.204). The change is arithmetic on a number the crate already computes,
  so there is no new pass, no new buffer and no new binding.
- **The full gate.** `cargo test` 260 plus the 4 new public-API tests, clippy, doc, wasm, the
  GPU integration suite (22 files), the lib GPU modules clouds (10), terrain (8), cover (8) and
  dust (6), and `the_horizon_sky` alone: all pass.

### The next five items, ranked

1. **The scree is a graded band, not a scatter** — the grass meets the stone in a smooth wash
   where it should be stones of many sizes with turf between them. The idea and the place are
   right; the scatter is not there. The natural next piece of work on this material, and the
   rock's grain needs a *cell* structure for it (a Worley network of *stones*, not of walls).
2. **The dry patch's own texture.** The palette gives the ground a *hue*; a dry patch needs a
   *texture* too — the blades inside it should take the patch's hue rather than the turf's, and
   a dry patch is coarser than green grass. The colour half is in this crate; the geometry half
   (a grass mesh) belongs to `bevy_flora`.
3. **The overcast dome is flat**, with its blocker diagnosed in iteration 28 (the `sky_gain` leak
   at grazing); it carries its Eddington implementation and its measurements in `/tmp/i28`.
4. **A 25 mm/h storm hides the whole town under an orthographic camera**, carried. The rain's
   *readable* look is also less legible than the physical one (measured in iteration 28: the
   readable streaks carry 32 DN of contrast against the grass where the physical ones carry
   44), which is the defect the town capture keeps showing.
5. **A game's own light has no shadow of its own to fall on the crate's weather.** A lamp lights
   the town, but the crate's rain and snow are lit by the crate's lights alone, so a lamp in a
   downpour lights the *ground* and not the *rain*. A hook for a game's own lights in the
   precipitation and cloud passes would close it; the ownership map keeps the geometry in
   `bevy_flora`, but the lighting hook is this crate's.
