//! Saving and restoring the weather through `serde`: a restored director continues the same
//! scheduler chain, forced preset and blend as the run it was saved from, and a restored
//! snowpack the same snow and wetness. Headless, no GPU.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

/// One game hour per real second, stepped 0.1 s per update: 0.1 game hours per update.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ClimatePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(100)))
        .insert_resource(Climate::temperate().with_seed(42))
        .insert_resource(
            WorldClock::new(CivilDate::new(2026, 3, 1))
                .at_hour(6.0)
                .with_time_scale(3600.0),
        );
    app.update();
    app
}

fn run_for(app: &mut App, updates: usize) {
    for _ in 0..updates {
        app.update();
    }
}

/// The save file: everything the weather and the ground cannot re-derive.
fn save(app: &App) -> String {
    let w = app.world();
    serde_json::to_string(&(
        w.resource::<WorldClock>(),
        w.resource::<Climate>(),
        w.resource::<GeoLocation>(),
        w.resource::<WeatherDirector>(),
        w.resource::<SurfaceWeathering>(),
    ))
    .unwrap()
}

type SaveFile = (
    WorldClock,
    Climate,
    GeoLocation,
    WeatherDirector,
    SurfaceWeathering,
);

fn load(app: &mut App, file: &str) {
    let (clock, climate, location, director, ground): SaveFile =
        serde_json::from_str(file).unwrap();
    app.insert_resource(clock)
        .insert_resource(climate)
        .insert_resource(location)
        .insert_resource(director)
        .insert_resource(ground);
}

/// Ten game hours of forced snow, then the scheduler's weather for `after_release` updates: a
/// snowpack that settles and melts.
fn snowed(after_release: usize) -> App {
    let mut app = app();
    run_for(&mut app, 20);
    app.world_mut()
        .resource_mut::<WeatherDirector>()
        .force(WeatherPreset::Snow, Duration::from_secs(1800));
    run_for(&mut app, 100);
    app.world_mut()
        .resource_mut::<WeatherDirector>()
        .release(Duration::from_secs(3600));
    run_for(&mut app, after_release);
    assert!(app.world().resource::<SurfaceWeathering>().snow_depth_m > 0.01);
    app
}

/// Steps both apps together and returns how many updates showed the same weather, and how many
/// of those updates the original changed preset or was mid-blend.
fn compare(a: &mut App, b: &mut App, updates: usize) -> (usize, usize) {
    let (mut same, mut moving) = (0, 0);
    let mut last = *a.world().resource::<Weather>();
    for _ in 0..updates {
        a.update();
        b.update();
        let wa = *a.world().resource::<Weather>();
        let wb = *b.world().resource::<Weather>();
        assert_eq!(
            a.world().resource::<WorldClock>().hours_since_2000(),
            b.world().resource::<WorldClock>().hours_since_2000()
        );
        same += usize::from(wa == wb);
        moving += usize::from(wa.cloud_cover != last.cloud_cover);
        last = wa;
    }
    (same, moving)
}

#[test]
fn a_restored_director_continues_the_same_weather() {
    let mut original = app();
    run_for(&mut original, 720);
    let file = save(&original);

    let mut restored = app();
    load(&mut restored, &file);
    let (same, moving) = compare(&mut original, &mut restored, 720);
    assert_eq!(same, 720, "restored weather diverged");
    assert!(moving > 0, "the 72 h after the save never changed the weather");

    // Without the director the chain restarts at Clear and drifts away.
    let mut original = app();
    run_for(&mut original, 720);
    let mut clock_only = app();
    clock_only.insert_resource(original.world().resource::<WorldClock>().clone());
    let (same, _) = compare(&mut original, &mut clock_only, 720);
    assert!(same < 720, "a fresh director matched the saved run by chance");
}

#[test]
fn a_director_saved_mid_force_restores_forced_at_the_same_blend() {
    let mut original = app();
    run_for(&mut original, 100);
    original
        .world_mut()
        .resource_mut::<WeatherDirector>()
        .force(WeatherPreset::Thunderstorm, Duration::from_secs(2 * 3600));
    // One game hour into a two-hour blend.
    run_for(&mut original, 10);
    let mid = *original.world().resource::<Weather>();
    assert!(mid.precipitation_mm_h > 0.0 && mid.precipitation_mm_h < 25.0);
    let file = save(&original);

    let mut restored = app();
    load(&mut restored, &file);
    let director = restored.world().resource::<WeatherDirector>();
    assert!(director.is_forced());
    assert_eq!(director.current(), WeatherPreset::Thunderstorm);
    let (same, _) = compare(&mut original, &mut restored, 200);
    assert_eq!(same, 200);
    assert_eq!(
        restored.world().resource::<Weather>().precipitation_mm_h,
        25.0
    );
}

/// The published snow depth, m.
fn depth(app: &App) -> f32 {
    app.world().resource::<SurfaceWeathering>().snow_depth_m
}

#[test]
fn a_restored_snowpack_continues_the_same_snow() {
    let mut original = snowed(5);
    let saved = *original.world().resource::<SurfaceWeathering>();
    let file = save(&original);

    let mut restored = app();
    load(&mut restored, &file);
    // Issue #23: the whole pack compares equal, its private state (water equivalent, density,
    // the hour it last stepped) with it.
    assert_eq!(*restored.world().resource::<SurfaceWeathering>(), saved);
    for i in 0..720 {
        original.update();
        restored.update();
        assert_eq!(
            original.world().resource::<SurfaceWeathering>(),
            restored.world().resource::<SurfaceWeathering>(),
            "update {i} after the load"
        );
    }
    let end = depth(&original);
    assert!(
        end < saved.snow_depth_m,
        "72 h after the save the pack never settled or melted"
    );
}

#[test]
fn a_snowpack_loaded_a_day_later_neither_melts_nor_grows() {
    let original = snowed(40);
    let saved = *original.world().resource::<SurfaceWeathering>();
    let file = save(&original);
    let a_day_later = |app: &mut App| {
        let mut clock = app.world_mut().resource_mut::<WorldClock>();
        clock.date = clock.date.add_days(1);
    };

    let mut later = app();
    load(&mut later, &file);
    a_day_later(&mut later);
    later.update();
    assert_eq!(depth(&later), saved.snow_depth_m);

    // The same pack carried over without a save integrates the whole day in one step.
    let mut carried = app();
    load(&mut carried, &file);
    carried.insert_resource(saved);
    a_day_later(&mut carried);
    carried.update();
    let depth = depth(&carried);
    assert!(
        depth != saved.snow_depth_m,
        "a 24 h step left the pack at {depth}"
    );
}

#[test]
fn files_from_before_the_snow_season_load() {
    let mut climate = serde_json::to_value(Climate::alpine()).unwrap();
    let fields = climate.as_object_mut().unwrap();
    assert!(fields.remove("snow_season").is_some());
    let climate: Climate = serde_json::from_value(climate).unwrap();
    assert_eq!(climate.snow_season, None);

    let mut weather = serde_json::to_value(Weather::default()).unwrap();
    let fields = weather.as_object_mut().unwrap();
    assert!(fields.remove("snow_possible").is_some());
    let weather: Weather = serde_json::from_value(weather).unwrap();
    assert_eq!(weather.snow_possible, 1.0);

    // A season round-trips, wrapping the year.
    let season = Climate::alpine().with_snow_season(SnowSeason {
        first: (11, 14),
        last: (3, 31),
    });
    let back: Climate = serde_json::from_str(&serde_json::to_string(&season).unwrap()).unwrap();
    assert_eq!(back.snow_season, season.snow_season);
}
