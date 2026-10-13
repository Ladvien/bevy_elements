//! The plugin group, the system sets every domain orders against, the camera marker and the
//! quality knob.

use bevy::app::PluginGroupBuilder;
use bevy::camera::Exposure;
use bevy::camera::Hdr;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, AtmosphereEnvironmentMapLight, VolumetricFog};
use bevy::pbr::{AtmosphereSettings, DistanceFog};
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::shader::load_shader_library;

use crate::celestial::{CelestialPlugin, SunState, clock_plugin};
use crate::climate::{ClimatePlugin, Weather};
use crate::fog::FogPlugin;
use crate::gpu::readback::ReadbackPlugin;
use crate::query::EnvironmentSnapshot;
use crate::wind::{WindPlugin, WindState};

/// Every domain plugin that exists in this version, in dependency order.
///
/// Each member is also public on its own, so a game can drop one:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_elements::*;
/// App::new().add_plugins((DefaultPlugins, ElementsPlugins.build().disable::<FogPlugin>()));
/// ```
pub struct ElementsPlugins;

impl PluginGroup for ElementsPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ClimatePlugin)
            .add(CelestialPlugin)
            .add(WindPlugin)
            .add(FogPlugin)
    }
}

/// The phases of one environment frame, chained in this order in `Update`.
///
/// Systems that write `WorldClock`, `WeatherDirector` or settings run
/// `.before(ElementsSystems::Clock)`; systems that read `Environment` run
/// `.after(ElementsSystems::Readback)`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementsSystems {
    /// The world clock advances and the followed camera is chosen.
    Clock,
    /// The climate schedules weather and the weather blends toward its target.
    Weather,
    /// Sun, moon and stars are placed for the current clock.
    Celestial,
    /// GPU simulation inputs (uniforms, gusts, sources) are written for this frame.
    Simulation,
    /// Readback requests are issued and completed readbacks have filled `EnvironmentSnapshot`.
    Readback,
}

/// Marks the camera the environment follows and renders for.
///
/// Adding it inserts, where absent: `AtmosphereSettings`, `AtmosphereEnvironmentMapLight`,
/// `DepthPrepass`, `Msaa::Off`, `VolumetricFog`, `DistanceFog` and `Exposure`, and spawns one
/// Earth `Atmosphere` if the world has none. With more than one `ElementsCamera` the one with the
/// lowest `Camera::order` is followed and a warning is logged once. With none, the camera-relative
/// simulations (wind, fog volume) idle while the clock, weather and gameplay queries keep running.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[require(Hdr)]
pub struct ElementsCamera {
    /// Drive `Exposure::ev100` from the sun, moon and sky illuminance. Default `true`.
    pub auto_exposure: bool,
}

impl Default for ElementsCamera {
    fn default() -> Self {
        Self {
            auto_exposure: true,
        }
    }
}

/// Resolution and step counts for every domain. Read once per frame; changing it at runtime
/// re-creates the affected GPU resources.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum ElementsQuality {
    /// Integrated GPUs and the web.
    Low,
    /// Mid-range discrete GPUs.
    Medium,
    /// The default.
    #[default]
    High,
    /// Large budgets; the original research resolutions.
    Ultra,
}

impl ElementsQuality {
    /// Wind grid cells (x, y, z); each cell is 4 m on a side.
    pub fn wind_grid(self) -> UVec3 {
        match self {
            Self::Low => UVec3::new(32, 8, 32),
            Self::Medium => UVec3::new(48, 12, 48),
            Self::High => UVec3::new(64, 16, 64),
            Self::Ultra => UVec3::new(96, 24, 96),
        }
    }

    /// Jacobi pressure iterations per wind step.
    pub fn wind_pressure_iterations(self) -> u32 {
        match self {
            Self::Low => 10,
            Self::Medium => 20,
            Self::High => 30,
            Self::Ultra => 40,
        }
    }

    /// Ray-march steps for Bevy's volumetric fog.
    pub fn fog_steps(self) -> u32 {
        match self {
            Self::Low => 32,
            Self::Medium => 48,
            Self::High => 64,
            Self::Ultra => 96,
        }
    }
}

/// The camera the camera-relative simulations follow this frame.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub(crate) struct ElementsView {
    /// The followed `ElementsCamera`, if any.
    pub entity: Option<Entity>,
    /// Its world position, m.
    pub position: Vec3,
}

/// Shared setup every domain plugin needs: the system sets, the shared shader libraries, the
/// snapshot, the quality knob and the camera observer. Added once by whichever domain comes first.
pub(crate) struct ElementsCorePlugin;

impl Plugin for ElementsCorePlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "noise.wesl");
        load_shader_library!(app, "grid.wesl");
        load_shader_library!(app, "wind.wesl");
        clock_plugin(app);

        app.configure_sets(
            Update,
            (
                ElementsSystems::Clock,
                ElementsSystems::Weather,
                ElementsSystems::Celestial,
                ElementsSystems::Simulation,
                ElementsSystems::Readback,
            )
                .chain(),
        )
        .init_resource::<ElementsQuality>()
        .init_resource::<ElementsView>()
        .init_resource::<EnvironmentSnapshot>()
        .init_resource::<Weather>()
        .init_resource::<SunState>()
        .init_resource::<WindState>()
        .add_plugins(ReadbackPlugin)
        .add_observer(dress_camera)
        .add_systems(Update, follow_camera.in_set(ElementsSystems::Clock));
    }
}

/// Add [`ElementsCorePlugin`] unless another domain already did.
pub(crate) fn ensure_core(app: &mut App) {
    if !app.is_plugin_added::<ElementsCorePlugin>() {
        app.add_plugins(ElementsCorePlugin);
    }
}

fn dress_camera(
    add: On<Add<ElementsCamera>>,
    mut commands: Commands,
    quality: Res<ElementsQuality>,
    atmospheres: Query<(), With<Atmosphere>>,
    mut media: ResMut<Assets<ScatteringMedium>>,
) {
    commands.entity(add.entity).insert_if_new((
        AtmosphereSettings::default(),
        AtmosphereEnvironmentMapLight::default(),
        DepthPrepass,
        Msaa::Off,
        VolumetricFog {
            ambient_intensity: 0.0,
            step_count: quality.fog_steps(),
            ..default()
        },
        DistanceFog::default(),
        Exposure::SUNLIGHT,
    ));
    if atmospheres.is_empty() {
        let medium = media.add(ScatteringMedium::default());
        commands.spawn((Name::new("Elements atmosphere"), Atmosphere::earth(medium)));
    }
}

fn follow_camera(
    cameras: Query<(Entity, &Camera, &GlobalTransform), With<ElementsCamera>>,
    mut view: ResMut<ElementsView>,
    mut warned: Local<bool>,
) {
    let mut count = 0usize;
    let mut best: Option<(Entity, isize, Vec3)> = None;
    for (entity, camera, transform) in &cameras {
        count += 1;
        if best.is_none_or(|(_, order, _)| camera.order < order) {
            best = Some((entity, camera.order, transform.translation()));
        }
    }
    if count > 1 && !*warned {
        warn!(
            "{count} entities carry ElementsCamera; the environment follows the one with the lowest Camera::order"
        );
        *warned = true;
    }
    match best {
        Some((entity, _, position)) => {
            view.entity = Some(entity);
            view.position = position;
        }
        None => view.entity = None,
    }
}
