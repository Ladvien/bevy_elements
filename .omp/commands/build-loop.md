---
description: One iteration of the bevy_elements improvement loop
argument-hint: "[optional focus, e.g. 'clouds only']"
---

One run of this command is one iteration of a long-running loop on bevy_elements, a Bevy 0.20 crate for living environments (sky, weather, terrain, water, sand, snow) that ships in a real game. Do ONE improvement per iteration, the single most valuable one from a game developer's point of view, finish it completely, then stop.

This crate must be performant AND beautiful. Both are gates: a feature that is fast but merely "fine" looking is not done, and a gorgeous feature that blows the frame budget is not done.

You do all the work yourself, including looking at the images and judging them. Do not delegate visual review.

Extra instructions for this run (a focus such as "clouds only"): $ARGUMENTS
They override the choice of item in section 4, not the gates.

State lives in `docs/loop-log.md`; rules and traps live in `AGENTS.md`. Follow them; do not re-derive them.

## Your work is saved even if you are killed — read this before you start

**The driver saves whatever you leave uncommitted, without touching your branch.** Every ten minutes
it records the whole tree to a snapshot ref (`refs/wiggum/pass-N-...`), and again when the pass ends.
Your `git status`, your index and your branch are unchanged — so this is not a substitute for your own
commit step, which is still the only thing that puts work on `main`.
Recover a killed pass's tree with `git -C <repo> log --all --oneline --grep='WIP(pass'` then
`git show <ref>`.

1. **Commit as you go, not once at the end.** Your commit step is last, behind implement, gates and
   benchmark, so a pass killed at the ceiling never reaches it. Commit the research note alone before
   writing code; then the implementation; then the docs and log. Each commit that lands is work a
   killed pass cannot lose.
2. **A snapshot ref is not a commit on your branch.** `git log` will not show it. Do not look for
   one; if an earlier pass was killed, its tree is in `refs/wiggum/` and nobody's gate ran on it.
3. **Parked work is a branch, and you look for it before you pick an item.** When a pass is stopped or
   killed with its tree uncommitted, the driver's next start commits that tree to a branch
   (`wiggum/parked-pass-N-<time>`, or on the pass's own `loop/iter-N`) in a commit titled
   `WIP(pass N, parked)` and puts the clone back on `main`: you start clean and nobody tells you. So run
   `git branch --list 'wiggum/parked-*'` and `git log --all --oneline -n 5 --grep='parked'` once, after
   you have triaged the filed issues and before you commit to an item. If the newest parked tree is for
   the item you would pick, or for an issue or bug that outranks your pick, **adopt it**: it is a first
   draft that was never built, tested or gated and was cut off mid-work, so read it, bring its files into
   your own `loop/iter-N` branch (`git checkout <parked-branch> -- <paths>` or `git cherry-pick`),
   finish it, and run the real gates on the result before it ships under your name. Never merge the
   parked branch itself and never delete it. If it is stale (its files conflict with `main`, or the item
   was fixed meanwhile), say so in one line in the log and ignore it. A parked branch that exists only on
   this clone holds a lot of data: the driver keeps anything over 10 MB of new data off origin.

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

Build first (`cargo build --release --examples --all-features`; WESL shaders are embedded at build time, so a stale build silently captures the old shader). Keep everything under `/tmp/iN/` (`N` the iteration), the baseline under `/tmp/iN/before/`. Keep a worktree of the starting commit for later comparisons, **detached and at a commit**: `git worktree add --detach ~/iN_base <sha>`, built with `CARGO_TARGET_DIR=~/iN_base-target`. Section 9 says how to name, build and remove that baseline.

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

## Owner input: recommend, record, proceed

**This loop runs fully automated (owner, 2026-10-29): never stop to wait for the owner.** When a pass needs the owner's input (a design choice the issue or item does not settle, such as a colour, a coverage, an API shape, a schema change or which of two behaviours; or a protected file that must change), it answers the question itself:

1. **Recommend.** Name the two or three real options and pick one, with one line of why. Prefer the conservative choice: reversible, conventional, consistent with the existing code and the issue's stated intent, and the smallest that meets the acceptance.
2. **Assume the owner chose it, and build it this pass.** A decision is never a reason to skip an item or end a pass.
3. **Record it where the owner reviews.** On the issue, a comment beginning `Assumed decision (owner may override):` with the options, the pick and why, and the label `assumed-decision` (`gh label create assumed-decision --color FBCA04 2>/dev/null; gh issue edit <N> --add-label assumed-decision`). With no issue, in the commit body and the log line, beginning `Assumed decision:`.
4. **The owner's reply wins.** An owner comment that contradicts an assumed decision overrides it: the next pass that reads it reworks the item to the owner's choice before anything else.
5. **Earlier blocks are lifted.** An issue or backlog item logged as blocked on, or waiting for, an owner decision is workable now: decide it this way.
6. **Protected checks change only additively.** An assumed decision may add to a protected test or check (a new row or entry, a new field's empty default in a literal, a count or version raised by exactly what the change adds), with the commit message `assumed owner exception: <what>`. It never removes, loosens, skips or re-pins an existing assertion, golden, eval answer or threshold; that is the one block left, so say so on the issue and go on.
7. **Never, whatever you assume.** These need the owner's word in a conversation, and no prompt can give it: yanking a release or changing crate ownership; force-pushing, rewriting history or deleting branches; deleting files or work this pass did not create; editing this command file; spending money or sharing credentials.

## Filed issues come before new features

**A filed issue is a consumer telling you what broke, which outranks anything this pass invented.**
Before you pick an item, list the repository's open issues and triage them:

```sh
gh issue list --repo Ladvien/bevy_elements --state open --limit 100 \
  --json number,title,labels,createdAt --jq '.[] | "#\(.number) \(.title)"'
```

If `gh` is absent (it is on big_mac), run this instead. It needs no login and prints the same list:

```sh
curl -s "https://api.github.com/repos/Ladvien/bevy_elements/issues?state=open&per_page=100" \
  | python3 -c 'import json,sys; [print("#%s %s" % (i["number"], i["title"])) for i in json.load(sys.stdin) if "pull_request" not in i]'
```

A missing `gh` is never a reason to skip this step. If the command errors or prints a Python traceback, that is the rate-limit case below.

Then, in this order:

1. **A bug issue is the pass's item, ahead of any enhancement.** Anything reporting wrong behaviour —
   a crash, a panic, a visual artifact, a value that is implausible, a documented promise that is not
   kept — is a bug. Take the oldest such issue that is still reproducible, and say why it beats the
   others.
2. **An issue labelled `consumer` is the pass's item next, ahead of the series and your own candidates.**
   A project built on this crate needs it and is waiting. The `consumer:<name>` label says which project;
   `consumer:bevy_emerge` is the owner's editor. Its own work resumes when the issue is closed as completed
   and it moves its git pin to a `main` commit containing the fix. Take the oldest one you can work this pass.
   - **Build the general form.** The requester is the example, never a name in this crate's API, a path or a
     default: this crate serves other projects too. Where an issue asks for something shaped to one project,
     build the general form and say so on the issue.
   - **Prove it here**, with a test in this repository that fails before and passes after against the issue's
     acceptance. Never build or run the requesting project to prove it.
   - **Pushing to `main` is the delivery.** Requesters pin a git revision, so a consumer fix never needs a
     publish or a version bump.
   - **One that needs a decision the issue does not settle is decided, not skipped:** recommend, record and
     build it (see "Owner input: recommend, record, proceed"). Only a change that would weaken a protected
     check cannot be worked: say exactly what is needed on the issue (`gh issue comment`), write one log
     line, and go on. A blocked consumer issue never stops the pass.
   - **Read `git log` first.** While this loop is stopped, the requester's loop may fix the issue here at the
     source, in its own commits with tests; check whether that has happened so the work is not done twice.
3. **Every other open enhancement issue is the pass's item next, ahead of your own candidates** (the
   owner's rule, 2026-10-29): a filed request was asked for; a candidate this pass invents was not. The
   order of work is: **bugs first** (a bug issue, then a bug already recorded in the backlog or the log),
   then `consumer` issues, then every other open enhancement issue, oldest first. Only when each of them
   is done, blocked or a release issue does "Pick the single most valuable item" choose a feature of your
   own. A blocked issue gets a `gh issue comment` saying exactly what it needs and is skipped; it never
   stops the pass. Name the issues you skipped, and why, in the log line, so the next pass sees the list
   was worked through.
4. **Close the loop.** An issue your pass fixes is referenced in the commit body (`Fixes #N`) **and**
   in the report, so the next pass can see it was addressed. Do not close an issue you did not fix,
   and do not close one whose fix this pass did not verify. For a `consumer` issue the close is the signal
   the requester waits for: close it only through a verified fix pushed to `main`, never as not planned
   without the owner's word.
5. **A release issue is done by the next release.** An issue asking to publish or bump a version is closed by the pass whose release (the "Release" block in the ship section) publishes it, with the crates.io URL.

**When the API fallback answers `403` or `429`** (rate limit — 60 requests/hour per IP unauthenticated,
shared by everything on this network), do not treat it as "no issues": write one line saying the list
could not be read and continue. Never spend a second call to retry it in the same pass, and never infer
from silence that nothing is filed.

**A rate limit you have not hit is not a rate limit.** Make the call in every pass. It is one request; the driver sleeps at least 60 seconds between passes and backs off when passes find nothing, so this loop makes a few requests an hour against a limit of 60. Several idle passes of the sibling loop on 2026-10-27 logged "issues not re-read (rate-limit risk)" without the call ever having answered `403` or `429`, which leaves a filed bug unseen. Only an answer you actually received lets you write "could not be read".

**An empty issue list costs nothing:** if it is empty or `gh` is unreachable, write one line saying so
and continue — never spend a pass on the lookup.

## 4. Pick the single most valuable item

**You are here only when no bug and no open issue can be worked this pass** (see "Filed issues come before new features"); otherwise that issue is the item. Rank candidates from your findings, the loop log's backlog and the extra instructions, always asking what matters most to a game developer who will ship this:

1. Visible artifacts or physically wrong results (banding, seams, popping, flicker, energy blowouts, wrong sky for a date and time).
2. Features that look flat, dull or unconvincing next to what shipping games achieve. Beauty gaps rank here, not last.
3. API friction: missing knobs, confusing setup, missing `Environment` queries a game would want (start from the README's `Gameplay queries` section and `src/lib.rs`; propose a gap, never a duplicate).
4. Missing features a game would expect from this domain.
5. Performance that threatens a frame budget (the README's performance section).

Ownership is a gate: if `AGENTS.md`'s ownership map gives the item to a sibling crate, do not build it here; add a hook instead.

State in one sentence why your pick beats the runner-up. If it is too big, carve off the most valuable self-contained slice. Record the ranked candidate list in the log, not only the winner.

**A pass never ends idle** (owner, 2026-10-29: "If they're not addressing bugs or enhancing features, they need to evolve their own code base to support Bevy Emerge better or add new features. Research these things in home still, implement them. They shouldn't stay idle."). An empty issue list or a stale backlog is this step's input, never a reason to ship nothing, and a candidate is not "speculation" once it is researched. When the ranked list above gives no workable item, take the first of these that applies:
1. **What the owner's editor needs from this crate.** bevy_emerge configures this crate live from its own panels: the crate's own serde types are the data, saved in a game's project and edited live (its `AGENTS.md`, "Use the ladvien crate family"). Read its issues for this crate (`gh issue view <N> --repo Ladvien/bevy_emerge` for #98 sky and weather, #99 terrain, #100 water) and how the editor uses the crate today (a shallow clone of `Ladvien/bevy_emerge` under `/tmp`, removed afterwards; `ElementsSettings` and `apply_environment` landed there on 2026-10-29). Build the crate-side piece that makes that integration easier or richer, in this crate's general form and never named for the editor: settings that serialise and change at runtime, validation with a useful error, presets, a query or event the editor can show, a preview it can drive. Prove it with a test here.
2. **A feature a shipping game in this domain expects and this crate lacks**, found by research: home-still first (`distill_search`; for a paper the corpus lacks, `paper_search`, `paper_download`, `scribe_convert`, then `distill_index`), then shipped-game practice, then section 5 before any code.
3. **Depth on the weakest existing feature**: measure it against its research note's validation targets and improve the worst gap.

Record the ranked candidates and the pick in the log. A pass that ships nothing names the concrete blocker that stopped all three; "nothing new to do" is not a blocker. **"Not researched yet" is not a blocker either: it is this pass's work.** Take the top item, do section 5 for it now, commit the note, and build its first slice. "Building it would be speculation" means the research is missing, so do the research. A log-only iteration is a failed pass, not an outcome, and a pass never copies an earlier iteration's reasons for shipping nothing.

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
- Every suite passed on `main` on 2026-10-17. A GPU failure is checked against the baseline worktree (`~/iN_base`) before blaming your diff.
- Offload gates 1-4 to `sib` or `bmb` only when the local CPU is the bottleneck. Never offload GPU tests or captures.

## 8. Ship

Start only after section 7 passes. Then release (the "Release" block at the end of this section).

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

**Release, every pass that changed the package** (owner, 2026-10-29: loops publish, and never wait on CI to do it). After the push, from that pushed commit, on a clean tree with the gate green: if a file `cargo package --list` ships changed since the last published version (`git diff --quiet v<last> HEAD -- <those files>` fails; with no tag `v<last>`, diff from the commit that set that version in `Cargo.toml`, found with `git log -S'version = "<last>"' -- Cargo.toml`), release:
1. Decide the version with `cargo semver-checks check-release` (`cargo install cargo-semver-checks --locked` if missing). Pre-1.0: a breaking change bumps MINOR and resets PATCH, anything else bumps PATCH; never 1.0.0. Move `## [Unreleased]` in `CHANGELOG.md` to `## [<version>] - <date>`, bump `Cargo.toml` and `Cargo.lock`, and commit `release: v<version>`.
2. `cargo publish --dry-run`, then `cargo publish`, locally, with the credentials `cargo login` stored; never print them. Never wait for, watch or re-run a GitHub workflow to release.
3. `git tag -a v<version> -m v<version>`, then push the commit and the tag (merge, never force, on a refused push).
4. `cargo info <crate>@<version>` resolves; put the crates.io URL in the log entry.

A failed publish is logged with its error and retried next pass. Never yank, never reuse a version; fix a bad release forward.

### README, demos and docs stay true

A README is read before anything else, so a false line there costs more than a false line anywhere else.
An audit on 2026-10-27 found one in each of the four loop repos: a `tour.gif` captioned "at noon" over a HUD
reading 10.00 h with a running clock; a `ties.gif` captioned for rivalry colours that appear in none of its 240
frames; a hero caption promising a flame catch that its file never shows; and a demo video captioned as the full
recording of the GIF above it when it shows different objects. Before you push a change a user can see:

1. **Put it in the section it belongs to.** Do not append to the introduction or to the end, and do not
   restructure the README in a feature pass. It reads in this order — what it is in one line, the demo, what
   you get, install, quick start, then the detail sections — and an item added in the wrong place undoes it.
2. **A caption describes the file as it exists.** Open the GIF or still (frame count, size, any HUD burned into
   it) and write only what you can read off it; a figure quoted from program output comes from running that
   program now, not from the log. Recipes: `scripts/capture_gifs.sh`, and the `weather_tour` recipe named above.
3. **Media you cannot regenerate here is logged, not left lying.** With no GPU or display on this machine, name
   the stale file and its path in the report and change its caption to what it still shows.
4. **The README is the crate's doctest root** (`#![doc = include_str!("../README.md")]`), so every fenced Rust
   block in it compiles: after any README edit run `cargo test --release --doc`.

## 9. Clean up when you are done

Worktrees and build directories outside the clone are not free: a `target/` for this crate is tens of GB, big's
`/tmp` is RAM, and on 2026-10-27 earlier passes had left 37 GB of baseline builds and 5 GB of scratch behind.
**After you ship and before you write the pass's last log line, whether or not you shipped,** remove what *you*
made this pass, and list what earlier passes left.

1. **A baseline is a pair that can be found.** Make it with `git worktree add --detach <path> <sha>` (never a
   branch name: a worktree with a branch checked out stops anything else checking that branch out) and build it
   into `<path>-target`. Then `git worktree list`, plus the `*-target` directory beside each worktree, is the
   whole inventory.
2. **Remove yours.** `git worktree remove <path>` (plain, never `--force`) and `rm -rf <path>-target`, then your
   scratch under `/tmp` (captures, frames, venvs, gate logs, `rustdoctest*`). Remove only what you created this
   pass, and name each path in the log. If git refuses because the worktree has a change in it, name the file
   in the log and leave it.
3. **Leftovers from earlier passes are listed, never deleted.** A killed pass cannot clean up after itself, but
   an agent does not delete what it did not create, so list them once and let a person approve the removal:

   ```sh
   git worktree list --porcelain | awk '/^worktree /{sub(/^worktree /,""); print}' | tail -n +2   # all but the clone
   du -sh <wt> <wt>-target                       # the size of each one that is not yours
   git -C <wt> status --short                    # empty: nothing is lost by removing it
   find <wt> <wt>-target -mmin -120 -print -quit # empty: untouched for two hours, so no live build is using it
   ```

   Log each with its path, size, and whether it is clean, already on `origin/main` and idle (the likeliest to be
   stale). Say so, and still leave it.
4. **Never remove** the clone itself, its own `target/` (the build cache the next pass starts from), anything you
   did not create this pass, or anything a process is using.
5. Write one log line: what you removed and the space it freed (`df -h .` before and after), and the leftovers
   you listed.

## Rules

- One improvement per iteration, fully finished: researched, implemented, GREAT-signed-off, benchmarked, gated, committed, documented, logged.
- No feature ships without a written critique, a GREAT verdict and a clean performance check.
- If blocked, log the blocker and pick the next item. A failed gate or a visible defect is not a blocker: fix it. A failed push or merge is: log the exact error and stop. Never force-push.
