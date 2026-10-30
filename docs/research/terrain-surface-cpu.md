# Generating a procedural terrain's surface on the CPU

bevy_emerge's issue #99 (and this crate's #48) wants to start a saved heightfield from a preset (`ProceduralTerrain::hills(seed)`), headless, with no GPU and no `TerrainPlugin`, and keep the heights in its own file. The generator exists on the CPU only as a test mirror (`terrain::model::mirror::generate_surface`, `#[cfg(test)]`); the public path runs `generate.wesl` on the GPU.

## Sources

- `docs/research/terrain.md`: the generator (Musgrave fBm blended with a ridged multifractal, `F_HALF`, `R_LOW`, `R_SPAN`, octave rotation and gain), and its validation against the CPU mirror (median 0.01 m, p99 0.1 m, max 1 m at 256², 8 m cells; `the_gpu_generator_matches_the_cpu_mirror`).
- Corpus search for method phrases is not repeated: nothing new is modelled, the existing, already researched generator is opened to callers. Recorded.

## What great looks like

A tool bakes `ProceduralTerrain::hills(7).sample_surface(UVec2::splat(512), 4.0)` into a height file in a unit test or a build script, with no window, and the height map it respawns (`TerrainSnapshot::terrain`-style) shows the same hills the GPU would have drawn.

## Model adopted

The existing mirror, unchanged: the surface at cell `(x, z)` is `base + relief · unit(f, r, ridged)` of the two noise sums at `((x, z) + ½) · cell_m`, with `octave_count(octaves, feature, cell_m)` octaves. The mirror's modules move out of `#[cfg(test)]`: the generator to `terrain/model/surface.rs`, and the lattice hash and gradient noise it needs (`pcg3d`, `hash33`, `lattice_gradient`, `perlin3_d`) from `gpu/noise_mirror.rs` to `gpu/perlin.rs`. The test mirror re-exports them, so every existing test compares against the same code the public function runs.

## Parameters

`ProceduralTerrain::sample_surface(&self, size: UVec2, cell_m: f32) -> Vec<f32>`: row-major `z * size.x + x`, metres, relative to the terrain transform. A zero size or a `cell_m` that is not positive and finite gives an empty vector (the other choices, a panic or a clamp, hide a caller's bug or invent a grid). Sediment is not part of the surface.

## GPU mapping

None. It is the CPU twin of `generate.wesl`; the GPU reads back through `Environment::terrain_snapshot()`.

## Validation targets

- Deterministic per seed, different between seeds, for hills, mountains, dunes and plains (`tests/terrain_surface.rs`).
- Heights within `[base, base + relief]` and spanning more than 20 % of the relief on a 64² grid.
- Cell `(x, z)` is independent of the grid's size around it, rows run along x.
- Agreement with the GPU kernel within the existing tolerance (`terrain::gpu_tests::the_gpu_generator_matches_the_cpu_mirror`, now calling the public function; ignored, runs on a GPU host).

## Deviations from this plan

1. No new model and no changed default. The CPU evaluates in `f32` with `sin_cos` and `powi` where the GPU uses its own intrinsics, so the two agree within the tolerance above and not bit for bit. The doc comment says so.

## Pending index

None: no new document.
