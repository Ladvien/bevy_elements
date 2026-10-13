# bevy_elements

> ⚠️ **Vibe Coded** — written by an AI agent working from a human's direction. It is used in a shipping game and covered by tests, but it has had no line-by-line human audit. Read it before you trust it.

**Living environments for Bevy 0.20.** A sun, a moon with phases and a star field from the real sky, a seeded climate that schedules weather, a GPU wind field, volumetric clouds, rain, snow and fog, lightning and thunder, heightfield terrain with hydraulic and thermal erosion, flooding, snow cover and wetness, and dust storms. Simulation runs in compute shaders; gameplay asks the `Environment` system parameter and is answered from a throttled async readback.

Every system was studied in the home-still paper library first. The note for each one is in [`docs/research/`](docs/research) and names the papers, the equations adopted and the numbers the tests assert.

## What is in it

| Plugin | Module | Status |
|---|---|---|
| — | — | Repository foundation only; the first plugin lands in 0.1.0. |

## Install

```toml
[dependencies]
bevy_elements = "0.1"
```

## Quick start

```rust
use bevy::app::App;

fn main() {
    // Filled in with the 0.1.0 plugin group.
    let _app = App::new();
}
```

## The API

One subsection per domain, filled in per milestone.

## Gameplay queries

`Environment` is a `SystemParam` answering from the latest GPU readback snapshot.

## Plays well with

`bevy_flora` (foliage and tree wind), `isomesh` / `bevy_isomesh` (volumetric meshing), `bevy_carnage` and `bevy_light_grid` (plain-data glue only: they are on Bevy 0.19).

## Performance

Measured budgets are recorded here per milestone.

## Platforms

Native (Vulkan, Metal, DX12) and wasm WebGPU. WebGL2 is unsupported: the simulation needs compute.

## What it deliberately does not do

- Foliage, trees and leaf visuals: `bevy_flora` owns them.
- Isosurface and voxel meshing: `isomesh` owns it.
- Oceans and FFT waves.
- Bit-deterministic GPU simulation. Astronomy, the weather scheduler and lightning are deterministic and seeded; GPU results are not.

## Compatibility

| bevy_elements | bevy |
|---|---|
| 0.1 | 0.20 |

## Research

Per-system notes are in [`docs/research/`](docs/research).

## Examples

Run with `cargo run --release --example <name>`.

## License

MIT OR Apache-2.0, at your option.
