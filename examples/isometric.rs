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
/// The walker's torch: how far in front of them and how high it hangs, m, and how far it lights.
const TORCH_FORWARD_M: f32 = 0.45;
const TORCH_HEIGHT_M: f32 = 1.5;
const TORCH_RANGE_M: f32 = 14.0;
/// The distance from the torch to the patch of ground it lights, m: the torch hangs 1.5 m up
/// and reaches 1.2 m forward, so the ground it lights is 1.92 m away.
const TORCH_GROUND_M: f32 = 1.92;
/// The torch's cone: the half-angle at which it is still at full strength and the half-angle
/// at which it has fallen to nothing, radians.
const TORCH_INNER_ANGLE: f32 = 0.5;
const TORCH_OUTER_ANGLE: f32 = 1.0;
/// The street lamp over the town's square: how high it hangs, m, and how far it lights, m.
/// It hangs under the roofs (the tallest is 7 m), so its pool falls on the square's paving and
/// not on a roof.
const LAMP_HEIGHT_M: f32 = 4.5;
const LAMP_RANGE_M: f32 = 22.0;
/// Where the street lamp stands, m: on the square, clear of the walker's loop.
const LAMP_AT: Vec2 = Vec2::new(0.0, 0.0);
/// The scene-linear value the ground under a light reads at, the game's own art direction.
///
/// A scene-linear value of 0.18 is 18 % grey — the middle of the picture, wherever the hour is
/// (the crate meters its exposure so that an 18 % grey surface reads at 0.18 at noon and at
/// midnight alike), so these are *picture* values and not physics: the torch's pool is a
/// readable step below the lamp's, which is the square's pool of light.
const TORCH_DISPLAY: f32 = 0.10;
const LAMP_DISPLAY: f32 = 0.30;
/// The scene-linear value the lamp's own head reads at: the brightest thing in the frame, a
/// step above the pool it makes.
const LAMP_GLOW_DISPLAY: f32 = 8.0;
/// The albedo of the paving the lights fall on.
const TORCH_ALBEDO: f32 = 0.14;

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
        )
        // The torch and the lamp read the crate's exposure, so they run after the celestial set
        // has metered it this frame.
        .add_systems(
            Update,
            (torch, lamp, lamp_glow).after(ElementsSystems::Celestial),
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

/// The walker's torch: the game's own light, its power balanced against the crate's exposure.
#[derive(Component)]
struct Torch;

/// The street lamp over the square: the game's own light, a point source.
#[derive(Component)]
struct Lamp;

/// The lamp's own head: emissive, because it is the source and not a lit surface.
#[derive(Component)]
struct Glow;

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

/// The walker's torch: the game's own light, balanced against the crate's exposure.
///
/// The crate knows the camera's exposure and the game knows its own art direction (the
/// scene-linear value the ground under the torch should read at) and its surface's albedo, so
/// the two together give the one number Bevy's `SpotLight` wants — no guessing at the crate's
/// exposure curve, and no re-tuning when the hour or the weather changes
/// (`docs/research/game-lights.md`).
fn torch(env: Environment, mut lights: Query<&mut SpotLight, With<Torch>>) {
    for mut light in &mut lights {
        // The luminance the ground under the torch must have to read at the wanted value.
        let luminance = env.luminance_for_display(TORCH_DISPLAY);
        // The power that puts a surface of that albedo at that luminance.
        light.intensity = env.light_intensity(luminance, TORCH_GROUND_M, TORCH_ALBEDO);
        // The night's colour, so the torch and the sky are one night.
        let c = env.night_light_colour();
        light.color = Color::linear_rgb(c.x, c.y, c.z);
    }
}

/// The street lamp: the same physics, a point source, a different art direction.
fn lamp(env: Environment, mut lights: Query<&mut PointLight, With<Lamp>>) {
    for mut light in &mut lights {
        let luminance = env.luminance_for_display(LAMP_DISPLAY);
        light.intensity = env.light_intensity(luminance, LAMP_HEIGHT_M, TORCH_ALBEDO);
        let c = env.night_light_colour();
        light.color = Color::linear_rgb(c.x, c.y, c.z);
    }
}

/// The lamp's own head: the source, so it is emissive rather than lit.
///
/// Its radiance is a *scene-linear* value, like every other number in the crate, so the head
/// reads the same at every hour: a lamp's head is the brightest thing in the frame, and at
/// 8.0 it is a step above the 0.30 its pool of light reads at — bright, but not blown.
fn lamp_glow(
    env: Environment,
    mut materials: ResMut<Assets<WeatheredMaterial>>,
    glows: Query<&MeshMaterial3d<WeatheredMaterial>, With<Glow>>,
) {
    let radiance = env.luminance_for_display(LAMP_GLOW_DISPLAY);
    for handle in glows.iter().map(|m| &m.0) {
        if let Some(mut material) = materials.get_mut(handle) {
            let c = env.night_light_colour();
            material.base.emissive = LinearRgba::new(c.x, c.y, c.z, 1.0) * radiance;
        }
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

    /// A material that glows: a lamp's own head, which is the source and not a lit surface.
    fn emissive(&mut self, color: Color, radiance: f32) -> Handle<WeatheredMaterial> {
        self.materials.add(WeatheredMaterial {
            base: StandardMaterial {
                base_color: color,
                emissive: color.to_linear() * radiance,
                perceptual_roughness: 0.4,
                ..default()
            },
            extension: WeatheredExtension::with_exposure(1.0),
        })
    }

    fn cuboid(&mut self, size: Vec3, at: Vec2, material: Handle<WeatheredMaterial>) {
        self.cuboid_at(size, at, 0.0, material);
    }

    /// A box of `size` standing on the ground at `at`, or floating at `y` if that is given.
    fn cuboid_at(
        &mut self,
        size: Vec3,
        at: Vec2,
        y: f32,
        material: Handle<WeatheredMaterial>,
    ) {
        self.commands.spawn((
            Mesh3d(self.meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(material),
            Transform::from_xyz(at.x, y + size.y * 0.5, at.y),
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
    // The walker's torch, a child of the walker so it follows: the game's own light, its power
    // set each frame from the crate's exposure (`docs/research/game-lights.md`). A torch is a
    // cone, so it is a `SpotLight`: it hangs in front of the walker's chest and points down
    // and forward, at the ground ahead of them.
    let torch_at = Vec3::new(0.0, TORCH_HEIGHT_M, TORCH_FORWARD_M);
    commands.spawn((
        Torch,
        SpotLight {
            intensity: 0.0,
            range: TORCH_RANGE_M,
            inner_angle: TORCH_INNER_ANGLE,
            outer_angle: TORCH_OUTER_ANGLE,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(torch_at)
            .looking_at(torch_at + Vec3::new(0.0, -1.4, 1.2), Vec3::Y),
    ))
    .set_parent_in_place(walker);
    // A street lamp over the square: the game's own light, a point source, its power balanced
    // the same way (`docs/research/game-lights.md`). It stands at the square's edge, clear of
    // the walker's loop, so its pool falls across the paving and not on the walker's head.
    commands.spawn((
        Lamp,
        PointLight {
            intensity: 0.0,
            range: LAMP_RANGE_M,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(LAMP_AT.x, LAMP_HEIGHT_M, LAMP_AT.y),
    ));
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
    // The square's paving: a slab with grass around it, so the lamp's pool has an edge to read
    // against. A slab wide enough to catch the whole pool would flatten the frame.
    let paving = b.material(Color::srgb(0.42, 0.40, 0.37), 0.8, 0.0);
    b.cuboid(Vec3::new(14.0, 0.05, 14.0), Vec2::ZERO, paving);
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
    // The street lamp's post, so the light has a visible source in the frame. The head hangs
    // above the light, which is how a lamp is built: a source at the head's own height would
    // light the head from 0.15 m away and blow it out.
    let post = b.material(Color::srgb(0.22, 0.22, 0.24), 0.5, 0.6);
    b.cuboid(
        Vec3::new(0.16, LAMP_HEIGHT_M + 0.5, 0.16),
        LAMP_AT,
        post,
    );
    let head = b.emissive(Color::srgb(1.0, 0.97, 0.88), 1.0);
    b.commands.spawn((
        Glow,
        Mesh3d(b.meshes.add(Cuboid::new(0.7, 0.3, 0.7))),
        MeshMaterial3d(head),
        Transform::from_xyz(LAMP_AT.x, LAMP_HEIGHT_M + 0.35 + 0.15, LAMP_AT.y),
    ));
}
