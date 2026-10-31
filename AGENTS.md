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
2. **Ownership map.** Foliage, trees, tree wind sway, leaf colour and snow on branches belong to `bevy_flora`. Isosurface and voxel meshing belongs to `isomesh`/`bevy_isomesh`. Blood wetness belongs to `bevy_carnage`. Creature light grids belong to `bevy_light_grid`. WFC and authored world layout belong to `bevy_emerge`. This crate owns sky, celestial bodies, weather, atmosphere-driving, wind fields, precipitation, clouds, fog, lightning, heightfield terrain and its simulation, ground cover, and the snow slab over a voxel volume.
3. **Hooks over dependencies.** This crate never makes a sibling depend on it. It feeds siblings through their existing public resources behind an optional feature (`flora`, `isomesh`). Hook data is published as plain resources and messages (`WindState`, `WindGust`, `SeasonState`, `SurfaceWeathering`, `Environment`) that any crate or game can read.
4. **A needed sibling change lands in the sibling's repo**, in its own commit with tests there, and is published before this crate depends on it.
5. **Units are SI** at every boundary (m, s, m/s, °C, lux, mm/h). Wind direction is the unit XZ vector the wind blows *toward*, which matches `FloraWind::direction`.
6. **No game vocabulary** in `src/`, enforced by `tests/leaf.rs`.

## Rules

- **One plugin per domain; split by domain, not by kind.** Settings are `Resource + Default + Clone + Debug + Reflect` added with `init_resource`, so a caller who inserts one first overrides it. Messages use `add_message`. No `prelude`: everything is re-exported flat from the crate root. Plugins are `*Plugin`, system sets `*Systems`.
- **No `unwrap()`/`expect()` outside tests.** One path per feature: no fallbacks, shims or stubs.
- **A missing `Res<T>` panics its system.** Take `Option<Res<T>>` or `init_resource`. Run conditions are all evaluated, with no short-circuit.
- **`Single<..>` silently skips its system on a non-unique match.**
- **More than ten GPU tests in one process exhaust the adapter.** Each test builds a Bevy app and wgpu instance; on the eleventh `request_adapter` returns `None` and the test panics with `GPU_NOT_FOUND` at `bevy_render-0.20.0/src/renderer/mod.rs:308`, while the same test passes alone. `clouds::gpu_tests` has eleven: run the suite with `--skip the_horizon_sky`, then the skipped test on its own.
- **The examples' harness burns a status line into the top ~56 rows** of every capture (a semi-transparent band over the sky). A reviewer may read it as an atmospheric shelf; crop below it when judging horizons, and read it for the weather state at the capture (preset, cover, visibility).
- **GPU determinism.** GPU results are not bit-deterministic. Astronomy, the weather scheduler and lightning are deterministic and seeded; goldens are named `the_<thing>_is_frozen`.
- **WebGPU limits.** Storage textures are write-only `rgba16float`/`rgba8unorm`, at most 4 per stage; ping-pong pairs, no read_write textures. Simulation state lives in storage buffers.
- Close any Bevy app you launch for a test.
- **A baseline worktree that shares the clone's `CARGO_TARGET_DIR` hands the clone the baseline's test binaries.** `cargo test --release --test gpu_rainbow` on the fixed clone ran the binary the baseline worktree had just built (same file, same unit hash, test source older than the binary) and reported the baseline's numbers to the digit, so a GPU test "failed" on code that fixes it (iteration 104). When you share one target to save disk, `touch` the test file after switching trees and check the binary's mtime in `target/release/deps` against the run; a result identical to the baseline's is this until proven otherwise.
- Do not hard-wrap markdown.
- **Do not run `cargo fmt` on the crate.** Several files are not rustfmt-clean (`src/terrain/model/mirror.rs`, `src/celestial/mod.rs`, `examples/isometric.rs` among them), and a crate-wide format restyles code a change never touched. Format the files you edited with `rustfmt --edition 2024 <file>` and check that its diff is only your hunks: rustfmt also formats a file's child modules (`mod common;`, `mod mirror;`).
- **Judge images yourself, as a hostile art director.** Read the captures and zoomed crops directly; write the critique (file, region, severity) before the verdict `GREAT`, `NOT YET` or `FAIL`. Only GREAT ships; "fine", "acceptable" and "good" are NOT YET.

## Bevy 0.20 traps found here

