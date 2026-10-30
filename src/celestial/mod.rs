//! Sun, moon and stars from the real sky: a world clock, Meeus-accuracy ephemerides, the sun and
//! moon as directional lights for Bevy's atmosphere, and exposure that follows them.

pub mod astro;
mod clock;
mod moon;
mod stars;

use bevy::camera::Exposure;
use bevy::light::light_consts::lux;
use bevy::light::{CascadeShadowConfigBuilder, SunDisk, VolumetricLight};
use bevy::prelude::*;

pub(crate) use clock::plugin as clock_plugin;
pub use clock::{CivilDate, GeoLocation, WorldClock};

use crate::climate::Weather;
use crate::clouds::CloudShadows;
use crate::plugin::{ElementsCamera, ElementsSystems, ensure_gpu_core};

/// Marks the sun's `DirectionalLight`.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct Sun;

/// Marks the moon's `DirectionalLight`.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct Moon;

/// Where the sun and moon are and how much of the moon is lit.
#[derive(Resource, Clone, Copy, Debug, Reflect)]
pub struct SunState {
    /// Unit world vector toward the sun (refracted), +Y up, east +X, north −Z.
    pub direction: Vec3,
    /// True (unrefracted) altitude of the sun's centre, degrees.
    pub altitude_deg: f32,
    /// Azimuth of the sun from north through east, degrees.
    pub azimuth_deg: f32,
    /// Unit world vector toward the moon (topocentric, refracted).
    pub moon_direction: Vec3,
    /// True topocentric altitude of the moon's centre, degrees.
    pub moon_altitude_deg: f32,
    /// Place in the synodic month: 0 new, 0.25 first quarter, 0.5 full, 0.75 last quarter.
    pub moon_phase: f32,
    /// Fraction of the moon's disc that is lit, 0–1.
    pub moon_illuminated_fraction: f32,
}

impl Default for SunState {
    fn default() -> Self {
        Self {
            direction: Vec3::Y,
            altitude_deg: 90.0,
            azimuth_deg: 180.0,
            moon_direction: Vec3::NEG_Y,
            moon_altitude_deg: -90.0,
            moon_phase: 0.0,
            moon_illuminated_fraction: 0.0,
        }
    }
}

/// Sky geometry for the star field and the moon disc, recomputed every frame.
#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct CelestialFrame {
    /// Takes a J2000 equatorial unit vector `(cos δ cos α, cos δ sin α, sin δ)` to world space
    /// (precession to date, Earth rotation, latitude).
    pub j2000_to_world: Mat3,
    /// Unit world vector toward the north ecliptic pole of date.
    pub ecliptic_north: Vec3,
    /// Angular diameter of the moon's disc as seen by the observer, degrees.
    pub moon_angular_diameter_deg: f32,
    /// Illuminance of the moon outside the atmosphere, lx.
    pub moon_lux: f32,
}

impl Default for CelestialFrame {
    fn default() -> Self {
        Self {
            j2000_to_world: Mat3::IDENTITY,
            ecliptic_north: Vec3::Y,
            moon_angular_diameter_deg: 0.5,
            moon_lux: 0.0,
        }
    }
}

/// Sun, moon and exposure.
///
/// Spawns one [`Sun`] and one [`Moon`] `DirectionalLight` at startup and moves them every frame
/// in [`ElementsSystems::Celestial`]; keeps [`SunState`] current; drives `Exposure` on every
/// [`ElementsCamera`] with `auto_exposure` set.
pub struct CelestialPlugin;

impl Plugin for CelestialPlugin {
    fn build(&self, app: &mut App) {
        ensure_gpu_core(app);
        stars::plugin(app);
        moon::plugin(app);
        app.init_resource::<CelestialFrame>()
            .add_systems(Startup, spawn_lights)
            .add_systems(
                Update,
                (update_sky, auto_exposure)
                    .chain()
                    .in_set(ElementsSystems::Celestial),
            );
    }
}

/// Atmospheric pressure and temperature the refraction correction assumes, mbar and °C
/// (Saemundsson's reference state).
const REFRACTION_PRESSURE_MBAR: f64 = 1010.0;
const REFRACTION_TEMP_C: f64 = 10.0;
/// Below this true altitude a light is switched off, degrees.
const SUN_OFF_DEG: f32 = -2.0;
const MOON_OFF_DEG: f32 = -0.5;
/// Moon shadows are drawn only when the sun is lower than this, degrees (civil twilight).
const MOON_SHADOWS_BELOW_SUN_DEG: f32 = -6.0;

