//! The sky-and-weather settings as one value a host can save, validate and apply to a running app.
//!
//! [`EnvironmentSetup`] bundles the four resources that decide a scene's light and weather
//! ([`WorldClock`], [`GeoLocation`], [`Climate`]) and the director's forced preset, which
//! [`WeatherDirector`] keeps private. A level file, an editor panel or a debug console reads one
//! value, [`validate`](EnvironmentSetup::validate)s it and [`apply`](EnvironmentSetup::apply)s it
//! live; [`capture`](EnvironmentSetup::capture) reads the running state back so it can be saved.
//! `docs/research/environment-setup.md`.

use std::fmt;
use std::time::Duration;

use bevy::prelude::*;

use crate::{Climate, GeoLocation, WeatherDirector, WeatherPreset, WorldClock};

/// A setup that failed [`EnvironmentSetup::validate`], or could not be applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupError {
    /// The field that broke its bound, as a path (`location.latitude_deg`).
    pub field: &'static str,
    /// What is wrong with it.
    pub problem: String,
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.problem)
    }
}

impl std::error::Error for SetupError {}

fn bad(field: &'static str, problem: impl Into<String>) -> SetupError {
    SetupError {
        field,
        problem: problem.into(),
    }
}

fn within(field: &'static str, v: f64, lo: f64, hi: f64) -> Result<(), SetupError> {
    if v.is_finite() && (lo..=hi).contains(&v) {
        Ok(())
    } else {
        Err(bad(field, format!("{v} is outside [{lo}, {hi}]")))
    }
}

/// A scene's clock, place, climate and forced weather: what an author chooses and a game loads.
///
/// With the `serde` feature it reads and writes any serde format; absent fields take their
/// defaults and unknown fields are an error, so a typo is not silently ignored.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default, deny_unknown_fields)
)]
pub struct EnvironmentSetup {
    /// Date, time of day, time scale and whether the clock runs.
    pub clock: WorldClock,
    /// Where the observer stands.
    pub location: GeoLocation,
    /// The climate the weather scheduler draws from.
    pub climate: Climate,
    /// `Some(preset)` holds that weather; `None` leaves it to the climate's scheduler.
    pub forced_weather: Option<WeatherPreset>,
    /// Game minutes a forced or released weather takes to blend, so the sky does not jump.
    pub blend_minutes: f32,
}

impl Default for EnvironmentSetup {
    fn default() -> Self {
        Self {
            clock: WorldClock::default(),
            location: GeoLocation::default(),
            climate: Climate::default(),
            forced_weather: None,
            blend_minutes: 10.0,
        }
    }
}

impl EnvironmentSetup {
    /// Checks every field against the range the models are defined on.
    pub fn validate(&self) -> Result<(), SetupError> {
        let c = &self.clock;
        within("clock.hour", c.hour, 0.0, 24.0)?;
        if c.hour >= 24.0 {
            return Err(bad("clock.hour", format!("{} is outside [0, 24)", c.hour)));
        }
        if !(c.time_scale.is_finite() && c.time_scale >= 0.0) {
            return Err(bad("clock.time_scale", format!("{} is not finite and >= 0", c.time_scale)));
        }
        let l = &self.location;
        within("location.latitude_deg", l.latitude_deg, -90.0, 90.0)?;
        within("location.longitude_deg", l.longitude_deg, -180.0, 180.0)?;
        within("location.utc_offset_hours", l.utc_offset_hours, -14.0, 14.0)?;

        let k = &self.climate;
        for (field, v) in [
            ("climate.mean_humidity", k.mean_humidity),
            ("climate.wet_hour_probability", k.wet_hour_probability),
            ("climate.storm_share", k.storm_share),
            ("climate.fog_share", k.fog_share),
            ("climate.dust_share", k.dust_share),
        ] {
            within(field, v as f64, 0.0, 1.0)?;
        }
        for (field, v) in [
            ("climate.annual_amplitude_c", k.annual_amplitude_c),
            ("climate.diurnal_amplitude_c", k.diurnal_amplitude_c),
            ("climate.mean_wind_ms", k.mean_wind_ms),
        ] {
            if !(v.is_finite() && v >= 0.0) {
                return Err(bad(field, format!("{v} is not finite and >= 0")));
            }
        }
        for (field, v) in [
            ("climate.mean_temperature_c", k.mean_temperature_c),
            ("climate.reference_elevation_m", k.reference_elevation_m),
        ] {
            if !v.is_finite() {
                return Err(bad(field, format!("{v} is not finite")));
            }
        }
        if !(1..=366).contains(&k.warmest_day) {
            return Err(bad("climate.warmest_day", format!("{} is outside [1, 366]", k.warmest_day)));
        }
        let w = k.prevailing_wind;
        if !(w.is_finite() && w.length_squared() > 1e-12) {
            return Err(bad("climate.prevailing_wind", format!("{w} is not a finite, non-zero direction")));
        }
        if !(self.blend_minutes.is_finite() && self.blend_minutes >= 0.0) {
            return Err(bad("blend_minutes", format!("{} is not finite and >= 0", self.blend_minutes)));
        }
        Ok(())
    }

