//! The record of recent days through `ClimatePlugin` (`docs/research/climate-history.md`):
//! deterministic for a climate and a seed, and the same days at a finer frame step. Headless,
//! no GPU.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

/// One game hour per real second, `step_ms` of real time per update, from midnight on
/// 10 January (a temperate winter: frosts and wet days).
fn app(step_ms: u64) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ClimatePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            step_ms,
        )))
        .insert_resource(Climate::temperate().with_seed(42))
        .insert_resource(
            WorldClock::new(CivilDate::new(2026, 1, 10))
                .at_hour(0.0)
                .with_time_scale(3600.0),
        );
    app.update();
    app
}

/// `days` game days at `step_ms` a frame.
fn run_days(step_ms: u64, days: u64) -> App {
    let mut app = app(step_ms);
    for _ in 0..days * 24 * 1000 / step_ms {
        app.update();
    }
    app
}

#[test]
fn the_record_is_the_same_on_two_runs() {
    let (a, b) = (run_days(100, 10), run_days(100, 10));
    let (a, b) = (
        a.world().resource::<ClimateHistory>(),
        b.world().resource::<ClimateHistory>(),
    );
    assert_eq!(a.days().len(), 10);
    assert_eq!(a.days(), b.days());
    assert_eq!(a.today(), b.today());
    let week = a.summary(7).expect("a week of days");
    assert_eq!(week.days, 7);
    assert_eq!(Some(week), b.summary(7));
}

/// At a quarter of the frame step a temperate January week differs by at most 0.004 °C in the
/// daily means, 0.014 °C in the minima, 0.0007 in the humidity and nothing in the totals
/// (measured): the bounds are a few times that, and the day counts must match exactly.
#[test]
fn a_finer_frame_step_records_the_same_days() {
    let (coarse, fine) = (run_days(100, 7), run_days(25, 7));
    let (coarse, fine) = (
        coarse.world().resource::<ClimateHistory>(),
        fine.world().resource::<ClimateHistory>(),
    );
    assert_eq!(coarse.days().len(), 7);
    assert_eq!(coarse.days().len(), fine.days().len());
    for (c, f) in coarse.days().iter().zip(fine.days()) {
        assert_eq!(c.date, f.date);
        let close = |a: f32, b: f32, tolerance: f32, what: &str| {
            assert!(
                (a - b).abs() <= tolerance,
                "{:?} {what}: {a} vs {b}",
                c.date
            );
        };
        close(c.observed_hours, f.observed_hours, 1e-3, "hours");
        close(c.mean_temperature_c, f.mean_temperature_c, 0.02, "mean");
        close(c.min_temperature_c, f.min_temperature_c, 0.05, "minimum");
        close(c.max_temperature_c, f.max_temperature_c, 0.05, "maximum");
        close(
            c.mean_relative_humidity,
            f.mean_relative_humidity,
            3e-3,
            "humidity",
        );
        close(
            c.precipitation_mm,
            f.precipitation_mm,
            0.02 + 0.01 * c.precipitation_mm,
            "rain",
        );
        close(
            c.snowfall_mm,
            f.snowfall_mm,
            0.02 + 0.01 * c.snowfall_mm,
            "snow",
        );
    }
    let (c, f) = (coarse.summary(7), fine.summary(7));
    let counts =
        |s: Option<ClimateSummary>| s.map(|s| (s.days, s.wet_days, s.frost_days, s.icing_days));
    assert_eq!(counts(c), counts(f));
}
