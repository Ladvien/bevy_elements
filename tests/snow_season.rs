//! The snow season (`Climate::snow_season`, issue #15): Sanctuary's November, hour by hour.
//! Headless, no GPU. See `docs/research/snow-season.md` (Validation targets).

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

/// Sanctuary's calendar: rain first, snow from 14 November to the end of March.
const SEASON: SnowSeason = SnowSeason {
    first: (11, 14),
    last: (3, 31),
};

/// Sanctuary's climate (issue #15): mean 9.4 °C, annual 10 °C, diurnal 4 °C, seed 1.
fn sanctuary(season: Option<SnowSeason>) -> Climate {
    Climate {
        mean_temperature_c: 9.4,
        annual_amplitude_c: 10.0,
        diurnal_amplitude_c: 4.0,
        snow_season: season,
        ..Climate::temperate().with_seed(1)
    }
}

/// One game hour per update from 00:00 on `date`, at 42.5° N.
fn app(climate: Climate, date: CivilDate) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ClimatePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)))
        .insert_resource(climate)
        .insert_resource(GeoLocation {
            latitude_deg: 42.5,
            ..default()
        })
        .insert_resource(WorldClock::new(date).at_hour(0.0).with_time_scale(3600.0));
    app.update();
    app
}

/// What one hour shows.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hour {
    preset: WeatherPreset,
    weather: Weather,
    snow_cover: f32,
    snow_depth_m: f32,
    wetness: f32,
}

fn hour(app: &App) -> Hour {
    let world = app.world();
    let ground = world.resource::<SurfaceWeathering>();
    Hour {
        preset: world.resource::<WeatherDirector>().current(),
        weather: *world.resource::<Weather>(),
        snow_cover: ground.snow_cover,
        snow_depth_m: ground.snow_depth_m,
        wetness: ground.wetness,
    }
}

/// `hours` updates, one hour each, and what each showed.
fn run(app: &mut App, hours: usize) -> Vec<Hour> {
    (0..hours)
        .map(|_| {
            app.update();
            hour(app)
        })
        .collect()
}

fn is_snow(p: WeatherPreset) -> bool {
    matches!(p, WeatherPreset::Snow | WeatherPreset::Blizzard)
}

#[test]
fn no_snow_falls_or_lies_before_the_season() {
    let start = CivilDate::new(2026, 11, 1);
    // 1 November 01:00 to 13 November 23:00.
    let hours = 13 * 24 - 1;
    let with = run(&mut app(sanctuary(Some(SEASON)), start), hours);
    let without = run(&mut app(sanctuary(None), start), hours);

    for (i, (a, b)) in with.iter().zip(&without).enumerate() {
        let when = format!("day {} {:02}:00", 1 + (i + 1) / 24, (i + 1) % 24);
        assert!(
            !is_snow(a.preset),
            "{when}: {:?} out of the season",
            a.preset
        );
        assert_eq!(a.snow_cover, 0.0, "{when}: snow cover");
        assert_eq!(a.snow_depth_m, 0.0, "{when}: snow depth");
        assert_eq!(a.weather.snow_fraction(), 0.0, "{when}: snow fraction");
        // The same weather falls, all of it as rain.
        assert_eq!(
            a.weather.precipitation_mm_h, b.weather.precipitation_mm_h,
            "{when}: precipitation"
        );
    }
    // The season is what keeps the ground bare: without it the cold rain lies as snow.
    let cold_wet = with
        .iter()
        .filter(|h| h.weather.precipitation_mm_h > 0.0 && h.weather.temperature_c < 2.0)
        .count();
    let snowy = without.iter().map(|h| h.snow_cover).fold(0.0, f32::max);
    println!(
        "RESULT 1-13 Nov: {cold_wet} cold wet hours; snow cover without a season up to {snowy:.2}"
    );
    assert!(cold_wet > 0, "no hour below 2 °C with precipitation");
    assert!(
        snowy > 0.5,
        "without a season the snow cover only reached {snowy}"
    );
    assert!(
        with.iter().any(|h| h.wetness > 0.5),
        "the rain never wet the ground"
    );
}

