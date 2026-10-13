//! Fog: distance fog from the weather's visibility, and a camera-following ground-fog volume
//! whose density a compute kernel writes every frame. See `docs/research/fog.md`.

mod gpu;

use bevy::asset::embedded_asset;
use bevy::camera::Exposure;
use bevy::light::FogVolume;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

use crate::celestial::{CelestialFrame, SunState, cloud_transmission, scene_illuminance};
use crate::climate::{Weather, WeatherDirector, WeatherPreset};
use crate::gpu::grid::storage_volume;
use crate::plugin::{ElementsCamera, ElementsSystems, ElementsView, ensure_gpu_core};

/// Koschmieder's constant for a 2 % contrast threshold: extinction = 3.912 / visibility.
pub const KOSCHMIEDER: f32 = 3.912;
/// Size of the ground-fog volume, m.
pub const GROUND_FOG_EXTENT: Vec3 = Vec3::new(800.0, 120.0, 800.0);
/// Texels of the ground-fog density texture.
pub const GROUND_FOG_TEXELS: UVec3 = UVec3::new(128, 32, 128);
/// Fog colour in full light, linear RGB.
const FOG_COLOUR: Vec3 = Vec3::new(0.75, 0.80, 0.85);
/// Tint the fog takes as the sun nears the horizon.
const DUSK_TINT: Vec3 = Vec3::new(1.0, 0.55, 0.3);
/// Sun altitude below which the dusk tint starts, degrees.
const DUSK_ONSET_DEG: f32 = 10.0;
/// Colour of airborne dust, linear RGB.
const DUST_ALBEDO: Vec3 = Vec3::new(0.76, 0.60, 0.42);
/// Ground-fog scale height in the Fog preset and otherwise, m.
const FOG_SCALE_HEIGHT_M: f32 = 25.0;
const HAZE_SCALE_HEIGHT_M: f32 = 60.0;
/// Game seconds over which the scale height follows a preset change.
const SCALE_HEIGHT_TAU_S: f32 = 600.0;
/// Single-scattering albedo of fog droplets and of dust.
const FOG_ALBEDO: f32 = 0.95;
const DUST_SINGLE_SCATTER: f32 = 0.9;
/// Largest |n| of the 2-octave `fbm3`, measured over 10⁵ points by
/// `the_kernel_normalisation_constants_match_the_noise`; maps the noise onto f ∈ [0, 1].
pub(crate) const FBM2_MAX: f32 = 0.522;
/// Noise offsets wrap at this many metres to keep f32 precision.
const OFFSET_WRAP_M: f32 = 80.0 * 4096.0;

/// Marks the camera-following ground-fog `FogVolume` this crate spawns.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct GroundFog;

/// Extinction coefficient for a visibility, m⁻¹ (Koschmieder, 2 % contrast).
pub fn extinction_for_visibility(visibility_m: f32) -> f32 {
    KOSCHMIEDER / visibility_m.max(1.0)
}

/// Distance fog for a visibility: grey extinction σ_V in every channel (fog, haze and dust
/// extinguish independently of wavelength), so every colour reaches 2 % transmittance at exactly
/// `visibility_m`; in-scattering at the same rate, so scene and fog weights sum to 1 and a fully
/// fogged pixel takes the fog colour.
pub fn distance_fog_falloff(visibility_m: f32) -> FogFalloff {
    let k = Vec3::splat(extinction_for_visibility(visibility_m));
    FogFalloff::Atmospheric {
        extinction: k,
        inscattering: k,
    }
}

/// Linear-RGB fog colour before brightness: the fog hue, tinted toward sunset orange as the sun
/// drops below 10°, and toward dust as `dust` rises.
pub fn fog_hue(sun_altitude_deg: f32, dust: f32) -> Vec3 {
    let dusk = (1.0 - sun_altitude_deg / DUSK_ONSET_DEG).clamp(0.0, 1.0);
    let tinted = FOG_COLOUR * Vec3::ONE.lerp(DUSK_TINT, dusk);
    tinted.lerp(DUST_ALBEDO, dust.clamp(0.0, 1.0))
}

/// Uniform of `density.wesl`.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub(crate) struct FogParams {
    origin: Vec4,
    extent: Vec4,
    shape: Vec4,
    size: UVec4,
}

/// The density texture and this frame's kernel parameters; extracted every frame.
#[derive(Resource, Clone)]
pub(crate) struct FogGpu {
    pub texture: Handle<Image>,
    pub params: FogParams,
    pub active: bool,
}

#[derive(Resource)]
struct FogRuntime {
    scale_height: f32,
    wind_offset: Vec2,
}

impl Default for FogRuntime {
    fn default() -> Self {
        Self {
            scale_height: HAZE_SCALE_HEIGHT_M,
            wind_offset: Vec2::ZERO,
        }
    }
}

/// Distance fog on every [`ElementsCamera`] and a ground-fog volume around the followed one.
pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        ensure_gpu_core(app);
        embedded_asset!(app, "density.wesl");
        app.init_resource::<FogRuntime>()
            .add_systems(Startup, spawn_volume)
            .add_systems(
                Update,
                (update_volume, update_distance_fog).in_set(ElementsSystems::Simulation),
            );
        gpu::plugin(app);
    }
}

