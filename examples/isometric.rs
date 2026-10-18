//! An isometric town under a fixed orthographic camera that stands 1500 m back from a walker and
//! follows it: `ElementsFocus::Entity` puts the rain and snow, the wind, the ground fog and the
//! strikes around the walker rather than around the far-away camera, and the clouds overhead are
//! marched along the camera's parallel rays (`docs/research/view-anchor.md`).
//!
//! `cargo run --release --example isometric`
//!
//! The weather cycles Rain, Thunderstorm, Snow and PartlyCloudy, 30 s each of real time, at a
//! clock 30 times real time. `ELEMENTS_PRESET=Snow` (or any preset) fixes it,
//! `ELEMENTS_HOUR=14` starts the clock at that hour, `ELEMENTS_SCALE=300` changes the time scale,
//! `ELEMENTS_ZOOM=60` sets the view's height in metres (default 40), `ELEMENTS_PITCH=35` the
//! camera's angle below the horizon (default 30), and `ELEMENTS_PRECIPITATION=readable` draws the
//! rain and snow in `PrecipitationLook::readable()` (`docs/research/precipitation-look.md`).
//! `ELEMENTS_LOOK=game`
//! keeps a game's own look (`docs/research/integration.md`): its own warm key light, a flat
//! ambient and a plain clear colour, no physical sky, fog or exposure from the crate, and the
//! crate's weather on top (`ElementsCamera::bare()` with `msaa_off`, `CelestialSettings` off).
//! For a still: `ELEMENTS_PRESET=Rain ELEMENTS_FRAMES=600 ELEMENTS_SHOT=/tmp/x.png`.
//!
//! The town's ground is the game's own (a plain box in `WeatheredMaterial`, no crate `Terrain`):
//! a `TrailGround` at y = 0 and the ground's material `with_trails()` let the walker's feet press
//! prints into it (`docs/research/trails-game-ground.md`), in a window of 8 cm texels wide enough
//! for the view. `ELEMENTS_SNOWPACK=0.15` starts the scene under that many metres of snow
//! (`SurfaceWeathering::with_snowpack`), e.g. with `ELEMENTS_PRESET=Clear ELEMENTS_HOUR=7.5` for a
//! dawn after a snowfall.

mod common;

use std::time::Duration;

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy_elements::*;
use common::ExampleHarness;

/// Each preset of the cycle, s of real time.
const PRESET_S: f32 = 30.0;
const CYCLE: [WeatherPreset; 4] = [
    WeatherPreset::Rain,
    WeatherPreset::Thunderstorm,
    WeatherPreset::Snow,
    WeatherPreset::PartlyCloudy,
];
/// How far back the camera stands from the walker, m: far enough that a camera-centred domain
/// would sit far off screen.
const BACK_M: f32 = 1500.0;
/// The walker's loop around the town, m, and its speed, m/s.
const LOOP_RADIUS_M: f32 = 14.0;
const WALK_MS: f32 = 1.4;
/// The walker's gait: a step every half second, the feet 11 cm either side of the line, each
/// sole an ellipse 0.10 × 0.26 m under 760 N (77.5 kg on one foot).
const STEP_S: f32 = 0.5;
const STEP_WIDTH_M: f32 = 0.11;
const FOOT_HALF: Vec2 = Vec2::new(0.05, 0.13);
const FOOT_LOAD_N: f32 = 760.0;

fn env_f64(name: &str) -> Option<f64> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

fn fixed_preset() -> Option<WeatherPreset> {
    let name = std::env::var("ELEMENTS_PRESET").ok()?;
    WeatherPreset::ALL
        .into_iter()
        .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
}

/// `ELEMENTS_LOOK=game`: the game's own lights and look, the crate's weather.
fn game_look() -> bool {
    std::env::var("ELEMENTS_LOOK").is_ok_and(|v| v.eq_ignore_ascii_case("game"))
}

