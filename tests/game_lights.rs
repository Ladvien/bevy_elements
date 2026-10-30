//! A game lights its own night: the crate's public surface for it, from outside the crate.
//!
//! The promise these tests hold: a game that places a light and asks the crate how bright it
//! has to be gets an answer that is *the same at every hour*, so its torch reads the same at
//! noon and at midnight and it never has to re-tune it.

mod support;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};
use bevy_elements::*;

/// The celestial plugin's shader libraries need the render plugins, but nothing renders: the
/// exposure is metered on the CPU. The plugins are the full set, windowless.
fn world_at(hour: f32) -> App {
    // The asset plugin's async loader needs the task pools, which `App::run` starts.
    IoTaskPool::get_or_init(bevy::tasks::TaskPool::default);
    ComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
    AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
    let mut app = App::new();
    app.add_plugins(support::headless_plugins())
        .add_plugins((ClimatePlugin, CelestialPlugin))
        .insert_resource(WorldClock::new(CivilDate::new(2026, 4, 15)).at_hour(hour as f64))
        // The location the sky is computed for; a game's app inserts one.
        .insert_resource(GeoLocation::default());
    support::finish(&mut app);
    app
}

/// The camera's exposure after one update.
fn exposure_at(app: &mut App) -> f32 {
    app.update();
    app.world().resource::<ElementsExposure>().exposure
}

#[test]
fn the_crate_publishes_the_camera_s_exposure() {
    let mut app = world_at(12.0);
    let noon = exposure_at(&mut app);
    assert!(noon > 0.0, "the exposure is a positive number: {noon}");

    let mut app = world_at(22.0);
    let night = exposure_at(&mut app);
    assert!(night > 0.0, "the exposure is a positive number: {night}");
}

#[test]
fn the_night_is_exposed_far_more_than_the_noon() {
    let mut app = world_at(12.0);
    let noon = exposure_at(&mut app);
    let mut app = world_at(22.0);
    let night = exposure_at(&mut app);
    assert!(
        night > noon * 10.0,
        "the night is opened up: {night} against {noon}"
    );
}

#[test]
fn a_light_balanced_at_noon_still_reads_at_midnight() {
    // The art direction: a surface of this albedo should read at this scene-linear value.
    const ALBEDO: f32 = 0.14;
    const DISPLAY: f32 = 0.18;
    const DISTANCE_M: f32 = 2.0;

    let mut app = world_at(12.0);
    let noon = exposure_at(&mut app);
    let noon_luminance = bevy_elements::light::luminance_for_display(DISPLAY, noon);
    let noon_intensity = bevy_elements::light::light_intensity(noon_luminance, DISTANCE_M, ALBEDO);

    let mut app = world_at(22.0);
    let night = exposure_at(&mut app);
    let night_luminance = bevy_elements::light::luminance_for_display(DISPLAY, night);
    let night_intensity =
        bevy_elements::light::light_intensity(night_luminance, DISTANCE_M, ALBEDO);

    // The same light power would read differently, so the power has to change with the hour.
    // The night is *opened up*, so the same surface needs far less light there: the power
    // scales inversely with the exposure.
    assert!(
        noon_intensity > night_intensity * 1000.0,
        "the noon needs far more lumens for the same picture value: {noon_intensity} against {night_intensity}"
    );
    // And the two are in the ratio of the two exposures: the night's exposure is the bigger one.
    let ratio = noon_intensity / night_intensity;
    let expected = night / noon;
    assert!(
        (ratio - expected).abs() <= 0.01 * expected,
        "the power ratio {ratio} should be the exposure ratio {expected}"
    );

    // But the *surface* reads the same in both, which is the point.
    for (exposure, intensity) in [(noon, noon_intensity), (night, night_intensity)] {
        let illuminance = bevy_elements::light::light_illuminance(intensity, DISTANCE_M);
        let luminance = illuminance * ALBEDO / std::f32::consts::PI;
        let read = bevy_elements::light::display_value(luminance, exposure);
        assert!(
            (read - DISPLAY).abs() <= 1e-3 * DISPLAY,
            "at the exposure {exposure} the surface reads at {read}, not {DISPLAY}"
        );
    }
}

#[test]
fn a_game_reads_the_exposure_through_the_environment() {
    let mut app = world_at(22.0);
    app.update();
    let exposure = app.world().resource::<ElementsExposure>().exposure;
    assert!(exposure > 0.0);

    // The crate's night colour is the scotopic one: the blue reads above the green.
    let c = bevy_elements::light::night_light_colour();
    assert!(c.z > c.y, "the scotopic colour is cool: {c:?}");
    assert!(c.z > c.x, "the scotopic colour is cool: {c:?}");
}
