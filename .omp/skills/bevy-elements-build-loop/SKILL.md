---
name: bevy-elements-build-loop
description: "Run one iteration of the bevy_elements improvement loop: capture every feature from many angles, inspect and judge the images yourself as a hostile art director, pick the single most valuable improvement for a game developer, research it in home-still, implement it, and ship it only on a written critique, a GREAT verdict and a clean benchmark. Commits and pushes; never bumps the version or publishes. Use when asked to iterate, advance, or build out bevy_elements. Run from the repo root."
---

One run of this skill is one iteration of a long-running loop on bevy_elements, a Bevy 0.20 crate for living environments (sky, weather, terrain, water, sand, snow) that ships in a real game. Do ONE improvement per iteration, the single most valuable one from a game developer's point of view, finish it completely, then stop.

This crate must be performant AND beautiful. Both are gates: a feature that is fast but merely "fine" looking is not done, and a gorgeous feature that blows the frame budget is not done.

You do all the work yourself, including looking at the images and judging them. Do not delegate visual review.

Extra instructions for this run (a focus such as "clouds only") come from the invoking prompt: `/elements-loop <focus>` or the text after the skill name. They override the choice of item in section 4, not the gates.

State lives in `docs/loop-log.md`; rules and traps live in `AGENTS.md`. Follow them; do not re-derive them.

## 1. Orient

1. `AGENTS.md`: rules, Bevy 0.20 traps, gates.
2. `docs/loop-log.md` (create it if missing): what earlier iterations did, found and deferred, and the newest iteration's ranked backlog. The next iteration number is the last one plus one.
3. `git log` since the last logged iteration.
4. The `docs/research/<system>.md` note of any system you may touch, and `docs/research/corpus.md` for the reading list per domain.
5. `git status --short`. The driver (`scripts/bevy-build-loop.sh`) starts a pass only on `main` with no tracked changes. If you find tracked changes you did not make, stop and log them; do not commit or revert them.

Preflight home-still now, so research is not blocked later:

- `system_status` (MCP) or `hs status`. If distill is down, run `hs restart` once; if it stays down, skip `distill_search` and `distill_index` and list each unindexed note under `Pending index`.
- State on 2026-10-16: distill (192.168.1.110:7434) is healthy, but `distill_index` fails on new documents with an ONNX out-of-memory error (about 2 GB). Record affected stems under `Pending index`; do not retry in a loop.
- Scribe: 192.168.1.110:7435 is unavailable; bmb.local:7433 is healthy with no free slots. `scribe_convert` may queue or fail; then rely on the corpus.
- Corpus: `~/.cache/bevy_elements_corpus/markdown`. If it is missing, sync it per `docs/research/corpus.md`.

## 2. Capture the current state

Build first (`cargo build --release --examples --all-features`; WESL shaders are embedded at build time, so a stale build silently captures the old shader). Keep everything under `/tmp/iN/` (`N` the iteration), the baseline under `/tmp/iN/before/`. Keep a worktree of the starting commit for later comparisons: `git worktree add /tmp/base HEAD`.

Capture each feature from MULTIPLE angles:

