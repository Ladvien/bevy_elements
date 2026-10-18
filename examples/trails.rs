//! Trails: a walker and a four-wheeled vehicle press tracks into snow, sand or mud.
//!
//! `cargo run --release --example trails`
//!
//! `ELEMENTS_GROUND=snow` (default: the hills under forced snow; grow it before the first frame
//! with `ELEMENTS_GROW_HOURS=24`, then the sky clears), `sand` (the dunes preset, dry) or `mud`
//! (the hills after rain: `ELEMENTS_GROW_HOURS=6`). The walker
//! circles 6 m around the scene's centre, planting a foot every half second (an ellipse
//! 0.10 × 0.26 m, 760 N); the vehicle (four wheels, 0.20 × 0.32 m, 3400 N each) drives a 14 m loop
//! at 4 m/s. `ELEMENTS_HOUR` holds an hour (default 10.5), `ELEMENTS_EYE=x,y,z` places the camera
//! that many metres above the ground (default `0,2.2,-11`), `ELEMENTS_LOOK=x,z` aims it (default
//! the centre), `ELEMENTS_PRESET` replaces the weather after the ground is grown (e.g. `Snow` to
//! watch new snow fill the tracks, with `ELEMENTS_SCALE=600` for ten game minutes a second).
//! `ELEMENTS_FRAMES` and `ELEMENTS_SHOT` work as in the other examples.

mod common;

use std::f32::consts::TAU;
use std::time::Duration;

use bevy::prelude::*;
use bevy_elements::*;
use common::{ExampleHarness, camera};

/// The loops' centre, world XZ.
const CENTRE: Vec2 = Vec2::new(0.0, 0.0);
const WALK_RADIUS_M: f32 = 6.0;
const WALK_SPEED_MS: f32 = 1.4;
const STEP_S: f32 = 0.5;
const STEP_WIDTH_M: f32 = 0.11;
const FOOT_HALF: Vec2 = Vec2::new(0.05, 0.13);
const FOOT_LOAD_N: f32 = 760.0;
const DRIVE_RADIUS_M: f32 = 14.0;
const DRIVE_SPEED_MS: f32 = 4.0;
const WHEELBASE_M: f32 = 2.6;
const TRACK_M: f32 = 1.5;
const WHEEL_HALF: Vec2 = Vec2::new(0.10, 0.16);
const WHEEL_LOAD_N: f32 = 3400.0;

#[derive(Clone, Copy, PartialEq)]
enum Ground {
    Snow,
    Sand,
    Mud,
}

#[derive(Resource)]
struct Scene {
    ground: Ground,
    preset: Option<WeatherPreset>,
}

/// A foot of the walker: left (-1) or right (+1).
#[derive(Component)]
struct Foot(f32);

/// A wheel: (+1 front, -1 rear), (-1 left, +1 right).
#[derive(Component)]
struct Wheel(f32, f32);

#[derive(Component)]
struct Walker;

#[derive(Component)]
struct Vehicle;

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_vec(name: &str) -> Option<Vec<f32>> {
    std::env::var(name)
        .ok()
        .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
}

