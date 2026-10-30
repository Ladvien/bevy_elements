//! Render-world side of the heat haze: per view, the screen rectangle the plumes cover, a copy of
//! that region of the HDR main texture, and the refraction pass that reads the copy and writes the
//! main texture inside the rectangle. Nothing runs for a view no plume reaches.

use bevy::asset::load_embedded_asset;
use bevy::core_pipeline::prepass::ViewPrepassTextures;
use bevy::core_pipeline::{Core3d, Core3dSystems, FullscreenShader};
use bevy::math::URect;
use bevy::prelude::*;
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::binding_types::{
    sampler, texture_2d, texture_depth_2d, uniform_buffer,
};
use bevy::render::render_resource::{
    AddressMode, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
    CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState,
    LoadOp, Operations, Origin3d, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StoreOp, TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, UniformBuffer,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::texture::{CachedTexture, TextureCache};
use bevy::render::view::{
    ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};

use super::{HazeParams, HeatHazeGpu, HeatHazeView, MAX_SHIFT_PX};

impl ExtractResource<RenderApp> for HeatHazeGpu {
    type Source = HeatHazeGpu;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

#[derive(Resource)]
struct HazePipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedRenderPipelineId,
    sampler: Sampler,
}

#[derive(Resource, Default)]
struct HazeUniform(UniformBuffer<HazeParams>);

/// A view the plumes reach this frame: the scratch copy of its main texture, the rectangle the
/// pass draws and the larger one it copies, physical pixels.
#[derive(Component)]
struct HazeTarget {
    scratch: CachedTexture,
    draw: URect,
    copy: URect,
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        ExtractResourcePlugin::<HeatHazeGpu>::default(),
        ExtractComponentPlugin::<HeatHazeView>::default(),
    ));
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<HazeUniform>()
        .add_systems(RenderStartup, init_pipeline)
        .add_systems(
            Render,
            (prepare_uniform, prepare_views).in_set(RenderSystems::PrepareResources),
        )
        .add_systems(
            Core3d,
            draw.in_set(Core3dSystems::EarlyPostProcess)
                .after(crate::precipitation::PrecipitationDraw),
        );
}

fn layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "elements_heat_haze",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<HazeParams>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_depth_2d(),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    )
}

