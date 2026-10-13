//! Shared example scaffolding: a free-fly camera, clock and weather keys, a status line, and the
//! `ELEMENTS_FRAMES` / `ELEMENTS_SHOT` switches used for unattended runs.
//!
//! - `ELEMENTS_FRAMES=N` exits after N frames.
//! - `ELEMENTS_SHOT=/tmp/x.png` saves a screenshot of the primary window on the last frame, then exits.
//!
//! Keys: WASD + Q/E move (Shift ×5), hold the right mouse button to look, `[`/`]` halve/double the
//! time scale, `T` +1 hour, `1`–`0` force a weather preset, `R` release to the scheduler.

#![allow(dead_code)]

mod ground;

use std::time::Duration;

use ground::GroundPlane;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy_elements::*;

/// Adds the harness to an example.
pub struct ExampleHarness;

/// The free-fly camera.
#[derive(Component, Default)]
pub struct FlyCamera {
    yaw: f32,
    pitch: f32,
}

impl FlyCamera {
    /// Start looking along yaw/pitch, in radians.
    pub fn looking(yaw: f32, pitch: f32) -> Self {
        Self { yaw, pitch }
    }
}

#[derive(Resource)]
struct RunConfig {
    frames: Option<u32>,
    shot: Option<String>,
    frame: u32,
    shot_taken: bool,
}

#[derive(Component)]
struct StatusLine;

impl Plugin for ExampleHarness {
    fn build(&self, app: &mut App) {
        let frames = std::env::var("ELEMENTS_FRAMES")
            .ok()
            .and_then(|v| v.parse().ok());
        let shot = std::env::var("ELEMENTS_SHOT").ok();
        app.insert_resource(RunConfig {
            frames,
            shot,
            frame: 0,
            shot_taken: false,
        })
        .add_systems(Startup, spawn_status)
        .add_systems(
            Update,
            (fly, clock_keys, weather_keys).before(ElementsSystems::Clock),
        )
        .add_systems(
            Update,
            (settle_on_ground, status, run_limit).after(ElementsSystems::Readback),
        );
        ground::plugin(app);
    }
}

/// The standard example camera: an `ElementsCamera` that flies.
pub fn camera(position: Vec3, look_at: Vec3) -> impl Bundle {
    let dir = (look_at - position).normalize_or(Vec3::NEG_Z);
    let yaw = (-dir.x).atan2(-dir.z);
    let pitch = dir.y.clamp(-1.0, 1.0).asin();
    (
        Camera3d::default(),
        ElementsCamera::default(),
        FlyCamera::looking(yaw, pitch),
        Transform::from_translation(position).with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw,
            pitch,
            0.0,
        )),
    )
}

/// Marks a camera that moves to `height` metres above the terrain at its XZ once the terrain's
/// heights have been read back (the heights are not known on the first frames).
#[derive(Component)]
pub struct StandOnGround {
    height: f32,
}

/// See [`StandOnGround`].
pub fn stand_on_ground(height: f32) -> StandOnGround {
    StandOnGround { height }
}

fn settle_on_ground(
    mut commands: Commands,
    env: Environment,
    mut cameras: Query<(Entity, &mut Transform, &StandOnGround)>,
) {
    for (entity, mut transform, stand) in &mut cameras {
        if let Some(ground) = env.terrain_height(transform.translation.xz()) {
            transform.translation.y = ground + stand.height;
            commands.entity(entity).remove::<StandOnGround>();
        }
    }
}

fn fly(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time<Real>>,
    mut cameras: Query<(&mut Transform, &mut FlyCamera)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut fly) in &mut cameras {
        if mouse.pressed(MouseButton::Right) {
            fly.yaw -= motion.delta.x * 0.003;
            fly.pitch = (fly.pitch - motion.delta.y * 0.003).clamp(-1.54, 1.54);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
        }
        let mut wish = Vec3::ZERO;
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();
        for (key, dir) in [
            (KeyCode::KeyW, forward),
            (KeyCode::KeyS, -forward),
            (KeyCode::KeyD, right),
            (KeyCode::KeyA, -right),
            (KeyCode::KeyE, Vec3::Y),
            (KeyCode::KeyQ, -Vec3::Y),
        ] {
            if keys.pressed(key) {
                wish += dir;
            }
        }
        let speed = if keys.pressed(KeyCode::ShiftLeft) {
            60.0
        } else {
            12.0
        };
        transform.translation += wish.normalize_or_zero() * speed * dt;
    }
}

