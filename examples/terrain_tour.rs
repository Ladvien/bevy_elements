//! The terrain presets, flown by hand, with switches for unattended screenshots.
//!
//! - `ELEMENTS_PRESET=hills|mountains|dunes|plains|flat` (default `hills`), `ELEMENTS_SEED`.
//! - `ELEMENTS_HOUR=15.5` starts the clock there (default 12), `ELEMENTS_WEATHER=0..9` forces a
//!   preset (the number keys' order).
//! - `ELEMENTS_EYE=x,y,z` and `ELEMENTS_LOOK=x,y,z` place the camera; with `ELEMENTS_AGL=6` the
//!   eye's `y` is metres above the ground there instead.
//! - `ELEMENTS_UNDERLAY=1` puts a bright red plane under the terrain: a crack between patches
//!   shows red.
//! - `ELEMENTS_FLY=8` glides the camera forward at 8 m/s along its view's heading, held
//!   `ELEMENTS_AGL` (default 6) metres above the ground: a moving capture for what streams with
//!   the camera (the scattered rocks' rings and their fade). Use it with `ELEMENTS_SHOT_DIR`.
//!
//! `cargo run --release --example terrain_tour`

mod common;

use std::time::Duration;

use bevy::prelude::*;
use bevy_elements::*;
use common::{ExampleHarness, camera, stand_on_ground, units, vec3_env};

fn main() {
    let hour = std::env::var("ELEMENTS_HOUR")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(12.0);
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(
            WorldClock::new(CivilDate::new(2026, 6, 21))
                .at_hour(hour)
                .with_time_scale(1.0),
        )
        .insert_resource(Climate::temperate())
        .add_systems(Startup, setup)
        .add_systems(Update, glide)
        .run();
}

/// The camera's glide (`ELEMENTS_FLY`): world units per second and above the ground.
#[derive(Component)]
struct Glide {
    speed: f32,
    agl: f32,
}

fn glide(env: Environment, time: Res<Time>, mut cameras: Query<(&mut Transform, &Glide)>) {
    for (mut transform, glide) in &mut cameras {
        let heading =
            (transform.forward().as_vec3() * Vec3::new(1.0, 0.0, 1.0)).normalize_or_zero();
        transform.translation += heading * glide.speed * time.delta_secs();
        if let Some(ground) = env.terrain_height(transform.translation.xz()) {
            transform.translation.y = ground + glide.agl;
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut director: ResMut<WeatherDirector>,
) {
    let seed = std::env::var("ELEMENTS_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);
    let terrain = match std::env::var("ELEMENTS_PRESET").as_deref() {
        Ok("mountains") => Terrain::procedural(ProceduralTerrain::mountains(seed)),
        Ok("dunes") => Terrain::procedural(ProceduralTerrain::dunes(seed)),
        Ok("plains") => Terrain::procedural(ProceduralTerrain::plains(seed)),
        Ok("flat") => Terrain::flat(10.0),
        _ => Terrain::procedural(ProceduralTerrain::hills(seed)),
    };
    commands.spawn(terrain);
    if let Some(n) = std::env::var("ELEMENTS_WEATHER")
        .ok()
        .and_then(|w| w.parse::<usize>().ok())
        && let Some(preset) = WeatherPreset::ALL.get(n)
    {
        director.force(*preset, Duration::ZERO);
    }
    let eye = vec3_env("ELEMENTS_EYE").unwrap_or(Vec3::new(0.0, 200.0, 0.0));
    let look = vec3_env("ELEMENTS_LOOK").unwrap_or(eye + Vec3::new(150.0, -40.0, 80.0));
    let mut camera = commands.spawn(camera(eye, look));
    let agl: Option<f32> = std::env::var("ELEMENTS_AGL")
        .ok()
        .and_then(|a| a.parse().ok());
    if let Some(speed) = std::env::var("ELEMENTS_FLY")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
    {
        camera.insert(Glide {
            speed: speed * units(),
            agl: agl.unwrap_or(6.0) * units(),
        });
    } else if let Some(agl) = agl {
        camera.insert(stand_on_ground(agl));
    }
    if std::env::var("ELEMENTS_UNDERLAY").is_ok() {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(8000.0, 8000.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.0, -40.0, 0.0),
        ));
    }
}
