//! Heat haze: the air above a fire shimmers. Each [`HeatSource`] is a fire of a heat release rate
//! and a base radius; its plume's temperature, width, rise speed, pulse and lean in the wind come
//! from fire-engineering correlations ([`model`]), its refractive-index deficit from the air's
//! density, and a screen-space pass bends each pixel's ray through the plumes it passes and draws
//! the background point the bent ray reaches, only over the pixels the plumes cover
//! (`docs/research/heat-haze.md`).
//!
//! Drawn into every [`ElementsCamera`] after the main pass and the precipitation, before bloom and
//! tonemapping; it reads the depth prepass, so the camera needs `Msaa::Off` and an HDR target, as
//! every `ElementsCamera` has.

mod gpu;
pub(crate) mod model;

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;

use crate::climate::Weather;
use crate::plugin::{
    ElementsCamera, ElementsSystems, ElementsUnits, ElementsView, ensure_gpu_core,
    needs_depth_prepass,
};
use crate::wind::{WindState, log_wind_speed};
use model::{
    KELVIN, flame_height, height_at_excess, plume_sigma, puffing_frequency, refractivity,
    rise_speed, rise_time,
};

/// Most heat sources drawn per frame; the nearest to the camera win.
pub const MAX_HEAT_SOURCES: usize = 16;
/// The plume ends where its centreline is this much warmer than the air, K (Deviation 5).
const PLUME_TOP_EXCESS_K: f32 = 10.0;
/// The wind tilts the chord from the fire to where the plume is this much warmer, K (Model 7).
const TILT_REFERENCE_EXCESS_K: f32 = 30.0;
/// The steepest lean the wind gives a plume, from the vertical, radians (75°, Deviation 3).
const MAX_TILT: f32 = 1.308_997;
/// A plume is drawn out to this many 1/e radii from its axis.
const PLUME_REACH_SIGMAS: f32 = 3.0;
/// The largest shift the pass draws, px; the copied region is padded by it.
pub(crate) const MAX_SHIFT_PX: f32 = 48.0;
/// The shader's clock wraps at this many seconds so f32 keeps its resolution.
const TIME_WRAP_S: f32 = 1024.0;

/// A fire, or anything that heats the air above it: the plume rises from the entity's position
/// (its `GlobalTransform` translation; rotation and scale are ignored, a plume rises whatever the
/// entity's orientation) and bends the light behind it. Plain data a host can put on its own
/// flame entities.
///
/// Typical heat release rates: a candle 0.08 kW on a 1 cm radius, a torch 1–5 kW on 5 cm, a
/// campfire 20–100 kW on 0.3 m, a bonfire 0.5 MW on 0.75 m, a burning car 2–5 MW on 1.5 m. A
/// source with no power or no radius is ignored.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[require(Transform)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeatSource {
    /// Heat release rate, kW.
    pub power_kw: f32,
    /// Radius of the burning base, m.
    pub radius_m: f32,
}

impl HeatSource {
    /// Heskestad's mean flame height above the base, m: `D (3.7 Q*^{2/5} − 1.02)`, zero for a
    /// fire too weak for its base. Size a drawn flame with it so it agrees with its haze. Valid
    /// for turbulent fires (a torch and up); a candle's laminar flame is shorter.
    pub fn flame_height_m(&self) -> f32 {
        flame_height(self.power_kw, 2.0 * self.radius_m)
    }

    /// How often the fire sheds a vortex ring and its flame and plume pulse, Hz: `1.5/√D`
    /// (2 Hz for a campfire, 5 Hz for a torch). Flicker a drawn flame at it.
    pub fn puffing_hz(&self) -> f32 {
        puffing_frequency(2.0 * self.radius_m)
    }

    /// The mean temperature excess on the plume's axis `height_m` above the base in air at
    /// `air_c`, °C (McCaffrey's three regimes: about 850 K in the flame, falling as `z^{−5/3}`
    /// in the plume).
    pub fn excess_temperature_c(&self, height_m: f32, air_c: f32) -> f32 {
        if self.power_kw <= 0.0 {
            return 0.0;
        }
        let z_star = height_m.max(0.0) / self.power_kw.powf(0.4);
        model::excess_ratio(z_star) * (air_c + KELVIN).max(1.0)
    }

    /// Height above the base at which the plume is 10 K warmer than air at `air_c`, m: where the
    /// haze ends (1.6 `Q^{2/5}` at 15 °C).
    pub fn plume_top_m(&self, air_c: f32) -> f32 {
        height_at_excess(self.power_kw, (air_c + KELVIN).max(1.0), PLUME_TOP_EXCESS_K)
    }