#[test]
fn the_season_itself_is_unchanged() {
    let start = CivilDate::new(2026, 11, 14);
    let hours = 40 * 24;
    let with = run(&mut app(sanctuary(Some(SEASON)), start), hours);
    let without = run(&mut app(sanctuary(None), start), hours);
    for (i, (a, b)) in with.iter().zip(&without).enumerate() {
        assert_eq!(a, b, "hour {} of the season", i + 1);
    }
    let snow_hours = with.iter().filter(|h| h.snow_cover > 0.5).count();
    println!("RESULT 14 Nov + 40 days: {snow_hours} hours under snow, identical to no season");
    assert!(snow_hours > 0, "the season never snowed");
}

/// A climate at exactly 0 °C every hour: a Rain hour is at −3 °C.
fn freezing(season: SnowSeason) -> Climate {
    Climate {
        mean_temperature_c: 0.0,
        annual_amplitude_c: 0.0,
        diurnal_amplitude_c: 0.0,
        snow_season: Some(season),
        ..Climate::temperate()
    }
}

fn forced(climate: Climate, date: CivilDate, preset: WeatherPreset, hours: usize) -> Hour {
    let mut app = app(climate, date);
    app.world_mut()
        .resource_mut::<WeatherDirector>()
        .force(preset, Duration::ZERO);
    run(&mut app, hours)
        .last()
        .copied()
        .unwrap_or_else(|| hour(&app))
}

#[test]
fn a_cold_rain_hour_is_rain_outside_the_season_and_snow_inside() {
    let outside = forced(
        freezing(SEASON),
        CivilDate::new(2026, 7, 1),
        WeatherPreset::Rain,
        6,
    );
    assert!((outside.weather.temperature_c - -3.0).abs() < 1e-4);
    assert_eq!(outside.weather.snow_fraction(), 0.0);
    assert_eq!(outside.snow_depth_m, 0.0);
    assert!(outside.wetness > 0.9, "{}", outside.wetness);

    let inside = forced(
        freezing(SEASON),
        CivilDate::new(2026, 12, 1),
        WeatherPreset::Rain,
        6,
    );
    assert!((inside.weather.temperature_c - -3.0).abs() < 1e-4);
    assert_eq!(inside.weather.snow_fraction(), 1.0);
    assert!(inside.snow_depth_m > 0.0);
}

#[test]
fn forcing_snow_outside_the_season_still_snows() {
    let snow = forced(
        sanctuary(Some(SEASON)),
        CivilDate::new(2026, 11, 5),
        WeatherPreset::Snow,
        10,
    );
    assert_eq!(snow.preset, WeatherPreset::Snow);
    assert_eq!(snow.weather.snow_possible, 1.0);
    assert_eq!(snow.weather.snow_fraction(), 1.0);
    // Ten hours of the Snow preset lie a few centimetres deep.
    assert!(snow.snow_depth_m > 0.02, "{}", snow.snow_depth_m);
}

#[test]
fn the_phase_changes_over_the_hour_before_the_season() {
    // 13 November 22:00 to 14 November 01:00 in quarter hours, at −3 °C under forced Rain.
    let mut app = app(freezing(SEASON), CivilDate::new(2026, 11, 13));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        250,
    )));
    app.world_mut().resource_mut::<WorldClock>().hour = 22.0;
    app.world_mut()
        .resource_mut::<WeatherDirector>()
        .force(WeatherPreset::Rain, Duration::ZERO);
    let mut seen = Vec::new();
    for _ in 0..12 {
        app.update();
        let clock = app.world().resource::<WorldClock>();
        seen.push((
            clock.date.day,
            clock.hour,
            app.world().resource::<Weather>().snow_fraction(),
        ));
    }
    for &(day, hour, fraction) in &seen {
        let want = match (day, hour) {
            (13, h) if h <= 23.0 => 0.0,
            (13, h) => (h - 23.0) as f32,
            _ => 1.0,
        };
        assert!(
            (fraction - want).abs() < 1e-3,
            "13→14 Nov, day {day} {hour:.2} h: {fraction} (want {want})"
        );
    }
    assert!(seen.iter().any(|&(_, _, f)| f > 0.0 && f < 1.0), "{seen:?}");
}
