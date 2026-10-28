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
//! - `ELEMENTS_TINT=r,g,b` and `ELEMENTS_SATURATION=s` grade the bare ground (`TerrainLook`),
//!   `ELEMENTS_DETAIL=k` scales its micro-detail and `ELEMENTS_FINEST_M=m` drops the detail finer
//!   than the camera's metres per pixel (issue #42);
//!   `ELEMENTS_GRADE_GLIDE=8` glides between the default and that grade and back every 8 s,
//!   written every frame, as a palette blend would.
//! - `ELEMENTS_ORTHO=18` looks through an orthographic camera showing 18 m of ground top to
//!   bottom, 457 m back from an `ElementsFocus` on the ground at `ELEMENTS_LOOK`'s XZ (issue #36):
//!   `ELEMENTS_PITCH` (default 35.26°) and `ELEMENTS_HEADING` (default 45°) aim it and
//!   `ELEMENTS_WALK=1.4` walks the focus along the heading at 1.4 m/s.
//! - `ELEMENTS_PINS=1` stands thin posts on `Environment::terrain_height` every 2 m over 40 m
//!   around `ELEMENTS_LOOK`, each on a red foot 10 cm tall: where the drawn ground is not the
//!   height the query answers, a foot sinks out of sight or stands above the ground.
//!
//! `cargo run --release --example terrain_tour`

mod common;

use std::time::Duration;

use bevy::prelude::*;
use bevy_elements::*;
use common::{
    ExampleHarness, camera, ortho_camera, pins, stand_on_ground, terrain_look_env, units, vec3_env,
};

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
        .add_systems(Update, (glide, grade_glide))
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

/// The grade the ground glides to and back (`ELEMENTS_GRADE_GLIDE`), and the period, s.
#[derive(Resource)]
struct GradeGlide {
    target: TerrainLook,
    period: f32,
}

fn grade_glide(time: Res<Time>, glide: Option<Res<GradeGlide>>, mut look: ResMut<TerrainLook>) {
    let Some(glide) = glide else {
        return;
    };
    let t = 0.5 - 0.5 * (std::f32::consts::TAU * time.elapsed_secs() / glide.period).cos();
    let base = TerrainLook::default();
    *look = TerrainLook {
        tint: base.tint.lerp(glide.target.tint, t),
        saturation: base.saturation + (glide.target.saturation - base.saturation) * t,
        ..base
    };
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
    if let Some(target) = terrain_look_env() {
        match std::env::var("ELEMENTS_GRADE_GLIDE")
            .ok()
            .and_then(|p| p.parse::<f32>().ok())
        {
            Some(period) => commands.insert_resource(GradeGlide { target, period }),
            None => commands.insert_resource(target),
        }
    }
    if let Some(n) = std::env::var("ELEMENTS_WEATHER")
        .ok()
        .and_then(|w| w.parse::<usize>().ok())
        && let Some(preset) = WeatherPreset::ALL.get(n)
    {
        director.force(*preset, Duration::ZERO);
    }
    let eye = vec3_env("ELEMENTS_EYE").unwrap_or(Vec3::new(0.0, 200.0, 0.0));
    let look = vec3_env("ELEMENTS_LOOK").unwrap_or(eye + Vec3::new(150.0, -40.0, 80.0));
    if let Some(pins) = pins(look) {
        commands.insert_resource(pins);
    }
    if let Some(ortho) = ortho_camera(look) {
        commands.spawn(ortho);
    } else {
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