    fn is_drawn(&self) -> bool {
        self.power_kw > 0.0 && self.radius_m > 0.0 && self.power_kw.is_finite()
    }
}

/// Settings of the heat haze ([`HeatHazePlugin`]). Insert one before the plugin to override.
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeatHazeSettings {
    /// The deflection drawn over the physical one; 0 switches the pass off. Default 10: a real
    /// fire bends light by 20–60″, plain to an eye that resolves 5″ of misalignment but a tenth
    /// of a 1080p pixel, so the physical value is drawn ten times larger
    /// (`docs/research/heat-haze.md`, Deviation 1).
    pub gain: f32,
}

impl Default for HeatHazeSettings {
    fn default() -> Self {
        Self { gain: 10.0 }
    }
}

/// Draws the shimmer of the air above every [`HeatSource`].
pub struct HeatHazePlugin;

impl Plugin for HeatHazePlugin {
    fn build(&self, app: &mut App) {
        ensure_gpu_core(app);
        needs_depth_prepass(app);
        embedded_asset!(app, "haze.wesl");
        app.init_resource::<HeatHazeSettings>()
            .init_resource::<HeatHazeGpu>()
            .register_type::<HeatSource>()
            .register_type::<HeatHazeSettings>()
            .add_observer(mark_camera)
            .add_systems(
                Update,
                update_heat_haze.in_set(ElementsSystems::Simulation),
            );
        gpu::plugin(app);
    }
}

/// Marks a camera the heat haze draws into; added to every [`ElementsCamera`].
#[derive(Component, Clone, Copy, Default, ExtractComponent)]
#[extract_app(RenderApp)]
pub(crate) struct HeatHazeView;

fn mark_camera(add: On<Add<ElementsCamera>>, mut commands: Commands) {
    commands.entity(add.entity).insert(HeatHazeView);
}

/// One plume for `haze.wesl`, in the metre frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub(crate) struct HazeSourceGpu {
    /// xyz: the fire's base, m. w: the plume's top along its axis, m.
    base: Vec4,
    /// xyz: the plume's unit axis (tilted by the wind). w: the base radius, m.
    axis: Vec4,
    /// x: `Q^{2/5}`, kW^{2/5}. y: `Q^{1/5}`. z: puffing frequency, Hz. w: height over which the
    /// pulse fades, m.
    fire: Vec4,
    /// x: the turbulence's crossfade period, s. y: the source's seed. zw: unused.
    look: Vec4,
}

/// Uniform of `haze.wesl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub(crate) struct HazeParams {
    /// x: shader clock, s. y: world units per metre. z: source count. w: largest shift, px.
    globals: Vec4,
    /// x: the air's refractivity times the gain. yzw: unused.
    optics: Vec4,
    sources: [HazeSourceGpu; MAX_HEAT_SOURCES],
}

/// What the render world draws this frame: the uniform and each plume's world-space box.
#[derive(Resource, Clone, Default)]
pub(crate) struct HeatHazeGpu {
    pub params: HazeParams,
    /// Sources in `params`.
    pub count: u32,
    /// Each drawn plume's axis-aligned box, world units, for the views' screen rectangles.
    pub bounds: Vec<(Vec3, Vec3)>,
}

/// One drawn plume, reduced on the CPU.
struct Plume {
    gpu: HazeSourceGpu,
    /// World-space box, world units.
    bounds: (Vec3, Vec3),
}

