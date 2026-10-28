//! Trails on a real GPU (`docs/research/trails.md`, Validation 2–5): an imprint presses its
//! depth and raises its rim, a fast one leaves an unbroken rut, new snow fills a trench, and the
//! window keeps a trail at its world position as the camera moves. Each test lays 30 cm of snow at
//! 100 kg/m³ on a flat terrain by writing the cover's snow layer, holds the clock (no melt, no
//! fall), and reads the published field.
//!
//! Needs a GPU: `cargo test --release --test gpu_trails -- --ignored --test-threads=1`.

mod support;

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

const CELLS: u32 = 64;
const CELL_M: f32 = 2.0;
/// The walker's sole: 77.5 kg on an ellipse 0.05 × 0.13 m (37.2 kPa).
const FOOT: Vec2 = Vec2::new(0.05, 0.13);
const LOAD_N: f32 = 760.0;
/// 30 cm of snow at 100 kg/m³: the CRREL cap of 0.3 × (1 − 100/500) = 24 cm binds (Bekker 31 cm).
const SNOW_M: f32 = 0.3;
const SNOW_SWE_M: f32 = 0.03;
const EXPECTED_DEPTH_M: f32 = 0.24;

#[derive(Component)]
struct Eye;

struct Sim {
    app: App,
}

impl Sim {
    fn new() -> Self {
        Self::with(TrailSettings::default())
    }

    fn with(trails: TrailSettings) -> Self {
        let mut app = App::new();
        app.add_plugins((
            support::headless_plugins(),
            ElementsPlugins
                .build()
                .disable::<PrecipitationPlugin>()
                .disable::<LightningPlugin>()
                .disable::<CloudPlugin>(),
        ))
        .insert_resource(trails)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 1, 15)).at_hour(12.0)
        })
        .insert_resource(ErosionSettings {
            enabled: false,
            ..default()
        })
        .insert_resource(HydrologySettings {
            time_scale: 0.0,
            ..default()
        })
        .add_systems(
            Startup,
            |mut commands: Commands,
             mut images: ResMut<Assets<Image>>,
             mut director: ResMut<WeatherDirector>| {
                commands.spawn((
                    Camera3d::default(),
                    support::image_target(&mut images),
                    ElementsCamera::default(),
                    Eye,
                    Transform::from_xyz(0.0, 3.0, 0.0)
                        .looking_at(Vec3::new(0.0, 0.0, 4.0), Vec3::Y),
                ));
                commands.spawn(
                    Terrain::flat(0.0)
                        .with_extent(Vec2::splat(CELLS as f32 * CELL_M))
                        .with_resolution(UVec2::splat(CELLS)),
                );
                director.force(WeatherPreset::Clear, Duration::ZERO);
            },
        );
        support::finish(&mut app);
        let mut sim = Self { app };
        sim.run_until("the trail field", |app| {
            app.world().get_resource::<TrailState>().is_some()
                && app.world().get_resource::<CoverState>().is_some()
                && app
                    .world()
                    .get_resource::<TerrainState>()
                    .is_some_and(|t| t.is_live())
        });
        sim.lay_snow(SNOW_M, SNOW_SWE_M);
        sim
    }

    fn run_until(&mut self, what: &str, mut done: impl FnMut(&mut App) -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(120);
        while std::time::Instant::now() < deadline {
            self.app.update();
            if done(&mut self.app) {
                return;
            }
        }
        panic!("timed out waiting for {what}");
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.app.update();
        }
    }

    /// Every cell of the cover's snow layer to `depth` m holding `swe` m of water, then frames
    /// for the cover to pack it and the field to see it.
    fn lay_snow(&mut self, depth: f32, swe: f32) {
        let handle = self.app.world().resource::<CoverState>().snow.clone();
        let data: Vec<f32> = (0..CELLS * CELLS)
            .flat_map(|_| [depth, swe, 0.0, 0.0])
            .collect();
        {
            let mut buffers = self.app.world_mut().resource_mut::<Assets<ShaderBuffer>>();
            let Some(mut buffer) = buffers.get_mut(&handle) else {
                panic!("the snow layer exists");
            };
            buffer.clear();
            buffer.extend_from_slice(&data);
        }
        self.frames(90);
    }

    /// Every cell of an `array<f32>` buffer to `value`, then frames for the cover and terrain
    /// packs to carry it to the kernel.
    fn fill(&mut self, handle: Handle<ShaderBuffer>, value: f32) {
        let data = vec![value; (CELLS * CELLS) as usize];
        {
            let mut buffers = self.app.world_mut().resource_mut::<Assets<ShaderBuffer>>();
            let Some(mut buffer) = buffers.get_mut(&handle) else {
                panic!("the layer exists");
            };
            buffer.clear();
            buffer.extend_from_slice(&data);
        }
        self.frames(30);
    }

    fn lay_wetness(&mut self, wetness: f32) {
        let handle = self.app.world().resource::<CoverState>().wetness.clone();
        self.fill(handle, wetness);
    }

    fn lay_sediment(&mut self, depth_m: f32) {
        let handle = self.app.world().resource::<TerrainState>().sediment.clone();
        self.fill(handle, depth_m);
        // The terrain packs a layer write only after a hydrology step bumps its change counter
        // (`TerrainGpu` is crate-private), so run the hydrology for a few frames and hold it again.
        let held = self.app.world().resource::<HydrologySettings>().clone();
        self.app.world_mut().insert_resource(HydrologySettings {
            time_scale: 1.0,
            ..held.clone()
        });
        self.frames(30);
        self.app.world_mut().insert_resource(held);
        self.frames(10);
    }

    fn spawn_imprint(&mut self, at: Vec3) -> Entity {
        self.app
            .world_mut()
            .spawn((Imprint::new(FOOT, LOAD_N), Transform::from_translation(at)))
            .id()
    }

    fn move_eye(&mut self, to: Vec3) {
        let world = self.app.world_mut();
        let mut q = world.query_filtered::<&mut Transform, With<Eye>>();
        for mut t in q.iter_mut(world) {
            t.translation = to;
        }
    }

    /// The published field's `h` per texel, and the window's (texels, texel edge).
    fn field(&mut self) -> (Vec<f32>, u32, f32) {
        let state = self.app.world().resource::<TrailState>().clone();
        let n = state.texels as usize;
        let bytes = support::read_texture(&mut self.app, state.field, n * 8, n);
        let h = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|t| support::half(u16::from_le_bytes([t[0], t[1]])))
            .collect();
        (h, state.texels, state.texel_m)
    }

    fn finish(mut self) {
        self.app.world_mut().write_message(AppExit::Success);
        self.app.update();
    }
}