fn main() {
    let ground = match std::env::var("ELEMENTS_GROUND").as_deref() {
        Ok("sand") => Ground::Sand,
        Ok("mud") => Ground::Mud,
        _ => Ground::Snow,
    };
    let preset = std::env::var("ELEMENTS_PRESET").ok().and_then(|name| {
        WeatherPreset::ALL
            .into_iter()
            .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
    });
    // Snow in January; sand and mud in June, so the rain stays rain.
    let date = if ground == Ground::Snow {
        CivilDate::new(2026, 1, 15)
    } else {
        CivilDate::new(2026, 6, 15)
    };
    let clock = WorldClock::new(date)
        .at_hour(f64::from(env_f32("ELEMENTS_HOUR", 10.5)))
        .with_time_scale(env_f32("ELEMENTS_SCALE", 1.0));
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(clock)
        .insert_resource(if ground == Ground::Sand {
            Climate::desert()
        } else {
            Climate::temperate()
        })
        .insert_resource(ErosionSettings {
            enabled: false,
            ..default()
        })
        .insert_resource(Scene { ground, preset })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (settle_weather, walk, drive).after(ElementsSystems::Readback),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    scene: Res<Scene>,
    mut director: ResMut<WeatherDirector>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Terrain::procedural(match scene.ground {
        Ground::Sand => ProceduralTerrain::dunes(7),
        _ => ProceduralTerrain::hills(42),
    }));
    // The ground is grown under its weather (`ELEMENTS_GROW_HOURS` in the harness), then the
    // sky clears in `settle_weather`.
    director.force(
        match scene.ground {
            Ground::Snow => WeatherPreset::Snow,
            Ground::Sand => WeatherPreset::Clear,
            Ground::Mud => WeatherPreset::Rain,
        },
        Duration::ZERO,
    );
    let eye = env_vec("ELEMENTS_EYE").unwrap_or_else(|| vec![0.0, 2.2, -11.0]);
    let look = env_vec("ELEMENTS_LOOK").unwrap_or_else(|| vec![CENTRE.x, CENTRE.y]);
    let (eye, look) = (
        Vec3::new(eye[0], 100.0 + eye[1], eye[2]),
        Vec3::new(look[0], 100.0, look[1]),
    );
    commands.spawn((camera(eye, look), EyeHeight(eye.y - 100.0), Aim(look.xz())));

    let body = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.27, 0.32),
        perceptual_roughness: 0.8,
        ..default()
    });
    commands.spawn((
        Walker,
        Mesh3d(meshes.add(Capsule3d::new(0.22, 1.2))),
        MeshMaterial3d(body.clone()),
        Transform::from_xyz(0.0, -100.0, 0.0),
    ));
    for side in [-1.0, 1.0] {
        commands.spawn((Foot(side), Imprint::new(FOOT_HALF, FOOT_LOAD_N)));
    }
    let paint = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.12, 0.08),
        perceptual_roughness: 0.4,
        ..default()
    });
    commands.spawn((
        Vehicle,
        Mesh3d(meshes.add(Cuboid::new(1.7, 1.1, 3.6))),
        MeshMaterial3d(paint),
        Transform::from_xyz(0.0, -100.0, 0.0),
    ));
    for front in [-1.0, 1.0] {
        for side in [-1.0, 1.0] {
            commands.spawn((Wheel(front, side), Imprint::new(WHEEL_HALF, WHEEL_LOAD_N)));
        }
    }
}

/// The camera's height above the ground and the point it looks at, applied once the terrain's
/// heights are read back.
#[derive(Component)]
struct EyeHeight(f32);

#[derive(Component)]
struct Aim(Vec2);

/// The surface's world height (terrain plus snow) at XZ, once read back.
fn surface(env: &Environment, xz: Vec2) -> Option<f32> {
    Some(env.terrain_height(xz)? + env.snow_depth(xz).unwrap_or(0.0))
}

/// Once the ground is grown: the camera settles, and the sky clears (or turns to
/// `ELEMENTS_PRESET`).
fn settle_weather(
    mut commands: Commands,
    env: Environment,
    scene: Res<Scene>,
    mut director: ResMut<WeatherDirector>,
    mut cameras: Query<(Entity, &mut Transform, &EyeHeight, &Aim)>,
    mut done: Local<bool>,
) {
    for (entity, mut transform, eye, aim) in &mut cameras {
        let (Some(here), Some(there)) = (
            surface(&env, transform.translation.xz()),
            surface(&env, aim.0),
        ) else {
            continue;
        };
        transform.translation.y = here + eye.0;
        transform.look_at(Vec3::new(aim.0.x, there + 0.3, aim.0.y), Vec3::Y);
        commands.entity(entity).remove::<(EyeHeight, Aim)>();
        if !*done {
            *done = true;
            director.force(scene.preset.unwrap_or(WeatherPreset::Clear), Duration::ZERO);
        }
    }
}