/// Reduce a source at `base` (metres) to its plume: the axis leant by a wind of `wind_10m`
/// (m/s, at 10 m, blowing toward the unit XZ `toward`), its top, its regime constants, its pulse,
/// its crossfade period and its world box.
fn plume(
    source: &HeatSource,
    base: Vec3,
    air_k: f32,
    wind_10m: f32,
    toward: Vec2,
    seed: f32,
    units: f32,
) -> Plume {
    let q = source.power_kw;
    let q_two_fifths = q.powf(0.4);
    let q_fifth = q.powf(0.2);
    let top = height_at_excess(q, air_k, PLUME_TOP_EXCESS_K);
    let reference = height_at_excess(q, air_k, TILT_REFERENCE_EXCESS_K);
    // A parcel that has risen to the reference height has drifted with the wind meanwhile.
    let wind = log_wind_speed(wind_10m.max(0.0), (0.5 * reference).max(1.0));
    let drift = wind * rise_time(reference / q_two_fifths, q_fifth);
    let tilt = (drift / reference.max(1e-3)).atan().min(MAX_TILT);
    let toward = toward.normalize_or_zero();
    let axis = (Vec3::Y * tilt.cos() + Vec3::new(toward.x, 0.0, toward.y) * tilt.sin())
        .normalize_or(Vec3::Y);
    let flame = source.flame_height_m();
    // Eddies turn over in about 1.5 σ/V; a layer of the crossfade lives two turnovers, measured
    // two flame heights up, where the haze is plainest.
    let probe = 2.0 * flame.max(source.radius_m);
    let period = (3.0 * plume_sigma(source.radius_m, probe)
        / rise_speed(probe / q_two_fifths, q_fifth).max(0.1))
    .clamp(0.08, 1.0);
    let reach = PLUME_REACH_SIGMAS * plume_sigma(source.radius_m, top);
    let tip = base + axis * top;
    let bounds = (
        (base.min(tip) - Vec3::splat(reach)) * units,
        (base.max(tip) + Vec3::splat(reach)) * units,
    );
    Plume {
        gpu: HazeSourceGpu {
            base: base.extend(top),
            axis: axis.extend(source.radius_m),
            fire: Vec4::new(
                q_two_fifths,
                q_fifth,
                source.puffing_hz(),
                3.0 * flame.max(source.radius_m),
            ),
            look: Vec4::new(period, seed, 0.0, 0.0),
        },
        bounds,
    }
}

/// A source's seed, from its entity, so its pattern does not jump when the nearest set changes.
fn seed_of(entity: Entity) -> f32 {
    let bits = entity.to_bits();
    let mixed = bits.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40;
    (mixed % 997) as f32
}

