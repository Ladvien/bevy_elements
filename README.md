# bevy_elements

> ⚠️ **Vibe Coded** — written by an AI agent working from a human's direction. It is used in a shipping game and covered by tests, but it has had no line-by-line human audit. Read it before you trust it.

**Living environments for Bevy 0.20.** A sun and moon placed by real ephemerides, the bright-star sky, a seeded climate that schedules hourly weather, a GPU wind field, distance and ground fog, and exposure that keeps night looking like night. Simulation runs in compute shaders; gameplay asks the `Environment` system parameter and is answered from a throttled async readback.

| | | |
|---|---|---|
| ![Solstice noon at latitude 45°: a 68° sun, short shadows](https://raw.githubusercontent.com/Ladvien/bevy_elements/main/docs/noon.png) | ![19:30 on the solstice: a 2° sun, long warm shadows, the sky graded to the horizon](https://raw.githubusercontent.com/Ladvien/bevy_elements/main/docs/sunset.png) | ![23:00: stars from the Yale Bright Star Catalogue, a half moon low in the east, moon shadows](https://raw.githubusercontent.com/Ladvien/bevy_elements/main/docs/night.png) |
| Noon, 21 June, 45° N | Sunset 19:30 | 23:00, half moon |
| ![The Fog preset: a bright ground-fog bank hiding everything past 100 m, blue sky overhead](https://raw.githubusercontent.com/Ladvien/bevy_elements/main/docs/fog.png) | ![The Clear preset of the same scene](https://raw.githubusercontent.com/Ladvien/bevy_elements/main/docs/clear.png) | ![bevy_flora trees in autumn colour moved by this crate's wind and rain](https://raw.githubusercontent.com/Ladvien/bevy_elements/main/docs/flora_wind.png) |
| `Fog` preset | `Clear` preset | with `bevy_flora` |

Every system was studied in the home-still paper library first. The note for each one, in [`docs/research/`](docs/research), names the papers, the equations adopted, every deviation from the plan with its source, and the numbers the tests assert.

## What is in it

| Plugin | Module | What it does |
|---|---|---|
| `ClimatePlugin` | `climate` | `Climate` presets, seasons, a seeded hourly weather scheduler, `WeatherDirector` to force and release weather, `SurfaceWeathering` (wetness, snow cover) |
| `CelestialPlugin` | `celestial` | `WorldClock`, `GeoLocation`, Meeus-accuracy sun and moon (`astro`), sun and moon `DirectionalLight`s for Bevy's atmosphere, the bright-star sky, a shaded moon disc, exposure |
| `WindPlugin` | `wind` | A camera-centred stable-fluids wind field on the GPU, curl-noise turbulence, travelling gusts, a public WGSL import |
| `FogPlugin` | `fog` | Distance fog from the weather's visibility and a camera-following ground-fog volume written by compute |
| `ElementsFloraPlugin` (feature `flora`) | `flora` | Drives `bevy_flora`'s wind, gusts, season and wetness |

`ElementsPlugins` adds the first four. Each is public on its own, so `ElementsPlugins.build().disable::<FogPlugin>()` works.

## Install

```toml
[dependencies]
bevy_elements = "0.1"
```

## Quick start

```rust,no_run
use bevy::prelude::*;
use bevy_elements::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins))
        .insert_resource(
            WorldClock::new(CivilDate::new(2026, 6, 21))
                .at_hour(7.0)
                .with_time_scale(120.0),
        )
        .insert_resource(Climate::temperate())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // The environment follows and renders for the `ElementsCamera`.
    commands.spawn((
        Camera3d::default(),
        ElementsCamera::default(),
        Transform::from_xyz(0.0, 30.0, 0.0).looking_at(Vec3::new(100.0, 10.0, 0.0), Vec3::Y),
    ));
    // Terrain arrives in 0.3; until then, any ground.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(2000.0, 2000.0))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
    ));
}
```

## The API

### Frame order

Everything runs in `Update` in the chained sets `ElementsSystems::{Clock, Weather, Celestial, Simulation, Readback}`. Systems that write `WorldClock`, `WeatherDirector` or settings run `.before(ElementsSystems::Clock)`; systems that read `Environment` run `.after(ElementsSystems::Readback)`.

### The camera

`ElementsCamera` marks the camera the environment follows. Adding it inserts, where absent: `AtmosphereSettings`, `AtmosphereEnvironmentMapLight`, `DepthPrepass`, `Msaa::Off`, `VolumetricFog`, `DistanceFog`, `Exposure`, and a zero per-camera `AmbientLight` (sky light comes from the atmosphere's environment map), and it spawns one Earth `Atmosphere` if there is none. With `auto_exposure` (the default) the exposure follows the sun, sky and moon: illuminance-metered like Ghost of Tsushima, mapped with Ward's contrast-based scale factor over Larson et al.'s rod-and-cone thresholds, so a moonlit scene stays dark instead of being exposed up to daylight. `ElementsQuality` (`Low`, `Medium`, `High`, `Ultra`) sets every resolution and step count.

### Clock, sun, moon and stars

`WorldClock` holds the local date and hour and advances by `Time<Virtual>` × `time_scale`; `GeoLocation` places the observer. `SunState` has the sun's and moon's directions, true altitudes, the synodic phase and the lit fraction. The `astro` module exposes the ephemeris functions (Meeus ch. 7–48: sidereal time, nutation, the sun at 0.01°, the moon from a 51 + 34-term series, phase, refraction, the USNO total-illuminance factor), each tested against the book's worked examples and the SPA reference case. Stars are the 8404 Yale Bright Star Catalogue stars to V = 6.5, precessed to the date, coloured from B−V; the moon is a Lommel–Seeliger-shaded disc from NASA's LROC map. See [`src/celestial/PROVENANCE.md`](src/celestial/PROVENANCE.md).

### Climate, seasons and weather

`Climate` (`temperate`, `alpine`, `desert`, `tropical`, `tundra`) drives an hourly Markov scheduler over ten `WeatherPreset`s, seeded and frozen by a golden test, with temperature from annual and diurnal harmonics and a −6.5 °C/km lapse rate. `Weather` is the blended state every domain reads (cover, precipitation, temperature, humidity, wind, gustiness, visibility, storminess, dust). `WeatherDirector::force(preset, transition)` overrides it and `release(transition)` hands it back to the scheduler, which kept stepping meanwhile. `SeasonState` carries meteorological seasons and 0–1 autumn, leaf-fall and blossom curves; `SurfaceWeathering` the wetness, snow cover and snow depth of exposed ground. Messages: `WeatherChanged`, `SeasonChanged`.

### Wind

`WindState` has the prevailing wind, the 10 m speed, the gust factor, the wind at the camera and the simulated field: a `rgba16float` 3D texture, toroidally addressed, around the camera (64 × 16 × 64 cells of 4 m at High). Shaders sample it with the supported WGSL import:

```wgsl
import bevy_elements::wind::{WindFieldParams, elements_wind};
let v = elements_wind(wind_field, wind_sampler, wind_params, world_position);
```

bound to `WindState::field` and `WindState::field_params()`. `WindGust` is written when a gust band starts. `bevy_elements::noise` (gradient noise with analytic derivatives, fBm, Worley, curl) and `bevy_elements::grid` (bilinear taps into the crate's 2D grids) are also supported imports.

### Fog

`DistanceFog` follows `Weather::visibility_m` (Koschmieder, 2 % contrast, grey extinction) with a colour lit like the scene; a 240 × 120 × 240 m ground-fog `FogVolume` around the camera thickens toward the ground and drifts with the wind.

## Gameplay queries

```rust
use bevy::prelude::*;
use bevy_elements::*;

fn seek_shelter(env: Environment) {
    let weather = env.weather();
    if weather.precipitation_mm_h > 2.0 || (!env.is_daylight() && env.temperature_c() < 0.0) {
        // ...
    }
    let gust_at_head = env.wind_at(Vec3::new(10.0, 1.8, -4.0));
    let _ = (gust_at_head, env.snapshot_age());
}
```

`wind_at` answers from the GPU field as of the last readback (4 Hz by default, `ElementsReadback`) and falls back to the prevailing wind outside it or before the first readback; `snapshot_age` says how old the answer is.

## Plays well with

- **`bevy_flora`** (feature `flora`): add `ElementsFloraPlugin` next to `FloraPlugin`. It writes `FloraWind` (direction, strength = 10 m speed / 20 m/s, turbulence, gust strength, and every `WindGust` as a `GustFront`), `FloraSeason` (autumn, leaf fall, blossom, snow = `SurfaceWeathering::snow_cover`) and `FloraWetness`. Trees and their sway stay `bevy_flora`'s.
- **`isomesh` / `bevy_isomesh`**: volumetric snow on voxel ground arrives in 0.5.
- **`bevy_carnage`, `bevy_light_grid`** (Bevy 0.19): no dependency either way. Glue in the game copies plain data across, e.g. `SurfaceWeathering::wetness` into a blood-drying rate, or `Environment::sun()` into a light grid's sky term.

## Performance

GPU time per frame on an RTX 3090 (Vulkan), the minimum over Bevy's diagnostic history so it is the full-clock cost (`examples/bench_elements.rs`; a lightly loaded GPU idles at 210–450 MHz and reads 5–10× slower).

| Quality | Wind grid | Jacobi iterations | Wind | Ground-fog density | Total |
|---|---|---|---|---|---|
| Low | 32 × 8 × 32 | 10 | 0.087 ms | 0.037 ms | 0.124 ms |
| Medium | 48 × 12 × 48 | 10 | 0.119 ms | 0.037 ms | 0.156 ms |
| High | 64 × 16 × 64 | 12 | 0.233 ms | 0.037 ms | 0.270 ms |
| Ultra | 96 × 24 × 96 | 40 | 1.704 ms | 0.037 ms | 1.741 ms |

The budget for wind + fog at High is 0.3 ms. Ultra keeps 40 Jacobi iterations and the plan's grid; it is for large budgets.

## Platforms

Native (Vulkan, Metal, DX12) and wasm WebGPU. WebGL2 is unsupported: the simulation needs compute. Simulation state lives in storage buffers and published fields in write-only `rgba16float` storage textures: the wind kernels share one layout of six storage buffers and one storage texture, the fog kernel one storage texture, inside WebGPU's default limits of 8 and 4 per stage (devices limited to the `downlevel` 4 storage buffers are not supported). GPU timestamps for the bench exist on Vulkan and DX12 only.

## What it deliberately does not do

- Trees, foliage, leaf colour and snow on branches: `bevy_flora` owns them.
- Isosurface and voxel meshing: `isomesh` owns it.
- Oceans and FFT waves.
- Bit-deterministic GPU simulation. Astronomy, the weather scheduler and (from 0.2) lightning are deterministic and seeded; GPU results are not.
- A Purkinje shift or other low-light colour change: exposure keeps night dark, but colour handling is left to the tonemapper.

## Compatibility

| bevy_elements | bevy |
|---|---|
| 0.1 | 0.20 |

## Research

- [Bevy 0.20 render APIs](docs/research/bevy-0.20-render.md) — the verified paths and signatures this crate builds on.
- [What the home-still corpus holds](docs/research/corpus.md) — the reading list per domain.
- [Celestial](docs/research/celestial.md) — Meeus, Reda & Andreas (SPA), Krisciunas & Schaefer, Jensen et al., Ballesteros, USNO Circular 171, Larson et al. (tone reproduction).
- [Climate and weather](docs/research/climate-weather.md) — Richardson 1981, ISO 2533, Jennings et al. 2018.
- [Wind](docs/research/wind.md) — Stam 1999, Harris 2004, Bridson et al. 2007, Kim et al. 2008, WMO.
- [Fog](docs/research/fog.md) — Wronski 2014, Hillaire 2015, Wenzel 2006, Real-Time Rendering.

## Examples

Run with `cargo run --release --example <name>`. Every example takes `ELEMENTS_FRAMES=N` (exit after N frames) and `ELEMENTS_SHOT=/path.png` (screenshot on the last frame). Keys: WASD + Q/E to fly (Shift ×5), right mouse to look, `[`/`]` time scale, `T` +1 hour, `1`–`0` force a preset, `R` release.

| Example | Shows |
|---|---|
| `quickstart` | The quick start with a test scene |
| `day_night` | A solstice day at 45° N in two minutes; `ELEMENTS_HOUR=19.5` holds an hour |
| `weather_tour` | Every preset for 20 s; `ELEMENTS_PRESET=Fog` holds one |
| `flora_wind` | `bevy_flora` trees in this crate's wind, season and rain (`--features flora`) |
| `bench_elements` | Per-pass GPU time; `ELEMENTS_QUALITY`, `ELEMENTS_RESOLUTION` |

## License

MIT OR Apache-2.0, at your option.