/// How far the sun's shadow cascades reach, m. Volumetric fog takes its sunlight through the
/// shadow map, so beyond the last cascade fog gets no light at all and turns into a dark band
/// toward the horizon; the cascades cover the ground-fog volume (±400 m) with margin.
const SUN_SHADOW_DISTANCE_M: f32 = 600.0;

fn spawn_lights(mut commands: Commands) {
    commands.spawn((
        Name::new("Sun"),
        Sun,
        DirectionalLight {
            illuminance: lux::RAW_SUNLIGHT,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            maximum_distance: SUN_SHADOW_DISTANCE_M,
            first_cascade_far_bound: 15.0,
            ..default()
        }
        .build(),
        VolumetricLight,
        Transform::default(),
    ));
    commands.spawn((
        Name::new("Moon"),
        Moon,
        DirectionalLight {
            illuminance: 0.0,
            shadow_maps_enabled: false,
            color: Color::srgb(0.88, 0.92, 1.0),
            ..default()
        },
        SunDisk::OFF,
        Transform::default(),
    ));
}

/// East-north-up to world: +X east, +Y up, −Z north.
fn enu_to_world(v: [f64; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[2] as f32, -v[1] as f32)
}

/// Fraction of clear-sky solar irradiance reaching the ground under fractional cloud cover
/// (Kasten & Czeplak 1980, Solar Energy 24: 177): `1 − 0.75·cover^3.4`.
pub(crate) fn cloud_transmission(cover: f32) -> f32 {
    1.0 - 0.75 * cover.clamp(0.0, 1.0).powf(3.4)
}

/// Factor on the sun's and moon's light: with the cloud shadow map, the Kasten & Czeplak overcast
/// floor with the obscuration measured, `0.25 + 0.75 · T̄` (T̄ the camera's mean direct
/// transmittance); without [`CloudShadows`], the cover-only [`cloud_transmission`].
pub(crate) fn sky_dimming(cover: f32, shadows: Option<&CloudShadows>) -> f32 {
    match shadows {
        Some(shadows) => 0.25 + 0.75 * shadows.camera_transmittance.clamp(0.0, 1.0),
        None => cloud_transmission(cover),
    }
}

fn light_transform(toward: Vec3) -> Transform {
    let up = if toward.y.abs() > 0.999 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    Transform::default().looking_to(-toward, up)
}

