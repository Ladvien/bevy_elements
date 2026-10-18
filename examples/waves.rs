//! Waves: bodies disturbing a pond. A hull circles the pond, a float bobs, a pebble drops in every
//! few seconds and a wader's two legs walk along the shallows; each leaves its ripples and wakes
//! on the water (`WaterContact`, `docs/research/waves.md`).
//!
//! `cargo run --release --example waves`
//!
//! The pond is the `lake` example's basin (50 m across, 4 m deep), filled by a `WaterSource`
//! that closes when the water is 30 cm below the rim; the bodies start once it is full.
//! `ELEMENTS_HOUR` holds an hour (default 16.5), `ELEMENTS_EYE=x,y,z` places the camera (default
//! `-14,8.7,-20`), `ELEMENTS_LOOK=x,y,z` aims it (default the hull's circle), `ELEMENTS_FOLLOW=
//! right,up,behind` has it follow the hull at that offset in the hull's frame instead (default
//! `4,2.5,8` when set empty), `ELEMENTS_QUALITY` (`low`, `medium`, `high`, `ultra`; default
//! `high`) sets the window, `ELEMENTS_PRESET` the weather (default `Clear`), and
//! `ELEMENTS_BODIES=hull,float,pebble,wader` picks the bodies (default all). `ELEMENTS_WAVES=off`
//! leaves the waves out. `ELEMENTS_FRAMES` and `ELEMENTS_SHOT` work as in the other examples.

mod common;

use std::f32::consts::{PI, TAU};
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_elements::*;
use common::{ExampleHarness, camera};

/// Cells per axis and cell edge of the height map, m.
const CELLS: u32 = 256;
const CELL_M: f32 = 1.0;
/// Height range the normalised map covers, m.
const RANGE_M: f32 = 12.0;
/// The plain's height, the basin's radius and depth, m.
const PLAIN_M: f32 = 6.0;
const BASIN_RADIUS_M: f32 = 25.0;
const BASIN_DEPTH_M: f32 = 4.0;
/// The water at the centre when the tap closes, m, and so the still surface's height.
const FULL_DEPTH_M: f32 = 3.7;
const WATER_Y: f32 = PLAIN_M - BASIN_DEPTH_M + FULL_DEPTH_M;
/// The hull: an ellipsoid of these half extents, half its height under water, circling at this
/// radius and speed.
const HULL_HALF: Vec3 = Vec3::new(0.6, 0.35, 1.6);
const HULL_RADIUS_M: f32 = 11.0;
const HULL_SPEED_MS: f32 = 2.0;
/// The float: a 25 cm sphere bobbing 8 cm at 0.7 Hz.
const FLOAT_AT: Vec2 = Vec2::new(-6.0, -4.0);
const FLOAT_RADIUS_M: f32 = 0.25;
/// The pebble: a 6 cm sphere dropped from 1.5 m every 4 s.
const PEBBLE_AT: Vec2 = Vec2::new(2.0, -10.0);
const PEBBLE_RADIUS_M: f32 = 0.06;
const PEBBLE_PERIOD_S: f32 = 4.0;
/// The wader: two legs of 8 cm half width walking the shallows at this radius and speed.
const WADE_RADIUS_M: f32 = 17.5;
const WADE_SPEED_MS: f32 = 1.1;
const LEG_HALF: Vec3 = Vec3::new(0.07, 0.6, 0.07);

#[derive(Resource)]
struct Scene {
    bodies: Vec<String>,
    /// Seconds since the pond filled.
    since_full: Option<f32>,
    /// The camera's offset from the hull in its frame (right, up, behind), m, when it follows.
    follow: Option<Vec3>,
}

#[derive(Component)]
struct Eye;

#[derive(Component)]
struct Hull;

#[derive(Component)]
struct Float;

#[derive(Component)]
struct Pebble;

/// A leg of the wader: left (-1) or right (+1).
#[derive(Component)]
struct Leg(f32);

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_vec3(name: &str, default: Vec3) -> Vec3 {
    std::env::var(name)
        .ok()
        .map(|v| {
            v.split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect::<Vec<f32>>()
        })
        .filter(|v| v.len() == 3)
        .map_or(default, |v| Vec3::new(v[0], v[1], v[2]))
}

