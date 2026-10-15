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
