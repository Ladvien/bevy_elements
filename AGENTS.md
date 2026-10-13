# bevy_elements — notes for agents

Living environments for Bevy: celestial bodies, climate-driven weather, GPU wind, clouds, precipitation, fog, lightning, heightfield terrain with erosion and flooding, and ground cover, simulated in compute shaders. Single crate; each domain is a module with one plugin.

## Version pin

Pinned to **Bevy 0.20.0**. Read the vendored source and migration guides, not bevy.org (which tracks `main`): `~/.cargo/registry/src/index.crates.io-*/bevy-0.20.0/` with `examples/` and `_release-content/migration-guides/`. The verified 0.20 render API paths are in `docs/research/bevy-0.20-render.md`.

## Source of truth

This repository, [`Ladvien/bevy_elements`](https://github.com/Ladvien/bevy_elements), is canonical. MIT OR Apache-2.0. Consumers (Sanctuary at `/Users/ladvien/sanctuary` on the Mac) get fixes here, in their own commits with tests, and never patch a copy.

## Research first

No system is implemented without its `docs/research/<system>.md`, committed first. Protocol:

1. **Search the corpus first.** The library already holds 12 178 converted documents, many under non-DOI stems (SIGGRAPH *Advances* course notes as `s<year>-advances-*`, books as `book_*`). `docs/research/corpus.md` lists what is there per domain and how to sync and search it (`~/.cache/bevy_elements_corpus/markdown`, phrase search, not catalog titles). Every note cites the relevant corpus documents, not only the plan's seeds.
2. Preflight with `hs status` (MCP `system_status`). If distill is down, run `hs restart` once; if it stays down, research continues without `distill_search` and `distill_index` is skipped and listed under the note's `Pending index`.
3. For each seed DOI not in the corpus: `hs paper get --doi <doi>`, then `hs paper download --doi <doi>` (MCP `paper_download`). Paywalled: skill `ingest-dois-via-anna-scidb`. Non-DOI sources: `hs paper search "<title>"`, else record the URL and read it.
4. Convert with `hs scribe convert <pdf> --out <md>` (MCP `scribe_convert {stem}`; stem = lowercased DOI, `/` → `_`). If scribe drops regions, read the PDF's text directly and say so in the note.
5. Read with MCP `markdown_read {stem, page}` or the synced file; index with `distill_index {stem}` when healthy.
6. Write `docs/research/<system>.md` with headings: `## Sources`, `## Model adopted`, `## Parameters`, `## GPU mapping`, `## Validation targets`, `## Deviations from this plan`, `## Pending index`. A default changes only with a cited source recorded under `Deviations`.

## Plays well with the family

1. **Bevy pin parity.** Depend only on crates pinned to the same Bevy minor. Couple to 0.19 crates (`bevy_carnage`, `bevy_light_grid`, `bevy_stigmergy`) only through plain data in game glue, documented in the README `Plays well with` section.
2. **Ownership map.** Foliage, trees, tree wind sway, leaf colour and snow on branches belong to `bevy_flora`. Isosurface and voxel meshing belongs to `isomesh`/`bevy_isomesh`. Blood wetness belongs to `bevy_carnage`. Creature light grids belong to `bevy_light_grid`. WFC and authored world layout belong to `bevy_emerge`. This crate owns sky, celestial bodies, weather, atmosphere-driving, wind fields, precipitation, clouds, fog, lightning, heightfield terrain and its simulation, and ground cover.
3. **Hooks over dependencies.** This crate never makes a sibling depend on it. It feeds siblings through their existing public resources behind an optional feature (`flora`, `isomesh`). Hook data is published as plain resources and messages (`WindState`, `WindGust`, `SeasonState`, `SurfaceWeathering`, `Environment`) that any crate or game can read.
4. **A needed sibling change lands in the sibling's repo**, in its own commit with tests there, and is published before this crate depends on it.
5. **Units are SI** at every boundary (m, s, m/s, °C, lux, mm/h). Wind direction is the unit XZ vector the wind blows *toward*, which matches `FloraWind::direction`.
6. **No game vocabulary** in `src/`, enforced by `tests/leaf.rs`.

## Rules

- **One plugin per domain; split by domain, not by kind.** Settings are `Resource + Default + Clone + Debug + Reflect` added with `init_resource`, so a caller who inserts one first overrides it. Messages use `add_message`. No `prelude`: everything is re-exported flat from the crate root. Plugins are `*Plugin`, system sets `*Systems`.
- **No `unwrap()`/`expect()` outside tests.** One path per feature: no fallbacks, shims or stubs.
- **A missing `Res<T>` panics its system.** Take `Option<Res<T>>` or `init_resource`. Run conditions are all evaluated, with no short-circuit.
- **`Single<..>` silently skips its system on a non-unique match.**
- **GPU determinism.** GPU results are not bit-deterministic. Astronomy, the weather scheduler and lightning are deterministic and seeded; goldens are named `the_<thing>_is_frozen`.
- **WebGPU limits.** Storage textures are write-only `rgba16float`/`rgba8unorm`, at most 4 per stage; ping-pong pairs, no read_write textures. Simulation state lives in storage buffers.
- Close any Bevy app you launch for a test.
- Do not hard-wrap markdown.

## Build and test

```sh
cargo build --release && cargo test && cargo build --examples --all-features
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features
cargo build --lib --target wasm32-unknown-unknown
cargo test -- --ignored   # GPU tests, need a GPU
ELEMENTS_FRAMES=600 ELEMENTS_SHOT=/tmp/x.png cargo run --release --example quickstart
```

`cargo build --release` runs first in CI because the full `bevy` dev-dependency hides holes in the trimmed feature list.