fn main() {
    let quality = match std::env::var("ELEMENTS_QUALITY").as_deref() {
        Ok("low") => ElementsQuality::Low,
        Ok("medium") => ElementsQuality::Medium,
        Ok("ultra") => ElementsQuality::Ultra,
        _ => ElementsQuality::High,
    };
    let bodies = std::env::var("ELEMENTS_BODIES")
        .unwrap_or_else(|_| "hull,float,pebble,wader".into())
        .split(',')
        .map(|s| s.trim().to_owned())
        .collect();
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 6, 21))
                .at_hour(f64::from(env_f32("ELEMENTS_HOUR", 16.5)))
        })
        .insert_resource(Climate::temperate())
        .insert_resource(quality)
        .insert_resource(WaveSettings {
            enabled: std::env::var("ELEMENTS_WAVES").as_deref() != Ok("off"),
            ..default()
        })
        .insert_resource(HydrologySettings {
            // The quality's substeps every frame, so the pond fills in a few seconds.
            time_scale: 6.0,
            boundary: HydrologyBoundary::Closed,
            evaporation: false,
            ..default()
        })
        .insert_resource(ErosionSettings {
            enabled: false,
            ..default()
        })
        .insert_resource(Scene {
            bodies,
            since_full: None,
            follow: std::env::var("ELEMENTS_FOLLOW")
                .is_ok()
                .then(|| env_vec3("ELEMENTS_FOLLOW", Vec3::new(4.0, 2.5, 8.0))),
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (close_the_tap, move_bodies, follow_hull)
                .chain()
                .after(ElementsSystems::Readback),
        )
        .run();
}

/// The ground height at world (x, z), m: a plain with a round basin, smooth at the rim.
fn ground_m(x: f32, z: f32) -> f32 {
    let r = (x * x + z * z).sqrt() / BASIN_RADIUS_M;
    // A cosine bowl: flat at the rim and at the centre, the steepest bank at half the radius.
    let bowl = if r < 1.0 {
        0.5 * (1.0 + (PI * r).cos())
    } else {
        0.0
    };
    PLAIN_M - BASIN_DEPTH_M * bowl
}

