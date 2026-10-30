# Readback pacing: gameplay queries in an app with no window

Every gameplay query on `Environment` (`wind_at`, `water_depth`, `water_velocity`, `snow_depth`, `wetness`, `cloud_shadow_at`, the map-backed `SurfaceWeathering`) answers from `EnvironmentSnapshot`, which `src/gpu/readback.rs` fills from one-shot GPU readbacks every `ElementsReadback::interval` (250 ms). The contract in `src/query.rs` is "up to one readback interval old". This note covers an app that renders only to images (a headless server, a simulation-only tool, every GPU test in this crate): there the contract broke.

## Sources

- wgpu 30 API, `Queue::on_submitted_work_done` and `Device::poll` (vendored `wgpu-29.0.4/src/api/queue.rs:291-302`, `device.rs:86-94`; Bevy 0.20 depends on wgpu 30, same text): a map or work-done callback runs only when `queue.submit`, `instance.poll_all` or `device.poll` is called after the GPU work completes; `poll` blocks with a wait variant and is a no-op on WebGPU, where devices are polled automatically. Primary.
- Bevy 0.20 `bevy_render/src/gpu_readback.rs` (vendored): `ReadbackOnce` is removed in `First`; `prepare_buffers` (`PrepareResources`) copies the source into a pooled `MAP_READ` buffer, `submit_readback_commands` encodes the copy after the render graph (`renderer/mod.rs:96-106`), `map_buffers` (`Cleanup`) calls `map_async`, and `sync_readbacks` (`ExtractSchedule`) triggers `ReadbackComplete` once the callback has sent the bytes. Nothing in the readback path waits for the GPU. Primary.
- Bevy 0.20 `bevy_render/src/renderer/mod.rs:108-125` and `view/window/mod.rs`: the only per-frame wait is presenting the swap chain of an `ExtractedWindow`; an app with no window never presents.
- Xu, J. (2021). *Practical GPU Graphics with wgpu and Rust*, ch. on compute (corpus stem `Practical GPU Graphics with wgpu and Rust -- Jack Xu -- 2021 …`, ll. 9033-9075, 9137): the readback idiom copy → submit → `map_async` → wait for the device before reading the mapped range.
- Scarpino, M. (2024). *The WebGPU Sourcebook* (corpus stem `We/WebGPU_Sourcebook`, ll. 6802-6876, 7210): `mapAsync` resolves only after the GPU has completed the work that writes the buffer; the read side is asynchronous by design.
- Corpus search (phrases `frames in flight`, `CPU-GPU synchroniz`, `GPU readback`, `frame pacing`, `readback latency`) found nothing on unpaced headless rendering; the 2007 Advances notes (`s2007-advances-course-notes-0-98-mb`) only recommend avoiding readbacks. The model below rests on the API sources and on the measurements in Validation.

## Model adopted

A frame's GPU work is retired asynchronously. With a window, `present` blocks the render thread when the swap chain is full, so the CPU can run at most a few frames ahead and a readback is delivered a bounded number of frames after it was requested. Without a window, `App::update` returns as soon as the frame is submitted. When the GPU needs longer per frame than the CPU (the simulation kernels at `Ultra`, the cover's catch-up steps), the queue grows by the difference every frame and the latency of a readback grows with it; a request then completes later than `drop_stale`'s timeout (`4 · interval + 1 s`), is dropped and reissued behind an even longer queue, and the snapshot stops changing.

Measured in `cover::gpu_tests::gpu_wetness_soaks_and_dries_on_the_formula` (32² terrain, `High`, `ElementsReadback::interval` 1 ms and 50 ms, traced with temporary prints in `readback.rs`): the request-to-delivery latency went 2, 4, 8, … frames; the last delivery of any field was at 3.1 s of the run; 85 s later `Environment::wetness` still answered 0.325 (and the map-backed `SurfaceWeathering` the state of hour 1) while the map read 0.0. A `Poll` (non-blocking) after each frame moved the freeze to 23 s: the callbacks were not the bottleneck, the queue was.

Adopted: in the render world, after `Cleanup` (so after `map_buffers`), a frame with no `ExtractedWindow` waits for its own submission, `RenderDevice::poll(PollType::wait_indefinitely())`. That bounds the queue at one frame, runs the map callback of this frame's readback before the next extract, and so delivers every readback on the next frame. A frame that has a window returns at once: presenting already paces it, and waiting there would only remove the CPU/GPU overlap.

## Parameters

| Name | Value | Source |
|---|---|---|
| Readback interval | 250 ms (unchanged) | `ElementsReadback` default |
| Wait | `PollType::wait_indefinitely()`, only when no `ExtractedWindow` exists | wgpu `Device::poll` |
| Schedule | `Render`, `RenderSystems::PostCleanup` | after `map_buffers` (`Cleanup`) |

## GPU mapping

No kernel and no buffer. One render-world system, `pace_without_window` in `src/gpu/readback.rs`. On WebGPU `poll` is a no-op, and a browser app has a canvas window, so the wasm build is unaffected. Cost: none with a window (an empty query); without one, the CPU stops running ahead of the GPU, so a headless frame takes the GPU's time for that frame instead of the CPU's.

## Validation targets

1. **Freshness without a window (GPU integration test, `tests/gpu_readback.rs`):** a headless app at `Ultra` with a 256² terrain and a 100 ms readback interval: over 1200 frames after bring-up, `Environment::snapshot_age()` never exceeds the interval plus six frames (200 ms), and `water_depth` and `wetness` are `Some`. Fails before (oldest 683 ms), passes after.
2. **The three failures that stood on pristine HEAD since iteration 4 pass:** `cover::gpu_tests::gpu_wetness_soaks_and_dries_on_the_formula` (its `Environment::wetness` probe), `cover::gpu_tests::gpu_surface_weathering_follows_the_map` (map vs point model, hour by hour), `voxel::gpu_tests::gpu_voxel_snow_covers_the_top` (the "+2.1 h swap bias" of the log was this stale snapshot).
3. **Windowed apps unchanged (visual):** `weather_tour` under `PartlyCloudy` at noon, before and after, is the same frame within GPU noise: no visual element is created or altered, so this before/after pair is the whole inventory.
4. **Readback latency with a window unchanged:** `cloud_shadow_probes` logs `snapshot_age` 250 ms (one interval) over 1500 frames before the change (measured with a temporary log line); the windowed path does not run the wait.

## Deviations from this plan

1. **The headless harness is paced by the crate, not by each test.** The alternative, a waiting system in `tests/support` and in each lib-test harness, would leave the crate's own contract ("up to one readback interval old") false for every game that runs without a window, which is a run path the loop names as a candidate. The wait lives where the readbacks are issued.
2. **`drop_stale` is kept.** With the queue bounded it no longer fires in a healthy app; it still recovers a request whose source buffer was not on the GPU yet (its original purpose).

## Pending index

None: the sources are the vendored crates and two corpus books already indexed; no new document was downloaded.