/// `h` at world XZ (the nearest texel) in a field read with `Sim::field`.
fn h_at(field: &(Vec<f32>, u32, f32), xz: Vec2) -> f32 {
    let (h, n, cell) = field;
    let w = (xz / *cell).floor().as_ivec2();
    let n = *n as i32;
    let s = ((w % n) + n) % n;
    h[(s.y * n + s.x) as usize]
}

/// Validation 2: a still imprint on flat snow presses to the model's depth, and the rim holds
/// about `2ν` = 10 % of what it carved.
#[test]
#[ignore = "needs a GPU"]
fn an_imprint_presses_its_depth_and_raises_its_rim() {
    let _lock = serial();
    let mut sim = Sim::new();
    let foot = sim.spawn_imprint(Vec3::new(0.0, SNOW_M - 0.05, 4.0));
    sim.frames(20);
    sim.app.world_mut().despawn(foot);
    sim.frames(4);
    let field = sim.field();
    let (h, _, cell) = &field;
    let deepest = h.iter().copied().fold(0.0f32, f32::min);
    let carved: f32 = h.iter().filter(|v| **v < 0.0).map(|v| -v).sum::<f32>() * cell * cell;
    let rim: f32 = h.iter().filter(|v| **v > 0.0).sum::<f32>() * cell * cell;
    println!(
        "deepest {deepest:.4} m (model {EXPECTED_DEPTH_M}), carved {carved:.5} m³, rim {rim:.5} m³ ({:.3})",
        rim / carved
    );
    assert!(
        (-deepest - EXPECTED_DEPTH_M).abs() <= 0.03 * EXPECTED_DEPTH_M,
        "deepest {deepest}"
    );
    assert!(
        (0.075..=0.125).contains(&(rim / carved)),
        "rim share {}",
        rim / carved
    );
    sim.finish();
}

/// The deepest `|h|` under one still foot at `SNOW_M - 0.05` above `snow_m` of snow, after the
/// field has seen it for 20 frames.
fn deepest_after_a_step(trails: TrailSettings, snow_m: f32, snow_swe_m: f32) -> f32 {
    let mut sim = Sim::with(trails);
    sim.lay_snow(snow_m, snow_swe_m);
    sim.spawn_imprint(Vec3::new(0.0, snow_m - 0.05, 4.0));
    sim.frames(20);
    let (h, _, _) = sim.field();
    sim.finish();
    -h.iter().copied().fold(0.0f32, f32::min)
}

