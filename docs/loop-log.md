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

**Commit:** (this iteration) · **Sibling repos:** none touched.

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
