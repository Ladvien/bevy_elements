//! Gameplay queries: one system parameter that answers from the latest GPU readback.

use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::celestial::{
    CelestialFrame, CloudLight, SunState, WorldClock, beam_transmittance, scene_illuminance,
};
use crate::climate::Weather;
use crate::clouds::CloudShadowSnapshot;
use crate::cover::CoverSnapshot;
use crate::hydrology::{WaterDepthSnapshot, WaterVelocitySnapshot};
use crate::plugin::ElementsUnits;
use crate::terrain::{TerrainLeaves, TerrainSnapshot};
use crate::wind::{WindSnapshot, WindState};

/// CPU copies of GPU simulation state, filled by the throttled readback
/// ([`ElementsReadback`](crate::ElementsReadback)). Read it through [`Environment`].
#[derive(Resource, Default, Clone, Debug)]
pub struct EnvironmentSnapshot {
    /// The wind field at the last readback, if any.
    pub(crate) wind: Option<WindSnapshot>,
    /// The cloud shadow map's query grid at the last readback.
    pub(crate) cloud_shadow: Option<CloudShadowSnapshot>,
    /// The terrain surface heights at the last readback, if a terrain exists.
    pub(crate) terrain: Option<TerrainSnapshot>,
    /// The terrain's leaf-patch bounds at the last readback.
    pub(crate) terrain_leaves: Option<TerrainLeaves>,
    /// The water depth at the last readback, if a terrain with hydrology exists.
    pub(crate) water_depth: Option<WaterDepthSnapshot>,
    /// The depth-weighted water velocity at the last readback (half resolution).
    pub(crate) water_velocity: Option<WaterVelocitySnapshot>,
    /// The snow depth and wetness at the last readback, if a terrain with ground cover exists.
    pub(crate) cover: Option<CoverSnapshot>,
    /// Real time of the last completed readback.
    pub(crate) updated_at: Option<Duration>,
}

impl EnvironmentSnapshot {
    /// Height of the ground at `xz` (metre frame: world ÷ `ElementsUnits::units_per_metre`), m:
    /// the terrain surface at the last readback where there is terrain, the plane y = 0 elsewhere
    /// (outside it, without one, or before the first readback).
    pub(crate) fn ground_height(&self, xz: Vec2) -> f32 {
        self.terrain
            .as_ref()
            .and_then(|t| t.height(xz))
            .unwrap_or(0.0)
    }
}