pub(crate) fn update_sky(
    clock: Res<WorldClock>,
    location: Res<GeoLocation>,
    weather: Res<Weather>,
    shadows: Option<Res<CloudShadows>>,
    mut state: ResMut<SunState>,
    mut frame: ResMut<CelestialFrame>,
    mut lights: Query<(&mut Transform, &mut DirectionalLight, Has<Sun>, Has<Moon>)>,
) {
    use astro::*;
    let jd = clock.julian_day(location.utc_offset_hours);
    let year = f64::from(clock.date.year) + (f64::from(clock.date.month) - 0.5) / 12.0;
    let t = centuries(jd + delta_t_seconds(year) / 86_400.0);
    let nut = nutation(t);
    let lat = location.latitude_deg;
    let lst = local_sidereal_deg(jd, &nut, location.longitude_deg);

    let (sun, sun_lambda) = sun_apparent(t);
    let (sun_alt, sun_az) = horizontal(sun.ra, sun.dec, lat, lst);
    let sun_app = sun_alt + refraction_deg(sun_alt, REFRACTION_PRESSURE_MBAR, REFRACTION_TEMP_C);
    let sun_dir = enu_to_world(enu(sun_app, sun_az)).normalize_or(Vec3::Y);

    let (moon, moon_ecl) = moon_apparent(t, &nut);
    let (geo_alt, geo_az) = horizontal(moon.ra, moon.dec, lat, lst);
    let (moon_alt, moon_az) = topocentric(geo_alt, geo_az, moon.distance_km);
    let moon_app = moon_alt + refraction_deg(moon_alt, REFRACTION_PRESSURE_MBAR, REFRACTION_TEMP_C);
    let moon_dir = enu_to_world(enu(moon_app, moon_az)).normalize_or(Vec3::NEG_Y);
    let i = phase_angle(&sun, &moon);
    let lit = illuminated_fraction(i);
    let synodic = normalize_deg(moon_ecl.longitude + nut.dpsi - sun_lambda) / 360.0;
    let observer_distance = moon.distance_km - EARTH_RADIUS_KM * (moon_alt.to_radians()).sin();

    *state = SunState {
        direction: sun_dir,
        altitude_deg: sun_alt as f32,
        azimuth_deg: sun_az as f32,
        moon_direction: moon_dir,
        moon_altitude_deg: moon_alt as f32,
        moon_phase: synodic as f32,
        moon_illuminated_fraction: lit as f32,
    };

    let to_enu = mat_mul(equatorial_to_enu(lat, lst), precession_matrix(t));
    let col = |j: usize| enu_to_world([to_enu[0][j], to_enu[1][j], to_enu[2][j]]);
    let eps = nut.eps().to_radians();
    let pole = [0.0, -eps.sin(), eps.cos()];
    let pole_enu = equatorial_to_enu(lat, lst);
    let pole_world = enu_to_world([
        (0..3).map(|k| pole_enu[0][k] * pole[k]).sum(),
        (0..3).map(|k| pole_enu[1][k] * pole[k]).sum(),
        (0..3).map(|k| pole_enu[2][k] * pole[k]).sum(),
    ]);
    let moon_lux = (FULL_MOON_LUX * moon_phase_factor(i)) as f32;
    *frame = CelestialFrame {
        j2000_to_world: Mat3::from_cols(col(0), col(1), col(2)),
        ecliptic_north: pole_world.normalize_or(Vec3::Y),
        moon_angular_diameter_deg: moon_angular_diameter(observer_distance) as f32,
        moon_lux,
    };

    let clouds = sky_dimming(weather.cloud_cover, shadows.as_deref());
    for (mut transform, mut light, is_sun, is_moon) in &mut lights {
        if is_sun {
            *transform = light_transform(sun_dir);
            let up = state.altitude_deg > SUN_OFF_DEG;
            light.illuminance = if up { lux::RAW_SUNLIGHT * clouds } else { 0.0 };
            set_shadows(&mut light, up);
        } else if is_moon {
            *transform = light_transform(moon_dir);
            let up = state.moon_altitude_deg > MOON_OFF_DEG;
            light.illuminance = if up { moon_lux * clouds } else { 0.0 };
            set_shadows(
                &mut light,
                up && state.altitude_deg < MOON_SHADOWS_BELOW_SUN_DEG,
            );
        }
    }
}

/// Write the shadow flag only when it changes, so the light is not re-flagged as modified.
fn set_shadows(light: &mut Mut<DirectionalLight>, on: bool) {
    if light.shadow_maps_enabled != on {
        light.shadow_maps_enabled = on;
    }
}

/// Real-time constant of the exposure adaptation, s.
const EXPOSURE_TAU_S: f32 = 1.5;
/// Illuminance floor so a moonless night still has a finite exposure, lx.
const ILLUMINANCE_FLOOR_LUX: f32 = 0.0005;

/// Horizontal illuminance from the sun and the moon, direct plus sky, lx: each body's
/// illuminance outside the atmosphere times the USNO total-illuminance factor at its refracted
/// altitude (which carries twilight), dimmed by cloud, plus a floor.
pub(crate) fn scene_illuminance(state: &SunState, moon_lux: f32, clouds: f32) -> f32 {
    let factor = |true_altitude: f32| {
        let h = f64::from(true_altitude);
        let apparent = h + astro::refraction_deg(h, REFRACTION_PRESSURE_MBAR, REFRACTION_TEMP_C);
        astro::horizontal_illuminance_factor(apparent) as f32
    };
    let sun = lux::RAW_SUNLIGHT * factor(state.altitude_deg);
    let moon = moon_lux * factor(state.moon_altitude_deg);
    (sun + moon) * clouds + ILLUMINANCE_FLOOR_LUX
}

/// Peak luminance of the display the exposure targets, cd/m² (an SDR monitor).
const DISPLAY_MAX_NITS: f32 = 100.0;
/// Reflectance of the surface the eye is assumed to adapt to (mid grey).
const ADAPTING_REFLECTANCE: f32 = 0.18;

