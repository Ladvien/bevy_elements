//! Trails: footprints, ruts and tracks pressed into snow, sand, soil and mud
//! (`docs/research/trails.md`).
//!
//! An entity with an [`Imprint`] presses into the ground under it while its translation (the
//! sole) is within 0.1 m of the undisturbed surface: as deep as the ground gives under its load
//! (Bekker–Wong sinkage, the CRREL compaction limit in snow), with a rim of the material it
//! cannot compress. The field is a camera-following window of centimetre texels, simulated on the
//! GPU (stamp, merge, slump and wind) and drawn per pixel by the terrain and ground materials. New
//! snow fills it, wind smooths it, melt takes it, wet sand and mud slump.

mod gpu;
pub(crate) mod model;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, ShaderType, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::storage::ShaderBuffer;

use crate::celestial::SunState;
use crate::celestial::{CelestialFrame, WorldClock};
use crate::climate::Weather;
use crate::clouds::CloudShadows;
use crate::cover::{CoverState, SnowSettings};
use crate::dust::DustSettings;
use crate::gpu::grid::zeroed_buffer;
use crate::plugin::{ElementsQuality, ElementsSystems, ElementsView, ensure_gpu_core};
use crate::terrain::{TerrainGpu, TerrainParamsGpu, TerrainState};
use model::{ImprintGpu, MAX_IMPRINTS, MAX_SWEEP_M, window_origin};

/// Presses into the ground under it: a footprint, a hoof, a wheel, a body.
///
/// The contact patch is an ellipse of `half_extents` (m) along the entity's local X and Z, centred
/// on its translation, which is the sole: it presses while the sole is no more than 0.1 m above
/// the undisturbed ground (terrain plus snow) under it, so lift it to stop. It sinks as deep as
/// the ground gives under `load_n` newtons spread over the ellipse: about 3 cm into dry sand, a
/// few millimetres into dry soil, up to 30 cm into mud and fresh snow for a 77 kg walker on a
/// 10 × 26 cm sole. Between frames it carves the whole path it swept, so a fast wheel leaves an
/// unbroken rut. The 64 nearest to the camera press each frame.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[require(Transform)]
pub struct Imprint {
    /// Half extents of the contact ellipse along the entity's local X and Z, m.
    pub half_extents: Vec2,
    /// The load it presses with, N (a standing 77.5 kg person on one foot: 760 N).
    pub load_n: f32,
}

impl Imprint {
    /// An ellipse of `half_extents` (m) loaded with `load_n` newtons.
    pub fn new(half_extents: Vec2, load_n: f32) -> Self {
        Self {
            half_extents,
            load_n,
        }
    }
}

/// Whether trails are pressed and drawn. Insert it before the plugin to override the default.
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
pub struct TrailSettings {
    /// Press and draw trails. Default `true`.
    pub enabled: bool,
}

impl Default for TrailSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// The published trail field, for a custom ground shader: an `rgba16float` texture of
/// `(h, occupancy, ∂h/∂x, ∂h/∂z)` per texel, `h` the surface's offset from the undisturbed ground
/// (m, negative in a trench), `occupancy` the largest `|h|` over the texel's 8 × 8 block and the
/// eight blocks around it (0 where there is nothing to draw), the gradient the field's Sobel
/// gradient; toroidal by world texel. Sample it at `world_xz / (texel_m · texels)` with a
/// repeating sampler, within `texels · texel_m / 2` of the followed camera's texel
/// (`bevy_elements::trails::surface` does it).
#[derive(Resource, Clone, Debug)]
pub struct TrailState {
    /// The field texture.
    pub field: Handle<Image>,
    /// Texel edge, m.
    pub texel_m: f32,
    /// Texels a side.
    pub texels: u32,
    /// The share of the light on a horizontal surface that comes from the sun or moon's beam
    /// (the self-shadow darkens only that share).
    pub(crate) direct_share: f32,
}

/// Uniform of the trail kernels. Mirrors `TrailParams` of `trails.wesl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub(crate) struct TrailParams {
    terrain: TerrainParamsGpu,
    origin: IVec2,
    prev_origin: IVec2,
    /// x: texels a side. y: imprints uploaded. z: 1 to clear the window. w: stamp tile edge.
    counts: UVec4,
    /// x: texel edge, m. y: game time of this step, s. z, w: wind diffusivity in snow and sand,
    /// m²/s.
    sim: Vec4,
}