- **A camera needs a render target.** Bevy's atmosphere environment probe unwraps the view uniforms, so an `ElementsCamera` with no window or image target panics the render world. GPU tests render into an `Image` target (`tests/support`).
- **Pipelined rendering moves the render app away.** After `App::finish()` the `RenderApp` sub-app is gone from the `App` (it runs on its own thread). Register render-world systems and resources before `finish()`, and report back through an `Arc<Mutex<…>>` resource. Wait for `PluginsState::Ready` (ticking the task pools) before `finish()`: the renderer is created asynchronously.
- **Pipelines compile asynchronously.** A kernel that is not ready yet skips the frame; anything read back meanwhile is zeros. GPU state is published as live (`WindGpu::live`) only after the first real dispatch. Tests use `RenderPlugin { synchronous_pipeline_compilation: true }`.
- **`FogVolume`s are extracted regardless of `Visibility`.** Switch one off with `density_factor = 0`.
- **`FogVolume` light attenuation uses the bounding-sphere radius.** Every sample's light is attenuated by `exp(−density · R)`: keep volumes small or compensate with `light_intensity` (`fog::volume_light_compensation`).
- **Bevy's atmosphere is lit by every `DirectionalLight`'s `color · illuminance`.** Dimming the sun light for cloud also darkens the sky, the aerial perspective and the environment map; there is no per-light opt-out (light render layers gate only shadow casting), so a fill light would scatter in the sky too. The lights therefore carry the cloud shadow map's *mean* over 8 km (`CloudShadows::mean_transmittance`), never the shadow at the camera, which once turned the sky black under a lone cumulus; surfaces take their own shadow per pixel relative to it, and the cloud composite restores the dome with `sky_gain` (`src/clouds/composite.wesl`, `docs/research/broken-cloud-light.md`). `DirectionalLightTexture` would shadow every `StandardMaterial`, but it needs clustered decals, which Metal and WebGPU lack.
- **`AmbientLight` diffuse is shaded by `F_AB(1, NdotV).x` ≈ 0.452** (`pbr_ambient.wesl:18`): divide the wanted radiance by it. It is uniform over normals, so it cannot shade tops brighter than sides, and its specular term takes the same boost (2.2× the sky in water at grazing). The followed camera's cloud light therefore comes through the environment map (the clouds' sky probe, `docs/research/sky-light.md`), and its ambient is 0 while the probe is live.
- **An `Atmosphere`'s `GlobalTransform` is its planet's centre.** Bevy moves a default one `inner_radius` below the origin when the component is added; a `Transform` on the entity overwrites that with its own translation at the next propagation (identity: the camera inside the planet, a beige sky of radial streaks). The crate's atmosphere carries `plugin::planet_transform`.
- **`GeneratedEnvironmentMapLight` is filtered from its `environment_map` at the start of the next frame** (`downsampling_system`/`filtering_system` run in `Render` before the `Core3d` graph) and re-extracted every frame, so a cube written in `Core3d` can replace the source as long as it keeps the source's size (the filtered maps are sized from the source once).
- **`DistanceFog::color` is added after exposure**, so it is a display-referred value: scale it by the camera's `Exposure::exposure()`.
- **Bevy's default `GlobalAmbientLight` is a constant 80 cd/m²**, which a night exposure lifts to white; `ElementsCamera` zeroes the per-camera `AmbientLight`.
- **Shaders are WESL.** `embedded://bevy_elements/noise.wesl` is importable as `bevy_elements::noise`; shared libraries are loaded with `load_shader_library!`.
- **GPU timings on a lightly loaded NVIDIA card read 5–10× slow** (P8/P5 clocks under Xvfb). Report the minimum over the diagnostic history and log `nvidia-smi` clocks in the same shell. At 1920 × 1080 the card hops between P2, P3 and P5 from run to run, so one interleaved pair can mislead by a factor of two (iteration 45 logged a ground-texture cost of 0.04 ms that was really +67 %); at 3840 × 2160 it stays in P5 but the history's minimum reads `0.000`, so compare the pass's **mean**, base and change interleaved, with the early prepass equal across them as the check that the clocks matched.
- **A branch that never runs still costs, in a long shader.** The terrain's fragment shader is long, and a GPU allocates registers for its peak: code that holds many values at once (three hex taps' texels and three layers' maps) raised the peak and cost every pixel of the pass, textured or not (a ground with no set paid 27 % for code behind a uniform `if` it never entered). Put an optional path behind a shader def keyed on the material (`TerrainExtensionKey`, `GROUND_TEXTURES`), and accumulate into running sums rather than holding samples for a blend at the end (`docs/research/ground-textures.md`, Deviation 19).
- **Screenshots without a display:** `xvfb-run -a -s "-screen 0 1920x1080x24" env ELEMENTS_FRAMES=… ELEMENTS_SHOT=… ./target/release/examples/<name>` renders on the GPU. A capture that panics with `XOpenDisplayFailed` and leaves no image has no X server: `/tmp` (a 16 GB tmpfs on big) may be full, so Xvfb cannot make its socket, or the display `-a` picks (`:99`) may be held by something this namespace cannot see (no lock file, no socket, yet every server there fails). Check `df -h /tmp`, pass `xvfb-run -n <N>` with a fresh number per run, and retry a run that leaves no image (iteration 44 lost two whole baselines to it). Keep worktrees and their `target/` off `/tmp`.
- **A run ended by `timeout` can flood its log.** A `bench_elements` run cut off at its deadline writes `CommandQueue has un-applied commands being dropped` warnings without end: 185 MB to 2.8 GB per log (iterations 50 and 51). Logs in `/tmp` then fill the tmpfs, and the next link there hangs: a doctest's `rust-lld` slept 2.5 h in `cargo test` (iteration 51) and the others failed with "No space left on device". Filter a run's output through `grep -v CommandQueue` before it reaches a file, and check `df -h /tmp` before a gate.
- **A `Camera3d` requires `Msaa` (4 samples by default)**, so an observer's `insert_if_new(Msaa::Off)` never applies; `ElementsCamera` inserts `Msaa::Off` unconditionally. The cloud and precipitation passes bind the depth prepass single-sampled and skip a view that has MSAA.
- **Bevy's `ScatteringMedium::earth` falloffs are fractions of the atmosphere thickness** (100 km): effective scale heights 13.3 km (Rayleigh) and 2 km (Mie), not Hillaire's 8 and 1.2 km. Oracles for the sky or cloud light use `bevy_optical_transmittance` (`src/clouds/model.rs`).
- **`ReadbackOnce` lives for one frame**: Bevy removes it in `First`. Spawn it from a system in `Update` (as `gpu/readback.rs` does), never between `app.update()` calls, or use `Readback` and despawn it (`tests/support`). Bevy's readback pool can return a buffer larger than the one asked for: decode only the length you know.
- **An app with no window is not paced**: only presenting a swap chain makes the CPU wait for the GPU, so a headless app (every GPU test) queues frames without bound and readback latency grows until the snapshot freezes. `gpu/readback.rs` `pace_without_window` waits for each windowless frame's GPU work; a stale `Environment` answer in a GPU test means that wait is gone (`docs/research/readback-pacing.md`).
- **`active` is a reserved word in WGSL**: the shader compiler rejects it as an identifier and the kernel then stays idle (look for the `ERROR bevy_elements::gpu::kernel` line).
- **`meta` (and other WGSL reserved words) in a WESL struct field** fails the shader at runtime with "use of a reserved word", not at build time: the terrain material is dark until you read the log for `failed to process shader`.
- **`patch`, `target` and `filter` are WGSL reserved words too**: a local named `patch` in `water.wesl` or a binding named `target` in a kernel fails the shader at runtime (the water or the kernel stays dark/idle until you read the log).
- **A `MaterialExtension`'s `Option<Handle<Image>>` texture must be `Rgba8Unorm`.** An `rgba16float` CPU-filled image bound as `#[texture(N)] var mask: texture_2d<f32>` read `0.0` at every `uv` **and** at the texture's own centre on the GPU, in `Rgba16Float` and `Rgba32Float`, as `RenderAssetUsages::default()` and `RENDER_WORLD`, and with a forced constant field; the same image as `Rgba8Unorm` (the format of the terrain's own ground-texture array, iteration 50) arrives. The trail field is `rgba16float` but it is compute-written with `STORAGE_BINDING`, which is the difference (iterations 52-54).
- **A signed distance field's coverage runs inward from the edge.** `smoothstep(0, fade, -distance)` is *positive outside* a route, so the surface is transparent in the middle and opaque outside — a faint rim that looks like nothing rendered. Correct: `smoothstep(0, -fade, distance)`. A CPU mirror of a shader formula can be wrong in the same way as the shader: only a readout from the GPU (a debug paint of the sampled value) settles it (iterations 53-54).
- **`descriptor.fragment` is present in the forward pipeline and `None` in the prepass when `UserSpecialize` runs.** A `let Some(fragment) = descriptor.fragment.as_mut()` guard therefore applies a shader def to the forward pass only and silently skips the prepass — do not read the skip as "the def was never pushed". And a runtime environment check inside `specialize` cannot change an already-compiled pipeline: a diagnostic shader def must be part of the `Material`'s `BindGroupData` key (iteration 53).
- **A WESL error is silent at build time and loud only in a capture.** The shaders are embedded
  with `embedded_asset!`, so a struct constructed with the wrong number of fields, an unimported
  name, a shadowing local or a `pow(x, p)` with `p` in a `let` all *build*, are embedded, and
  then fail in the GPU driver: the surface renders as a flat dark grey and the only clue is a
  `failed to process shader` line in the run's log, which a capture script throws away. **Check
  the shaders before capturing:**
  ```sh
  cargo run --release --manifest-path tools/wesl-check/Cargo.toml -- src
  ```
  It compiles every one of the crate's modules to WGSL, keeping each module's own declarations,
  and parses the result with naga, in about twenty seconds and without a GPU, and it names the
  line. (Before iteration 45 it stripped a library module to an empty string, and "the WGSL is
  ok" checked nothing of a library's functions past the parser.) Bevy's own modules are
  stubbed with their real structs and signatures (`stub_source`), so a wrong call into Bevy fails
  too; an import with no stub is a failure. Add a stub when a shader imports a new Bevy item.
- **A name used from another WESL module must be in the import list.** A call to a function that
  `common.wesl` exports but that `render.wesl` does not import compiles *without an error* — naga
  emits a broken shader, no `failed to process shader` line appears, and the surface renders as a
  dark navy (the terrain's ground at 1 DN). The same symptom comes from a local that **shadows**
  another local in the same scope, and from **`pow(x, p)` with `p` in a `let`** (a literal
  exponent works). When a surface goes dark for no reason, check the import list, the shadowing
  and the `pow` first, not the maths.
- **Do not name a local after a struct field the same function reads.** In a function imported
  into the terrain's material, `let w = …` beside reads of a struct's field `g.w` rendered wrong
  on the GPU with no error: a hex blend came out biased toward its darkest taps, the dunes 10-17 %
  darker and more so in blue, and renaming either `w` fixed it. The `wesl` output and naga's IR are
  both correct in isolation, so the fault is downstream (`docs/research/ground-textures.md`,
  Outcome). A surface the wrong *shade* — not dark navy, just off — can be this too: bisect by
  renaming before suspecting the maths, and compare a capture's mean against a CPU emulation of
  the same formula.
- **The rock's cell networks change with the shader's register schedule, not only its maths.**
  Any edit to the terrain fragment that changes how the driver schedules it — even a pure no-op
  that keeps an early value (`w.w`, `trench`) alive to the end — moves the rock's float results in
  the last bits, and pixels near a cell wall flip to the neighbouring block: 4-22 % of a cliff's
  pixels by up to ~40 levels, structurally identical, while each build renders the same frame
  run after run (iteration 47's bisection, `docs/loop-log.md`). Compare rock captures by
  structure and band means, never bit-wise; a dry-rock "regression" after an unrelated shader
  edit is this until a no-op of the same shape says otherwise.
- **A value carried across `rock_material` costs the whole terrain pass.** The terrain
  fragment's cost is its peak register count, which sits in the rock's material: a canopy share
  made of the layer weights, the trench and the scree band — all dead before that call on `main` —
  cost +5 % of the 4K opaque pass on dry ground, and folding the three into one value before the
  call cost +20 %, while the same share read from the albedo after the call was free (iteration
  47). Derive a late quantity from what is live there (the albedo, `n`, `rough`, `rockness`,
  `depth`) rather than carrying an early one. A uniform branch that never runs costs too: a
  branch skipping `TerrainLook`'s default cost +6 %, the branchless operator nothing (iteration
  48); three uniform colour lanes replacing the layer constants cost the textured meadow +8 %
  even at a share of 0 (iteration 50), so an optional input goes behind its own shader def and
  pipeline-key bit (`GROUND_COLOURS`), not a uniform. Measure with the opaque pass over the
  early prepass of the same run: the ratio cancels Xvfb's clock hopping to about ±3 % within a
  P-state (prepass 0.15-0.18 ms at 4K, P5), where the means alone swing 2×; a run that lands in
  P2 (prepass ~0.075 ms) reads ~10 % higher and is
  discarded.
- **`ELEMENTS_GROW_HOURS` grows the cover maps, not the water.** It advances the `WorldClock`,
  which wetness and snow integrate; the hydrology runs on `HydrologySettings::time_scale` (real
  time by default), so a 10 s capture of 3 grown hours of rain has soaked ground and about
  0.01 mm of standing water. Standing water in a capture comes from a bake (`ErodeTerrain`), a
  `WaterSource` or a long real-time run; anything that looks like water on grown ground is the
  surface shading (iteration 46 blamed infiltration for what was the wet roughness).
- **An orthographic view has no distance at which a prop's screen size falls, so every distance-based draw rule culls the whole scene.** An isometric camera in this crate stands `ORTHO_BACK_M` = 457 m back from what it shows, and every prop on screen is that far away: the rocks' fade shrank each to `1e-3` and Bevy's `VisibilityRange` culled them outright (a 0.25 m stone's range is 25 m), so an isometric view drew **no rocks at all** for three passes. Measure a prop's draw distance from the terrain's LOD centre (`TerrainRuntime::morph_centre`, the focus under an orthographic view) and select it by the **view's own height** — a rock is drawn while it covers a fixed share of it, which is a threshold on its diameter (`src/rocks/mod.rs` `ViewScale`, `docs/research/orthographic-rocks.md`). Bevy's `VisibilityRange` cannot switch an LOD mesh under such a camera: it compares each entity with each *camera's* translation, one number for the whole scene, so the near and far children take the same verdict; the crate picks the child mesh itself.
- **A capture that shows a feature absent is checked against the camera before it is believed.** Iteration 57 signed off two "second seed" angles that pointed into a cliff face: `before` and `after` were byte-identical (mean 84.5) because there was no ground in frame, so "no rocks" was a frame with nothing in it. Look at the `before` capture and confirm the ground, and that the scene differs at all.
- **`SpecializedMeshPipelineError` has only `MissingVertexAttribute`**, so a material extension cannot reject a view without a depth prepass; the water shader needs `DEPTH_PREPASS` and fails to compile on a view without it.
- **`AlphaMode::Multiply` and `Add` need premultiplied output**: Bevy's `premultiply_alpha` runs in `main_pass_post_lighting_processing`; a shader that writes the result itself (the water's absorb pass) must write `(X·a, a)` for Multiply, because the blend is `dst·src.rgb + (1 − src.a)·dst`.
- **A displaced mesh needs its own `Aabb` and `NoAutoAabb`**, and `MeshTag::new(level)` (the struct has a debug-only private field). The terrain's vertex shader reads the patch origin from the instance matrix's translation, so patches are children of the terrain entity and the terrain `Transform` has no rotation or scale.
- **CDLOD morph needs `m_k ≥ r_{k−1} + diag_k`**, not `0.7 r_k`: a node about to subdivide must be unmorphed over its whole box, else the children pop in and an edge vertex cracks away from its finer neighbour. `terrain::lod::LodConfig::derive` computes the ranges; `adjacent_patches_meet_without_cracks` is the test, and a red plane under the terrain (`terrain_tour` with `ELEMENTS_UNDERLAY=1`) is the visual one.
- **A leaf's min/max must include a one-cell halo**: a leaf's vertices sit on its own cells' centres and its last row and column on the neighbour leaf's first cells.
- **The patch vertices sit on the height samples, half a cell in from the grid's corner** (`LodConfig::sample_offset`, `patch_transform`), and a vertex past the last sample is clamped onto it: level 0 draws the samples themselves. Anything that mirrors the drawn surface (the rocks' `PatchLevels`, the CPU `vertex_position`) offsets its lattice by it, and morphs about `TerrainRuntime::morph_centre` — the eye under a perspective camera, the focus under an orthographic one — never `ElementsView::eye` directly (`docs/research/orthographic-lod.md`).
- **Pressure projection with open window edges needs Dirichlet pressure there**: with Neumann on every edge a terrain that blocks one side of the wind window more than the other leaves unprojectable divergence (2.8×10⁻² s⁻¹ over hills, independent of the Jacobi count).
- **`ShaderBuffer::with_size` buffers keep their main-world asset** with `RenderAssetUsages::default()`: replace its `data` to rewrite the GPU copy; `RENDER_WORLD`-only buffers cannot be edited from the main world.
- **Particle and splash buffers** are read back in tests through `PrecipitationState` (`tests/gpu_precipitation.rs`); one test process builds one renderer at a time, so the GPU tests share a lock.
- **Pixel comparisons need `DebandDither::Disabled`**: the tonemapping dither depends on the pixel position, so two cubes with identical materials differ by a level. A render-target `Image` read back with `Readback` needs `TextureUsages::COPY_SRC` added to its descriptor (`Image::new_target_texture` has none), else the render world quits on a validation error (`tests/gpu_weathered.rs`).
- **A material's storage buffers all count against the fragment stage's eight** (Bevy uses five with an atmosphere camera): `WeatheredExtension` binds one, `ElementsGlobals` (128 B, shared, rewritten in place from `update_globals` with `RenderAssetUsages::default()` so no bind group is re-prepared). Values that change every frame reach `WeatheredMaterial` through the globals (the trail window's centre); the terrain and ground materials, which bind two storage buffers of their own and not the globals, carry them in their uniform and are re-prepared when they change (the window's centre as the focus crosses a texel, the ripple phases in rain).
- **`AsBindGroup` fields sharing a `#[uniform(N)]` index become one struct, in declaration order**: the WESL binds that struct (`WeatheredExtension`'s `exposure` and `trails` are `WeatheredParams`).
- **The camera-relative windows centre on the `ElementsFocus`, not the eye.** A shader that fades a window (`trail_fade`) takes its centre from the published state (`TrailState::centre`), never from `view.world_position`: under the isometric camera 1500 m back the eye's distance faded every trail out.
- **The crate simulates in a metre frame, world ÷ `ElementsUnits::units_per_metre`.** A new system that reads a `Transform` or the camera converts with `ElementsUnits::to_metres`/`point_to_metres` (or reads `ElementsView`, already metres); one that writes into Bevy's world (a transform, a light range, fog, an `Aabb`) multiplies back, extinction divides, point-light intensity takes u². A render shader divides every world position Bevy hands it by its uniform's `units` and multiplies what it hands back. `tests/gpu_units.rs` (a foot world answers as the metre world) is the check (`docs/research/units.md`).
- **Layers are written from a test through `Assets<ShaderBuffer>`**: after the terrain is live, `buffer.clear(); buffer.extend_from_slice(&data)` on the `TerrainState` handle re-uploads it with the same buffer identity (same size, label and usage take Bevy's `write_buffer` path). Hold the simulation first (`HydrologySettings { time_scale: 0.0, .. }`), write, then release it; `tests/gpu_hydrology.rs` shows the pattern, and a `time_scale` of `substeps_per_frame · 1.2` runs exactly the quality's substeps in a 1/60 s frame. The GPU tests run at `Ultra` (16 substeps a frame) so a thousand substeps are a hundred frames: the card is shared and a contended frame can take half a second.
- **An f32 water depth cannot hold a transfer below half its ulp**, so rain, evaporation and melt are delivered in quanta of at least 5 µm (`hydrology::model::QuantumAccumulator`), and a deep lake with a surface slope below about 10⁻⁵ does not conserve water exactly (`docs/research/hydrology.md`, Deviation 21). Tests that compare volumes under rain use dry ground or a uniform tank.
- **A pass that dispatches every frame needs a bring-up handshake.** The main world schedules substeps only after the render world says every pipeline is compiled and every buffer exists (`HydrologyGpu::ready`) and counts what was recorded (`executed`) and what was not (`dropped`); wait on `HydrologyStats::substeps`, never on frames.
- **An `Arc` atomic shared with the render world is seen by it at once, not at extraction.** With pipelined rendering the render frame in flight runs on the *previous* frame's extracted parameters, so a request made by bumping a shared counter (a seed, a reset) is carried out with stale parameters and marked done. Send requests as plain fields of the extracted resource, next to their parameters, and use atomics only to report back (`CoverGpu::seed_epoch` vs `seeded`; `cover::gpu_tests::gpu_a_restored_snowpack_seeds_the_maps` found it).
- **Aeolian transport is a scan, not a field.** `aeolian_flux` solves the flux along each row and column once per step from the upwind edge (one thread per line), because a one-cell-per-sweep Jacobi field cannot both conserve sand and keep the thickness non-negative. The dust passes dispatch hydrology's thermal kernels with their own `HydroParams::avalanche` uniform and run `.after(HydrologyDispatch).before(TerrainDispatch)`. A lone dune does not migrate under the shadow rule (`docs/research/dust.md` Deviation 12); the stripped front of a sand sheet does.
- **A crossfading `VisibilityRange` leaves black holes under the crate's depth prepass.** Every rock in its dithered fade band rendered as a black checkerboard: the pixels the main pass dithered away kept the clear colour, so the depth there was the rock's. `AlphaMode::Mask` (which makes Bevy 0.20 run `pbr_prepass.wesl`'s dither in the prepass) did not change it, measured; abrupt ranges have no holes. The scattered rocks use `VisibilityRange::abrupt` and shrink to nothing over the last fifth of their range instead (`src/rocks/mod.rs`, `docs/research/rock-scatter.md`).
- **`WorldClock::hours_since_2000()` can reach the next midnight while `date` is still the old day.** At 2.3×10⁵ hours, `days × 24 + 23.999999999999996` rounds in f64 to exactly the next midnight, so code that detects a day change from the hour count and labels the day with `clock.date` gives the new day the old date and shifts every later day by one (the climate record did, at 1.5-minute frames: two days dated 10 January). Derive the date from the same hour count (`2000-01-01 + floor(h / 24)`, `climate::history::date_of`).

- **A window screenshot is byte-black on a Mac with no GUI session for the ssh user.** `ELEMENTS_SHOT` writes the file, logs `Screenshot saved to …`, exits 0 — and the PNG is `mean 0.0` at the full 2560 × 1406. `screencapture` on the same host fails with "could not create image from display". Nothing in the run's log says so; check the image's mean before believing any capture. The fix is to capture on a host whose console session belongs to the ssh user (`mac_air` at 192.168.1.112 works; the laptop and `bmb` do not). `big` (Xvfb, RTX 3090) is the crate's documented host and the only one that reports GPU timestamps, and it can be down for a whole pass: check `ping` before planning captures, and do not compare across hosts.
- **A per-mesh derived table gives a prop's two levels of detail different data.** The scattered stones' palette was derived from each mesh's own facet areas, so the 1280-triangle near mesh and the 320-triangle far mesh of one shape got different tones (19° of hue) and the stone changed colour at the `22 · D` switch. Derive anything a mesh *wears* from the shape, once, at a canonical tessellation (`rocks::mesh::shape_tones`), and test it across both (`a_rocks_two_meshes_wear_the_same_rock`).
- **A lighting cube is not the sky.** The cloud pass's environment cube (`sky_probe`) carries a night fill, `params.atmo.w · NIGHT_SKY_COLOUR`, so the scene is not lit by black at midnight. A shader reading that cube as *radiance* must leave the fill out: read back into the below-horizon slab it drew `(9.4, 12.5, 15.5)` DN under a `(1.3, 1.0, 1.0)` sky — a grey sea under a black sky, across every night frame (`src/clouds/march.wesl` `horizon_sky`).
- **A hue ratio is undefined on a black sky, and its floor turns float noise into a large number.** `the_air_past_the_ground_is_the_sky_in_its_own_column` failed at midnight with all three bands exactly `0.0`: `(R − B) / max(luminance, 1e-9)` turned a `−2.8e-9` blue residue into a warmth of **−2.79**. A test that divides by a floored value needs its degenerate case asserted directly instead (a black sky leaves a black slab).

## Build and test

```sh
cargo build --release && cargo test && cargo build --examples --all-features
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features
cargo build --lib --target wasm32-unknown-unknown
cargo test --release --tests -- --ignored --test-threads=1 --skip ::gpu_tests::   # GPU integration tests, one process per file
for m in clouds terrain cover dust; do cargo test --release --lib --features=isomesh -- --ignored --test-threads=1 $m::gpu_tests::; done   # lib GPU tests, one process per module
ELEMENTS_FRAMES=600 ELEMENTS_SHOT=/tmp/x.png cargo run --release --example quickstart
```

`cargo build --release` runs first in CI because the full `bevy` dev-dependency hides holes in the trimmed feature list.
