//! A day passing at latitude 45° on the June solstice: sunrise near 04:11, a 68° noon sun,
//! sunset near 19:49, then stars and a waxing moon.
//!
//! `cargo run --release --example day_night`
//!
//! `ELEMENTS_HOUR=19.5` pauses the clock at that local hour (for screenshots); otherwise a day
//! passes in two minutes.

mod common;

use bevy::prelude::*;
use bevy_elements::*;
use common::{ExampleHarness, camera, test_scene};

fn main() {
    let fixed: Option<f64> = std::env::var("ELEMENTS_HOUR")
        .ok()
        .and_then(|h| h.parse().ok());
    let clock = WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(fixed.unwrap_or(4.0));
    let clock = match fixed {
        Some(_) => WorldClock {
            paused: true,
            ..clock
        },
        None => clock.with_time_scale(720.0),
    };
    let mut director = WeatherDirector::default();
    director.force(WeatherPreset::Clear, std::time::Duration::ZERO);
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(clock)
        .insert_resource(director)
        .insert_resource(GeoLocation {
            latitude_deg: 45.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Looking west-south-west, toward the summer sunset and the evening moon.
    commands.spawn(camera(
        Vec3::new(0.0, 8.0, 0.0),
        Vec3::new(-200.0, 30.0, 60.0),
    ));
    test_scene(&mut commands, &mut meshes, &mut materials);
}