/// #39: `min_depth_m` stops a print the ground would take shallower than it (dry soil, 0.5 cm),
/// keeps one in fresh snow, and at its default changes nothing.
#[test]
#[ignore = "needs a GPU"]
fn a_press_threshold_skips_dry_soil_and_keeps_snow() {
    let _lock = serial();
    let gate = TrailSettings {
        min_depth_m: 0.01,
        ..default()
    };
    let dry_default = deepest_after_a_step(TrailSettings::default(), 0.0, 0.0);
    let dry_gated = deepest_after_a_step(gate.clone(), 0.0, 0.0);
    let snow_gated = deepest_after_a_step(gate, SNOW_M, SNOW_SWE_M);
    println!("dry {dry_default:.4} m, dry gated {dry_gated:.4} m, snow gated {snow_gated:.4} m");
    assert!(dry_default > 0.003, "dry soil presses by default: {dry_default}");
    assert_eq!(dry_gated, 0.0, "dry soil under a 1 cm threshold");
    assert!(
        (snow_gated - EXPECTED_DEPTH_M).abs() <= 0.03 * EXPECTED_DEPTH_M,
        "snow presses through the threshold: {snow_gated}"
    );
}

/// Validation 3: an imprint moving at 20 m/s carves an unbroken rut.
#[test]
#[ignore = "needs a GPU"]
fn a_fast_imprint_leaves_an_unbroken_rut() {
    let _lock = serial();
    let mut sim = Sim::new();
    let wheel = sim.spawn_imprint(Vec3::new(-8.0, SNOW_M - 0.05, 4.0));
    for frame in 0..48 {
        let x = -8.0 + 20.0 * frame as f32 / 60.0;
        sim.app
            .world_mut()
            .entity_mut(wheel)
            .insert(Transform::from_xyz(x, SNOW_M - 0.05, 4.0));
        sim.app.update();
    }
    sim.app.world_mut().despawn(wheel);
    sim.frames(4);
    let field = sim.field();
    let shallowest = (0..140)
        .map(|i| h_at(&field, Vec2::new(-7.5 + 0.1 * i as f32, 4.0)))
        .fold(f32::NEG_INFINITY, f32::max);
    println!("shallowest point of the rut: {shallowest:.4} m");
    assert!(
        shallowest <= -0.5 * EXPECTED_DEPTH_M,
        "the rut breaks: {shallowest}"
    );
    sim.finish();
}

/// Validation 4: 5 cm of new snow shallows a trench by 5 cm and leaves the flat at zero.
#[test]
#[ignore = "needs a GPU"]
fn new_snow_fills_a_trench() {
    let _lock = serial();
    let mut sim = Sim::new();
    let foot = sim.spawn_imprint(Vec3::new(0.0, SNOW_M - 0.05, 4.0));
    sim.frames(20);
    sim.app.world_mut().despawn(foot);
    sim.frames(4);
    let before = h_at(&sim.field(), Vec2::new(0.0, 4.0));
    sim.lay_snow(SNOW_M + 0.05, SNOW_SWE_M + 0.005);
    let field = sim.field();
    let after = h_at(&field, Vec2::new(0.0, 4.0));
    let flat = h_at(&field, Vec2::new(5.0, 9.0));
    println!("trench {before:.4} → {after:.4} m, flat {flat:.4} m");
    assert!(
        ((after - before) - 0.05).abs() <= 0.005,
        "the trench rose by {}",
        after - before
    );
    assert!(flat.abs() < 1e-4, "the flat moved: {flat}");
    sim.finish();
}

/// Validation 5: a trail stays at its world position as the camera moves 10 m, and is gone once
/// the camera has been a window away and back.
#[test]
#[ignore = "needs a GPU"]
fn the_window_keeps_a_trail_in_place() {
    let _lock = serial();
    let mut sim = Sim::new();
    let foot = sim.spawn_imprint(Vec3::new(0.0, SNOW_M - 0.05, 4.0));
    sim.frames(20);
    sim.app.world_mut().despawn(foot);
    sim.frames(4);
    let here = h_at(&sim.field(), Vec2::new(0.0, 4.0));
    sim.move_eye(Vec3::new(10.0, 3.0, 0.0));
    sim.frames(10);
    let moved = h_at(&sim.field(), Vec2::new(0.0, 4.0));
    sim.move_eye(Vec3::new(70.0, 3.0, 0.0));
    sim.frames(10);
    sim.move_eye(Vec3::new(0.0, 3.0, 0.0));
    sim.frames(10);
    let back = h_at(&sim.field(), Vec2::new(0.0, 4.0));
    println!("trail {here:.4} m; after 10 m {moved:.4} m; after leaving and returning {back:.4} m");
    assert!(
        here < -0.2 && (moved - here).abs() < 1e-3,
        "{here} then {moved}"
    );
    assert!(back.abs() < 1e-4, "still there: {back}");
    sim.finish();
}