#[allow(clippy::too_many_arguments)]
fn update_heat_haze(
    time: Res<Time<Virtual>>,
    view: Res<ElementsView>,
    units: Res<ElementsUnits>,
    settings: Res<HeatHazeSettings>,
    weather: Option<Res<Weather>>,
    wind: Option<Res<WindState>>,
    sources: Query<(Entity, &HeatSource, &GlobalTransform)>,
    mut gpu: ResMut<HeatHazeGpu>,
    mut clock: Local<f32>,
) {
    *clock = (*clock + time.delta_secs()) % TIME_WRAP_S;
    gpu.count = 0;
    gpu.bounds.clear();
    if settings.gain <= 0.0 || view.entity.is_none() {
        return;
    }
    let u = units.units_per_metre;
    let mut near: Vec<(f32, Entity, HeatSource, Vec3)> = sources
        .iter()
        .filter(|(_, source, _)| source.is_drawn())
        .map(|(entity, source, transform)| {
            let base = units.point_to_metres(transform.translation());
            (base.distance_squared(view.eye), entity, *source, base)
        })
        .collect();
    if near.is_empty() {
        return;
    }
    if near.len() > MAX_HEAT_SOURCES {
        near.select_nth_unstable_by(MAX_HEAT_SOURCES - 1, |a, b| a.0.total_cmp(&b.0));
        near.truncate(MAX_HEAT_SOURCES);
    }
    let air_k = (weather.as_ref().map_or(15.0, |w| w.temperature_c) + KELVIN).max(200.0);
    let (wind_10m, toward) = wind
        .as_ref()
        .map_or((0.0, Vec2::X), |w| (w.speed_10m, w.prevailing));
    let mut params = HazeParams {
        globals: Vec4::new(*clock, u, near.len() as f32, MAX_SHIFT_PX),
        optics: Vec4::new(refractivity(air_k) * settings.gain, 0.0, 0.0, 0.0),
        ..default()
    };
    for (slot, (_, entity, source, base)) in near.iter().enumerate() {
        let p = plume(source, *base, air_k, wind_10m, toward, seed_of(*entity), u);
        params.sources[slot] = p.gpu;
        gpu.bounds.push(p.bounds);
    }
    gpu.count = near.len() as u32;
    gpu.params = params;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn campfire() -> HeatSource {
        HeatSource {
            power_kw: 50.0,
            radius_m: 0.3,
        }
    }

    /// Still air: the plume stands upright, its top where it is 10 K warm, its box around it.
    #[test]
    fn in_still_air_the_plume_stands_upright_over_its_fire() {
        let base = Vec3::new(4.0, 1.0, -2.0);
        let p = plume(&campfire(), base, 288.15, 0.0, Vec2::X, 0.0, 1.0);
        assert!(p.gpu.axis.truncate().abs_diff_eq(Vec3::Y, 1e-6));
        let top = campfire().plume_top_m(15.0);
        assert!((p.gpu.base.w - top).abs() < 1e-4);
        assert!(
            (campfire().excess_temperature_c(top, 15.0) - PLUME_TOP_EXCESS_K).abs() < 0.05,
            "{}",
            campfire().excess_temperature_c(top, 15.0)
        );
        let (lo, hi) = p.bounds;
        assert!(lo.y < base.y && hi.y > base.y + top);
        assert!(lo.x < base.x && hi.x > base.x);
        // A larger world unit scales the box, not the metre-frame plume.
        let feet = plume(&campfire(), base, 288.15, 0.0, Vec2::X, 0.0, 3.28);
        assert!((feet.bounds.1.y - hi.y * 3.28).abs() < 1e-3);
        assert_eq!(feet.gpu, p.gpu);
    }

    /// Validation target 9: a wind leans the plume downwind, more in a stronger wind, never past
    /// 75°.
    #[test]
    fn the_wind_leans_the_plume_downwind() {
        let toward = Vec2::new(0.0, 1.0);
        let lean = |wind: f32| {
            let p = plume(&campfire(), Vec3::ZERO, 288.15, wind, toward, 0.0, 1.0);
            p.gpu.axis.truncate()
        };
        let light = lean(2.0);
        let strong = lean(6.0);
        assert!(light.z > 0.0 && light.x.abs() < 1e-6, "{light}");
        assert!(strong.z > light.z, "{strong} vs {light}");
        assert!(strong.y.acos() > 10f32.to_radians(), "{strong}");
        let gale = lean(200.0);
        assert!(gale.y.acos() <= MAX_TILT + 1e-4, "{gale}");
    }

    /// Validation target 1, the host's queries: a campfire's flame, pulse and plume.
    #[test]
    fn a_source_answers_its_flame_pulse_and_plume() {
        let fire = campfire();
        assert!((0.5..=0.6).contains(&fire.flame_height_m()));
        assert!((fire.puffing_hz() - 1.94).abs() < 0.03, "{}", fire.puffing_hz());
        assert!(fire.excess_temperature_c(0.1, 15.0) > 800.0);
        assert!(fire.excess_temperature_c(3.0, 15.0) < 60.0);
        // Hotter air, the same excess ratio: the excess in kelvin scales with the air's.
        let warm = fire.excess_temperature_c(3.0, 35.0) / fire.excess_temperature_c(3.0, 15.0);
        assert!((warm - 308.15 / 288.15).abs() < 1e-4);
        let none = HeatSource {
            power_kw: 0.0,
            radius_m: 0.3,
        };
        assert!(!none.is_drawn());
        assert_eq!(none.excess_temperature_c(1.0, 15.0), 0.0);
    }

    /// The nearest sixteen sources to the camera are drawn, and nothing without a camera or at a
    /// gain of zero.
    #[test]
    fn the_nearest_sources_are_drawn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(crate::plugin::ElementsCorePlugin)
            .init_resource::<HeatHazeSettings>()
            .init_resource::<HeatHazeGpu>()
            .add_systems(Update, update_heat_haze.in_set(ElementsSystems::Simulation));
        let camera = app
            .world_mut()
            .spawn((
                Camera::default(),
                ElementsCamera::default(),
                GlobalTransform::from_xyz(0.0, 2.0, 0.0),
            ))
            .id();
        for k in 0..40 {
            app.world_mut().spawn((
                campfire(),
                GlobalTransform::from_xyz(5.0 + k as f32 * 3.0, 0.0, 0.0),
            ));
        }
        app.update();
        app.update();
        let gpu = app.world().resource::<HeatHazeGpu>();
        assert_eq!(gpu.count, MAX_HEAT_SOURCES as u32);
        assert_eq!(gpu.bounds.len(), MAX_HEAT_SOURCES);
        let farthest = (0..MAX_HEAT_SOURCES)
            .map(|k| gpu.params.sources[k].base.x)
            .fold(0.0f32, f32::max);
        assert!((farthest - (5.0 + 15.0 * 3.0)).abs() < 1e-3, "{farthest}");

        app.world_mut().resource_mut::<HeatHazeSettings>().gain = 0.0;
        app.update();
        assert_eq!(app.world().resource::<HeatHazeGpu>().count, 0);
        app.world_mut().resource_mut::<HeatHazeSettings>().gain = 10.0;
        app.world_mut().despawn(camera);
        app.update();
        app.update();
        assert_eq!(app.world().resource::<HeatHazeGpu>().count, 0);
    }
}
