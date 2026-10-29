//! `EnvironmentSetup`: validated, round-trippable, and applied to a running app without
//! restarting a forced blend it already holds. Headless, no GPU.

#![allow(clippy::field_reassign_with_default)]

use bevy::prelude::*;
use bevy_elements::*;

type Change = Box<dyn Fn(&mut EnvironmentSetup)>;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ClimatePlugin))
        .insert_resource(WorldClock::new(CivilDate::new(2026, 3, 1)).at_hour(6.0));
    app.update();
    app
}

#[test]
fn each_bound_rejects_a_value_just_outside_it_and_names_the_field() {
    let cases: Vec<(&str, Change)> = vec![
        ("clock.hour", Box::new(|s| s.clock.hour = 24.0)),
        ("clock.time_scale", Box::new(|s| s.clock.time_scale = -1.0)),
        (
            "location.latitude_deg",
            Box::new(|s| s.location.latitude_deg = 90.5),
        ),
        (
            "location.longitude_deg",
            Box::new(|s| s.location.longitude_deg = -180.5),
        ),
        (
            "location.utc_offset_hours",
            Box::new(|s| s.location.utc_offset_hours = 15.0),
        ),
        (
            "climate.mean_humidity",
            Box::new(|s| s.climate.mean_humidity = 1.01),
        ),
        (
            "climate.storm_share",
            Box::new(|s| s.climate.storm_share = f32::NAN),
        ),
        (
            "climate.mean_wind_ms",
            Box::new(|s| s.climate.mean_wind_ms = -0.1),
        ),
        (
            "climate.warmest_day",
            Box::new(|s| s.climate.warmest_day = 0),
        ),
        (
            "climate.prevailing_wind",
            Box::new(|s| s.climate.prevailing_wind = Vec2::ZERO),
        ),
        (
            "blend_minutes",
            Box::new(|s| s.blend_minutes = f32::INFINITY),
        ),
    ];
    for (field, change) in cases {
        let mut s = EnvironmentSetup::default();
        change(&mut s);
        let e = s.validate().expect_err(field);
        assert_eq!(e.field, field, "{e}");
    }
    // The edges themselves are valid.
    let mut s = EnvironmentSetup::default();
    s.location.latitude_deg = -90.0;
    s.location.longitude_deg = 180.0;
    s.clock.hour = 23.999;
    s.climate.warmest_day = 366;
    assert_eq!(s.validate(), Ok(()));
}

#[test]
fn an_invalid_setup_changes_nothing() {
    let mut app = app();
    let before = EnvironmentSetup::capture_from_world(app.world()).unwrap();
    let mut s = EnvironmentSetup::default();
    s.location.latitude_deg = 200.0;
    s.forced_weather = Some(WeatherPreset::Rain);
    assert!(s.apply_to_world(app.world_mut()).is_err());
    let after = EnvironmentSetup::capture_from_world(app.world()).unwrap();
    assert_eq!(after.location.latitude_deg, before.location.latitude_deg);
    assert_eq!(after.forced_weather, None);
}

#[test]
fn applying_forces_once_and_none_releases_only_a_forced_director() {
    let mut app = app();
    let mut s = EnvironmentSetup::default();
    s.forced_weather = Some(WeatherPreset::Rain);
    s.apply_to_world(app.world_mut()).unwrap();
    app.update();
    assert_eq!(
        app.world().resource::<WeatherDirector>().forced(),
        Some(WeatherPreset::Rain)
    );

    // The same setup again must not queue a new force request (which would restart the blend).
    let held = app.world().resource::<WeatherDirector>().clone();
    s.apply_to_world(app.world_mut()).unwrap();
    assert_eq!(*app.world().resource::<WeatherDirector>(), held);

    // A different preset forces again.
    s.forced_weather = Some(WeatherPreset::Snow);
    s.apply_to_world(app.world_mut()).unwrap();
    assert_eq!(
        app.world().resource::<WeatherDirector>().forced(),
        Some(WeatherPreset::Snow)
    );

    // `None` releases, and applying `None` to a released director leaves it alone.
    s.forced_weather = None;
    s.apply_to_world(app.world_mut()).unwrap();
    app.update();
    assert_eq!(app.world().resource::<WeatherDirector>().forced(), None);
    let released = app.world().resource::<WeatherDirector>().clone();
    s.apply_to_world(app.world_mut()).unwrap();
    assert_eq!(*app.world().resource::<WeatherDirector>(), released);
}

#[test]
fn capture_after_apply_returns_what_was_applied() {
    let mut app = app();
    let mut s = EnvironmentSetup::default();
    s.clock = WorldClock::new(CivilDate::new(2027, 12, 21)).at_hour(18.5);
    s.location.latitude_deg = -33.9;
    s.climate = Climate::temperate().with_seed(7);
    s.forced_weather = Some(WeatherPreset::Fog);
    s.apply_to_world(app.world_mut()).unwrap();
    let got = EnvironmentSetup::capture_from_world(app.world()).unwrap();
    assert_eq!(got.clock.date, CivilDate::new(2027, 12, 21));
    assert_eq!(got.clock.hour, 18.5);
    assert_eq!(got.location.latitude_deg, -33.9);
    assert_eq!(got.climate.seed, 7);
    assert_eq!(got.forced_weather, Some(WeatherPreset::Fog));
}

#[test]
fn a_world_without_the_plugins_is_an_error_that_loses_nothing() {
    let mut world = World::new();
    world.insert_resource(WorldClock::default());
    let e = EnvironmentSetup::default()
        .apply_to_world(&mut world)
        .unwrap_err();
    assert_eq!(e.field, "GeoLocation");
    assert!(world.contains_resource::<WorldClock>());
}

#[test]
fn json_round_trips_defaults_fill_in_and_a_typo_is_an_error() {
    let mut s = EnvironmentSetup::default();
    s.forced_weather = Some(WeatherPreset::Snow);
    s.location.longitude_deg = 12.5;
    let text = serde_json::to_string(&s).unwrap();
    let back: EnvironmentSetup = serde_json::from_str(&text).unwrap();
    assert_eq!(back.forced_weather, Some(WeatherPreset::Snow));
    assert_eq!(back.location.longitude_deg, 12.5);
    assert_eq!(serde_json::to_string(&back).unwrap(), text);

    let only_weather: EnvironmentSetup =
        serde_json::from_str(r#"{"forced_weather":"Rain"}"#).unwrap();
    assert_eq!(only_weather.forced_weather, Some(WeatherPreset::Rain));
    assert_eq!(
        only_weather.location.latitude_deg,
        GeoLocation::default().latitude_deg
    );

    assert!(serde_json::from_str::<EnvironmentSetup>(r#"{"forced_wether":"Rain"}"#).is_err());
}
