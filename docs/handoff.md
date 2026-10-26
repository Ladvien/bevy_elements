# Handoff — bevy_elements, end of iteration 56

Written 2026-10-26 at the owner's request to wrap up. Everything below is on `main`, pushed, gates
green. **Nothing is half-done in the working tree**: `git status --short` is empty.

## State of the repository

| | |
|---|---|
| Branch | `main`, in sync with `origin/main` at `e33891d` |
| Version | 0.5.3, **not** bumped (releases are the owner's call) |
| Lib tests | 325 passing, 35 ignored (the GPU ones) |
| Gates 1-4 | pass on `bmb` with `cargo +1.98.0` (build, test, examples, clippy `-D warnings`, `cargo doc -D warnings`, wasm32) |
| Gate 5 (GPU) | passed locally at iteration 55; iteration 56 changed no shader or GPU kernel |
| Compilation | all offloaded to **bmb** (`rsync -az --delete --exclude target/ --exclude tools/target/ --exclude .git/ ./ bmb:bevy_elements_iNN/`, then `cargo +1.98.0 …`). Local CPU is usually at load 17/24 from other tenants; **sib** was unreachable (192.168.1.114:22). |

## What shipped this session

1. **Iteration 55 — roads and paths** (`d02e835`). `PathNetwork` / `Path` / `PathKind` /
   `PathsPlugin`; the network is rasterised off-frame into an `rgba8unorm` coverage distance field
   over the terrain (its alpha a per-texel grain), which the terrain material draws: worn bare
   ground with the route's own surface, a dish cross-section, snow cleared where traffic runs,
   wheel ruts, and a route coloured by the ground it crosses. Cost within ±2 % of a scene with no
   network. README section, `docs/paths.png`, `docs/research/paths.md`.
2. **Iteration 56 — path surface queries** (`1c9a8e2`, plus `079b473`/`e33891d`). `Environment::
   surface_at`, `nearest_path`, `on_path` with the `PathSurface` / `PathHit` types; exact and
   current (CPU over the game's own polylines), sharing the shader's coverage and rut curves so the
   answer and the picture cannot disagree. 1000 queries over a 40-route village: 1.04 ms.

Both are documented in the README, the CHANGELOG's `## [Unreleased]`, and their research notes.

## Things a future pass must know (learned the hard way)

- **A signed distance field's coverage runs inward from the edge.** `smoothstep(0, fade, -distance)`
  is *positive outside* a route: the surface is transparent in the middle and opaque outside, which
  looks like nothing rendered. This cost iterations 52-53 two full passes and ten rounds of
  instrumentation. Correct: `smoothstep(0, -fade, distance)`. A CPU mirror of a shader formula
  repeats the shader's own error — only a **GPU readout** of the sampled value settles it.
- **A `MaterialExtension`'s CPU-filled `Option<Handle<Image>>` must be `Rgba8Unorm`.** `Rgba16Float`
  and `Rgba32Float` read 0.0 at every `uv` *and* at the texture's centre, in every usage config,
  even with a forced constant field. The compute-written trail field is `rgba16float` but uses
  `STORAGE_BINDING` — that is the difference.
- **`descriptor.fragment` is `Some` in the forward pipeline and `None` in the prepass** when
  `UserSpecialize` runs, so a `let Some(fragment)` guard silently skips the prepass only. And a
  runtime env check inside `specialize` cannot change an already-compiled pipeline: a diagnostic
  shader def must be part of the `Material`'s `BindGroupData` key. A def's presence is best proven
  by planting a deliberate type error inside its block.
- **A capture that shows a feature absent is checked against the *verified* camera first.**
  Iteration 56 nearly logged the shipped paths as a regression; the road simply ran behind the
  camera I had chosen. The verified camera for the `trails` example is
  `ELEMENTS_EYE=0,8,-22 ELEMENTS_LOOK=0,0` (no `ELEMENTS_AGL`).
- **Never assert wall clock in a test suite.** My query-cost test passed on bmb (1.04 ms) and failed
  locally (1.62 ms) against a 1.6 ms bound. Report the measurement in the log; assert only a
  catastrophic ceiling.
- **Do not run `cargo fmt` crate-wide** (several files are not rustfmt-clean). Format only edited
  files with `rustfmt --edition 2024 <file>` and check the diff.
- **`git checkout -- <file>` also reverts your own uncommitted edits in that file.** Put throwaway
  tests in a separate file, or commit first.

All of the above are in `AGENTS.md`'s trap list too.

## Backlog, ranked (from iteration 56's log)

1. **Shadow bands on the rock under an orthographic camera** (found in iteration 51, pre-existing):
   dark bands 5-12 levels deep across the mountain rock in the isometric view, diagonal and stronger
   at 9 h, absent at heading 135°. Evidence: `/tmp/i51/bands.png`, `/tmp/i51/bands_h9.png` (may be
   gone — `/tmp` is a tmpfs and old iteration directories get cleaned; re-capture with
   `terrain_tour ELEMENTS_ORTHO=18 ELEMENTS_PRESET=mountains ELEMENTS_LOOK=45,0,52 ELEMENTS_HOUR=9`).
2. **Scattered rocks under an orthographic camera**: none are drawn; their `VisibilityRange` and
   fade scale count from the eye 457 m back, not from the focus.
3. **The weather's precipitation climatology** (iteration 49): the temperate preset rains 2-13× an
   observed wet station's totals on ~1.5× its wet days.
4. **Stylistic variants of the ground** (the owner's ask): golem-generated texture sets in other
   styles, each taking its own colour through `with_colour`, with a capture tour.
5. **Textures on the rock**: a triplanar PBR detail over the procedural rock material, settable like
   the ground's.

## Where the evidence lives

`/tmp` is a 16 GB tmpfs and is **not durable**. The iteration directories (`/tmp/i51` … `/tmp/i56`)
hold the captures, critiques and logs referenced by `docs/loop-log.md`; they will vanish on reboot
or when the tmpfs fills. The durable record is `docs/loop-log.md` (what was done, why, the numbers,
the verdicts) and the research notes in `docs/research/`. If a claim in the log needs re-checking,
re-capture with the commands in the relevant example's doc comment rather than trusting a path under
`/tmp`.

## How to start the next pass

```sh
cd ~/bevy_elements
git status --short          # must be empty, on main
tail -n 120 docs/loop-log.md  # the newest iteration's findings and backlog
cargo build --release --examples --all-features   # on bmb if the local CPU is loaded
```
