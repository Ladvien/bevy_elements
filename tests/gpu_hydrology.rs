//! Hydrology and erosion on a real GPU: water conservation, rain delivery, sources and sinks, the
//! open boundary, erosion mass balance, thermal erosion, the bake and the flood gameplay query,
//! each read back from the terrain's layer buffers. See `docs/research/hydrology.md` and
//! `docs/research/erosion.md` (Validation targets).
//!
//! Needs a GPU: `cargo test --release --test gpu_hydrology -- --ignored --test-threads=1`. The
//! tests share one process and each builds a renderer, so they take a lock and run one at a time.

mod support;

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use bevy::app::AppExit;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::storage::ShaderBuffer;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

const DT: f64 = 1.0 / 60.0;
const STEP_S: f32 = 0.02;
/// Substeps per frame at `ElementsQuality::High`.
const PER_FRAME: u64 = 8;
/// The time scale that runs exactly `PER_FRAME` substeps in a 1/60 s frame.
const FULL_RATE: f32 = 9.6;

/// What the test pins the weather to after the climate has run, each frame.
#[derive(Resource, Clone, Copy)]
struct Pin {
    mm_h: f32,
    temperature_c: f32,
}

fn pin_weather(pin: Res<Pin>, mut weather: ResMut<Weather>) {
    weather.precipitation_mm_h = pin.mm_h;
    weather.temperature_c = pin.temperature_c;
}

/// Counts of the messages the tests wait for.
#[derive(Resource, Default)]
struct Baked(Vec<f32>);

fn record_baked(mut messages: MessageReader<TerrainBaked>, mut baked: ResMut<Baked>) {
    baked.0.extend(messages.read().map(|m| m.duration_s));
}

#[derive(Clone)]
struct Config {
    terrain: Terrain,
    hydrology: HydrologySettings,
    erosion: ErosionSettings,
    preset: WeatherPreset,
    mm_h: f32,
    temperature_c: f32,
    /// A source or sink spawned at startup: (world XZ, signed rate m³/s, radius m).
    sources: Vec<(Vec2, f32, f32)>,
    /// A precipitation blocker box (centre, half extents) spawned at startup.
    blockers: Vec<(Vec3, Vec3)>,
}

impl Config {
    fn new(terrain: Terrain) -> Self {
        Self {
            terrain,
            hydrology: HydrologySettings {
                boundary: HydrologyBoundary::Closed,
                evaporation: false,
                // Held until the test has written its layers.
                time_scale: 0.0,
                ..default()
            },
            erosion: ErosionSettings::default(),
            preset: WeatherPreset::Clear,
            mm_h: 0.0,
            temperature_c: 15.0,
            sources: Vec::new(),
            blockers: Vec::new(),
        }
    }
}

#[derive(Resource, Clone)]
struct Spawn(Config);

fn setup(
    mut commands: Commands,
    spawn: Res<Spawn>,
    mut director: ResMut<WeatherDirector>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn((
        Camera3d::default(),
        support::image_target(&mut images),
        ElementsCamera::default(),
        Transform::from_xyz(0.0, 30.0, 0.0),
    ));
    commands.spawn(spawn.0.terrain.clone());
    for &(at, rate, radius) in &spawn.0.sources {
        let transform = Transform::from_xyz(at.x, 0.0, at.y);
        if rate >= 0.0 {
            commands.spawn((
                WaterSource {
                    rate_m3_s: rate,
                    radius_m: radius,
                },
                transform,
            ));
        } else {
            commands.spawn((
                WaterSink {
                    rate_m3_s: -rate,
                    radius_m: radius,
                },
                transform,
            ));
        }
    }
    for &(centre, half_extents) in &spawn.0.blockers {
        commands.spawn((
            PrecipitationBlocker { half_extents },
            Transform::from_translation(centre),
        ));
    }
    director.force(spawn.0.preset, Duration::ZERO);
}

/// Update until `done` holds (at most 60 s of wall time).
fn run_until(app: &mut App, what: &str, mut done: impl FnMut(&mut App) -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        app.update();
        if done(app) {
            return;
        }
    }
    panic!("timed out waiting for {what}");
}

/// A running app with the terrain generated and the hydrology held.
struct Sim {
    app: App,
    terrain: TerrainState,
}

