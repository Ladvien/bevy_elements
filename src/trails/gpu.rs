//! Render-world side of the trails: pipelines, the bind group, and one compute pass a frame
//! (`elements_trails`: stamp, merge, block occupancy, relax) after the terrain's pack kernels, so
//! the maps it reads are this frame's.
//!
//! The render world remembers the window's origin of the last frame it ran, which is what the
//! field holds: the main world cannot know it under pipelined rendering.

use std::sync::atomic::Ordering;

use bevy::asset::load_embedded_asset;
use bevy::core_pipeline::schedule::camera_driver;
use bevy::prelude::*;
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{
    storage_buffer_read_only_sized, storage_buffer_sized, texture_storage_2d, uniform_buffer,
};
use bevy::render::render_resource::{
    BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
    ComputePassDescriptor, PipelineCache, ShaderStages, StorageBuffer, StorageTextureAccess,
    TextureFormat, UniformBuffer,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue};
use bevy::render::storage::{GpuShaderBuffer, ShaderBuffer};
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};

use super::model::ImprintGpu;
use super::{TrailGpu, TrailParams};
use crate::cover::CoverGpu;
use crate::gpu::kernel::{Kernel, groups, queue_kernel, ready};
use crate::terrain::{TerrainDispatch, TerrainGpu};

impl ExtractResource<RenderApp> for TrailGpu {
    type Source = TrailGpu;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

#[derive(Resource)]
struct TrailPipelines {
    stamp: Kernel,
    merge: Kernel,
    blocks: Kernel,
    relax: Kernel,
}

#[derive(Resource, Default)]
struct TrailUniforms {
    params: UniformBuffer<TrailParams>,
    imprints: StorageBuffer<Vec<ImprintGpu>>,
}

#[derive(Resource)]
struct TrailBindGroup(BindGroup);

/// What the field holds: the state buffer it was last run on and that run's window origin.
#[derive(Resource, Default)]
struct TrailMemory {
    buffer: Option<AssetId<ShaderBuffer>>,
    origin: Option<IVec2>,
    /// This frame's uniform, as prepared (with `prev_origin` and the clear flag filled in).
    params: Option<TrailParams>,
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(ExtractResourcePlugin::<TrailGpu>::default());
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<TrailUniforms>()
        .init_resource::<TrailMemory>()
        .add_systems(RenderStartup, init_pipelines)
        .add_systems(Render, prepare.in_set(RenderSystems::PrepareBindGroups))
        .add_systems(
            RenderGraph,
            dispatch.after(TerrainDispatch).before(camera_driver),
        );
}

fn layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "elements_trails",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<TrailParams>(false),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_read_only_sized(false, None),
                storage_buffer_read_only_sized(false, None),
                storage_buffer_read_only_sized(false, None),
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                storage_buffer_sized(false, None),
            ),
        ),
    )
}

fn init_pipelines(mut commands: Commands, assets: Res<AssetServer>, cache: Res<PipelineCache>) {
    let shader = load_embedded_asset!(assets.as_ref(), "trails.wesl");
    let kernel = |entry| queue_kernel(&cache, shader.clone(), entry, layout(), Vec::new());
    commands.insert_resource(TrailPipelines {
        stamp: kernel("trail_stamp"),
        merge: kernel("trail_merge"),
        blocks: kernel("trail_blocks"),
        relax: kernel("trail_relax"),
    });
}

