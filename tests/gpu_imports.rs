//! The public WESL imports compile in a consumer's shader: `bevy_elements::noise`,
//! `bevy_elements::grid` and `bevy_elements::wind`, exactly as their doc comments show.
//!
//! Needs a GPU: `cargo test --test gpu_imports -- --ignored`.

use std::borrow::Cow;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::RenderPlugin;
use bevy::render::render_resource::binding_types::{
    sampler, storage_buffer_sized, texture_3d, uniform_buffer, uniform_buffer_sized,
};
use bevy::render::render_resource::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedComputePipelineId,
    CachedPipelineState, ComputePipelineDescriptor, PipelineCache, SamplerBindingType,
    ShaderStages, TextureSampleType,
};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use bevy_elements::*;

const CONSUMER: &str = r#"
import bevy_elements::noise::{perlin3, perlin3_d, fbm3, worley3, curl3};
import bevy_elements::grid::{GridParams, grid_taps};
import bevy_elements::wind::{WindFieldParams, elements_wind};

@group(0) @binding(0) var wind_field: texture_3d<f32>;
@group(0) @binding(1) var wind_sampler: sampler;
@group(0) @binding(2) var<uniform> wind_params: WindFieldParams;
@group(0) @binding(3) var<uniform> grid_params: GridParams;
@group(0) @binding(4) var<storage, read_write> result: array<vec4<f32>>;

@compute @workgroup_size(1)
fn main() {
    let v = elements_wind(wind_field, wind_sampler, wind_params, vec3<f32>(1.0, 2.0, 3.0));
    let t = grid_taps(grid_params, vec2<f32>(1.0, 2.0));
    let n = perlin3(v) + fbm3(v, 2u) + worley3(v, 0) + curl3(v).x + perlin3_d(v).y;
    result[0] = vec4<f32>(v, f32(t.i00) + t.w.x + n);
}
"#;

#[derive(Resource)]
struct Check(CachedComputePipelineId);

#[test]
#[ignore = "needs a GPU"]
fn a_consumer_shader_can_import_every_public_module() {
    let mut app = App::new();
    app.add_plugins((
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
            .disable::<WinitPlugin>(),
        ElementsPlugins,
    ));
    let shader = app
        .world_mut()
        .resource_mut::<Assets<Shader>>()
        .add(Shader::from_wesl(CONSUMER, "embedded://import_check/consumer.wesl"));
    app.finish();
    app.cleanup();

    let layout = BindGroupLayoutDescriptor::new(
        "import_check",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_3d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<WindFieldParams>(false),
                uniform_buffer_sized(false, None),
                storage_buffer_sized(false, None),
            ),
        ),
    );
    let render = app.sub_app_mut(RenderApp);
    let id = render
        .world()
        .resource::<PipelineCache>()
        .queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::Borrowed("import_check")),
            layout: vec![layout],
            shader,
            entry_point: Some(Cow::Borrowed("main")),
            ..default()
        });
    render.insert_resource(Check(id));

    let mut state = String::new();
    for _ in 0..60 {
        app.update();
        let render = app.sub_app(RenderApp);
        let id = render.world().resource::<Check>().0;
        match render.world().resource::<PipelineCache>().get_compute_pipeline_state(id) {
            CachedPipelineState::Ok(_) => {
                state = "ok".into();
                break;
            }
            CachedPipelineState::Err(e) => state = format!("{e}"),
            _ => {}
        }
    }
    app.world_mut().write_message(AppExit::Success);
    assert_eq!(state, "ok", "the consumer shader did not compile: {state}");
}