/// #39: `min_depth_m` 0.01 on bare ground (no snow) of the given wetness and sediment depth:
/// the deepest `|h|` one still foot leaves.
fn gated_step_on(wetness: f32, sediment_m: f32) -> f32 {
    let mut sim = Sim::with(TrailSettings {
        min_depth_m: 0.01,
        ..default()
    });
    sim.lay_snow(0.0, 0.0);
    sim.lay_wetness(wetness);
    sim.lay_sediment(sediment_m);
    sim.spawn_imprint(Vec3::new(0.0, -0.05, 4.0));
    sim.frames(20);
    let (h, _, _) = sim.field();
    sim.finish();
    -h.iter().copied().fold(0.0f32, f32::min)
}

/// #39 acceptance 1 and 2: at `min_depth_m` 0.01 a foot leaves nothing on dry soil, presses at
/// least 1 cm on soil at wetness 0.5, and presses on dry sand (3.4 cm by the model).
#[test]
#[ignore = "needs a GPU"]
fn a_press_threshold_passes_wet_soil_and_sand() {
    let _lock = serial();
    let dry = gated_step_on(0.0, 0.0);
    let damp = gated_step_on(0.5, 0.0);
    let sand = gated_step_on(0.0, 1.0);
    println!("dry {dry:.4} m, wetness 0.5 {damp:.4} m, dry sand {sand:.4} m");
    assert_eq!(dry, 0.0, "dry soil under a 1 cm threshold");
    assert!(damp >= 0.01, "wet soil presses: {damp}");
    assert!(sand >= 0.01, "dry sand presses: {sand}");
}

/// A foot on ground laid to `snow_m` of snow holding `swe_m` of water, `wetness` and `sediment_m`
/// of sediment: the deepest `|h|` it leaves, and `Environment::sinkage_at` for the same foot at
/// the same point, asked once the readbacks have seen the ground.
fn pressed_and_asked(snow_m: f32, swe_m: f32, wetness: f32, sediment_m: f32) -> (f32, Option<f32>) {
    let mut sim = Sim::new();
    sim.lay_snow(snow_m, swe_m);
    sim.lay_wetness(wetness);
    sim.lay_sediment(sediment_m);
    sim.spawn_imprint(Vec3::new(0.0, snow_m - 0.05, 4.0));
    sim.frames(60);
    let (h, _, _) = sim.field();
    let asked = {
        use bevy::ecs::system::RunSystemOnce;
        sim.app
            .world_mut()
            .run_system_once(|env: Environment| env.sinkage_at(Vec2::new(0.0, 4.0), FOOT, LOAD_N))
            .expect("the query system runs")
    };
    sim.finish();
    (-h.iter().copied().fold(0.0f32, f32::min), asked)
}

/// #39 acceptance 3: `Environment::sinkage_at` answers what the kernel then presses, in fresh
/// snow, on soil at wetness 0.5, on dry soil and on dry sand. The field's deepest texel sits
/// 1.5–3.6 % under the target (the kernel's relaxation and the stamp's own profile on 5 cm
/// texels: 3.5 mm in snow, 1.2 mm on sand, under 1 mm on soil), never over it.
#[test]
#[ignore = "needs a GPU"]
fn the_depth_a_step_would_press_is_the_depth_it_presses() {
    let _lock = serial();
    let cases = [
        ("snow", SNOW_M, SNOW_SWE_M, 0.0, 0.0),
        ("dry soil", 0.0, 0.0, 0.0, 0.0),
        ("wet soil", 0.0, 0.0, 0.5, 0.0),
        ("dry sand", 0.0, 0.0, 0.0, 1.0),
    ];
    let mut misses = Vec::new();
    for (name, snow, swe, wet, sediment) in cases {
        let (pressed, asked) = pressed_and_asked(snow, swe, wet, sediment);
        println!("{name}: pressed {pressed:.4} m, asked {asked:?}");
        let asked = asked.unwrap_or_else(|| panic!("{name}: no answer"));
        misses.push((name, pressed, asked));
    }
    for (name, pressed, asked) in misses {
        assert!(
            pressed <= asked + 0.0002 && pressed >= 0.96 * asked,
            "{name}: pressed {pressed} vs asked {asked}"
        );
    }
}