impl Sim {
    fn new(config: Config) -> Self {
        let mut app = App::new();
        app.add_plugins((support::headless_plugins(), ElementsPlugins))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(DT)))
            .insert_resource(ElementsQuality::High)
            .insert_resource(config.hydrology.clone())
            .insert_resource(config.erosion.clone())
            .insert_resource(Pin {
                mm_h: config.mm_h,
                temperature_c: config.temperature_c,
            })
            .insert_resource(Spawn(config))
            .init_resource::<Baked>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                pin_weather
                    .after(ElementsSystems::Weather)
                    .before(ElementsSystems::Simulation),
            )
            .add_systems(Update, record_baked.after(ElementsSystems::Simulation));
        support::finish(&mut app);
        run_until(&mut app, "the terrain", |app| {
            app.world()
                .get_resource::<TerrainState>()
                .is_some_and(|s| s.is_live())
        });
        let terrain = app.world().resource::<TerrainState>().clone();
        Self { app, terrain }
    }

    fn run_until(&mut self, what: &str, done: impl FnMut(&mut App) -> bool) {
        run_until(&mut self.app, what, done);
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.app.update();
        }
    }

    fn stats(&self) -> HydrologyStats {
        self.app.world().resource::<HydrologyStats>().clone()
    }

    fn set_time_scale(&mut self, scale: f32) {
        self.app.world_mut().resource_mut::<HydrologySettings>().time_scale = scale;
    }

    /// Run at least `n` more substeps at the full per-frame rate, hold, and wait until every
    /// scheduled substep has been recorded. Returns the substeps run.
    fn run_substeps(&mut self, n: u64) -> u64 {
        let start = self.stats().substeps;
        self.set_time_scale(FULL_RATE);
        self.run_until("the substeps", |app| {
            app.world().resource::<HydrologyStats>().substeps >= start + n
        });
        self.set_time_scale(0.0);
        self.settle();
        self.stats().substeps - start
    }

    /// Update until the substep counter has not moved for 20 frames.
    fn settle(&mut self) {
        let mut last = self.stats().substeps;
        let mut quiet = 0;
        while quiet < 20 {
            self.app.update();
            let now = self.stats().substeps;
            if now == last {
                quiet += 1;
            } else {
                quiet = 0;
                last = now;
            }
        }
    }

    /// Replace the contents of a layer. The buffer keeps its identity, so bind groups stay valid.
    fn write(&mut self, handle: &Handle<ShaderBuffer>, data: &[f32]) {
        let mut buffers = self.app.world_mut().resource_mut::<Assets<ShaderBuffer>>();
        let buffer = buffers.get_mut(handle).expect("the layer exists");
        buffer.clear();
        buffer.extend_from_slice(data);
        // One frame to upload it.
        self.app.update();
    }

    fn read_bytes(&mut self, handle: &Handle<ShaderBuffer>) -> Vec<u8> {
        let slot: Arc<Mutex<Option<Vec<u8>>>> = Arc::default();
        let sink = slot.clone();
        let entity = self
            .app
            .world_mut()
            .spawn(Readback::buffer(handle.clone()))
            .observe(move |done: On<ReadbackComplete>| {
                let mut sink = sink.lock().expect("lock");
                if sink.is_none() {
                    *sink = Some(done.data.to_vec());
                }
            })
            .id();
        self.run_until("a buffer readback", |_| slot.lock().expect("lock").is_some());
        self.app.world_mut().despawn(entity);
        slot.lock().expect("lock").take().expect("the readback completed")
    }

    fn read_f32(&mut self, handle: &Handle<ShaderBuffer>, count: usize) -> Vec<f32> {
        let bytes = self.read_bytes(handle);
        bytemuck::pod_collect_to_vec::<u8, f32>(&bytes[..count * 4])
    }

    fn read_vec2(&mut self, handle: &Handle<ShaderBuffer>, count: usize) -> Vec<[f32; 2]> {
        let bytes = self.read_bytes(handle);
        bytemuck::pod_collect_to_vec::<u8, [f32; 2]>(&bytes[..count * 8])
    }

    fn water_depth(&mut self, p: Vec2) -> Option<f32> {
        self.app
            .world_mut()
            .run_system_once(move |env: Environment| env.water_depth(p))
            .expect("the query runs")
    }

    fn finish(mut self) {
        self.app.world_mut().write_message(AppExit::Success);
    }
}

fn tank(cells: u32, cell_m: f32) -> Terrain {
    Terrain::flat(0.0)
        .with_extent(Vec2::splat(cells as f32 * cell_m))
        .with_resolution(UVec2::splat(cells))
}

fn sum(values: &[f32]) -> f64 {
    values.iter().map(|v| f64::from(*v)).sum()
}