fn main() {
    let clock = WorldClock::new(CivilDate::new(2026, 4, 15))
        .at_hour(env_f64("ELEMENTS_HOUR").unwrap_or(13.0))
        .with_time_scale(env_f64("ELEMENTS_SCALE").unwrap_or(30.0) as f32);
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(clock)
        .insert_resource(Climate::temperate())
        // The ground is the game's: trails press into it without a crate terrain, in a window
        // of 8 cm texels (82 m across) that covers the view.
        .insert_resource(TrailGround::flat(0.0))
        .insert_resource(TrailSettings {
            texel_m: Some(env_f64("ELEMENTS_TRAIL_TEXEL").unwrap_or(0.08) as f32),
            ..default()
        })
        .insert_resource(CelestialSettings {
            lights: !game_look(),
            sky: !game_look(),
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (walk, step, follow).chain().before(ElementsSystems::Clock),
        );
    if let Some(depth) = env_f64("ELEMENTS_SNOWPACK") {
        app.insert_resource(SurfaceWeathering::with_snowpack(depth as f32, 150.0, 0.0));
    }
    if fixed_preset().is_none() {
        app.add_systems(Update, cycle.before(ElementsSystems::Clock));
    }
    // A game's own bright key light makes rain nearly as bright as the sunlit ground behind it (a
    // physical fact Lagarde 2012 says games should not follow): the game brightens its streaks.
    if game_look() && std::env::var("ELEMENTS_PRECIPITATION").as_deref() == Ok("readable") {
        app.insert_resource(PrecipitationLook {
            rain_tint: LinearRgba::new(2.4, 2.5, 2.8, 0.8),
            ..PrecipitationLook::readable()
        });
    }
    app.run();
}

/// The cycle of presets.
fn cycle(
    time: Res<Time<Real>>,
    mut director: ResMut<WeatherDirector>,
    mut current: Local<Option<WeatherPreset>>,
) {
    let k = (time.elapsed_secs() / PRESET_S) as usize % CYCLE.len();
    let want = CYCLE[k];
    if *current != Some(want) {
        director.force(want, Duration::from_secs(6));
        *current = Some(want);
    }
}

/// The figure the camera follows.
#[derive(Component)]
struct Walker;

/// A foot of the walker: left (-1) or right (+1).
#[derive(Component)]
struct Foot(f32);

/// A point on the walker's loop at arc length `s`, and the direction along it.
fn on_loop(s: f32) -> (Vec2, Vec2) {
    let (sin, cos) = (s / LOOP_RADIUS_M).sin_cos();
    (LOOP_RADIUS_M * Vec2::new(cos, sin), Vec2::new(-sin, cos))
}

/// Walk the loop around the town.
fn walk(time: Res<Time>, mut walker: Query<&mut Transform, With<Walker>>) {
    let (at, along) = on_loop(time.elapsed_secs() * WALK_MS);
    for mut t in &mut walker {
        *t = Transform::from_xyz(at.x, 0.9, at.y)
            .looking_to(Vec3::new(along.x, 0.0, along.y), Vec3::Y);
    }
}

/// The feet: each is planted for a step (5 cm below the snow's surface, so it presses) and
/// swings 30 cm up through the next.
fn step(
    time: Res<Time>,
    snow: Option<Res<SurfaceWeathering>>,
    mut feet: Query<(&Foot, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    let surface = snow.map_or(0.0, |s| s.snow_depth_m);
    let stride = WALK_MS * STEP_S;
    for (foot, mut transform) in &mut feet {
        // Left plants on even steps, right on odd ones, half a step ahead of the body.
        let offset = if foot.0 < 0.0 { 0.0 } else { 1.0 };
        let phase = t / STEP_S + offset;
        let index = (phase / 2.0).floor();
        let within = phase - 2.0 * index;
        let plant = (2.0 * index - offset) * stride + 0.5 * stride;
        let planted = within < 1.0;
        let (at, along) = on_loop(if planted {
            plant
        } else {
            plant + 2.0 * stride * (within - 1.0)
        });
        let xz = at + Vec2::new(along.y, -along.x) * foot.0 * STEP_WIDTH_M;
        let lift = if planted { -0.05 } else { 0.3 };
        *transform = Transform::from_xyz(xz.x, surface + lift, xz.y)
            .looking_to(Vec3::new(along.x, 0.0, along.y), Vec3::Y);
    }
}

/// The classic isometric direction: from the south-east, 30° down (`ELEMENTS_PITCH` degrees).
fn iso_offset() -> Vec3 {
    let pitch_deg = env_f64("ELEMENTS_PITCH").unwrap_or(30.0) as f32;
    let (yaw, pitch) = (45f32.to_radians(), pitch_deg.to_radians());
    BACK_M
        * Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        )
}

/// Keep the camera at its isometric offset from the walker.
fn follow(
    walker: Query<&Transform, (With<Walker>, Without<ElementsCamera>)>,
    mut cameras: Query<&mut Transform, (With<ElementsCamera>, Without<Walker>)>,
) {
    let Ok(walker) = walker.single() else {
        return;
    };
    let at = walker.translation;
    for mut camera in &mut cameras {
        *camera = Transform::from_translation(at + iso_offset()).looking_at(at, Vec3::Y);
    }
}

struct Builder<'a> {
    commands: Commands<'a, 'a>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<WeatheredMaterial>,
}

