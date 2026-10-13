//! Shared setup for the GPU tests: a windowless app that still renders and computes.

#![allow(dead_code)]

use bevy::app::{PluginGroupBuilder, PluginsState};
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::render_resource::TextureFormat;
use bevy::tasks::tick_global_task_pools_on_main_thread;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;

/// `DefaultPlugins` without a window or event loop, compiling pipelines on first use so frames
/// measure the simulation rather than the shader compiler.
pub fn headless_plugins() -> PluginGroupBuilder {
    DefaultPlugins
        .set(WindowPlugin {
            primary_window: None,
            exit_condition: ExitCondition::DontExit,
            ..default()
        })
        .set(RenderPlugin {
            synchronous_pipeline_compilation: true,
            ..default()
        })
        .disable::<WinitPlugin>()
}

/// Wait for the renderer (created asynchronously) and finish plugin setup, as `App::run` does.
pub fn finish(app: &mut App) {
    while app.plugins_state() == PluginsState::Adding {
        tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();
}

/// A camera needs a target to have a view: Bevy 0.20's atmosphere environment probe unwraps the
/// view uniforms, so a target-less camera panics the render world. 256 × 256 is enough.
pub fn image_target(images: &mut Assets<Image>) -> RenderTarget {
    RenderTarget::Image(
        images
            .add(Image::new_target_texture(
                256,
                256,
                TextureFormat::Rgba8UnormSrgb,
                None,
            ))
            .into(),
    )
}