/// Base-10 log of the just-noticeable luminance difference at adaptation luminance `la`, cd/m²
/// (Larson, Rushmeier & Piatko 1997, Table 1: rods and cones, after Ferwerda et al. 1996).
pub(crate) fn log10_threshold(la: f32) -> f32 {
    let x = la.max(1e-12).log10();
    if x < -3.94 {
        -2.86
    } else if x < -1.44 {
        (0.405 * x + 1.6).powf(2.18) - 2.86
    } else if x < -0.0184 {
        x - 0.395
    } else if x < 1.9 {
        (0.249 * x + 0.65).powf(2.7) - 0.72
    } else {
        x - 1.255
    }
}

/// Exposure for a scene lit by `lux` of horizontal illuminance.
///
/// Ward's contrast-based scale factor (Graphics Gems IV, 1994) with the threshold function of
/// Larson et al. 1997: world luminance is scaled by the ratio of the display's and the world's
/// just-noticeable differences, `ΔL(L_d,max/2) / ΔL(0.18·E/π)`, so a contrast that is just
/// visible in the world is just visible on screen. A moonlit scene therefore stays dark instead
/// of being exposed up to daylight. Bevy multiplies scene luminance by `2^−EV₁₀₀ / 1.2` and
/// treats 1 as display peak, so `EV₁₀₀ = −log₂(1.2 · m / L_d,max)`.
pub(crate) fn ev100_for(lux: f32) -> f32 {
    -(1.2 * exposure_for_adaptation(ADAPTING_REFLECTANCE * lux / std::f32::consts::PI)).log2()
}

/// The factor scene luminance is multiplied by for an eye adapted to luminance `la`, cd/m²
/// (`m / L_d,max` of [`ev100_for`]; Bevy's `Exposure::exposure()` for the matching EV).
pub(crate) fn exposure_for_adaptation(la: f32) -> f32 {
    let display = DISPLAY_MAX_NITS * 0.5;
    10f32.powf(log10_threshold(display) - log10_threshold(la)) / DISPLAY_MAX_NITS
}

