//! Snow and wet ground on the quickstart hills: the cover maps accumulate, shade and (for snow)
//! drift and melt on the terrain.
//!
//! `cargo run --release --example ground_cover`
//!
//! `ELEMENTS_COVER=snow` (default) forces Snow at a game hour per real second, so snow builds up
//! on the hills in under a minute; `ELEMENTS_PRESET=Blizzard` picks the heavier, windier preset
//! whose drifts pile up in the lee of the ridges. `ELEMENTS_COVER=rain` bakes a few millimetres
//! of rain into the hollows (`ErodeTerrain`, erosion off, so the hills stay as they are) while
//! the Rain preset wets the ground: darker, glossier ground, and puddles with ripples where water
//! stands deeper than 2 mm. `ELEMENTS_QUALITY=ultra` runs the bake four times faster per frame.
//! `ELEMENTS_FRAMES` and `ELEMENTS_SHOT` work as in the other examples.

mod common;

use std::time::Duration;

use bevy::prelude::*;
use bevy_elements::*;
use common::{ExampleHarness, camera, stand_on_ground};

fn main() {
    let rain = std::env::var("ELEMENTS_COVER").as_deref() == Ok("rain");
    let quality = match std::env::var("ELEMENTS_QUALITY").as_deref() {
        Ok("low") => ElementsQuality::Low,
        Ok("medium") => ElementsQuality::Medium,
        Ok("ultra") => ElementsQuality::Ultra,
        _ => ElementsQuality::High,
    };
    let clock = if rain {
        WorldClock::new(CivilDate::new(2026, 6, 21))
            .at_hour(15.0)
            .with_time_scale(600.0)
    } else {
        WorldClock::new(CivilDate::new(2026, 1, 15))
            .at_hour(11.0)
            .with_time_scale(3600.0)
    };
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(clock)
        .insert_resource(Climate::temperate())
        .insert_resource(quality)
        .insert_resource(Mode { rain })
        .insert_resource(ErosionSettings {
            enabled: false,
            ..default()
        })
        .add_systems(Startup, setup)
        .run();
}

#[derive(Resource)]
struct Mode {
    rain: bool,
}

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn setup(
    mut commands: Commands,
    mode: Res<Mode>,
    mut director: ResMut<WeatherDirector>,
    mut baking: MessageWriter<ErodeTerrain>,
) {
    commands.spawn(Terrain::procedural(ProceduralTerrain::hills(42)));
    commands.spawn((
        camera(Vec3::new(0.0, 160.0, 0.0), Vec3::new(150.0, 100.0, 80.0)),
        stand_on_ground(6.0),
    ));
    let default = if mode.rain {
        WeatherPreset::Rain
    } else {
        WeatherPreset::Snow
    };
    let preset = std::env::var("ELEMENTS_PRESET")
        .ok()
        .and_then(|name| {
            WeatherPreset::ALL
                .into_iter()
                .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
        })
        .unwrap_or(default);
    director.force(preset, Duration::ZERO);
    if mode.rain {
        baking.write(ErodeTerrain {
            duration_s: env_f32("ELEMENTS_BAKE_S", 240.0),
            rain_mm_h: env_f32("ELEMENTS_BAKE_MM_H", 120.0),
        });
    }
}