/// Validation 1: a closed bowl with a 1 m column conserves its water.
#[test]
#[ignore = "needs a GPU"]
fn gpu_hydrology_conserves_water() {
    let _lock = serial();
    let n = 256usize;
    let mut sim = Sim::new(Config::new(tank(256, 2.0)));
    let cells = n * n;
    // b = 6 m (r / 128)² around the grid centre, 1 m of water inside 8 cells.
    let mut bed = vec![0.0f32; cells];
    let mut water = vec![0.0f32; cells];
    for j in 0..n {
        for i in 0..n {
            let r = (((i as f32 - 127.5).powi(2)) + ((j as f32 - 127.5).powi(2))).sqrt();
            bed[j * n + i] = 6.0 * (r / 128.0).powi(2);
            if r <= 8.0 {
                water[j * n + i] = 1.0;
            }
        }
    }
    let v0 = sum(&water);
    assert_eq!(v0, 208.0, "the column covers 208 cells");
    let (bedrock, water_handle) = (sim.terrain.bedrock.clone(), sim.terrain.water.clone());
    sim.write(&bedrock, &bed);
    sim.write(&water_handle, &water);
    let ran = sim.run_substeps(2000);
    let after = sim.read_f32(&water_handle, cells);
    let v = sum(&after);
    let drift = (v / v0 - 1.0).abs();
    let min = after.iter().copied().fold(f32::INFINITY, f32::min);
    let max = after.iter().copied().fold(0.0, f32::max);
    let velocity = sim.read_vec2(&sim.terrain.velocity.clone(), cells);
    let fastest = velocity
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1]).sqrt())
        .fold(0.0, f32::max);
    println!(
        "conservation: {ran} substeps, V {v:.6} / {v0}, drift {drift:.2e}, min {min}, max {max:.5}, fastest {fastest:.2} m/s"
    );
    assert!(ran >= 2000);
    assert!(drift <= 1e-3, "the plan's 0.1 %: {drift}");
    assert!(drift <= 1e-5, "regression guard: {drift}");
    assert!(min >= 0.0, "negative depth {min}");
    assert!(max <= 1.001, "the column rose to {max} m");
    assert!(fastest <= 0.5 * 2.0 / STEP_S, "speed {fastest} above the clamp");
    // The water has spread: the column is no longer a column.
    assert!(
        after.iter().filter(|d| **d > 1e-3).count() > 208,
        "the column did not spread"
    );
    sim.finish();
}

/// Validation 2: the open boundary loses water and never gains it; closed conserves it.
#[test]
#[ignore = "needs a GPU"]
fn an_open_boundary_loses_water_and_never_gains_it() {
    let _lock = serial();
    let n = 256usize;
    let mut config = Config::new(tank(256, 2.0));
    config.hydrology.boundary = HydrologyBoundary::Open;
    let mut sim = Sim::new(config);
    let cells = n * n;
    let mut water = vec![0.0f32; cells];
    for j in 0..n {
        for i in 0..n {
            let r = (((i as f32 - 127.5).powi(2)) + ((j as f32 - 127.5).powi(2))).sqrt();
            if r <= 8.0 {
                water[j * n + i] = 1.0;
            }
        }
    }
    let v0 = sum(&water);
    let water_handle = sim.terrain.water.clone();
    sim.write(&water_handle, &water);
    let mut previous = v0;
    let mut highest = 1.0f32;
    for _ in 0..12 {
        sim.run_substeps(250);
        let after = sim.read_f32(&water_handle, cells);
        let v = sum(&after);
        highest = highest.max(after.iter().copied().fold(0.0, f32::max));
        assert!(
            v <= previous + 1e-6 * v0,
            "the volume rose from {previous} to {v}"
        );
        previous = v;
    }
    println!("open boundary: V {previous:.3} of {v0}, highest depth {highest}");
    assert!(previous < v0, "no water left through the open edge");
    assert!(highest <= 1.001, "a cell rose to {highest} m");
    let stats = sim.stats();
    println!(
        "open boundary: stats volume {:.3}, lost {:.3}",
        stats.volume_m3, stats.lost_m3
    );
    sim.finish();
}