fn clock_keys(keys: Res<ButtonInput<KeyCode>>, mut clock: ResMut<WorldClock>) {
    if keys.just_pressed(KeyCode::BracketLeft) {
        clock.time_scale *= 0.5;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        clock.time_scale *= 2.0;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        clock.advance_hours(1.0);
    }
}

fn weather_keys(keys: Res<ButtonInput<KeyCode>>, mut director: ResMut<WeatherDirector>) {
    const DIGITS: [KeyCode; 10] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Digit0,
    ];
    for (key, preset) in DIGITS.into_iter().zip(WeatherPreset::ALL) {
        if keys.just_pressed(key) {
            director.force(preset, Duration::from_secs(120));
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        director.release(Duration::from_secs(120));
    }
}

fn spawn_status(mut commands: Commands) {
    commands.spawn((
        StatusLine,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
    ));
}

/// A 2 km ground plane (given per-pixel cloud shadows by the harness) and a ring of boxes,
/// columns and spheres to read shadows and fog depth.
pub fn test_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        GroundPlane,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(2000.0, 2000.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.30, 0.36, 0.22),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.53, 0.50),
        perceptual_roughness: 0.8,
        ..default()
    });
    let white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.85),
        perceptual_roughness: 0.4,
        ..default()
    });
    let cube = meshes.add(Cuboid::new(6.0, 6.0, 6.0));
    let column = meshes.add(Cylinder::new(1.5, 24.0));
    let ball = meshes.add(Sphere::new(4.0));
    for i in 0..24 {
        let angle = i as f32 / 24.0 * std::f32::consts::TAU;
        let radius = 40.0 + (i % 4) as f32 * 60.0;
        let at = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let (mesh, material, y) = match i % 3 {
            0 => (cube.clone(), stone.clone(), 3.0),
            1 => (column.clone(), stone.clone(), 12.0),
            _ => (ball.clone(), white.clone(), 4.0),
        };
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(at + Vec3::Y * y),
        ));
    }
}

fn status(
    env: Environment,
    director: Res<WeatherDirector>,
    mut text: Query<&mut Text, With<StatusLine>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let clock = env.clock();
    let weather = env.weather();
    let sun = env.sun();
    text.0 = format!(
        "{:04}-{:02}-{:02} {:05.2} h x{}  |  {:?}{}  |  {:.1} C  cover {:.2}  {:.1} mm/h  wind {:.1} m/s  vis {:.0} m  |  sun {:.1} deg  moon {:.1} deg ({:.0}%)",
        clock.date.year,
        clock.date.month,
        clock.date.day,
        clock.hour,
        clock.time_scale,
        director.current(),
        if director.is_forced() {
            " (forced)"
        } else {
            ""
        },
        env.temperature_c(),
        weather.cloud_cover,
        weather.precipitation_mm_h,
        weather.wind_speed_ms,
        weather.visibility_m,
        sun.altitude_deg,
        sun.moon_altitude_deg,
        sun.moon_illuminated_fraction * 100.0,
    );
}

fn run_limit(
    mut config: ResMut<RunConfig>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    config.frame += 1;
    let Some(limit) = config.frames else {
        return;
    };
    if config.frame < limit {
        return;
    }
    match config.shot.clone() {
        Some(path) if !config.shot_taken => {
            config.shot_taken = true;
            let mut save = save_to_disk(path);
            commands.spawn(Screenshot::primary_window()).observe(
                move |captured: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                    save(captured);
                },
            );
        }
        // Waiting for the screenshot; give up after two seconds' worth of frames.
        Some(_) if config.frame < limit + 120 => {}
        _ => {
            exit.write(AppExit::Success);
        }
    }
}