    /// Validates, then writes the setup into the resources. On an error nothing is changed.
    ///
    /// The director is forced only when its forced preset differs from
    /// [`forced_weather`](Self::forced_weather), so applying an unchanged setup (a file reloaded
    /// after an unrelated edit) does not restart a blend in progress. `None` releases a forced
    /// director and leaves a scheduled one alone.
    pub fn apply(
        &self,
        clock: &mut WorldClock,
        location: &mut GeoLocation,
        climate: &mut Climate,
        director: &mut WeatherDirector,
    ) -> Result<(), SetupError> {
        self.validate()?;
        *clock = self.clock.clone();
        *location = self.location.clone();
        *climate = self.climate.clone();
        if director.forced() != self.forced_weather {
            let blend = Duration::from_secs_f32(self.blend_minutes * 60.0);
            match self.forced_weather {
                Some(preset) => director.force(preset, blend),
                None => director.release(blend),
            }
        }
        Ok(())
    }

    /// The running state as a setup, to save. `blend_minutes` is the default, which the resources
    /// do not keep.
    pub fn capture(
        clock: &WorldClock,
        location: &GeoLocation,
        climate: &Climate,
        director: &WeatherDirector,
    ) -> Self {
        Self {
            clock: clock.clone(),
            location: location.clone(),
            climate: climate.clone(),
            forced_weather: director.forced(),
            ..Self::default()
        }
    }

    /// [`apply`](Self::apply) to a world; errors when the sky and weather plugins are not added.
    pub fn apply_to_world(&self, world: &mut World) -> Result<(), SetupError> {
        self.validate()?;
        Self::capture_from_world(world)?;
        let mut clock = take::<WorldClock>(world, "WorldClock")?;
        let mut location = take::<GeoLocation>(world, "GeoLocation")?;
        let mut climate = take::<Climate>(world, "Climate")?;
        let mut director = take::<WeatherDirector>(world, "WeatherDirector")?;
        let result = self.apply(&mut clock, &mut location, &mut climate, &mut director);
        world.insert_resource(clock);
        world.insert_resource(location);
        world.insert_resource(climate);
        world.insert_resource(director);
        result
    }

    /// [`capture`](Self::capture) from a world; errors when the plugins are not added.
    pub fn capture_from_world(world: &World) -> Result<Self, SetupError> {
        let get = |name: &'static str| bad(name, "resource missing: add the sky and weather plugins");
        Ok(Self::capture(
            world.get_resource::<WorldClock>().ok_or_else(|| get("WorldClock"))?,
            world.get_resource::<GeoLocation>().ok_or_else(|| get("GeoLocation"))?,
            world.get_resource::<Climate>().ok_or_else(|| get("Climate"))?,
            world.get_resource::<WeatherDirector>().ok_or_else(|| get("WeatherDirector"))?,
        ))
    }
}

/// Removes a resource, or errors if the world lacks it. Callers reinsert it.
fn take<R: Resource>(world: &mut World, name: &'static str) -> Result<R, SetupError> {
    world
        .remove_resource::<R>()
        .ok_or_else(|| bad(name, "resource missing: add the sky and weather plugins"))
}
