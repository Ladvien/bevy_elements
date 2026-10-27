//! Render-world side of the horizon colour: one small compute kernel that reads the sky's own
//! radiance along a band just above the horizon. The readback turns it into `DistanceFog`'s
//! colour. See `docs/research/horizon-colour.md`.

use bevy::asset::load_embedded_asset;
use bevy::core_pipeline::core_3d::main_opaque_pass_3d;
use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::pbr::resources::AtmosphereTextures;
use bevy::prelude::*;
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{
    sampler, storage_buffer_sized, texture_2d, uniform_buffer,
};
use bevy::render::render_resource::{
    AddressMode, BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
    ComputePassDescriptor, FilterMode, MipmapFilterMode, PipelineCache, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderStages, ShaderType, TextureSampleType, UniformBuffer,
};
use bevy::render::renderer::{
    RenderContext, RenderDevice, RenderQueue, ViewQuery,
};
use bevy::render::storage::{GpuShaderBuffer, ShaderBuffer};
use bevy::render::{RenderApp, RenderStartup};

use crate::gpu::kernel::{Kernel, queue_kernel, ready};

/// Azimuths the kernel averages, one per lane of its workgroup.
pub(crate) const HORIZON_AZIMUTHS: u32 = 32;

/// Elevation of the band above the true horizon, rad: 0.5°. Above the sky-view LUT's own row
/// spacing at the horizon (its horizon-detail mapping concentrates rows there) and above a
/// distant haze, so the band is sky and never the ground.
pub(crate) const HORIZON_BAND_RAD: f32 = 0.5 * std::f32::consts::PI / 180.0;

/// Uniform of `horizon.wesl`.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub(crate) struct HorizonParams {
    /// x: planet `inner_radius`, m. y, z: the sky-view LUT's texels. w: unused.
    pub planet: Vec4,
    /// xy: the sky-view LUT's texels. z: the band's elevation, rad. w: the camera's radius from
    /// the planet's centre, m.
    pub view: Vec4,
}

/// The horizon kernel's output buffer and this frame's parameters, extracted every frame.
#[derive(Resource, Clone)]
pub(crate) struct HorizonGpu {
    /// `HORIZON_AZIMUTHS` `vec4`s: each azimuth's sky radiance, cd/m², unexposed.
    pub samples: Handle<ShaderBuffer>,
    /// The camera's radius from the planet's centre, m: the band keeps the same elevation above
    /// the true horizon from wherever the camera stands.
    pub camera_radius_m: f32,
    /// The planet's `inner_radius`, m.
    pub inner_radius_m: f32,
    pub active: bool,
}

impl ExtractResource<RenderApp> for HorizonGpu {
    type Source = HorizonGpu;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

#[derive(Resource)]
struct HorizonPipeline(Kernel);

#[derive(Resource, Default)]
struct HorizonUniform(UniformBuffer<HorizonParams>);

/// The LUT's sampler: linear, clamped, as Bevy samples it (`atmosphere_lut_sampler`).
#[derive(Resource)]
struct HorizonSampler(Sampler);

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(ExtractResourcePlugin::<HorizonGpu>::default());
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<HorizonUniform>()
        .add_systems(RenderStartup, init_sampler)
        .add_systems(RenderStartup, init_pipeline)
        .add_systems(
            Core3d,
            // The atmosphere LUTs are filled before `MainPass`, and the sky is drawn from them
            // there: the band is measured at the same point in the frame.
            measure
                .in_set(Core3dSystems::MainPass)
                .before(main_opaque_pass_3d)
                .run_if(resource_exists::<HorizonGpu>),
        )
        ;
}

fn init_sampler(mut commands: Commands, device: Res<RenderDevice>) {
    commands.insert_resource(HorizonSampler(device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Nearest,
        ..default()
    })));
}

fn init_pipeline(mut commands: Commands, assets: Res<AssetServer>, cache: Res<PipelineCache>) {
    let layout = BindGroupLayoutDescriptor::new(
        "elements_fog_horizon",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<HorizonParams>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                storage_buffer_sized(false, None),
            ),
        ),
    );
    let shader = load_embedded_asset!(assets.as_ref(), "horizon.wesl");
    commands.insert_resource(HorizonPipeline(queue_kernel(
        &cache,
        shader,
        "fog_horizon",
        layout,
        Vec::new(),
    )));
}

/// Measure the horizon band for the current view and dispatch the kernel.
fn measure(
    view: ViewQuery<&AtmosphereTextures>,
    horizon: Res<HorizonGpu>,
    pipeline: Option<Res<HorizonPipeline>>,
    mut uniform: ResMut<HorizonUniform>,
    sampler: Option<Res<HorizonSampler>>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut ctx: RenderContext,
) {
    if !horizon.active {
        return;
    }
    let (Some(pipeline), Some(sampler)) = (pipeline, sampler) else {
        return;
    };
    let Some(buffer) = buffers.get(&horizon.samples) else {
        return;
    };
    let Some(compute) = ready(&cache, &pipeline.0) else {
        return;
    };
    let atmosphere = view.into_inner();
    let size = atmosphere.sky_view_lut.texture.size();
    uniform.0.set(HorizonParams {
        planet: Vec4::new(
            horizon.inner_radius_m,
            size.width as f32,
            size.height as f32,
            0.0,
        ),
        view: Vec4::new(
            size.width as f32,
            size.height as f32,
            HORIZON_BAND_RAD,
            horizon.camera_radius_m,
        ),
    });
    uniform.0.write_buffer(&device, &queue);
    let group: BindGroup = device.create_bind_group(
        "elements_fog_horizon",
        &cache.get_bind_group_layout(&pipeline.0.layout),
        &BindGroupEntries::sequential((
            &uniform.0,
            &atmosphere.sky_view_lut.default_view,
            &sampler.0,
            buffer.buffer.as_entire_buffer_binding(),
        )),
    );
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let mut pass = ctx
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor {
            label: Some("elements_fog_horizon"),
            ..default()
        });
    let span = diagnostics.pass_span(&mut pass, "elements_fog_horizon");
    pass.set_pipeline(compute);
    pass.set_bind_group(0, &group, &[]);
    pass.dispatch_workgroups(1, 1, 1);
    span.end(&mut pass);
}
