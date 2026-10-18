//! Trails on a game's own ground (issue #26, `docs/research/trails-game-ground.md`, Validation
//! 1–4 and 6): no crate `Terrain`, a `TrailGround`, the snow `SurfaceWeathering`'s. An imprint
//! walked 20 m leaves a trail `Environment::trail_depth` reads, a sole presses only when it is on
//! the game's ground, a deeper snowpack fills the print, and a `WeatheredMaterial` draws the
//! print only `with_trails()`.
//!
//! Needs a GPU: `cargo test --release --test gpu_trail_ground -- --ignored --test-threads=1`.

mod support;

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use bevy::app::AppExit;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

/// The walker's sole: 77.5 kg on an ellipse 0.05 × 0.13 m (37.2 kPa).
const FOOT: Vec2 = Vec2::new(0.05, 0.13);
const LOAD_N: f32 = 760.0;
/// 10 cm of snow at 150 kg/m³: the CRREL cap 0.10 × (1 − 150/500) = 7 cm binds (Bekker 31 cm).
const SNOW_M: f32 = 0.10;
const SNOW_DENSITY: f32 = 150.0;
const EXPECTED_DEPTH_M: f32 = 0.07;

const SIZE: u32 = 256;

struct Sim {
    app: App,
    /// What the camera renders into (readable).
    image: Handle<Image>,
}

impl Sim {
    fn new(ground: TrailGround) -> Self {
        Self::with_camera(
            ground,
            Transform::from_xyz(0.0, 3.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 4.0), Vec3::Y),
        )
    }

    fn with_camera(ground: TrailGround, eye: Transform) -> Self {
        let mut target = Image::new_target_texture(
            SIZE,
            SIZE,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            None,
        );
        // The readback copies out of the texture.
        target.texture_descriptor.usage |= bevy::render::render_resource::TextureUsages::COPY_SRC;
        let mut app = App::new();
        app.add_plugins((
            support::headless_plugins(),
            ElementsPlugins
                .build()
                .disable::<PrecipitationPlugin>()
                .disable::<LightningPlugin>()
                .disable::<CloudPlugin>(),
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 1, 15)).at_hour(12.0)
        })
        .insert_resource(ground)
        .insert_resource(SurfaceWeathering::with_snowpack(SNOW_M, SNOW_DENSITY, 0.0));
        support::finish(&mut app);
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(target);
        app.world_mut().spawn((
            Camera3d::default(),
            bevy::camera::RenderTarget::Image(image.clone().into()),
            bevy::core_pipeline::tonemapping::DebandDither::Disabled,
            ElementsCamera::default(),
            eye,
        ));
        app.world_mut()
            .resource_mut::<WeatherDirector>()
            .force(WeatherPreset::Clear, Duration::ZERO);
        let mut sim = Self { app, image };
        sim.run_until("the trail field", |app| {
            app.world().get_resource::<TrailState>().is_some()
        });
        sim.frames(30);
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

    fn spawn_imprint(&mut self, at: Vec3) -> Entity {
        self.app
            .world_mut()
            .spawn((Imprint::new(FOOT, LOAD_N), Transform::from_translation(at)))
            .id()
    }

    /// Press a still sole at `at` for 20 frames and lift it.
    fn press(&mut self, at: Vec3) {
        let foot = self.spawn_imprint(at);
        self.frames(20);
        self.app.world_mut().despawn(foot);
        self.frames(4);
    }

    fn trail_depth(&mut self, xz: Vec2) -> Option<f32> {
        self.app
            .world_mut()
            .run_system_once(move |env: Environment| env.trail_depth(xz))
            .unwrap_or_else(|e| panic!("the query runs: {e}"))
    }

    /// Frames for a second of real time (four readback intervals: the last readback was asked
    /// for after the last carve and has landed), then the depth at each point.
    fn fresh_depths(&mut self, points: &[Vec2]) -> Vec<Option<f32>> {
        let until = std::time::Instant::now() + Duration::from_secs(1);
        while std::time::Instant::now() < until {
            self.app.update();
        }
        self.frames(4);
        points.iter().map(|p| self.trail_depth(*p)).collect()
    }

    /// The camera's image, RGBA8 sRGB.
    fn render(&mut self) -> Vec<u8> {
        self.frames(6);
        support::read_texture(
            &mut self.app,
            self.image.clone(),
            SIZE as usize * 4,
            SIZE as usize,
        )
    }

    fn finish(mut self) {
        self.app.world_mut().write_message(AppExit::Success);
        self.app.update();
    }
}