fn init_pipeline(
    mut commands: Commands,
    assets: Res<AssetServer>,
    cache: Res<PipelineCache>,
    fullscreen: Res<FullscreenShader>,
    device: Res<RenderDevice>,
) {
    let layout = layout();
    let shader = load_embedded_asset!(assets.as_ref(), "haze.wesl");
    let pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("elements_heat_haze".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen.to_vertex_state(),
        fragment: Some(FragmentState {
            shader,
            entry_point: Some("heat_haze".into()),
            targets: vec![Some(ColorTargetState {
                // `ElementsCamera` requires `Hdr`.
                format: TextureFormat::Rgba16Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("elements_heat_haze"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    commands.insert_resource(HazePipeline {
        layout,
        pipeline,
        sampler,
    });
}

fn prepare_uniform(
    gpu: Option<Res<HeatHazeGpu>>,
    mut uniform: ResMut<HazeUniform>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let Some(gpu) = gpu.filter(|g| g.count > 0) else {
        return;
    };
    uniform.0.set(gpu.params);
    uniform.0.write_buffer(&device, &queue);
}

fn prepare_views(
    mut commands: Commands,
    gpu: Option<Res<HeatHazeGpu>>,
    views: Query<(Entity, &ExtractedView, &ViewTarget), With<HeatHazeView>>,
    mut textures: ResMut<TextureCache>,
    device: Res<RenderDevice>,
) {
    let bounds = gpu
        .as_ref()
        .filter(|g| g.count > 0)
        .map_or(&[][..], |g| &g.bounds[..]);
    for (entity, view, target) in &views {
        let clip_from_world = view
            .clip_from_world
            .unwrap_or_else(|| view.clip_from_view * view.world_from_view.affine().inverse());
        let Some(draw) = screen_rect(clip_from_world, view.viewport, bounds) else {
            commands.entity(entity).remove::<HazeTarget>();
            continue;
        };
        let copy = padded(draw, view.viewport, MAX_SHIFT_PX as u32 + 2);
        let scratch = textures.get(
            &device,
            TextureDescriptor {
                label: Some("elements_heat_haze_scratch"),
                size: target.main_texture().size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: target.main_texture_format(),
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
        );
        commands.entity(entity).insert(HazeTarget {
            scratch,
            draw,
            copy,
        });
    }
}

/// The physical-pixel rectangle of the viewport `(x, y, width, height)` that the world-space boxes
/// cover through `clip_from_world`, or `None` if none is on screen. A box wholly behind the eye is
/// skipped; one with corners on both sides of the eye's plane covers the whole viewport (its
/// projection is unbounded).
fn screen_rect(clip_from_world: Mat4, viewport: UVec4, boxes: &[(Vec3, Vec3)]) -> Option<URect> {
    let (lo_px, size) = (viewport.xy().as_vec2(), viewport.zw().as_vec2());
    let mut lo = Vec2::splat(f32::INFINITY);
    let mut hi = Vec2::splat(f32::NEG_INFINITY);
    for &(min, max) in boxes {
        let mut box_lo = Vec2::splat(f32::INFINITY);
        let mut box_hi = Vec2::splat(f32::NEG_INFINITY);
        let mut behind = 0;
        for corner in 0..8 {
            let p = Vec3::new(
                if corner & 1 == 0 { min.x } else { max.x },
                if corner & 2 == 0 { min.y } else { max.y },
                if corner & 4 == 0 { min.z } else { max.z },
            );
            let clip = clip_from_world * p.extend(1.0);
            if clip.w <= 1e-6 {
                behind += 1;
                continue;
            }
            let ndc = clip.xy() / clip.w;
            let px = lo_px + Vec2::new(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * size;
            box_lo = box_lo.min(px);
            box_hi = box_hi.max(px);
        }
        match behind {
            0 => {}
            8 => continue,
            _ => {
                return Some(URect::from_corners(
                    viewport.xy(),
                    viewport.xy() + viewport.zw(),
                ));
            }
        }
        lo = lo.min(box_lo);
        hi = hi.max(box_hi);
    }
    let lo = lo.max(lo_px).floor();
    let hi = hi.min(lo_px + size).ceil();
    if !(hi.x > lo.x && hi.y > lo.y) {
        return None;
    }
    Some(URect::from_corners(lo.as_uvec2(), hi.as_uvec2()))
}

/// `rect` grown by `pad` on every side, within the viewport.
fn padded(rect: URect, viewport: UVec4, pad: u32) -> URect {
    let lo = rect.min.saturating_sub(UVec2::splat(pad)).max(viewport.xy());
    let hi = (rect.max + UVec2::splat(pad)).min(viewport.xy() + viewport.zw());
    URect::from_corners(lo, hi)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw(
    view: ViewQuery<(
        &ViewTarget,
        &ViewUniformOffset,
        &ViewPrepassTextures,
        &ExtractedView,
        &Msaa,
        &HazeTarget,
    )>,
    cache: Res<PipelineCache>,
    pipeline: Option<Res<HazePipeline>>,
    uniform: Res<HazeUniform>,
    view_uniforms: Res<ViewUniforms>,
    mut ctx: RenderContext,
    mut warned: Local<bool>,
) {
    let Some(pipeline) = pipeline else {
        return;
    };
    let (target, view_offset, prepass, extracted, msaa, haze) = view.into_inner();
    let main = target.main_texture();
    if *msaa != Msaa::Off
        || target.main_texture_format() != TextureFormat::Rgba16Float
        || !main.usage().contains(TextureUsages::COPY_SRC)
    {
        if !*warned {
            warn!(
                "heat haze is not drawn on a view with {msaa:?}, a {:?} target or a main texture without COPY_SRC; it needs Msaa::Off, an HDR camera and the default CameraMainTextureUsages",
                target.main_texture_format()
            );
            *warned = true;
        }
        return;
    }
    let (Some(render_pipeline), Some(view_binding), Some(params), Some(depth)) = (
        cache.get_render_pipeline(pipeline.pipeline),
        view_uniforms.uniforms.binding(),
        uniform.0.binding(),
        prepass.depth_only_view(),
    ) else {
        return;
    };
    let group = ctx.render_device().create_bind_group(
        "elements_heat_haze",
        &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((
            view_binding,
            params,
            &haze.scratch.default_view,
            depth,
            &pipeline.sampler,
        )),
    );

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let span = diagnostics.time_span(ctx.command_encoder(), "elements_heat_haze");
    let copy_origin = Origin3d {
        x: haze.copy.min.x,
        y: haze.copy.min.y,
        z: 0,
    };
    ctx.command_encoder().copy_texture_to_texture(
        TexelCopyTextureInfo {
            texture: main,
            mip_level: 0,
            origin: copy_origin,
            aspect: TextureAspect::All,
        },
        TexelCopyTextureInfo {
            texture: &haze.scratch.texture,
            mip_level: 0,
            origin: copy_origin,
            aspect: TextureAspect::All,
        },
        Extent3d {
            width: haze.copy.width(),
            height: haze.copy.height(),
            depth_or_array_layers: 1,
        },
    );
    {
        let mut pass = ctx
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("elements_heat_haze"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target.main_texture_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        let vp = extracted.viewport.as_vec4();
        pass.set_viewport(vp.x, vp.y, vp.z, vp.w, 0.0, 1.0);
        pass.set_scissor_rect(
            haze.draw.min.x,
            haze.draw.min.y,
            haze.draw.width(),
            haze.draw.height(),
        );
        pass.set_pipeline(render_pipeline);
        pass.set_bind_group(0, &group, &[view_offset.offset]);
        pass.draw(0..3, 0..1);
    }
    span.end(ctx.command_encoder());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looking_down_minus_z() -> Mat4 {
        let proj = Mat4::perspective_infinite_reverse_rh(60f32.to_radians(), 1.0, 0.1);
        let world_from_view = Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0));
        proj * world_from_view.inverse()
    }

    /// A plume ahead covers a rectangle around its projection; one behind the eye covers nothing;
    /// one straddling the eye's plane covers the whole viewport; the union of two spans both.
    #[test]
    fn the_pass_covers_only_what_the_plumes_reach() {
        let clip = looking_down_minus_z();
        let viewport = UVec4::new(0, 0, 1000, 1000);
        let ahead = (Vec3::new(-1.0, 0.0, -21.0), Vec3::new(1.0, 8.0, -19.0));
        let rect = screen_rect(clip, viewport, &[ahead]).unwrap_or_default();
        assert!(rect.min.x > 400 && rect.max.x < 600, "{rect:?}");
        // The box rises from 2 m below the eye to 6 m above it: the bottom edge is below centre.
        assert!(rect.max.y > 500 && rect.min.y < 500, "{rect:?}");
        assert!(rect.width() < 200, "{rect:?}");

        let behind = (Vec3::new(-1.0, 0.0, 19.0), Vec3::new(1.0, 8.0, 21.0));
        assert_eq!(screen_rect(clip, viewport, &[behind]), None);
        let aside = (Vec3::new(200.0, 0.0, -21.0), Vec3::new(202.0, 8.0, -19.0));
        assert_eq!(screen_rect(clip, viewport, &[aside]), None);
        assert_eq!(screen_rect(clip, viewport, &[]), None);

        let around = (Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 8.0, 1.0));
        assert_eq!(
            screen_rect(clip, viewport, &[around]),
            Some(URect::new(0, 0, 1000, 1000))
        );

        let left = (Vec3::new(-6.0, 0.0, -21.0), Vec3::new(-4.0, 8.0, -19.0));
        let both = screen_rect(clip, viewport, &[ahead, left]).unwrap_or_default();
        let only_left = screen_rect(clip, viewport, &[left]).unwrap_or_default();
        assert_eq!(both.min.x, only_left.min.x);
        assert_eq!(both.max.x, rect.max.x);

        // An offset viewport places the rectangle inside itself.
        let side = UVec4::new(1000, 200, 1000, 1000);
        let moved = screen_rect(clip, side, &[ahead]).unwrap_or_default();
        assert_eq!(moved.min, rect.min + UVec2::new(1000, 200));
        let grown = padded(moved, side, 50);
        assert_eq!(grown.min, moved.min - UVec2::splat(50));
        let clamped = padded(URect::new(1000, 200, 1100, 300), side, 50);
        assert_eq!(clamped.min, UVec2::new(1000, 200));
    }
}