/// Ask about the environment from gameplay systems. Order such systems
/// `.after(ElementsSystems::Readback)`.
///
/// Positions are world units ([`ElementsUnits`]: the queries convert them to the crate's metre
/// frame), and so is [`terrain_height`](Self::terrain_height)'s answer; every other answer is SI
/// (depths in m, speeds in m/s).
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
    /// The moon's illuminance; absent without the celestial plugin (no moonlight then).
    frame: Option<Res<'w, CelestialFrame>>,
    wind: Res<'w, WindState>,
    time: Res<'w, Time<Real>>,
    units: Res<'w, ElementsUnits>,
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
            .and_then(|w| w.sample(self.units.point_to_metres(position)))
            .unwrap_or_else(|| self.wind.prevailing_velocity())
    }

    /// World height of the terrain surface (bedrock + sediment) at `position` (world XZ), in
    /// world units. `None` outside the terrain, when there is none, and before the first
    /// readback; the value is the surface at the last readback, up to one readback interval old.
    pub fn terrain_height(&self, position: Vec2) -> Option<f32> {
        self.terrain_height_metres(self.xz_to_metres(position))
            .map(|h| self.units.world(h))
    }

    /// [`terrain_height`](Self::terrain_height) in the metre frame: at a metre-frame XZ, in m.
    pub(crate) fn terrain_height_metres(&self, position: Vec2) -> Option<f32> {
        self.snapshot
            .terrain
            .as_ref()
            .and_then(|t| t.height(position))
    }

    /// A world XZ in the metre frame.
    fn xz_to_metres(&self, position: Vec2) -> Vec2 {
        position / self.units.per_metre()
    }

    /// Depth of the water above the ground at `position` (world XZ), m: `Some(0.0)` on dry
    /// ground, `None` outside the terrain, when there is none, and before the first readback
    /// (the simulation has to have run). A bilinear sample of the last readback, up to one
    /// readback interval old.
    pub fn water_depth(&self, position: Vec2) -> Option<f32> {
        self.water_depth_metres(self.xz_to_metres(position))
    }

    /// [`water_depth`](Self::water_depth) at a metre-frame XZ.
    pub(crate) fn water_depth_metres(&self, position: Vec2) -> Option<f32> {
        self.snapshot
            .water_depth
            .as_ref()
            .and_then(|w| w.depth_at(position))
    }

    /// Depth-averaged velocity of the water at `position` (world XZ), m/s along world X and Z:
    /// zero where the water is shallower than a millimetre. `None` outside the terrain, when
    /// there is none, and before the first readback. Sampled from a half-resolution field.
    pub fn water_velocity(&self, position: Vec2) -> Option<Vec2> {
        self.snapshot
            .water_velocity
            .as_ref()
            .and_then(|w| w.velocity_at(self.xz_to_metres(position)))
    }

    /// Depth of the snow on the ground at `position` (world XZ), m: `Some(0.0)` on bare ground,
    /// `None` outside the terrain, when there is none, and before the first readback (the cover
    /// simulation has to have run). A bilinear sample of the last readback, up to one readback
    /// interval old.
    pub fn snow_depth(&self, position: Vec2) -> Option<f32> {
        self.snapshot
            .cover
            .as_ref()
            .and_then(|c| c.depth_at(self.xz_to_metres(position)))
    }

    /// How wet the ground is at `position` (world XZ), 0 dry to 1 soaked (a full film; puddles
    /// are water depth, see [`water_depth`](Self::water_depth)). `None` outside the terrain,
    /// when there is none, and before the first readback. A bilinear sample of the last
    /// readback, up to one readback interval old.
    pub fn wetness(&self, position: Vec2) -> Option<f32> {
        self.snapshot
            .cover
            .as_ref()
            .and_then(|c| c.wetness_at(self.xz_to_metres(position)))
    }

    /// Direct transmittance of the clouds toward the sun (the moon once the sun is 4° below the
    /// horizon) at a world position, 0 under a thick cloud's shadow to 1 in the open: the cloud
    /// shadow map, sampled every 31.25 m over 8 km around the camera, with `position` projected
    /// along the light as the ground's material does; 1 outside the map. `None` before the first
    /// readback. Terrain, buildings and other meshes do not shadow it: it is the cloud's shadow
    /// only.
    pub fn cloud_shadow_at(&self, position: Vec3) -> Option<f32> {
        self.snapshot
            .cloud_shadow
            .as_ref()
            .map(|s| s.transmittance_at(self.units.point_to_metres(position)))
    }

    /// Horizontal illuminance at a world position, lx: the sun's and the moon's clear-sky
    /// illuminance (direct plus sky, twilight included) times `0.25 + 0.75 · T`, with `T`
    /// [`cloud_shadow_at`](Self::cloud_shadow_at) (the cover-only estimate `1 − cover^3.4`
    /// before the first readback). The cloud's shadow only, like `cloud_shadow_at`; about
    /// 0.0005 lx on a moonless night (and without the celestial plugin, the moon is not counted).
    pub fn illuminance_at(&self, position: Vec3) -> f32 {
        let beam = self
            .cloud_shadow_at(position)
            .unwrap_or_else(|| beam_transmittance(self.weather.cloud_cover));
        let moon_lux = self.frame.as_ref().map_or(0.0, |f| f.moon_lux);
        scene_illuminance(&self.sun, moon_lux, CloudLight::from_beam(beam))
    }

    /// Real time since the snapshot was last filled; `Duration::MAX` before the first readback.
    pub fn snapshot_age(&self) -> Duration {
        self.snapshot
            .updated_at
            .map_or(Duration::MAX, |at| self.time.elapsed().saturating_sub(at))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::celestial::CelestialFrame;
    use crate::clouds::CloudShadowSnapshot;

    /// A world with the sun at 40° toward +X and a shadow grid whose cell (10, 20) is
    /// shadowed (T = 0) and every other cell open (T = 1).
    fn world(snapshot: Option<CloudShadowSnapshot>) -> World {
        let mut world = World::new();
        let elevation = 40f32.to_radians();
        world.insert_resource(EnvironmentSnapshot {
            cloud_shadow: snapshot,
            ..default()
        });
        world.insert_resource(Weather {
            cloud_cover: 0.5,
            ..default()
        });
        world.insert_resource(WorldClock::default());
        world.insert_resource(SunState {
            direction: Vec3::new(elevation.cos(), elevation.sin(), 0.0),
            altitude_deg: 40.0,
            moon_altitude_deg: -30.0,
            ..default()
        });
        world.insert_resource(CelestialFrame::default());
        world.insert_resource(WindState::default());
        world.insert_resource(Time::<Real>::default());
        world.insert_resource(ElementsUnits::default());
        world
    }

    fn shadowed_cell() -> (CloudShadowSnapshot, Vec2) {
        let grid = CloudShadowSnapshot::grid(Vec2::splat(-4000.0));
        let mut t = vec![1.0; (grid.size.x * grid.size.y) as usize];
        t[grid.index(IVec2::new(10, 20))] = 0.0;
        let centre = grid.origin + (Vec2::new(10.0, 20.0) + 0.5) * grid.cell_m;
        let elevation = 40f32.to_radians();
        let snapshot = CloudShadowSnapshot {
            grid,
            light: Vec3::new(elevation.cos(), elevation.sin(), 0.0),
            transmittance: Arc::from(t),
            mean: 1.0,
        };
        (snapshot, centre)
    }

    fn ask<T: Send + 'static>(world: &mut World, f: fn(&Environment) -> T) -> T {
        world
            .run_system_once(move |env: Environment| f(&env))
            .expect("the query system runs")
    }

    #[test]
    fn the_cloud_shadow_is_unknown_before_a_readback_and_estimated_from_cover_for_light() {
        let mut w = world(None);
        assert_eq!(ask(&mut w, |e| e.cloud_shadow_at(Vec3::ZERO)), None);
        let estimated = ask(&mut w, |e| e.illuminance_at(Vec3::ZERO));
        let open = {
            let mut w = world(None);
            w.resource_mut::<Weather>().cloud_cover = 0.0;
            ask(&mut w, |e| e.illuminance_at(Vec3::ZERO))
        };
        let expected = 0.25 + 0.75 * (1.0 - 0.5f32.powf(3.4));
        assert!(
            (estimated / open - expected).abs() < 1e-3,
            "{estimated} / {open}"
        );
    }

    #[test]
    fn a_point_in_a_cloud_shadow_is_dark_and_one_beside_it_or_above_it_is_lit() {
        let (snapshot, centre) = shadowed_cell();
        let cell = snapshot.grid.cell_m;
        let mut w = world(Some(snapshot));
        w.insert_resource(Shadowed(centre, cell));
        let (at, beside, above, raised, outside, lit, dark) = w
            .run_system_once(|env: Environment, s: Res<Shadowed>| {
                let (c, cell) = (s.0, s.1);
                let on = Vec3::new(c.x, 0.0, c.y);
                // 100 m up, the shadow of the same cloud lies 100 / tan 40° further toward the
                // light, so the point that sees through the shadowed cell is that far along +X.
                let shift = 100.0 / 40f32.to_radians().tan();
                (
                    env.cloud_shadow_at(on),
                    env.cloud_shadow_at(on + Vec3::X * 2.0 * cell),
                    env.cloud_shadow_at(on + Vec3::Y * 100.0),
                    env.cloud_shadow_at(on + Vec3::new(shift, 100.0, 0.0)),
                    env.cloud_shadow_at(Vec3::new(9000.0, 0.0, 0.0)),
                    env.illuminance_at(on + Vec3::X * 2.0 * cell),
                    env.illuminance_at(on),
                )
            })
            .expect("the query system runs");
        assert_eq!(at, Some(0.0));
        assert_eq!(beside, Some(1.0));
        assert_eq!(above, Some(1.0));
        assert!(raised.is_some_and(|t| t < 1e-3), "{raised:?}");
        assert_eq!(outside, Some(1.0));
        // The shadow keeps the overcast quarter of the clear-sky light.
        assert!((dark / lit - 0.25).abs() < 1e-3, "{dark} / {lit}");
    }

    #[derive(Resource)]
    struct Shadowed(Vec2, f32);

    /// A flat terrain 2 m above the origin: 32 cells of 1 m from (0, 0) in the metre frame.
    fn flat_terrain() -> TerrainSnapshot {
        TerrainSnapshot {
            grid: crate::terrain::TerrainGrid {
                origin: Vec2::ZERO,
                cell_m: 1.0,
                size: UVec2::splat(32),
                y0: 2.0,
            },
            surface: Arc::from(vec![0.0; 32 * 32]),
            version: 1,
        }
    }

    #[test]
    fn in_feet_the_terrain_height_is_in_world_units_and_the_position_is_read_in_world_units() {
        let mut w = world(None);
        w.resource_mut::<EnvironmentSnapshot>().terrain = Some(flat_terrain());
        w.insert_resource(ElementsUnits {
            units_per_metre: 3.2808,
        });
        // (10 m, 10 m) is (32.808, 32.808) feet; 2 m is 6.5616 feet.
        let height = ask(&mut w, |e| e.terrain_height(Vec2::splat(32.808)));
        assert!(
            height.is_some_and(|h| (h - 6.5616).abs() < 1e-4),
            "{height:?}"
        );
        // 40 m is outside the 32 m grid, though 40 feet is not.
        assert_eq!(
            ask(&mut w, |e| e.terrain_height(Vec2::splat(40.0 * 3.2808))),
            None
        );
        assert!(ask(&mut w, |e| e.terrain_height(Vec2::splat(40.0))).is_some());
    }

    #[test]
    fn in_metres_the_terrain_height_is_unchanged() {
        let mut w = world(None);
        w.resource_mut::<EnvironmentSnapshot>().terrain = Some(flat_terrain());
        assert_eq!(
            ask(&mut w, |e| e.terrain_height(Vec2::splat(10.0))),
            Some(2.0)
        );
    }
}
