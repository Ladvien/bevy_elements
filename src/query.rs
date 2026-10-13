//! Gameplay queries: one system parameter that answers from the latest GPU readback.

use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::celestial::{SunState, WorldClock};
use crate::climate::Weather;
use crate::wind::{WindSnapshot, WindState};

/// CPU copies of GPU simulation state, filled by the throttled readback
/// ([`ElementsReadback`](crate::ElementsReadback)). Read it through [`Environment`].
#[derive(Resource, Default, Clone, Debug)]
pub struct EnvironmentSnapshot {
    /// The wind field at the last readback, if any.
    pub(crate) wind: Option<WindSnapshot>,
    /// Mean direct cloud transmittance around the camera at the last readback, 0–1.
    pub(crate) cloud_transmittance: Option<f32>,
    /// Real time of the last completed readback.
    pub(crate) updated_at: Option<Duration>,
}

/// Ask about the environment from gameplay systems. Order such systems
/// `.after(ElementsSystems::Readback)`.
///
/// Spatial answers come from a CPU snapshot of GPU state taken every
/// [`ElementsReadback::interval`](crate::ElementsReadback) (4 Hz by default), so they are up to
/// one interval plus a frame or two old; [`snapshot_age`](Self::snapshot_age) says how old.
#[derive(SystemParam)]
pub struct Environment<'w> {
    snapshot: Res<'w, EnvironmentSnapshot>,
    weather: Res<'w, Weather>,
    clock: Res<'w, WorldClock>,
    sun: Res<'w, SunState>,
    wind: Res<'w, WindState>,
    time: Res<'w, Time<Real>>,
}

impl Environment<'_> {
    /// The weather now.
    pub fn weather(&self) -> &Weather {
        &self.weather
    }

    /// The world clock.
    pub fn clock(&self) -> &WorldClock {
        &self.clock
    }

    /// Air temperature at the climate's reference elevation, °C.
    pub fn temperature_c(&self) -> f32 {
        self.weather.temperature_c
    }

    /// Sun and moon positions and the moon's phase.
    pub fn sun(&self) -> &SunState {
        &self.sun
    }

    /// Whether the sun's upper limb is above the horizon (true altitude > −0.833°).
    pub fn is_daylight(&self) -> bool {
        self.sun.altitude_deg > -0.833
    }

    /// Wind velocity at a world position, m/s. Inside the simulated field this is the GPU wind
    /// at the last readback; outside it, or before the first readback, the prevailing wind at
    /// 10 m.
    pub fn wind_at(&self, position: Vec3) -> Vec3 {
        self.snapshot
            .wind
            .as_ref()
            .and_then(|w| w.sample(position))
            .unwrap_or_else(|| self.wind.prevailing_velocity())
    }

    /// Real time since the snapshot was last filled; `Duration::MAX` before the first readback.
    pub fn snapshot_age(&self) -> Duration {
        self.snapshot
            .updated_at
            .map_or(Duration::MAX, |at| self.time.elapsed().saturating_sub(at))
    }
}