- Cameras: ground level, eye level, elevated, aerial, looking toward and away from the sun, and at least one grazing-light angle (`ELEMENTS_EYE`, `ELEMENTS_LOOK`, `ELEMENTS_AGL`, `ELEMENTS_PITCH`, `ELEMENTS_HEADING`, per example; each example's doc comment lists its knobs).
- A time-of-day sweep (dawn, noon, dusk, night: `ELEMENTS_HOUR`), every relevant weather preset (`ELEMENTS_PRESET`, or `ELEMENTS_WEATHER=0..9` in `terrain_tour`), and at least two seeds (`ELEMENTS_SEED`).
- Accumulated state: `ELEMENTS_GROW_HOURS=N` integrates N game hours of a held preset before the first frame. A held clock (`ELEMENTS_HOUR`) integrates no game time, so without it snow and wetness stay bare.
- Frame sequences or GIFs for anything temporal: cloud accumulation (4/16 frames), rain, lightning, wind, water flow, snow melt (`ELEMENTS_SHOT_DIR`, `ELEMENTS_SHOT_EVERY`, `ELEMENTS_SHOT_FROM`; assemble with `ffmpeg` or `magick`). Stills hide ghosting and smearing. Skip the first ~13 frames of a sequence (pipelines and particle buffers initialise); steady state is ~frame 20.
- Fixed seed, clock, camera and resolution, so before and after are comparable. `ELEMENTS_FRAMES` pins the run at a fixed 1/60 s step. GPU output is not bit-deterministic. If no deterministic capture path exists for what you need, building one IS the top item.

Full-frame command:

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" env ELEMENTS_FRAMES=600 ELEMENTS_SHOT=/tmp/iN/before/x.png \
  ELEMENTS_PRESET=Snow ELEMENTS_HOUR=12 ./target/release/examples/<example>
```

Every capture carries the harness's status band in its top ~56 rows (preset, cover, visibility, wind, sun and moon). Keep it in full frames, where it names the weather state; crop below it when judging horizons. Launch long capture scripts as their own bash call with `timeout: 0`; never poll.

## 3. Inspect closely (you judge the images yourself)

You do all the work yourself, including looking at the images and judging them. Do not delegate visual review. Look hard: read each image with `read` (it decodes PNGs inline) and zoom in on every feature.

- Crop and upscale (nearest-neighbour, 2x-4x) the critical regions, each from several angles: `magick in.png -crop WxH+X+Y +repage -filter point -resize 400% out.png`.
- Use OpenCV or Pillow where useful (not installed system-wide; run scripts with `uv run --no-project --with opencv-python-headless --with pillow python script.py`): crops, side-by-side and difference images, contrast stretches, histograms or gradient plots for banding, edge maps for seams and aliasing, per-frame diffs for flicker and ghosting. Quote pixel values when a number settles it (`magick file.png -crop 1x400+X+Y +repage txt:`).
- Always view at least one actual-resolution full frame, because a crop can pass while the whole looks wrong.
- Cover, at minimum: horizon, sun and moon discs, star field, atmosphere gradient, cloud edges and shadows, light shafts, precipitation against dark and bright backgrounds, water surface, foam and shoreline, terrain blend seams, snow edges, CDLOD transitions, fog against geometry.
- For temporal features, step through frames in sequence order.

Write down concrete findings with locations (file and region `x,y,w,h` or pixel coordinates) and severity (blocker / noticeable / cosmetic), not impressions: banding, seams, popping, flicker, ghosting, aliasing, clipping, wrong colour, flat lighting, unreadable detail. Judge beauty separately from correctness: composition, colour harmony, contrast, depth cues, lighting realism, atmosphere. Keep the findings in `/tmp/iN/findings.md`; they feed section 4 and the log.

## 4. Pick the single most valuable item

Rank candidates from your findings, the loop log's backlog and the extra instructions, always asking what matters most to a game developer who will ship this:

1. Visible artifacts or physically wrong results (banding, seams, popping, flicker, energy blowouts, wrong sky for a date and time).
2. Features that look flat, dull or unconvincing next to what shipping games achieve. Beauty gaps rank here, not last.
3. API friction: missing knobs, confusing setup, missing `Environment` queries a game would want (start from the README's `Gameplay queries` section and `src/lib.rs`; propose a gap, never a duplicate).
4. Missing features a game would expect from this domain.
5. Performance that threatens a frame budget (the README's performance section).

Ownership is a gate: if `AGENTS.md`'s ownership map gives the item to a sibling crate, do not build it here; add a hook instead.

State in one sentence why your pick beats the runner-up. If it is too big, carve off the most valuable self-contained slice. Record the ranked candidate list in the log, not only the winner.

## 5. Research thoroughly with home-still (before any code)

Every feature gets researched, new or improved, with no exceptions.

- Search home-still first and go deep. In the corpus, grep by method phrase (`grep -ril "<phrase>" ~/.cache/bevy_elements_corpus/markdown`; method terms such as `Henyey`, `froxel`, `talus`, not topic words such as `rain`), then `distill_search` (`include_text=false`) and `markdown_read {stem, page}`.
- Cover both academic papers (SIGGRAPH, EGSR, I3D, JCGT) and game-developer literature (GDC and SIGGRAPH course talks, engine and studio technical write-ups; the corpus holds SIGGRAPH *Advances* notes as `s<year>-advances-*`).
- Fill gaps with `paper_search` and `paper_download`, so the library grows; `scribe_convert` only if scribe is healthy. Paywalled PDFs: skill `ingest-dois-via-anna-scidb`. Non-DOI sources: record the URL and read it.
- Research both axes: how to make it physically right, AND how shipping games make it look great and run fast (art direction, tonemapping and grading, temporal reprojection, LOD, GPU cost tricks).
- Write or update `docs/research/<system>.md` BEFORE implementing, with the headings `AGENTS.md` requires (`## Sources`, `## Model adopted`, `## Parameters`, `## GPU mapping`, `## Validation targets`, `## Deviations from this plan`, `## Pending index`) plus `## What great looks like`: the visual qualities and reference techniques you are aiming for. `Validation targets` gives one checkable target per inventory element: what it must look like at 1x, in close-up, and from which angle.
- Any changed default goes under `Deviations` with a citation. No uncited default changes.
- Commit the note alone before writing code: `git add docs/research/<system>.md && git commit -m "Research: <system>"`.

## 6. Implement within the crate's rules

- Keep every rule in `AGENTS.md`. In particular: no game vocabulary in `src/` (`tests/leaf.rs` enforces it); no `unwrap()`/`expect()` outside tests; settings are `Resource + Default + Clone + Debug + Reflect` added with `init_resource`; messages use `add_message`; everything is re-exported flat from the crate root; systems go in the existing chained sets; one path per feature, no fallbacks or shims; delete the code the change replaces.
- WebGPU limits: storage textures write-only, at most 4 per stage, ping-pong pairs, state in storage buffers. The wasm32 build must pass.
- Sibling crates (bevy_flora, isomesh and others) are coupled by data only. A needed sibling change lands in that repo first, with its own tests and commit, and is named in the log.
- A change to a seeded system (astronomy, weather scheduler, lightning) that moves a `the_<thing>_is_frozen` golden needs a cited reason. Never regenerate a golden just to pass.
- Keep public API changes backward compatible where reasonable; record breaks in the CHANGELOG with a migration note.
- Tests: for a bug, a test that fails before and passes after. Permanent tests check consumer-visible behaviour, boundaries and invariants; throwaway checks go in `/tmp`.

## 7. Verify and sign off: it must look GREAT

You are grading your own work, so grade it as a hostile art director would.

- Rebuild the examples, re-capture the same scenes, seeds, clocks, cameras and angles as section 2 under `/tmp/iN/after/`, and rebuild the same zoomed evidence set.
- View before and after side by side, every angle, and the temporal sequence frame by frame.
- Write the critique BEFORE the verdict (`/tmp/iN/critique.md`): list every flaw you can find in the "after", at zoom and at full frame, and compare against the research note's `What great looks like` and `Validation targets`. Look for what is wrong, not for confirmation that your change worked.
- Then give each feature exactly one verdict: GREAT, NOT YET or FAIL. GREAT means you would be proud to ship it in a showcase. "Fine", "acceptable" and "good" are NOT YET.
- On NOT YET or FAIL, write the specific changes that would move it to GREAT, apply them, rebuild, re-capture and re-judge. Do not lower the bar to pass.
- Every feature you touched needs GREAT across all angles and the temporal sequence.
- After 5 failed rounds on one feature, stop: log the blocker, the remaining flaws and your best hypothesis, and revert to the last GREAT state rather than shipping something weaker.
- Performance gate, equal in weight to the visual one: benchmark with `bench_elements` and report the minimum over the pass's `Diagnostic::values()`, with `nvidia-smi --query-gpu=pstate,clocks.gr --format=csv,noheader -lms 1000` logged in the same shell. An unloaded GPU under Xvfb reads 5-10x slow, so never compare across machines or loads. Compare against the previous iteration's logged numbers; no regression without a logged, justified trade.
- Run the full gate (the `AGENTS.md` set, which covers features and GPU modules):
  1. `cargo build --release && cargo test && cargo build --examples --all-features`
  2. `cargo clippy --all-targets --all-features -- -D warnings`
  3. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
  4. `cargo build --lib --target wasm32-unknown-unknown`
  5. GPU tests, locally only, one process per file or module: `cargo test --release --tests -- --ignored --test-threads=1 --skip ::gpu_tests::`, then `for m in clouds terrain cover dust; do cargo test --release --lib --features=isomesh -- --ignored --test-threads=1 $m::gpu_tests:: --skip the_horizon_sky; done`, then `the_horizon_sky` alone (eleven clouds GPU tests exhaust the adapter).
- Every suite passed on `main` on 2026-10-17. A GPU failure is checked against `/tmp/base` before blaming your diff.
- Offload gates 1-4 to `sib` or `bmb` only when the local CPU is the bottleneck. Never offload GPU tests or captures.

## 8. Ship

Start only after section 7 passes. Never bump the version, tag or publish; releases are the owner's call.

1. Document: rustdoc for every changed public item, the README for user-visible changes, the research note's `Deviations` and `Pending index`, and an entry under `## [Unreleased]` in `CHANGELOG.md` (`Added`, `Changed`, `Fixed`, `Removed`).
2. Regenerate the GIFs and stills in `docs/` that this change affects, from the new build (`docs/tour.gif` recipe: `examples/weather_tour.rs` doc comment).
3. Log: append `## Iteration N — <date> — <title>` to `docs/loop-log.md`:
   - what changed and why it ranked first (and the runner-up)
   - the sources used (home-still stems or links)
   - your critique and verdict per angle, with evidence filenames
   - benchmark numbers, before and after, with clocks
   - the commit hash, and any sibling-repo commits
   - the top 5 remaining items, ranked, for the next iteration
4. Branch and commit: `git checkout -b loop/iter-N` from `main`. Commit by path with messages that state the user-visible change: the research note (already committed in section 5), the implementation, the docs and changelog, then the log (`Loop log: iteration N, <title>`). `git status --short` must be empty afterwards.
5. Push and merge: `git push -u origin loop/iter-N`, then `git checkout main && git pull --ff-only && git merge --no-ff loop/iter-N -m "Merge loop/iter-N: <title>"`. On a conflict, `git merge --abort`, log it, and stop.
6. Re-run gates 1-4 on the merged tree; a failure stops the pass before the push. Then `git push origin main`.

## Rules

- One improvement per iteration, fully finished: researched, implemented, GREAT-signed-off, benchmarked, gated, committed, documented, logged.
- No feature ships without a written critique, a GREAT verdict and a clean performance check.
- If blocked, log the blocker and pick the next item. A failed gate or a visible defect is not a blocker: fix it. A failed push or merge is: log the exact error and stop. Never force-push.