/// A point on a circle of `radius` round the centre at arc length `s`, and the direction along it.
fn on_circle(radius: f32, s: f32) -> (Vec2, Vec2) {
    let a = s / radius;
    let (sin, cos) = a.sin_cos();
    (CENTRE + radius * Vec2::new(cos, sin), Vec2::new(-sin, cos))
}

/// The walker: each foot is planted for a step and swings through the next; a planted foot sits
/// 5 cm below the surface the readback reports, so it is in contact whatever the readback's lag.
fn walk(
    env: Environment,
    time: Res<Time<Virtual>>,
    mut feet: Query<(&Foot, &mut Transform), Without<Walker>>,
    mut body: Query<&mut Transform, With<Walker>>,
) {
    let t = time.elapsed_secs();
    let s = WALK_SPEED_MS * t;
    let (centre, _) = on_circle(WALK_RADIUS_M, s);
    if let (Ok(mut b), Some(y)) = (body.single_mut(), surface(&env, centre)) {
        b.translation = Vec3::new(centre.x, y + 0.85, centre.y);
    }
    let step = WALK_SPEED_MS * STEP_S;
    for (foot, mut transform) in &mut feet {
        // Left plants on even steps, right on odd ones; a foot plants where the body will be
        // half a step later, and swings over the next step.
        let phase = t / STEP_S + if foot.0 < 0.0 { 0.0 } else { 1.0 };
        let index = (phase / 2.0).floor();
        let within = phase - 2.0 * index;
        let planted = within < 1.0;
        let plant_s = (2.0 * index * STEP_S - if foot.0 < 0.0 { 0.0 } else { STEP_S })
            * WALK_SPEED_MS
            + 0.5 * step;
        let (at, along) = if planted {
            on_circle(WALK_RADIUS_M, plant_s)
        } else {
            let k = within - 1.0;
            on_circle(WALK_RADIUS_M, plant_s + 2.0 * step * k)
        };
        let side = Vec2::new(along.y, -along.x) * foot.0 * STEP_WIDTH_M;
        let xz = at + side;
        let Some(y) = surface(&env, xz) else {
            continue;
        };
        let lift = if planted {
            -0.05
        } else {
            0.3 * (TAU * 0.5 * (within - 1.0)).sin().abs() + 0.3
        };
        *transform = Transform::from_xyz(xz.x, y + lift, xz.y)
            .looking_to(Vec3::new(along.x, 0.0, along.y), Vec3::Y);
    }
}

/// The vehicle: four wheels on a circle, each pressing where it is.
fn drive(
    env: Environment,
    time: Res<Time<Virtual>>,
    mut wheels: Query<(&Wheel, &mut Transform), Without<Vehicle>>,
    mut body: Query<&mut Transform, With<Vehicle>>,
) {
    let s = DRIVE_SPEED_MS * time.elapsed_secs();
    let (centre, along) = on_circle(DRIVE_RADIUS_M, s);
    let side = Vec2::new(along.y, -along.x);
    if let (Ok(mut b), Some(y)) = (body.single_mut(), surface(&env, centre)) {
        *b = Transform::from_xyz(centre.x, y + 0.85, centre.y)
            .looking_to(Vec3::new(along.x, 0.0, along.y), Vec3::Y);
    }
    for (wheel, mut transform) in &mut wheels {
        let xz = centre + along * wheel.0 * 0.5 * WHEELBASE_M + side * wheel.1 * 0.5 * TRACK_M;
        let Some(y) = surface(&env, xz) else {
            continue;
        };
        *transform = Transform::from_xyz(xz.x, y - 0.05, xz.y)
            .looking_to(Vec3::new(along.x, 0.0, along.y), Vec3::Y);
    }
}