fn basin_map() -> Image {
    let mut data = Vec::with_capacity((CELLS * CELLS) as usize * 4);
    for j in 0..CELLS {
        for i in 0..CELLS {
            let x = (i as f32 + 0.5 - CELLS as f32 * 0.5) * CELL_M;
            let z = (j as f32 + 0.5 - CELLS as f32 * 0.5) * CELL_M;
            data.extend_from_slice(&(ground_m(x, z) / RANGE_M).to_le_bytes());
        }
    }
    Image::new(
        Extent3d {
            width: CELLS,
            height: CELLS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R32Float,
        RenderAssetUsages::default(),
    )
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut director: ResMut<WeatherDirector>,
    scene: Res<Scene>,
) {
    let map = images.add(basin_map());
    commands.spawn(
        Terrain::from_heightmap(map, 0.0..RANGE_M)
            .with_extent(Vec2::splat(CELLS as f32 * CELL_M))
            .with_resolution(UVec2::splat(CELLS)),
    );
    commands.spawn((
        WaterSource {
            rate_m3_s: 150.0,
            radius_m: 3.0,
        },
        Transform::from_xyz(0.0, ground_m(0.0, 0.0), 0.0),
    ));
    let eye = env_vec3("ELEMENTS_EYE", Vec3::new(-14.0, WATER_Y + 3.0, -20.0));
    let look = env_vec3("ELEMENTS_LOOK", Vec3::new(0.0, WATER_Y, -6.0));
    commands.spawn((camera(eye, look), Eye));
    let preset = std::env::var("ELEMENTS_PRESET")
        .ok()
        .and_then(|name| {
            WeatherPreset::ALL
                .into_iter()
                .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
        })
        .unwrap_or(WeatherPreset::Clear);
    director.force(preset, Duration::ZERO);

    let paint = |r: f32, g: f32, b: f32| StandardMaterial {
        base_color: Color::srgb(r, g, b),
        perceptual_roughness: 0.6,
        ..default()
    };
    let hidden = Transform::from_xyz(0.0, -100.0, 0.0);
    let has = |name: &str| scene.bodies.iter().any(|b| b == name);
    if has("hull") {
        // The contact is unscaled; the unit sphere mesh is stretched on a child.
        let mesh = meshes.add(Sphere::new(1.0));
        let paint = materials.add(paint(0.55, 0.14, 0.08));
        commands
            .spawn((
                Hull,
                WaterContact::new(HULL_HALF),
                hidden,
                Visibility::default(),
            ))
            .with_child((
                Mesh3d(mesh),
                MeshMaterial3d(paint),
                Transform::from_scale(HULL_HALF),
            ));
    }
    if has("float") {
        commands.spawn((
            Float,
            WaterContact::sphere(FLOAT_RADIUS_M),
            Mesh3d(meshes.add(Sphere::new(FLOAT_RADIUS_M))),
            MeshMaterial3d(materials.add(paint(0.85, 0.75, 0.2))),
            hidden,
        ));
    }
    if has("pebble") {
        commands.spawn((
            Pebble,
            WaterContact::sphere(PEBBLE_RADIUS_M),
            Mesh3d(meshes.add(Sphere::new(PEBBLE_RADIUS_M))),
            MeshMaterial3d(materials.add(paint(0.3, 0.3, 0.32))),
            hidden,
        ));
    }
    if has("wader") {
        let mesh = meshes.add(Capsule3d::new(LEG_HALF.x, 2.0 * (LEG_HALF.y - LEG_HALF.x)));
        let cloth = materials.add(paint(0.2, 0.25, 0.35));
        for side in [-1.0, 1.0] {
            commands.spawn((
                Leg(side),
                WaterContact::new(LEG_HALF),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(cloth.clone()),
                hidden,
            ));
        }
    }
}

/// Close the source once the pond is full, so it does not spill over the plain.
fn close_the_tap(
    env: Environment,
    time: Res<Time>,
    mut scene: ResMut<Scene>,
    mut sources: Query<&mut WaterSource>,
) {
    if let Some(t) = scene.since_full.as_mut() {
        *t += time.delta_secs();
        return;
    }
    if env
        .water_depth(Vec2::ZERO)
        .is_some_and(|depth| depth >= FULL_DEPTH_M)
    {
        for mut source in &mut sources {
            source.rate_m3_s = 0.0;
        }
        scene.since_full = Some(0.0);
    }
}

/// Every body on its path, from the time since the pond filled (hidden before).
#[allow(clippy::type_complexity)]
fn move_bodies(
    scene: Res<Scene>,
    mut bodies: ParamSet<(
        Query<&mut Transform, With<Hull>>,
        Query<&mut Transform, With<Float>>,
        Query<&mut Transform, With<Pebble>>,
        Query<(&mut Transform, &Leg)>,
    )>,
) {
    // Let the filling settle for 3 s first.
    let Some(t) = scene.since_full.map(|t| t - 3.0).filter(|t| *t >= 0.0) else {
        return;
    };
    for mut transform in &mut bodies.p0() {
        let a = t * HULL_SPEED_MS / HULL_RADIUS_M;
        let at = Vec2::new(a.cos(), a.sin()) * HULL_RADIUS_M;
        // The hull's long axis (local Z) along its heading, counter-clockwise.
        let heading = Vec2::new(-a.sin(), a.cos());
        transform.translation = Vec3::new(at.x, WATER_Y, at.y);
        transform.rotation = Quat::from_rotation_y(heading.x.atan2(heading.y));
    }
    for mut transform in &mut bodies.p1() {
        let bob = 0.08 * (TAU * 0.7 * t).sin();
        transform.translation = Vec3::new(FLOAT_AT.x, WATER_Y + 0.05 + bob, FLOAT_AT.y);
    }
    for mut transform in &mut bodies.p2() {
        // Falls from 1.5 m, then sinks at 0.5 m/s to the bed.
        let s = t % PEBBLE_PERIOD_S;
        let fall = (2.0 * 1.5 / 9.81f32).sqrt();
        let y = if s < fall {
            WATER_Y + 1.5 - 0.5 * 9.81 * s * s
        } else {
            WATER_Y - 0.5 * (s - fall)
        };
        let floor = ground_m(PEBBLE_AT.x, PEBBLE_AT.y) + PEBBLE_RADIUS_M;
        transform.translation = Vec3::new(PEBBLE_AT.x, y.max(floor), PEBBLE_AT.y);
    }
    for (mut transform, leg) in &mut bodies.p3() {
        // Each leg is planted for a step and swings through the next, 0.7 m a step.
        let a = t * WADE_SPEED_MS / WADE_RADIUS_M + PI;
        let along = Vec2::new(-a.sin(), a.cos());
        let out = Vec2::new(a.cos(), a.sin());
        let phase = (t * WADE_SPEED_MS / 0.7 + if leg.0 < 0.0 { 0.0 } else { 0.5 }).fract();
        let swing = if phase < 0.5 {
            0.0
        } else {
            0.35 * (1.0 - (TAU * (phase - 0.5)).cos())
        };
        let at = out * (WADE_RADIUS_M + 0.12 * leg.0) + along * (swing - 0.35);
        let bed = ground_m(at.x, at.y);
        transform.translation = Vec3::new(at.x, bed + LEG_HALF.y, at.y);
    }
}

/// With `ELEMENTS_FOLLOW`, the camera rides at its offset from the hull (in the hull's frame) or,
/// with `ELEMENTS_FOLLOW_BODY=wader`, from the wader's legs (in the world's), looking at it.
#[allow(clippy::type_complexity)]
fn follow_hull(
    scene: Res<Scene>,
    hulls: Query<&Transform, (With<Hull>, Without<Eye>)>,
    legs: Query<&Transform, (With<Leg>, Without<Eye>)>,
    mut eyes: Query<&mut Transform, With<Eye>>,
) {
    let Some(offset) = scene.follow else {
        return;
    };
    let (target, at) = if std::env::var("ELEMENTS_FOLLOW_BODY").as_deref() == Ok("wader") {
        let n = legs.iter().count();
        if n == 0 {
            return;
        }
        let mid = legs.iter().map(|t| t.translation).sum::<Vec3>() / n as f32;
        let target = Vec3::new(mid.x, WATER_Y, mid.z);
        (target, target + offset)
    } else {
        let Ok(hull) = hulls.single() else {
            return;
        };
        let at = hull.translation + hull.rotation * Vec3::new(offset.x, offset.y, -offset.z);
        (hull.translation, at)
    };
    for mut eye in &mut eyes {
        *eye = Transform::from_translation(at).looking_at(target, Vec3::Y);
    }
}