/// Validation 1, the issue's acceptance: a sole walked 20 m over a game's flat ground under 10 cm
/// of snow leaves a trail deeper than 5 cm all along it, read through `Environment::trail_depth`,
/// and no `Terrain` exists.
#[test]
#[ignore = "needs a GPU"]
fn an_imprint_walked_20_m_over_game_ground_leaves_a_trail() {
    let _lock = serial();
    let mut sim = Sim::new(TrailGround::flat(0.0));
    let sole_y = SNOW_M - 0.03;
    let foot = sim.spawn_imprint(Vec3::new(-10.0, sole_y, 4.0));
    // 20 m at 3 m/s, 5 cm a frame.
    for frame in 0..=400 {
        let x = -10.0 + 0.05 * frame as f32;
        sim.app
            .world_mut()
            .entity_mut(foot)
            .insert(Transform::from_xyz(x, sole_y, 4.0));
        sim.app.update();
    }
    sim.app.world_mut().despawn(foot);
    let path: Vec<Vec2> = (0..=38)
        .map(|i| Vec2::new(-9.5 + 0.5 * i as f32, 4.0))
        .collect();
    let mut points = path.clone();
    points.push(Vec2::new(0.0, 8.0));
    points.push(Vec2::new(200.0, 4.0));
    let depths = sim.fresh_depths(&points);
    let along: Vec<f32> = depths[..path.len()]
        .iter()
        .map(|d| d.unwrap_or(0.0))
        .collect();
    let shallowest = along.iter().copied().fold(f32::INFINITY, f32::min);
    println!(
        "trail depth along 20 m: shallowest {shallowest:.4} m, deepest {:.4} m (model {EXPECTED_DEPTH_M}); beside {:?}; far away {:?}",
        along.iter().copied().fold(0.0f32, f32::max),
        depths[path.len()],
        depths[path.len() + 1],
    );
    assert!(
        shallowest > 0.05,
        "the trail is shallower than 5 cm: {along:?}"
    );
    assert!(
        along
            .iter()
            .all(|d| (d - EXPECTED_DEPTH_M).abs() <= 0.1 * EXPECTED_DEPTH_M),
        "the trail is not the model's depth: {along:?}"
    );
    assert_eq!(depths[path.len()], Some(0.0), "untouched snow beside it");
    assert_eq!(depths[path.len() + 1], None, "outside the window");
    let terrains = sim
        .app
        .world_mut()
        .query::<&Terrain>()
        .iter(sim.app.world())
        .count();
    assert_eq!(terrains, 0, "no terrain is spawned");
    sim.finish();
}

/// Validation 2: on a game's sloped ground a sole that follows the slope presses, and one 0.3 m
/// above it presses nothing.
#[test]
#[ignore = "needs a GPU"]
fn a_sole_presses_only_on_the_game_ground() {
    let _lock = serial();
    let slope = |xz: Vec2| 0.1 * xz.x;
    let mut sim = Sim::new(TrailGround::new(slope));
    let on = Vec2::new(4.0, 4.0);
    let above = Vec2::new(-4.0, 4.0);
    sim.press(Vec3::new(on.x, slope(on) + SNOW_M - 0.03, on.y));
    sim.press(Vec3::new(above.x, slope(above) + SNOW_M + 0.3, above.y));
    let depths = sim.fresh_depths(&[on, above]);
    println!(
        "on the slope {:?}, 0.3 m above it {:?}",
        depths[0], depths[1]
    );
    assert!(
        depths[0].is_some_and(|d| (d - EXPECTED_DEPTH_M).abs() <= 0.1 * EXPECTED_DEPTH_M),
        "the sole on the ground: {:?}",
        depths[0]
    );
    assert_eq!(depths[1], Some(0.0), "the sole in the air pressed");
    sim.finish();
}