impl Builder<'_> {
    fn material(
        &mut self,
        color: Color,
        roughness: f32,
        metallic: f32,
    ) -> Handle<WeatheredMaterial> {
        self.materials.add(WeatheredMaterial {
            base: StandardMaterial {
                base_color: color,
                perceptual_roughness: roughness,
                metallic,
                ..default()
            },
            extension: WeatheredExtension::with_exposure(1.0),
        })
    }

    fn cuboid(&mut self, size: Vec3, at: Vec2, material: Handle<WeatheredMaterial>) {
        self.commands.spawn((
            Mesh3d(self.meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(material),
            Transform::from_xyz(at.x, size.y * 0.5, at.y),
        ));
    }

    /// A house with a gable roof of two slabs tilted 35°, ridge along x.
    fn house(&mut self, size: Vec3, at: Vec2, wall: Color, roof: Color) {
        let wall = self.material(wall, 0.9, 0.0);
        self.cuboid(size, at, wall);
        let roof = self.material(roof, 0.85, 0.0);
        let theta = 35.0_f32.to_radians();
        let overhang = 0.5;
        let slab = Vec3::new(
            size.x + 2.0 * overhang,
            0.25,
            (size.z * 0.5 + overhang) / theta.cos(),
        );
        let rise = (size.z * 0.5) * theta.tan();
        for side in [-1.0_f32, 1.0] {
            self.commands.spawn((
                Mesh3d(self.meshes.add(Cuboid::new(slab.x, slab.y, slab.z))),
                MeshMaterial3d(roof.clone()),
                Transform::from_xyz(
                    at.x,
                    size.y + rise * 0.5 + 0.125 / theta.cos(),
                    at.y + side * size.z * 0.25,
                )
                .with_rotation(Quat::from_rotation_x(side * theta)),
            ));
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WeatheredMaterial>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(preset) = fixed_preset() {
        commands.queue(move |world: &mut World| {
            world
                .resource_mut::<WeatherDirector>()
                .force(preset, Duration::ZERO);
        });
    }
    let walker = commands
        .spawn((
            Walker,
            Mesh3d(meshes.add(Capsule3d::new(0.3, 1.2))),
            MeshMaterial3d(standard.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.25, 0.15),
                perceptual_roughness: 0.6,
                ..default()
            })),
            Transform::from_xyz(LOOP_RADIUS_M, 0.9, 0.0),
        ))
        .id();
    for side in [-1.0, 1.0] {
        commands.spawn((Foot(side), Imprint::new(FOOT_HALF, FOOT_LOAD_N)));
    }
    let zoom = env_f64("ELEMENTS_ZOOM").unwrap_or(40.0) as f32;
    let mut camera = commands.spawn((
        Camera3d::default(),
        ElementsFocus::Entity(walker),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: zoom,
            },
            far: 2.0 * BACK_M,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(Vec3::new(LOOP_RADIUS_M, 0.9, 0.0) + iso_offset())
            .looking_at(Vec3::new(LOOP_RADIUS_M, 0.9, 0.0), Vec3::Y),
    ));
    if game_look() {
        // The game's palette: a plain sky colour, a cool flat ambient, its own warm key light
        // with shadow cascades around the scene 1500 m in front of the camera. The crate adds
        // only the weather; it keeps `Msaa::Off`, which the precipitation pass needs.
        camera.insert((
            ElementsCamera {
                msaa_off: true,
                ..ElementsCamera::bare()
            },
            AmbientLight {
                color: Color::srgb(0.80, 0.86, 1.0),
                brightness: 900.0,
                ..default()
            },
        ));
        camera.entry::<Camera>().and_modify(|mut c| {
            c.clear_color = ClearColorConfig::Custom(Color::srgb(0.62, 0.70, 0.78));
        });
        // Marked `Sun`, so the rain and snow are lit as the town is; with
        // `CelestialSettings::lights` off the crate never touches it.
        commands.spawn((
            Sun,
            DirectionalLight {
                illuminance: 6000.0,
                color: Color::srgb(1.0, 0.92, 0.80),
                shadow_maps_enabled: true,
                ..default()
            },
            bevy::light::CascadeShadowConfigBuilder {
                num_cascades: 1,
                minimum_distance: BACK_M - 300.0,
                first_cascade_far_bound: BACK_M + 300.0,
                maximum_distance: BACK_M + 300.0,
                ..default()
            }
            .build(),
            Transform::from_xyz(-3.0, 5.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    } else {
        camera.insert(ElementsCamera::default());
    }

    let mut b = Builder {
        commands: commands.reborrow(),
        meshes: &mut meshes,
        materials: &mut materials,
    };
    // The game's own ground draws the trails pressed into it.
    let ground = b.materials.add(WeatheredMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(0.30, 0.34, 0.22),
            perceptual_roughness: 0.95,
            ..default()
        },
        extension: WeatheredExtension::default().with_trails(),
    });
    b.commands.spawn((
        Mesh3d(b.meshes.add(Cuboid::new(600.0, 0.2, 600.0))),
        MeshMaterial3d(ground),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));
    let paving = b.material(Color::srgb(0.42, 0.40, 0.37), 0.8, 0.0);
    b.cuboid(Vec3::new(10.0, 0.05, 10.0), Vec2::ZERO, paving);
    b.house(
        Vec3::new(7.0, 5.0, 6.0),
        Vec2::new(-4.0, -6.0),
        Color::srgb(0.55, 0.26, 0.20),
        Color::srgb(0.22, 0.20, 0.20),
    );
    b.house(
        Vec3::new(6.0, 7.0, 6.0),
        Vec2::new(6.0, -5.0),
        Color::srgb(0.72, 0.64, 0.48),
        Color::srgb(0.30, 0.16, 0.12),
    );
    b.house(
        Vec3::new(5.0, 3.5, 4.0),
        Vec2::new(-6.0, 5.0),
        Color::srgb(0.38, 0.27, 0.16),
        Color::srgb(0.20, 0.20, 0.22),
    );
    b.house(
        Vec3::new(8.0, 4.0, 5.0),
        Vec2::new(5.0, 6.0),
        Color::srgb(0.62, 0.58, 0.52),
        Color::srgb(0.26, 0.24, 0.26),
    );
    let metal = b.material(Color::srgb(0.65, 0.66, 0.70), 0.35, 1.0);
    b.cuboid(Vec3::new(1.5, 1.2, 1.5), Vec2::new(0.5, 0.5), metal);
}