/// This frame's work; extracted to the render world every frame.
#[derive(Resource, Clone)]
pub(crate) struct TrailGpu {
    pub state_a: Handle<ShaderBuffer>,
    pub state_b: Handle<ShaderBuffer>,
    pub stamp: Handle<ShaderBuffer>,
    /// The largest |h| per 8 × 8 block of slots (the materials' occupancy).
    pub blocks: Handle<ShaderBuffer>,
    pub field: Handle<Image>,
    pub params: TrailParams,
    /// Never empty: `params.counts.y` says how many are real.
    pub imprints: Vec<ImprintGpu>,
    /// Run the kernels this frame.
    pub run: bool,
    /// Frames the kernels ran.
    pub executed: Arc<AtomicU64>,
    /// The render world has every pipeline and buffer it needs.
    pub ready: Arc<AtomicBool>,
}

/// Main-world bookkeeping.
#[derive(Resource, Default)]
struct TrailRuntime {
    /// The window the GPU resources were built for: (texels, texel edge).
    built: Option<(u32, f32)>,
    /// The terrain's water buffer they were built against (a new terrain clears them).
    terrain: Option<AssetId<ShaderBuffer>>,
    /// Where each imprint was last frame, world XZ.
    last: HashMap<Entity, Vec2>,
}

/// Trails pressed into the ground by [`Imprint`]s, drawn by the terrain and ground materials.
///
/// Needs a [`Terrain`](crate::Terrain) and [`GroundCoverPlugin`](crate::GroundCoverPlugin) (the
/// field reads their maps); idles without them. Inserts, if absent, [`TrailSettings`]. Add it
/// with [`ElementsPlugins`](crate::ElementsPlugins).
pub struct TrailsPlugin;

impl Plugin for TrailsPlugin {
    fn build(&self, app: &mut App) {
        ensure_gpu_core(app);
        embedded_asset!(app, "trails.wesl");
        app.init_resource::<TrailSettings>()
            .init_resource::<SnowSettings>()
            .init_resource::<TrailRuntime>()
            .add_systems(
                Update,
                (ensure_trails, run_trails)
                    .chain()
                    .in_set(ElementsSystems::Simulation),
            );
        gpu::plugin(app);
    }
}

/// A `size²` `rgba16float` texture written by a compute kernel and sampled by the materials with
/// a linear, repeating sampler (the field is toroidal).
fn field_image(images: &mut Assets<Image>, size: u32) -> Handle<Image> {
    let mut image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING
        | TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::COPY_SRC;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    images.add(image)
}

/// Create the field when a terrain with ground cover exists and trails are on; rebuild it for a
/// new terrain or a new window size; drop it otherwise.
#[allow(clippy::too_many_arguments)]
fn ensure_trails(
    mut commands: Commands,
    settings: Res<TrailSettings>,
    quality: Res<ElementsQuality>,
    terrain: Option<Res<TerrainState>>,
    cover: Option<Res<CoverState>>,
    gpu: Option<Res<TrailGpu>>,
    mut runtime: ResMut<TrailRuntime>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(terrain), Some(_cover), true) = (terrain, cover, settings.enabled) else {
        if gpu.is_some() {
            commands.remove_resource::<TrailGpu>();
            commands.remove_resource::<TrailState>();
            *runtime = TrailRuntime::default();
        }
        return;
    };
    let window = quality.trail_window();
    if gpu.is_some() && runtime.built == Some(window) && runtime.terrain == Some(terrain.water.id())
    {
        return;
    }
    let (size, texel_m) = window;
    let texels = u64::from(size) * u64::from(size);
    let field = field_image(&mut images, size);
    *runtime = TrailRuntime {
        built: Some(window),
        terrain: Some(terrain.water.id()),
        ..TrailRuntime::default()
    };
    commands.insert_resource(TrailState {
        field: field.clone(),
        texel_m,
        texels: size,
        direct_share: 0.0,
    });
    commands.insert_resource(TrailGpu {
        state_a: zeroed_buffer(&mut buffers, "elements_trails_state_a", texels * 8),
        state_b: zeroed_buffer(&mut buffers, "elements_trails_state_b", texels * 8),
        stamp: zeroed_buffer(&mut buffers, "elements_trails_stamp", texels * 8),
        blocks: zeroed_buffer(&mut buffers, "elements_trails_blocks", texels / 64 * 4),
        field,
        params: TrailParams::default(),
        imprints: vec![ImprintGpu::default()],
        run: false,
        executed: Arc::new(AtomicU64::new(0)),
        ready: Arc::new(AtomicBool::new(false)),
    });
}

/// The yaw of an entity's local X axis in world XZ.
fn yaw_of(transform: &GlobalTransform) -> f32 {
    let x = transform.right();
    x.z.atan2(x.x)
}