#[allow(clippy::too_many_arguments)]
fn prepare(
    mut commands: Commands,
    gpu: Option<Res<TrailGpu>>,
    terrain: Option<Res<TerrainGpu>>,
    cover: Option<Res<CoverGpu>>,
    pipelines: Option<Res<TrailPipelines>>,
    mut uniforms: ResMut<TrailUniforms>,
    mut memory: ResMut<TrailMemory>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
    images: Res<RenderAssets<GpuImage>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    commands.remove_resource::<TrailBindGroup>();
    memory.params = None;
    let (Some(gpu), Some(terrain), Some(cover), Some(pipelines)) = (gpu, terrain, cover, pipelines)
    else {
        return;
    };
    let get = |h: &Handle<ShaderBuffer>| buffers.get(h).map(|b| &b.buffer);
    let (Some(a), Some(b), Some(stamp), Some(cells), Some(cover_cells), Some(field), Some(blocks)) = (
        get(&gpu.state_a),
        get(&gpu.state_b),
        get(&gpu.stamp),
        get(&terrain.state.cells),
        get(&cover.cover_cells),
        images.get(&gpu.field),
        get(&gpu.blocks),
    ) else {
        gpu.ready.store(false, Ordering::Relaxed);
        return;
    };
    let compiled = ready(&cache, &pipelines.stamp).is_some()
        && ready(&cache, &pipelines.merge).is_some()
        && ready(&cache, &pipelines.blocks).is_some()
        && ready(&cache, &pipelines.relax).is_some();
    gpu.ready.store(compiled, Ordering::Relaxed);
    if !compiled || !gpu.run {
        return;
    }
    // A new state buffer holds nothing: clear the whole window on its first run.
    if memory.buffer != Some(gpu.state_a.id()) {
        memory.buffer = Some(gpu.state_a.id());
        memory.origin = None;
    }
    let mut params = gpu.params;
    params.prev_origin = memory.origin.unwrap_or(params.origin);
    params.counts.z = u32::from(memory.origin.is_none());
    memory.params = Some(params);

    uniforms.params.set(params);
    uniforms.params.write_buffer(&device, &queue);
    uniforms.imprints.set(gpu.imprints.clone());
    uniforms.imprints.write_buffer(&device, &queue);
    let Some(imprints) = uniforms.imprints.binding() else {
        return;
    };
    let group = device.create_bind_group(
        "elements_trails",
        &cache.get_bind_group_layout(&pipelines.stamp.layout),
        &BindGroupEntries::sequential((
            &uniforms.params,
            a.as_entire_buffer_binding(),
            b.as_entire_buffer_binding(),
            stamp.as_entire_buffer_binding(),
            imprints,
            cells.as_entire_buffer_binding(),
            cover_cells.as_entire_buffer_binding(),
            &field.texture_view,
            blocks.as_entire_buffer_binding(),
        )),
    );
    commands.insert_resource(TrailBindGroup(group));
}

fn dispatch(
    mut ctx: RenderContext,
    cache: Res<PipelineCache>,
    gpu: Option<Res<TrailGpu>>,
    pipelines: Option<Res<TrailPipelines>>,
    group: Option<Res<TrailBindGroup>>,
    mut memory: ResMut<TrailMemory>,
) {
    let (Some(gpu), Some(pipelines), Some(group), Some(params)) =
        (gpu, pipelines, group, memory.params)
    else {
        return;
    };
    let (Some(stamp), Some(merge), Some(blocks), Some(relax)) = (
        ready(&cache, &pipelines.stamp),
        ready(&cache, &pipelines.merge),
        ready(&cache, &pipelines.blocks),
        ready(&cache, &pipelines.relax),
    ) else {
        return;
    };
    let size = params.counts.x;
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let mut pass = ctx
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor {
            label: Some("elements_trails"),
            ..default()
        });
    let span = diagnostics.pass_span(&mut pass, "elements_trails");
    pass.set_bind_group(0, &group.0, &[]);
    if params.counts.y > 0 && params.counts.w > 0 {
        let tiles = groups(params.counts.w, 16);
        pass.set_pipeline(stamp);
        pass.dispatch_workgroups(tiles, tiles, params.counts.y);
    }
    pass.set_pipeline(merge);
    pass.dispatch_workgroups(groups(size, 8), groups(size, 8), 1);
    pass.set_pipeline(blocks);
    pass.dispatch_workgroups(groups(size / 8, 8), groups(size / 8, 8), 1);
    pass.set_pipeline(relax);
    pass.dispatch_workgroups(groups(size, 8), groups(size, 8), 1);
    span.end(&mut pass);
    drop(pass);
    memory.origin = Some(params.origin);
    gpu.executed.fetch_add(1, Ordering::Relaxed);
}