fn auto_exposure(
    time: Res<Time<Real>>,
    state: Res<SunState>,
    frame: Res<CelestialFrame>,
    weather: Res<Weather>,
    shadows: Option<Res<CloudShadows>>,
    mut cameras: Query<(Entity, &ElementsCamera, &mut Exposure)>,
    mut adapted: Local<Vec<Entity>>,
) {
    let target = ev100_for(scene_illuminance(
        &state,
        frame.moon_lux,
        sky_dimming(weather.cloud_cover, shadows.as_deref()),
    ));
    let k = 1.0 - (-time.delta_secs() / EXPOSURE_TAU_S).exp();
    for (entity, camera, mut exposure) in &mut cameras {
        if !camera.auto_exposure {
            continue;
        }
        if adapted.contains(&entity) {
            exposure.ev100 += (target - exposure.ev100) * k;
        } else {
            exposure.ev100 = target;
            adapted.push(entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn exposure_targets_match_the_research_note() {
        let zenith_sun = SunState {
            altitude_deg: 90.0,
            moon_altitude_deg: -10.0,
            ..default()
        };
        assert!((ev100_for(scene_illuminance(&zenith_sun, 0.0, 1.0)) - 13.3360).abs() < 1e-3);
        let full_moon = SunState {
            altitude_deg: -30.0,
            moon_altitude_deg: 90.0,
            ..default()
        };
        let ev = ev100_for(scene_illuminance(
            &full_moon,
            astro::FULL_MOON_LUX as f32,
            1.0,
        ));
        assert!((ev - -2.2353).abs() < 1e-3, "{ev}");
        let dark = SunState {
            altitude_deg: -30.0,
            moon_altitude_deg: -30.0,
            ..default()
        };
        assert!((ev100_for(scene_illuminance(&dark, 0.0, 1.0)) - -4.7465).abs() < 1e-3);
        // Sunrise and civil twilight keep the sky's light (USNO), so dawn is not metered as night.
        let sunrise = SunState {
            altitude_deg: 0.0,
            moon_altitude_deg: -30.0,
            ..default()
        };
        assert!((ev100_for(scene_illuminance(&sunrise, 0.0, 1.0)) - 6.4780).abs() < 1e-3);
        let civil = SunState {
            altitude_deg: -6.0,
            moon_altitude_deg: -30.0,
            ..default()
        };
        assert!((ev100_for(scene_illuminance(&civil, 0.0, 1.0)) - 0.8567).abs() < 2e-3);
    }

    #[test]
    fn overcast_passes_a_quarter_of_the_sun() {
        assert_eq!(cloud_transmission(0.0), 1.0);
        assert!((cloud_transmission(1.0) - 0.25).abs() < 1e-6);
    }

    fn shadows(camera_transmittance: f32) -> CloudShadows {
        CloudShadows {
            map: Handle::default(),
            origin: Vec2::ZERO,
            extent: 8000.0,
            light_direction: Vec3::Y,
            camera_transmittance,
        }
    }

    /// Run `update_sky` once at `hour` on 2026-06-21 and return the state, the frame, the sun's
    /// and the moon's `DirectionalLight::illuminance`.
    fn run_sky(
        hour: f64,
        cover: f32,
        shadows: Option<CloudShadows>,
    ) -> (SunState, CelestialFrame, f32, f32) {
        let mut world = World::new();
        world.insert_resource(WorldClock {
            date: CivilDate::new(2026, 6, 21),
            hour,
            ..default()
        });
        world.insert_resource(GeoLocation::default());
        world.insert_resource(Weather {
            cloud_cover: cover,
            ..default()
        });
        world.insert_resource(SunState::default());
        world.insert_resource(CelestialFrame::default());
        if let Some(shadows) = shadows {
            world.insert_resource(shadows);
        }
        let sun = world
            .spawn((Sun, DirectionalLight::default(), Transform::default()))
            .id();
        let moon = world
            .spawn((Moon, DirectionalLight::default(), Transform::default()))
            .id();
        assert!(world.run_system_once(update_sky).is_ok());
        let state = *world.resource::<SunState>();
        let frame = *world.resource::<CelestialFrame>();
        let lux_of = |entity| {
            world
                .get::<DirectionalLight>(entity)
                .map_or(f32::NAN, |light| light.illuminance)
        };
        let (sun_lux, moon_lux) = (lux_of(sun), lux_of(moon));
        (state, frame, sun_lux, moon_lux)
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-3 * b.abs().max(1e-9)
    }

    #[test]
    fn cloud_shadow_transmittance_dims_the_sun_and_moon() {
        let cover = 0.5;
        // A moon-up hour, found with the same ephemeris the system uses.
        let moon_hour = (0..24)
            .map(f64::from)
            .find(|&h| run_sky(h, cover, None).0.moon_altitude_deg > MOON_OFF_DEG);
        assert!(
            moon_hour.is_some(),
            "the moon is up at some hour of the day"
        );
        let moon_hour = moon_hour.unwrap_or(0.0);

        for (t, factor) in [(1.0, 1.0), (0.5, 0.625), (0.0, 0.25)] {
            let (state, _, sun_lux, _) = run_sky(12.0, cover, Some(shadows(t)));
            assert!(state.altitude_deg > SUN_OFF_DEG);
            assert!(
                close(sun_lux, lux::RAW_SUNLIGHT * factor),
                "sun T={t}: {sun_lux}"
            );

            let (state, frame, _, moon_lux) = run_sky(moon_hour, cover, Some(shadows(t)));
            assert!(state.moon_altitude_deg > MOON_OFF_DEG);
            assert!(frame.moon_lux > 0.0);
            assert!(
                close(moon_lux, frame.moon_lux * factor),
                "moon T={t}: {moon_lux}"
            );

            let dimmed = scene_illuminance(
                &state,
                frame.moon_lux,
                sky_dimming(cover, Some(&shadows(t))),
            );
            let open = scene_illuminance(&state, frame.moon_lux, 1.0) - ILLUMINANCE_FLOOR_LUX;
            assert!(close(dimmed, open * factor + ILLUMINANCE_FLOOR_LUX));
        }

        // Without the cloud shadow map the cover-only factor stays.
        let (_, _, sun_lux, _) = run_sky(12.0, cover, None);
        assert!(close(
            sun_lux,
            lux::RAW_SUNLIGHT * cloud_transmission(cover)
        ));
        let (_, frame, _, moon_lux) = run_sky(moon_hour, cover, None);
        assert!(close(moon_lux, frame.moon_lux * cloud_transmission(cover)));

        // Before the first readback the camera transmittance is `1 − cover^3.4`: same factor.
        for c in [0.0f32, 0.3, 0.7, 1.0] {
            let s = shadows(1.0 - c.powf(3.4));
            assert!((sky_dimming(c, Some(&s)) - cloud_transmission(c)).abs() < 1e-6);
            assert_eq!(sky_dimming(c, None), cloud_transmission(c));
        }
    }
}