/// The share of the light on a horizontal surface carried by the brightest body's beam: the
/// clear sky's direct share (the clouds' `sky_share`: 85 % at high sun, none in twilight) times
/// the beam the clouds let through over the camera; the moon's light is all beam
/// ([INFERENCE]: the note's self-shadow weighting).
fn direct_share(sun: &SunState, frame: &CelestialFrame, shadows: Option<&CloudShadows>) -> f32 {
    let beam = shadows
        .map_or(1.0, |s| s.camera_transmittance)
        .clamp(0.0, 1.0);
    if sun.altitude_deg > 0.0 {
        let t = ((sun.altitude_deg + 2.0) / 12.0).clamp(0.0, 1.0);
        let diffuse = 1.0 + (0.15 - 1.0) * t * t * (3.0 - 2.0 * t);
        (1.0 - diffuse) * beam
    } else if sun.moon_altitude_deg > 0.0 && frame.moon_lux > 0.0 {
        beam
    } else {
        0.0
    }
}

/// Gather this frame's imprints, place the window and fill the uniform.
#[allow(clippy::too_many_arguments)]
fn run_trails(
    view: Res<ElementsView>,
    weather: Res<Weather>,
    clock: Res<WorldClock>,
    time: Res<Time<Virtual>>,
    sun: Res<SunState>,
    frame: Res<CelestialFrame>,
    snow: Res<SnowSettings>,
    dust: Option<Res<DustSettings>>,
    shadows: Option<Res<CloudShadows>>,
    terrain: Option<Res<TerrainState>>,
    terrain_gpu: Option<Res<TerrainGpu>>,
    gpu: Option<ResMut<TrailGpu>>,
    state: Option<ResMut<TrailState>>,
    mut runtime: ResMut<TrailRuntime>,
    imprints: Query<(Entity, &Imprint, Ref<GlobalTransform>)>,
) {
    let (Some(terrain), Some(terrain_gpu), Some(mut gpu), Some(mut state)) =
        (terrain, terrain_gpu, gpu, state)
    else {
        return;
    };
    let Some((size, texel_m)) = runtime.built else {
        return;
    };
    state.direct_share = direct_share(&sun, &frame, shadows.as_deref());
    let camera = Vec2::new(view.position.x, view.position.z);
    let origin = window_origin(camera, texel_m, size);

    // The imprints: the swept segment from where each was last frame. An imprint's first frame
    // carves nothing (its `GlobalTransform` is not propagated yet), and a jump of more than
    // MAX_SWEEP_M in a frame (a teleport, a respawn) starts a new path.
    let mut seen = HashMap::with_capacity(runtime.last.len());
    let mut keyed: Vec<(f32, Entity, ImprintGpu)> = Vec::new();
    for (entity, imprint, transform) in &imprints {
        if transform.is_added() {
            continue;
        }
        let p = transform.translation();
        let xz = Vec2::new(p.x, p.z);
        let from = runtime
            .last
            .get(&entity)
            .copied()
            .filter(|from| from.distance(xz) <= MAX_SWEEP_M)
            .unwrap_or(xz);
        seen.insert(entity, xz);
        if let Some(upload) = ImprintGpu::new(
            imprint.half_extents,
            yaw_of(&transform),
            imprint.load_n,
            from,
            xz,
            p.y,
            texel_m,
        ) {
            keyed.push((xz.distance_squared(camera), entity, upload));
        }
    }
    runtime.last = seen;
    if keyed.len() > MAX_IMPRINTS {
        keyed.select_nth_unstable_by(MAX_IMPRINTS, |a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        keyed.truncate(MAX_IMPRINTS);
    }
    let tile = keyed
        .iter()
        .map(|(_, _, i)| i.tile.w as u32)
        .max()
        .unwrap_or(0)
        .div_ceil(16)
        * 16;

    // Wind smoothing: the saltation flux as a diffusivity, m²/s.
    let u = weather.wind_speed_ms;
    let snow_d = if snow.drift {
        snow.drift_coefficient * (u - snow.drift_threshold_ms).max(0.0).powi(3)
            / snow.new_density.max(1.0)
    } else {
        0.0
    };
    let sand_d = dust.as_deref().filter(|d| d.enabled).map_or(0.0, |d| {
        d.flux_coefficient * u * u * (u - d.threshold_ms).max(0.0)
    });
    let dt = if clock.paused {
        0.0
    } else {
        time.delta_secs() * clock.time_scale.max(0.0)
    };

    gpu.run = gpu.ready.load(Ordering::Relaxed)
        && terrain.is_live()
        && terrain_gpu.generated.load(Ordering::Relaxed);
    // The render world fills `prev_origin` and the clear flag: only it knows which frames ran.
    gpu.params = TrailParams {
        terrain: TerrainParamsGpu::new(&terrain.grid, 1, true),
        origin,
        prev_origin: origin,
        counts: UVec4::new(size, keyed.len() as u32, 0, tile),
        sim: Vec4::new(texel_m, dt, snow_d, sand_d),
    };
    gpu.imprints = if keyed.is_empty() {
        vec![ImprintGpu::default()]
    } else {
        keyed.into_iter().map(|(_, _, i)| i).collect()
    };
}