/// Validation 3: 5 cm more snow in the point pack shallows a print by 5 cm.
#[test]
#[ignore = "needs a GPU"]
fn a_deeper_snowpack_fills_a_print_on_game_ground() {
    let _lock = serial();
    let mut sim = Sim::new(TrailGround::flat(0.0));
    let at = Vec2::new(0.0, 4.0);
    sim.press(Vec3::new(at.x, SNOW_M - 0.03, at.y));
    let before = sim.fresh_depths(&[at])[0].unwrap_or(0.0);
    sim.app
        .world_mut()
        .insert_resource(SurfaceWeathering::with_snowpack(
            SNOW_M + 0.05,
            SNOW_DENSITY,
            0.0,
        ));
    sim.frames(10);
    let after = sim.fresh_depths(&[at])[0].unwrap_or(0.0);
    println!("print {before:.4} m deep, {after:.4} m after 5 cm of snow");
    assert!(
        ((before - after) - 0.05).abs() <= 0.005,
        "the print shallowed by {}",
        before - after
    );
    sim.finish();
}

/// The colour at pixel (x, y) of an image read with `Sim::render`.
fn pixel(image: &[u8], x: usize, y: usize) -> [u8; 3] {
    let i = (y * SIZE as usize + x) * 4;
    [image[i], image[i + 1], image[i + 2]]
}

fn max_diff(a: [u8; 3], b: [u8; 3]) -> u8 {
    (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0)
}

/// Validation 6: seen from straight above, a print in a snowed `WeatheredMaterial` ground drawn
/// `with_trails()` differs from the flat snow beside it; on the same ground without it, the print
/// pressed there does not show.
#[test]
#[ignore = "needs a GPU"]
fn a_weathered_ground_draws_a_print_only_with_trails() {
    let _lock = serial();
    let mut sim = Sim::with_camera(
        TrailGround::flat(0.0),
        Transform::from_xyz(0.0, 4.0, 0.0).looking_at(Vec3::ZERO, Vec3::NEG_Z),
    );
    let plane = sim
        .app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Plane3d::default().mesh().size(1.6, 1.6));
    for (x, trails) in [(-1.0, true), (1.0, false)] {
        let extension = if trails {
            WeatheredExtension::default().with_trails()
        } else {
            WeatheredExtension::default()
        };
        let material = sim
            .app
            .world_mut()
            .resource_mut::<Assets<WeatheredMaterial>>()
            .add(WeatheredMaterial {
                base: StandardMaterial {
                    base_color: Color::srgb(0.3, 0.3, 0.3),
                    perceptual_roughness: 0.9,
                    ..default()
                },
                extension,
            });
        sim.app.world_mut().spawn((
            Mesh3d(plane.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
    sim.press(Vec3::new(-1.0, SNOW_M - 0.03, 0.0));
    sim.press(Vec3::new(1.0, SNOW_M - 0.03, 0.0));
    let image = sim.render();
    // 4 m above with a 45° field of view: 1.66 m from the centre to the edge, 77 px a metre.
    let px = |x: f32, z: f32| {
        let k = 0.5 * SIZE as f32 / (4.0 * (std::f32::consts::FRAC_PI_8).tan());
        (
            (0.5 * SIZE as f32 + x * k) as usize,
            (0.5 * SIZE as f32 + z * k) as usize,
        )
    };
    let at = |x: f32, z: f32| {
        let (u, v) = px(x, z);
        pixel(&image, u, v)
    };
    let (drawn, beside) = (at(-1.0, 0.0), at(-1.0, 0.5));
    let (hidden, flat) = (at(1.0, 0.0), at(1.0, 0.5));
    println!(
        "with trails: print {drawn:?}, beside {beside:?}; without: print {hidden:?}, beside {flat:?}"
    );
    assert!(max_diff(drawn, beside) >= 8, "the print does not show");
    assert!(
        max_diff(hidden, flat) <= 2,
        "a print shows without with_trails()"
    );
    sim.finish();
}
