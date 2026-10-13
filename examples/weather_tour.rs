//! Every weather preset in turn, forced for 20 s each, with the preset named on screen.
//!
//! `cargo run --release --example weather_tour`
//!
//! `ELEMENTS_PRESET=Fog` holds one preset instead (for screenshots).

mod common;

use std::time::Duration;

use bevy::prelude::*;
use bevy_elements::*;
use common::{ExampleHarness, camera, test_scene};

#[derive(Resource)]
struct Tour {
    held: Option<WeatherPreset>,
    index: usize,
    timer: Timer,
}

#[derive(Component)]
struct PresetLabel;

fn main() {
    let held = std::env::var("ELEMENTS_PRESET").ok().and_then(|name| {
        WeatherPreset::ALL
            .into_iter()
            .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
    });
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(
            WorldClock::new(CivilDate::new(2026, 4, 20))
                .at_hour(10.0)
                .with_time_scale(1.0),
        )
        .insert_resource(Tour {
            held,
            index: 0,
            timer: Timer::from_seconds(20.0, TimerMode::Repeating),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, tour.before(ElementsSystems::Clock))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tour: ResMut<Tour>,
    mut director: ResMut<WeatherDirector>,
) {
    commands.spawn(camera(
        Vec3::new(0.0, 6.0, 0.0),
        Vec3::new(120.0, 8.0, 60.0),
    ));
    test_scene(&mut commands, &mut meshes, &mut materials);
    commands.spawn((
        PresetLabel,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(36.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            bottom: Val::Px(20.0),
            ..default()
        },
    ));
    let first = tour.held.unwrap_or(WeatherPreset::ALL[0]);
    tour.index = 0;
    director.force(first, Duration::ZERO);
}

fn tour(
    time: Res<Time>,
    mut tour: ResMut<Tour>,
    mut director: ResMut<WeatherDirector>,
    mut label: Query<&mut Text, With<PresetLabel>>,
) {
    if tour.held.is_none() && tour.timer.tick(time.delta()).just_finished() {
        tour.index = (tour.index + 1) % WeatherPreset::ALL.len();
        // Transitions are game time; at time scale 1 this is 4 s of blending.
        director.force(WeatherPreset::ALL[tour.index], Duration::from_secs(4));
    }
    if let Ok(mut text) = label.single_mut() {
        text.0 = format!("{:?}", director.current());
    }
}