fn spawn_volume(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let texture = storage_volume(&mut images, GROUND_FOG_TEXELS);
    commands.spawn((
        Name::new("Ground fog"),
        GroundFog,
        FogVolume {
            density_factor: 1.0,
            density_texture: Some(texture.clone()),
            absorption: 1.0 - FOG_ALBEDO,
            scattering: FOG_ALBEDO,
            ..default()
        },
        Transform::from_scale(GROUND_FOG_EXTENT),
        Visibility::Hidden,
    ));
    commands.insert_resource(FogGpu {
        texture,
        params: FogParams::default(),
        active: false,
    });
}

#[allow(clippy::too_many_arguments)]
fn update_volume(
    time: Res<Time<Virtual>>,
    view: Res<ElementsView>,
    weather: Res<Weather>,
    director: Res<WeatherDirector>,
    clock: Res<crate::celestial::WorldClock>,
    mut runtime: ResMut<FogRuntime>,
    gpu: Option<ResMut<FogGpu>>,
    mut volumes: Query<(&mut FogVolume, &mut Transform, &mut Visibility), With<GroundFog>>,
) {
    let Some(mut gpu) = gpu else {
        return;
    };
    let Ok((mut volume, mut transform, mut visibility)) = volumes.single_mut() else {
        return;
    };
    let Some(camera) = view.entity.map(|_| view.position) else {
        gpu.active = false;
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    visibility.set_if_neq(Visibility::Inherited);

    let dt = time.delta_secs();
    let game_dt = dt * clock.time_scale;
    let goal = if director.current() == WeatherPreset::Fog {
        FOG_SCALE_HEIGHT_M
    } else {
        HAZE_SCALE_HEIGHT_M
    };
    runtime.scale_height +=
        (goal - runtime.scale_height) * (1.0 - (-game_dt / SCALE_HEIGHT_TAU_S).exp());
    let wind = weather.wind_direction * weather.wind_speed_ms;
    runtime.wind_offset = (runtime.wind_offset + wind * dt).rem_euclid(Vec2::splat(OFFSET_WRAP_M));

    let texel = GROUND_FOG_EXTENT / GROUND_FOG_TEXELS.as_vec3();
    let ground = 0.0;
    let centre_xz = (camera.xz() / texel.xz()).round() * texel.xz();
    let centre = Vec3::new(centre_xz.x, ground + GROUND_FOG_EXTENT.y * 0.5, centre_xz.y);
    *transform = Transform::from_translation(centre).with_scale(GROUND_FOG_EXTENT);

    let dust = weather.dust.clamp(0.0, 1.0);
    let albedo = FOG_ALBEDO + (DUST_SINGLE_SCATTER - FOG_ALBEDO) * dust;
    volume.absorption = 1.0 - albedo;
    volume.scattering = albedo;
    volume.fog_color = Color::linear_rgb(1.0, 1.0, 1.0).mix(
        &Color::linear_rgb(DUST_ALBEDO.x, DUST_ALBEDO.y, DUST_ALBEDO.z),
        dust,
    );

    gpu.params = FogParams {
        origin: (centre - GROUND_FOG_EXTENT * 0.5).extend(1.0 / FBM2_MAX),
        extent: GROUND_FOG_EXTENT.extend(extinction_for_visibility(weather.visibility_m)),
        shape: Vec4::new(
            runtime.scale_height,
            ground,
            runtime.wind_offset.x,
            runtime.wind_offset.y,
        ),
        size: GROUND_FOG_TEXELS.extend(0),
    };
    gpu.active = true;
}

fn update_distance_fog(
    weather: Res<Weather>,
    sun: Res<SunState>,
    frame: Res<CelestialFrame>,
    mut cameras: Query<(&mut DistanceFog, &Exposure), With<ElementsCamera>>,
) {
    let falloff = distance_fog_falloff(weather.visibility_m);
    // Fog colour is added after exposure (bevy_pbr fog.wesl), so it carries the scene's
    // luminance times the camera's exposure: lit like everything else at a fixed exposure,
    // steady under auto exposure.
    let luminance = scene_illuminance(
        &sun,
        frame.moon_lux,
        cloud_transmission(weather.cloud_cover),
    ) / std::f32::consts::PI;
    let hue = fog_hue(sun.altitude_deg, weather.dust);
    for (mut fog, exposure) in &mut cameras {
        let brightness = luminance * exposure.exposure();
        let c = hue * brightness;
        fog.color = Color::linear_rgb(c.x, c.y, c.z);
        fog.falloff = falloff.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_channel_reaches_two_percent_at_the_visibility_and_energy_is_conserved() {
        let FogFalloff::Atmospheric {
            extinction,
            inscattering,
        } = distance_fog_falloff(300.0)
        else {
            unreachable!("distance fog is always atmospheric");
        };
        assert!((extinction - Vec3::splat(0.013_040)).abs().max_element() < 1e-6);
        assert_eq!(extinction, inscattering);
        let t = (-extinction * 300.0).exp();
        assert!((t - Vec3::splat(0.0200)).abs().max_element() < 2e-4);
        for d in [0.0, 50.0, 300.0, 5000.0] {
            let scene = (-extinction * d).exp();
            let fog = Vec3::ONE - (-inscattering * d).exp();
            assert!(((scene + fog) - Vec3::ONE).abs().max_element() < 1e-6);
        }
    }

    #[test]
    fn the_hue_tints_at_dusk_and_in_dust() {
        assert_eq!(fog_hue(90.0, 0.0), FOG_COLOUR);
        let dusk = fog_hue(5.0, 0.0) / FOG_COLOUR;
        assert!((dusk - Vec3::new(1.0, 0.775, 0.65)).length() < 1e-5);
        assert!((fog_hue(45.0, 1.0) - DUST_ALBEDO).length() < 1e-6);
    }
}