/// Validation 3: rain on a deep tank arrives in full, in quanta; cold air makes it snow; a blocker
/// shelters the cells under it.
#[test]
#[ignore = "needs a GPU"]
fn rain_is_delivered_exactly_in_quanta() {
    let _lock = serial();
    let n = 64usize;
    let cells = n * n;
    let mut config = Config::new(tank(64, 2.0));
    config.preset = WeatherPreset::Rain;
    config.mm_h = 4.0;
    // A 20 × 20 m roof at 5 m over the centre.
    config.blockers = vec![(Vec3::new(0.0, 5.0, 0.0), Vec3::new(10.0, 0.5, 10.0))];
    let mut sim = Sim::new(config);
    let water = sim.terrain.water.clone();
    sim.write(&water, &vec![2.0f32; cells]);
    let ran = sim.run_substeps(1800);
    let after = sim.read_f32(&water, cells);
    // The sheltered cells are those whose centre lies under the roof: |x|, |z| < 10 m.
    let mut open = Vec::new();
    let mut sheltered = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let x = (i as f32 + 0.5 - 32.0) * 2.0;
            let z = (j as f32 + 0.5 - 32.0) * 2.0;
            let d = f64::from(after[j * n + i]) - 2.0;
            if x.abs() < 9.0 && z.abs() < 9.0 {
                sheltered.push(d);
            } else if x.abs() > 11.0 || z.abs() > 11.0 {
                open.push(d);
            }
        }
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let ideal = ran as f64 * f64::from(STEP_S) * 4.0 / 3.6e6;
    println!(
        "rain: {ran} substeps, open gain {:.3e} (ideal {ideal:.3e}), sheltered gain {:.3e}",
        mean(&open),
        mean(&sheltered)
    );
    assert!(
        mean(&open) >= ideal - 5e-6 - 1e-7 && mean(&open) <= ideal + 1e-6,
        "open gain {} vs ideal {ideal}",
        mean(&open)
    );
    assert!(
        sheltered.iter().all(|d| d.abs() < 1e-9),
        "rain fell under the roof"
    );
    sim.finish();

    // Below freezing the same rate falls as snow: the water layer gains nothing.
    let mut cold = Config::new(tank(64, 2.0));
    cold.preset = WeatherPreset::Rain;
    cold.mm_h = 4.0;
    cold.temperature_c = -5.0;
    let mut sim = Sim::new(cold);
    let water = sim.terrain.water.clone();
    sim.write(&water, &vec![2.0f32; cells]);
    sim.run_substeps(1800);
    let after = sim.read_f32(&water, cells);
    assert!(
        after.iter().all(|d| *d == 2.0),
        "snow fell as rain: {}",
        after[0]
    );
    sim.finish();
}

/// A sink removes its rate from deep water, and only what is there.
#[test]
#[ignore = "needs a GPU"]
fn a_sink_removes_its_rate_and_a_source_adds_it() {
    let _lock = serial();
    let n = 64usize;
    let cells = n * n;
    let mut config = Config::new(tank(64, 2.0));
    config.sources = vec![(Vec2::new(-30.0, 0.0), -10.0, 4.0)];
    let mut sim = Sim::new(config);
    let water = sim.terrain.water.clone();
    sim.write(&water, &vec![1.0f32; cells]);
    let v0 = sum(&vec![1.0f32; cells]) * 4.0;
    let ran = sim.run_substeps(500);
    let v = sum(&sim.read_f32(&water, cells)) * 4.0;
    let seconds = ran as f64 * f64::from(STEP_S);
    println!(
        "sink: {ran} substeps ({seconds} s), removed {:.2} m³ (10 m³/s → {:.2})",
        v0 - v,
        10.0 * seconds
    );
    assert!(
        ((v0 - v) - 10.0 * seconds).abs() < 0.03 * 10.0 * seconds,
        "removed {}",
        v0 - v
    );
    sim.finish();

    // A source on dry ground fills it at its rate.
    let mut config = Config::new(tank(64, 2.0));
    config.sources = vec![(Vec2::new(0.0, 0.0), 5.0, 3.0)];
    let mut sim = Sim::new(config);
    let ran = sim.run_substeps(500);
    let water = sim.terrain.water.clone();
    let v = sum(&sim.read_f32(&water, cells)) * 4.0;
    let seconds = ran as f64 * f64::from(STEP_S);
    println!("source: added {v:.2} m³ (5 m³/s → {:.2})", 5.0 * seconds);
    assert!((v - 5.0 * seconds).abs() < 0.01 * 5.0 * seconds, "added {v}");
    // A sink on dry ground removes nothing.
    sim.finish();
    let mut config = Config::new(tank(64, 2.0));
    config.sources = vec![(Vec2::new(0.0, 0.0), -10.0, 3.0)];
    let mut sim = Sim::new(config);
    sim.run_substeps(200);
    let water = sim.terrain.water.clone();
    assert_eq!(sum(&sim.read_f32(&water, cells)), 0.0);
    sim.finish();
}
