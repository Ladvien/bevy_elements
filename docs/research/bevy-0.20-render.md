# Bevy 0.20.0 render API facts for bevy_elements

Verified by reading the vendored `bevy-0.20.0` family of crates under `~/.cargo/registry/src/index.crates.io-*/`. Nothing here was compiled; items tagged `[INFERENCE]` are unconfirmed. Parts A and B were written independently and are concatenated below.

## Deviations from the plan's 0.19.1 names

| Plan assumption | Bevy 0.20 fact | Consequence for this crate |
|---|---|---|
| WGSL with naga_oil `#import` | Shaders are WESL; `#import`, `#ifdef`, `#define_import_path` are gone | Shared `noise`, `grid`, `wind` modules are `.wesl` files imported as `bevy_elements::<module>::<file>`; kept alive with `load_shader_library!` |
| `On<Add, T>` | `On<Add<T>>` | Observer signatures |
| `DirectionalLight::shadows_enabled` | `shadow_maps_enabled` | Sun and moon lights |
| `AmbientLight` resource | `GlobalAmbientLight` resource; `AmbientLight` is a per-camera component | Night ambient |
| `Atmosphere` on camera | `Atmosphere` on its own planet entity with `Handle<ScatteringMedium>`; `AtmosphereSettings` on the camera; no `scene_units_to_m` | `ElementsCamera` observer spawns the planet entity |
| Moon disk contingency | `SunDisk::OFF` component disables a light's disk | Moon light uses `SunDisk::OFF`; contingency not needed |
| `FogVolume::density_texture` may be gone | Still exists; 3D filterable float image, `.r` read | Froxel fallback not needed |
| `ShaderBuffer::new(&[u8], ..)` | `ShaderBuffer::new` takes `Vec<T>`; default usage already `STORAGE \| COPY_SRC \| COPY_DST` | Grid buffers |
| Render graph | Render graph is a schedule; compute systems take `RenderContext` and run in `RenderGraph` `.before(camera_driver)`; per-camera passes in `Core3d` | Kernel dispatch and post-process passes |
| Auto exposure | Bevy `AutoExposure` range ±8 EV, too narrow | CPU-driven `Exposure::ev100` as planned |

# Bevy 0.20.0 render APIs, part A: compute, buffers, materials, post-process, diagnostics

All paths are under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (written `$R/`). Crate dirs: `bevy_render-0.20.0` (`render`), `bevy_material-0.20.0` (`material`), `bevy_shader-0.20.0`, `bevy_pbr-0.20.0`, `bevy_core_pipeline-0.20.0` (`cp`), `bevy_extract-0.20.0`, `bevy_asset-0.20.0`, `bevy_light-0.20.0`, `bevy_mesh-0.20.0`, `bevy_camera-0.20.0`. Public re-exports come through `bevy::render`, `bevy::core_pipeline`, `bevy::pbr`, `bevy::shader`, `bevy::material`, `bevy::asset`.

Read first, because it changes plan conventions:

- **Shaders are WESL, not naga_oil WGSL** (`bevy-0.20.0/_release-content/migration-guides/shaders_use_wesl.md`). `#import`, `#define_import_path`, `#ifdef`, `#{DEF}` are gone. Plain `.wgsl` files with no preprocessor directives still load (`bevy_shader-0.20.0/src/shader.rs:296`, `Shader::from_wgsl` at `:135`), but cannot import anything. Anything that shares code (`bevy_elements::noise`, `grid`, `wind`) MUST be `.wesl`. Bevy's own examples use `.wesl` (`examples/shader/*.rs` reference `shaders/*.wesl`; the `assets/` dir is not in the crates.io package, so there is no shader source to copy from the registry; use `$R/bevy_pbr-0.20.0/src/render/*.wesl` as syntax reference).
- **"Render graph" is now a schedule.** There is no `RenderGraph` node trait. Compute and render work are plain systems taking `RenderContext` that run in the `RenderGraph` schedule (root) or `Core3d` schedule (per camera).
- 0.19.1 -> 0.20 renames hit by this note: `ViewDepthTexture` -> `ViewDepthStencilTexture`; `ViewPrepassTextures::depth` is now `Option<DepthStencilAttachment>` (`bevy-0.20.0/_release-content/migration-guides/depth_attachment_changes.md`); `ShaderBuffer` fields/ctor changed (`shader_buffer_changes.md`); extraction moved to `bevy_extract` and `ExtractResource`/`ExtractComponent` need `#[extract_app(RenderApp)]` (`extract-extract.md`); `Tonemapping`/`DebandDither` moved to `bevy_render::view` (`tonemapping_moved_to_bevy_render.md`); `Msaa` is a `Component` (`render/src/view/mod.rs:248`).

## Compute and render scheduling

### Pipeline cache

- `PipelineCache::queue_compute_pipeline(&self, descriptor: ComputePipelineDescriptor) -> CachedComputePipelineId` — `render/src/render_resource/pipeline_cache.rs:461`. Takes `&self` (interior mutex), so `Res<PipelineCache>` is enough, usable in `RenderStartup`. Never deduplicates; every call makes a new pipeline.
- `PipelineCache::get_compute_pipeline(&self, id: CachedComputePipelineId) -> Option<&ComputePipeline>` — `pipeline_cache.rs:409`. `None` while queued/creating/errored.
- `PipelineCache::get_compute_pipeline_state(&self, id) -> &CachedPipelineState` — `pipeline_cache.rs:305`. An id not yet in the table reports `&CachedPipelineState::Queued`.
- `PipelineCache::get_bind_group_layout(&self, &BindGroupLayoutDescriptor) -> BindGroupLayout` — `pipeline_cache.rs:506` (cached by descriptor; call it every frame freely).
- `ComputePipelineDescriptor` — `material/src/descriptor.rs:104`, `#[derive(Clone, Debug, PartialEq, Default)]`. All fields:
  ```rust
  pub label: Option<Cow<'static, str>>,
  pub layout: Vec<BindGroupLayoutDescriptor>,
  pub immediate_size: u32,                       // 0 if no push constants/immediates
  pub shader: Handle<Shader>,
  pub shader_defs: Vec<ShaderDefVal>,
  pub entry_point: Option<Cow<'static, str>>,     // None = default entry point
  pub zero_initialize_workgroup_memory: bool,     // Default is false; set true unless you know better
  pub constants: Vec<(Cow<'static, str>, f64)>,   // `override` constants
  ```
  Changed vs older code: `push_constant_ranges` is now `immediate_size: u32`.
- `BindGroupLayoutDescriptor { pub label: Cow<'static, str>, pub entries: Vec<BindGroupLayoutEntry> }` with `BindGroupLayoutDescriptor::new(label: impl Into<Cow<'static, str>>, entries: &[BindGroupLayoutEntry])` — `material/src/descriptor.rs:13`. A descriptor, not a GPU layout: the GPU `BindGroupLayout` comes from `pipeline_cache.get_bind_group_layout(&desc)`. Build entries with `BindGroupLayoutEntries::sequential(ShaderStages::COMPUTE, (..))` and the helpers in `bevy::render::render_resource::binding_types` (`material/src/bind_group_layout_entries.rs`): `storage_buffer::<T>(has_dynamic_offset)` `:376`, `storage_buffer_read_only::<T>(has_dynamic_offset)` `:392`, `storage_buffer_sized` `:380`, `storage_buffer_read_only_sized` `:398`, `uniform_buffer::<T>(has_dynamic_offset)` `:410`, `texture_depth_2d()` `:473`, `texture_3d(sample_type)` `:521`, `texture_storage_2d(format, access)` `:543`, `texture_storage_3d(format, access)` `:567`.
- `CachedComputePipelineId(CachedPipelineId)` — `material/src/descriptor.rs:211`, `Copy + Eq + Hash`, has `INVALID`.
- `CachedPipelineState` — `pipeline_cache.rs:48`:
  ```rust
  pub enum CachedPipelineState {
      Queued,
      Creating(Task<Result<Pipeline, ShaderCacheError>>),
      Ok(Pipeline),
      Err(ShaderCacheError),
  }
  ```
- `ShaderCacheError` — `bevy_shader-0.20.0/src/shader_cache.rs:538` (re-exported `bevy::shader::ShaderCacheError`):
  ```rust
  ShaderNotLoaded(AssetId<Shader>), ProcessShaderError(String), ShaderImportNotYetAvailable, CreateShaderModule(String)
  ```
- Error behaviour (`pipeline_cache.rs:911-933`): `ShaderNotLoaded` and `ShaderImportNotYetAvailable` are transient; the cache resets them to `Queued` and retries every frame (a `debug!` only). `ProcessShaderError` and `CreateShaderModule` are terminal: the cache logs `error!("failed to process shader error:\n{..}")` / `error!("failed to create shader module: ..")` ONCE and leaves the state `Err`, never retried. Setting env `VERBOSE_SHADER_ERROR=1` adds pipeline context. So a crate-side helper only needs to treat `Err(ShaderNotLoaded | ShaderImportNotYetAvailable)` as "wait" and any other `Err` as permanently idle; Bevy already logged it.
- Pipeline creation is async on `AsyncComputeTaskPool` on native multithreaded; on wasm32, macOS, and non-`multi_threaded` builds it is synchronous (`pipeline_cache.rs:1067-1090`).

### Schedules and sets

- `RenderStartup` — `render/src/lib.rs:246`, a `ScheduleLabel` (`#[derive(ScheduleLabel, .., Default)] pub struct RenderStartup;`). Runs once in the render sub-app after the device exists. This is where pipelines are queued (`app.sub_app_mut(RenderApp).add_systems(RenderStartup, init_pipeline)`). `GpuResourceAppExt::init_gpu_resource::<R: Resource + FromWorld>()` (`lib.rs:255`, fn `init_gpu_resource` `:249`) re-runs `R::from_world` on `RenderStartup` (device re-creation).
- `Render` schedule with `RenderSystems` — `render/src/lib.rs:186` (`#[derive(SystemSet)] pub enum RenderSystems`). Variants in declaration order: `ExtractCommands, PrepareAssets, PrepareMeshes, CreateViews, Specialize, PrepareViews, Queue, QueueMeshes, QueueSweep, PhaseSort, Prepare, PrepareResources, PrepareResourcesBatchPhases, PrepareResourcesWritePhaseBuffers, PrepareResourcesCollectPhaseBuffers, PrepareResourcesFlush, PrepareBindGroups, Render, Cleanup, PostCleanup`. Chaining: `(ExtractCommands, PrepareAssets, PrepareMeshes, Prepare).chain_weak()` at `lib.rs:351`. Use `PrepareResources` for buffer uploads, `PrepareBindGroups` for bind groups (needs `RenderAssets<GpuShaderBuffer>` ready, which is filled in `PrepareAssets`).
- Root render graph: `pub struct RenderGraph;` is a `ScheduleLabel` — `render/src/renderer/mod.rs:48` (also in `bevy::render::prelude`, `render/src/lib.rs:103`). Runs once per frame inside `render_system` (`renderer/mod.rs:82`, `world.run_schedule(RenderGraph)` at `:94`). Sets `RenderGraphSystems::{Begin, Render, Submit, Finish}` (`renderer/mod.rs:68`), `chain_weak`. Camera rendering is the system `camera_driver` (`cp/src/schedule.rs:136`, `bevy::core_pipeline::schedule::camera_driver`) which runs each camera's own schedule (`Core3d`/`Core2d`), then submits. Its doc says: "Users can order any additional operations (e.g. one-off compute passes) before or after this system in the root render graph schedule." (`schedule.rs:127-135`).
  - Compute that must be finished before any camera draws: `.add_systems(RenderGraph, sim.before(camera_driver))`.
  - Per-camera compute/post work: `.add_systems(Core3d, f.in_set(Core3dSystems::..))` with `ViewQuery`.
- After `RenderGraph` the same frame, `render_system` records `submit_screenshot_commands` and `submit_readback_commands` (`renderer/mod.rs:103-104`) in a final encoder, so a `Readback` copy sees the compute output written earlier in that frame.
- `RenderContext<'w,'s>` — `render/src/renderer/render_context.rs:234`, a `SystemParam` usable in any system running under `RenderGraph`:
  - `fn command_encoder(&mut self) -> &mut CommandEncoder` `:259`
  - `fn begin_tracked_render_pass<'a>(&'a mut self, RenderPassDescriptor<'_>) -> TrackedRenderPass<'a>` `:265`
  - `fn diagnostic_recorder(&self) -> Option<Res<'w, DiagnosticsRecorder>>` `:254`
  - `fn render_device(&self) -> &RenderDevice`, `fn add_command_buffer(&mut self, CommandBuffer)`.
  Command buffers are flushed per system automatically (`FlushCommands` exists for early flush, `:290`). Systems that take `RenderContext` should not be reordered to rely on the encoder being shared; each system's encoder is submitted in order.
- `ViewQuery<'w,'s, D: QueryData, F = ()>` — `render_context.rs:317`: the per-view query inside `Core3d` systems (reads `CurrentView`); `.entity()`, `.into_inner()`.
- `Core3d` schedule — `cp/src/schedule.rs:38` (`bevy::core_pipeline::Core3d`). `Core3dSystems { Prepass, MainPass, EarlyPostProcess, PostProcess }` — `schedule.rs:50`, `chain_weak` in that order (`:62`). Systems registered in `Core3d` (`cp/src/core_3d/mod.rs:146-167`): `(early_prepass, early_deferred_prepass, late_prepass, late_deferred_prepass).chain()` in `Prepass`; `(main_opaque_pass_3d, main_transparent_pass_3d).chain()` in `MainPass`; `tonemapping` in `PostProcess`; `upscaling.after(PostProcess)`. Fns: `bevy::core_pipeline::core_3d::{main_opaque_pass_3d, main_transparent_pass_3d}` (`core_3d/mod.rs:47-48`), `bevy::core_pipeline::tonemapping::tonemapping` (`tonemapping/mod.rs:31`).
- Existing orderings to copy: Bevy's atmosphere runs `(atmosphere_luts, atmosphere_environment).chain().after(Prepass).before(MainPass)` and `render_sky.after(main_opaque_pass_3d).before(main_transparent_pass_3d)` (`bevy_pbr-0.20.0/src/atmosphere/mod.rs:191-200`); `volumetric_fog.after(MainPass).before(EarlyPostProcess)` (`volumetric_fog/mod.rs:100-103`); built-in post effects use `.in_set(PostProcess).before(tonemapping)`; bloom `.before(tonemapping)` `bevy_post_process-0.20.0/src/bloom/mod.rs:79`; DoF `.after(bloom).before(tonemapping)`; `FullscreenMaterial` default is `.in_set(Core3dSystems::PostProcess).before(tonemapping)` (`cp/src/fullscreen_material.rs:95-100`).

### Extraction

- `ExtractResourcePlugin<R, F = ()>` — type alias in `render/src/lib.rs:57` over `bevy_extract::extract_resource::ExtractResourcePlugin<R, RenderApp, F>`; add with `app.add_plugins(ExtractResourcePlugin::<R>::default())` in the MAIN app. `R: ExtractResource` (`bevy_extract-0.20.0/src/extract_resource.rs:18`: `type Source: Resource; fn extract_resource(&Source) -> Self`). Derive: `#[derive(Resource, Clone, ExtractResource)] #[extract_app(RenderApp)]` (the attribute is NEW and mandatory in 0.20; seen in `compute_shader_game_of_life.rs:114-115`).
- `ExtractComponentPlugin<C, F = ()>` — alias `render/src/lib.rs:46`; `C: ExtractComponent` derive needs `#[extract_app(RenderApp)]` (`gpu_readback.rs:~80`, `custom_post_processing.rs:269`). Also exported: `bevy::render::extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin}`.
- `Extract<'w,'s,P: ReadOnlySystemParam>` — `bevy_extract-0.20.0/src/extract_param.rs:51`, re-exported `bevy::render::Extract` (`render/src/lib.rs:109`). Used only in `ExtractSchedule` systems (`bevy::render::ExtractSchedule`, `MainWorld`, `lib.rs:110`) to read main-world state: `fn extract(mut commands: Commands, q: Extract<Query<..>>)`.
- Handles are extracted as plain `Handle<T>` clones; GPU data for assets lives in `RenderAssets<G>` (`bevy::render::render_asset::RenderAssets`).

### SNIPPET: compute structure (from `examples/shader/compute_shader_game_of_life.rs`, 289 lines, read in full; 0.20 shape, adapted to storage buffers)

Source structure: plugin (L93-112) extracts resources to the render world and registers `RenderStartup` -> pipeline init, `Render/PrepareBindGroups` -> bind groups, `Render/Prepare` -> state machine, `RenderGraph` -> dispatch system `.before(camera_driver)`. Pipeline init L176-211; state machine L221-256 (note the exact `ShaderNotLoaded` handling at L229-239); dispatch L258-289. The example's shader is `shaders/game_of_life.wesl`, workgroup 8x8, `dispatch_workgroups(SIZE.x / 8, SIZE.y / 8, 1)`.

```rust
use bevy::{
    core_pipeline::schedule::camera_driver,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{binding_types::{storage_buffer, uniform_buffer}, *},
        renderer::{RenderContext, RenderDevice, RenderGraph},
        storage::{GpuShaderBuffer, ShaderBuffer},
        Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::ShaderCacheError,
};

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<SimBuffers>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .add_systems(RenderStartup, init_sim_pipeline)
            .add_systems(Render, prepare_sim_bind_group.in_set(RenderSystems::PrepareBindGroups))
            .add_systems(RenderGraph, run_sim.before(camera_driver));
    }
}

#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
struct SimBuffers { a: Handle<ShaderBuffer>, b: Handle<ShaderBuffer> }

#[derive(Resource)]
struct SimPipeline { layout: BindGroupLayoutDescriptor, step: CachedComputePipelineId }

fn init_sim_pipeline(mut commands: Commands, assets: Res<AssetServer>, cache: Res<PipelineCache>) {
    let layout = BindGroupLayoutDescriptor::new(
        "sim",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (storage_buffer_read_only::<Vec<Vec4>>(false), storage_buffer::<Vec<Vec4>>(false)),
        ),
    );
    let shader = load_embedded_asset!(assets.as_ref(), "sim.wesl"); // see "Materials" for embedded paths
    let step = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("sim_step".into()),
        layout: vec![layout.clone()],
        shader,
        entry_point: Some("step".into()),
        zero_initialize_workgroup_memory: true,
        ..default()
    });
    commands.insert_resource(SimPipeline { layout, step });
}

#[derive(Resource)]
struct SimBindGroup(BindGroup);

fn prepare_sim_bind_group(
    mut commands: Commands, p: Option<Res<SimPipeline>>, cache: Res<PipelineCache>,
    device: Res<RenderDevice>, gpu_buffers: Res<RenderAssets<GpuShaderBuffer>>, bufs: Option<Res<SimBuffers>>,
) {
    let (Some(p), Some(bufs)) = (p, bufs) else { return };
    let (Some(a), Some(b)) = (gpu_buffers.get(&bufs.a), gpu_buffers.get(&bufs.b)) else { return };
    commands.insert_resource(SimBindGroup(device.create_bind_group(
        "sim_bg",
        &cache.get_bind_group_layout(&p.layout),
        &BindGroupEntries::sequential((a.buffer.as_entire_buffer_binding(), b.buffer.as_entire_buffer_binding())),
    )));
}

fn run_sim(mut ctx: RenderContext, cache: Res<PipelineCache>, p: Option<Res<SimPipeline>>, bg: Option<Res<SimBindGroup>>) {
    let (Some(p), Some(bg)) = (p, bg) else { return };
    // Readiness: `get_compute_pipeline` is None for Queued/Creating/Err. For error reporting:
    match cache.get_compute_pipeline_state(p.step) {
        CachedPipelineState::Ok(_) => {}
        CachedPipelineState::Err(ShaderCacheError::ShaderNotLoaded(_) | ShaderCacheError::ShaderImportNotYetAvailable) => return,
        CachedPipelineState::Err(_) => return, // already error!-logged once by PipelineCache
        _ => return,
    }
    let Some(pipeline) = cache.get_compute_pipeline(p.step) else { return };
    let mut pass = ctx.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some("sim"), ..default() });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bg.0, &[]);
    pass.dispatch_workgroups(64 / 8, 64 / 8, 1);
}
```

`BindGroupEntries::sequential` accepts `Buffer::as_entire_buffer_binding()` (seen at `examples/shader/gpu_readback.rs:176`) and `&TextureView`. `RenderDevice::create_bind_group(label: impl Into<Option<&str>>/None, &BindGroupLayout, &[BindGroupEntry]-like)` as used at `compute_shader_game_of_life.rs:148`.

## Buffers and readback

### ShaderBuffer

- `bevy::render::storage::{ShaderBuffer, GpuShaderBuffer, ShaderBufferData, StoragePlugin}` — `render/src/storage.rs`. `StoragePlugin` and `GpuReadbackPlugin::default()` are added by `RenderPlugin` (`render/src/lib.rs:415-416`), so `DefaultPlugins` already has both.
- Struct (`storage.rs:60`), `#[derive(Asset, Reflect, Debug, Clone)]`:
  ```rust
  pub struct ShaderBuffer {
      pub data: ShaderBufferData,            // Uninitialized(u64) | Initialized { data: AlignedVec, buffer_size: u64 }  (`:85`)
      pub label: Cow<'static, str>,
      pub buffer_usage: BufferUsages,        // Default = STORAGE | COPY_SRC | COPY_DST  (`:103`)
      pub asset_usage: RenderAssetUsages,    // Default = RenderAssetUsages::default() (MAIN | RENDER)  (`:104`)
      pub copy_on_resize: bool,
  }
  ```
  The default `buffer_usage` already includes `COPY_SRC` and `COPY_DST`; no `buffer_usage_descriptor` exists (that was replaced by the `label` + `buffer_usage` fields, see migration guide).
- Constructors: `ShaderBuffer::new<T: bytemuck::NoUninit>(data: Vec<T>, asset_usage: RenderAssetUsages) -> Self` `:118` (initialised with data, size = data bytes); `ShaderBuffer::with_size(size: u64, asset_usage: RenderAssetUsages) -> Self` `:129` (uninitialised: `prepare_asset` creates it with `mapped_at_creation: false`, `storage.rs:~390`; wgpu/WebGPU zero-initialise new buffers, but Bevy does not document it, so use `new(vec![0f32; n], ..)` when the sim needs a defined start); `impl<T: NoUninit> From<Vec<T>> for ShaderBuffer` `:268`; `extend_from_slice`, `extend`, `clear`, `resize`, `resize_buffer`, `buffer_size() -> u64` (`:260`), `cast_slice<T: AnyBitPattern>(&self) -> Option<&[T]>` `:190`, `cast_slice_mut` `:203`. Removed: `set_data`, `resize_in_place`, `From<T: ShaderType>`.
- Main-world creation for a GPU-only sim buffer:
  ```rust
  let mut buf = ShaderBuffer::new(vec![0.0f32; (w * h) as usize], RenderAssetUsages::RENDER_WORLD);
  buf.buffer_usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST; // already the default
  let handle: Handle<ShaderBuffer> = buffers.add(buf); // buffers: ResMut<Assets<ShaderBuffer>>
  ```
  `RenderAssetUsages::RENDER_WORLD` drops the CPU copy after upload (`GpuShaderBuffer::take_gpu_data` `storage.rs:316`; the extraction rejects a buffer whose data would be lost, test at `:605`). `BufferUsages` and `RenderAssetUsages` come from `bevy::render::render_resource::BufferUsages` and `bevy::asset::RenderAssetUsages`.
- `GpuShaderBuffer` (`storage.rs:293`): `pub buffer: Buffer`, `pub label`, `pub buffer_usage: BufferUsages`, `pub had_data: bool`. It is a `RenderAsset`, so in render systems it is `Res<RenderAssets<GpuShaderBuffer>>` (`bevy::render::render_asset::RenderAssets`), `.get(&Handle<ShaderBuffer>) -> Option<&GpuShaderBuffer>`. wgpu buffer: `gpu.buffer` (`Buffer`; `buffer.as_entire_buffer_binding()`; `buffer.size()`). Example: `examples/shader/gpu_readback.rs:167-178`.
- Reuse: if size/usage/label are unchanged the GPU buffer is reused across asset modifications (`storage.rs:643` test, `prepare_asset` `:334-411`). Resizing keeps contents only with `copy_on_resize = true` and `COPY_SRC|COPY_DST`.
- Materials bind these through `AsBindGroup` (see Materials section), which looks up `RenderAssets<GpuShaderBuffer>` itself (`render/src/render_resource/bind_group.rs:546`).

### Readback

- `bevy::render::gpu_readback::{Readback, ReadbackOnce, ReadbackComplete, GpuReadbackPlugin}` — `render/src/gpu_readback.rs`.
- `Readback` component (`:85`, derives `ExtractComponent` to `RenderApp`):
  ```rust
  pub enum Readback { Texture(Handle<Image>), Buffer { buffer: Handle<ShaderBuffer>, start_offset_and_size: Option<(u64, u64)> } }
  Readback::texture(image: Handle<Image>) -> Self             // :96
  Readback::buffer(buffer: Handle<ShaderBuffer>) -> Self       // :101
  Readback::buffer_range(buffer, start_offset: u64, size: u64) -> Self   // :110
  ```
  Buffer readback reads `Handle<ShaderBuffer>` (not a raw `Buffer`); texture readback reads `Handle<Image>`. Both exist.
- Cadence: "If this component is not removed, the readback will be attempted every frame" (doc at `:80-83`). `prepare_buffers` (`:294`) pushes one `GpuReadback` per `Readback` entity per frame, with a pooled staging buffer, and never checks for an outstanding request on the same entity (no in-flight gate). So for a throttled readback, spawn the entity on the tick and despawn it, or use `ReadbackOnce` (`:125`, a marker; `ReadbackOnce::buffer(handle) -> impl Bundle` `:134`, `::texture` `:129`, `::buffer_range` `:140`) which removes `(Readback, ReadbackOnce)` in `First` (`cleanup_readback_once` `:145`) after the first completion tick; it removes the components but does NOT despawn the entity (the doc comment says despawn, the code only removes components, `:145-149`). A request can be outstanding across frames while you despawn: completion is delivered later with `main_world.trigger(ReadbackComplete{ data, entity })` (`sync_readbacks` `:275-285`); an observer attached to a despawned entity will not fire [INFERENCE: not run]. Keep the entity alive until `ReadbackComplete`, then despawn inside the observer (matches the plan).
- Out-of-range `buffer_range` panics (`gpu_readback.rs:~329`: "Tried to read past the end of the buffer"). Mapping failure `expect("Failed to map buffer")` panics in the wgpu callback (`:407`).
- Timing: the copy is recorded after the `RenderGraph` schedule (`renderer/mod.rs:104`) and mapped in `RenderSystems::Cleanup` (`gpu_readback.rs:69`, `map_buffers` `:401`); results arrive at the next `ExtractSchedule` where `sync_readbacks` triggers the event, i.e. at least one frame later.
- `ReadbackComplete` (`:157`):
  ```rust
  #[derive(EntityEvent, Deref, DerefMut, Reflect, Debug)]
  pub struct ReadbackComplete { pub entity: Entity, #[deref] pub data: Vec<u8> }
  pub fn to_shader_type<T: ShaderType + ReadFrom + Default>(&self) -> T   // :165  (encase layout, e.g. Vec<u32>; expect()s on a bad reader)
  ```
  Raw bytes are in `event.data` (also via `Deref` to `Vec<u8>`); decode with `bytemuck::cast_slice::<u8, f32>(&event.data)` for tightly packed `array<f32>`. Texture readback rows are padded to `COPY_BYTES_PER_ROW_ALIGNMENT` (256) (`layout_data` `:436`, `get_aligned_size` `:431`); decode row by row for textures. Buffer readbacks are tightly packed.
- Observer signature (0.20 `On<..>`, not `Trigger`): `.observe(|event: On<ReadbackComplete>| { .. })` on the entity (`gpu_readback.rs` example L74, L121). Global observer variant: `app.add_observer(|ev: On<ReadbackComplete>, ..| ev.entity)`; `On<E: EntityEvent>` derefs to the event, `ev.entity` for an `EntityEvent`.

SNIPPET (verbatim shape from `examples/shader/gpu_readback.rs`, L85-126):

```rust
fn setup(mut commands: Commands, mut buffers: ResMut<Assets<ShaderBuffer>>) {
    let mut buffer = ShaderBuffer::from((0..16u32).collect::<Vec<u32>>());
    buffer.buffer_usage |= BufferUsages::COPY_SRC; // already in the default; kept in the example
    let buffer = buffers.add(buffer);

    // every frame until despawned
    commands.spawn(Readback::buffer(buffer.clone())).observe(|event: On<ReadbackComplete>| {
        let data: Vec<u32> = event.to_shader_type();
        info!("Buffer {:?}", data);
    });
    // a range
    commands.spawn(Readback::buffer_range(buffer.clone(), 4, 8)).observe(|e: On<ReadbackComplete>| { /* ... */ });
}
// one-shot (components removed after the first completion; entity stays):
commands.spawn(ReadbackOnce::buffer(handle)).observe(|e: On<ReadbackComplete>| { /* .. */ });
```

## Materials and storage buffers

### MaterialExtension

- `bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline}`, `bevy::pbr::MaterialPlugin` — `bevy_pbr-0.20.0/src/extended_material.rs`.
- `ExtendedMaterial<B: Material, E: MaterialExtension> { pub base: B, pub extension: E }` — `extended_material.rs:151`. `MaterialPlugin::<ExtendedMaterial<StandardMaterial, X>>::default()` must be added by the user/crate (example `examples/shader/extended_material.rs:20-24`).
- Trait (`extended_material.rs:31`), `pub trait MaterialExtension: Asset + AsBindGroup + Clone + Sized`; every method has a default:
  ```rust
  fn vertex_shader() -> ShaderRef                      // :34  default ShaderRef::Default (= StandardMaterial/mesh vertex shader)
  fn fragment_shader() -> ShaderRef                    // :40
  fn alpha_mode() -> Option<AlphaMode>                 // :45  None => base material's
  fn enable_prepass() -> bool                          // :52  true
  fn enable_shadows() -> bool                          // :58  true
  fn enable_oit() -> bool                              // :66  true
  fn prepass_vertex_shader() -> ShaderRef              // :72
  fn prepass_fragment_shader() -> ShaderRef            // :78
  fn deferred_vertex_shader() -> ShaderRef             // :84
  fn deferred_fragment_shader() -> ShaderRef           // :90
  // #[cfg(feature = "meshlet")]: meshlet_mesh_fragment_shader / meshlet_mesh_prepass_fragment_shader / meshlet_mesh_deferred_fragment_shader  :96-113
  fn specialize(pipeline: &MaterialExtensionPipeline, descriptor: &mut RenderPipelineDescriptor,
                layout: &MeshVertexBufferLayoutRef, key: MaterialExtensionKey<Self>) -> Result<(), SpecializedMeshPipelineError>  // :123
  ```
  Note `ShaderRef::Default` for the prepass vertex/fragment means Bevy's standard prepass shader: a terrain with a custom `vertex_shader` that displaces vertices MUST also override `prepass_vertex_shader` (and `prepass_fragment_shader` if discarding) with the same displacement, else shadows/depth prepass use undisplaced geometry. `enable_shadows`/`enable_prepass` can turn those passes off.
- `ShaderRef` — `bevy_shader-0.20.0/src/shader.rs:357`: `Default | Handle(Handle<Shader>) | Path(AssetPath<'static>)`; `From<&'static str>`/`From<String>`/`From<Handle<Shader>>` exist.
- `AlphaMode` — `material/src/alpha.rs:9` (`bevy::material::AlphaMode`, re-exported by `bevy::prelude`/`bevy::pbr`): `Opaque (default), Mask(f32), Blend, Premultiplied, AlphaToCoverage, Add, Multiply`. An extension's `alpha_mode()` overrides the base (`extended_material.rs:242`).
- The base `StandardMaterial` and extension bindings share ONE bind group: base uses slots 0..99 (bindless: the extension must start at 100). Example: `#[uniform(100)] quantize_steps: u32` (`examples/shader/extended_material.rs:99`). Struct attrs for extension: `#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]`.
- Material bind group index: `pub const MATERIAL_BIND_GROUP_INDEX: usize = 3;` — `bevy_pbr-0.20.0/src/material.rs:75` (group 3 in 0.20; groups 0 = view, 1 = view binding arrays/lightmaps, 2 = mesh; do not hard-code, use the shader constant). In WESL: `@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> ..;` (migration guide `shaders_use_wesl.md`, and `bevy_pbr-0.20.0/src/decal/forward.wesl:13`). `MATERIAL_BIND_GROUP` is set as a shader def at `material.rs:521-527` and `prepass/mod.rs:424`. The older doc strings `#{MATERIAL_BIND_GROUP}` in `material.rs:143` and `bind_group.rs:133-148` are stale naga_oil syntax.

### AsBindGroup derive (`render/src/render_resource/bind_group.rs`, docs `:120-277`)

```rust
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct GroundExtension {
    #[uniform(100)] params: GroundParams,                 // ShaderType struct/LinearRgba/f32...; same index = merged into one uniform
    #[storage(101, read_only)] height: Handle<ShaderBuffer>,   // array<f32>/array<vec4<f32>>; default visibility = vertex+fragment
    #[storage(102, read_only, visibility(vertex, fragment))] water: Handle<ShaderBuffer>,
    #[texture(103)] #[sampler(104)] normals: Option<Handle<Image>>,  // None => FallbackImage (white)
    #[texture(105, dimension = "3d", sample_type = "float", filterable = true)] #[sampler(106, sampler_type = "filtering")] density: Handle<Image>,
}
```
- `storage(N, read_only)` on `Handle<ShaderBuffer>` is the documented form (`bind_group.rs:134-135`); `buffer` flag = raw wgpu `Buffer` (`:136`). Args table `:213-221`: `visibility(...)` (`all|none|vertex|fragment|compute`, default `vertex, fragment`), `read_only`, `buffer`, `binding_array(..)` (bindless only). In WGSL: `@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<storage, read> height: array<f32>;`.
- `texture(N, dimension="2d|3d|2d_array|cube|cube_array|1d", sample_type="float|depth|s_int|u_int", filterable=bool, multisampled=bool, visibility(..))` `:171-178`; `sampler(N, sampler_type="filtering|non_filtering|comparison", visibility(..))` `:200`; `storage_texture(N, dimension, image_format, access, visibility)` `:186-192` (default visibility `compute`, default access `ReadWrite`, default format `Rgba8Unorm`). Fields without attributes are ignored.
- Binding a 3D `rgba16float` texture as a filterable sampled texture works; a write-only storage texture is not sampled in materials, so the plan's ping-pong/publish texture is bound with `#[texture(.., dimension = "3d")]`.
- Platform limit reminder: a vertex-stage read-only storage buffer consumes `max_storage_buffers_per_shader_stage` (wgpu `Limits::defaults` = 8, `downlevel_webgl2_defaults` = 0; `wgpu-types-30.0.1/src/limits.rs:434, 591`). Mesh view bindings already use storage buffers in the fragment stage (`mesh_view_bindings.wesl:28-47`), so budget carefully for wasm.

### MeshTag

`bevy::mesh::MeshTag` (also `bevy::prelude::MeshTag`) — `bevy_mesh-0.20.0/src/components.rs:205`: `#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)] pub struct MeshTag { #[deref] pub value: u32 }`. Per-instance, free for app use unless a `GpuComponentArrayBuffer` is on the same entity (then Bevy overwrites it). In shaders: `bevy_pbr::render::mesh_functions::get_tag(instance_index: u32) -> u32` (`bevy_pbr-0.20.0/src/render/mesh_functions.wesl:155`; the doc comment on `MeshTag` names the old `mesh_bindings::mesh[...].tag`, which is `mesh[instance_index].tag`). The vertex stage has `@builtin(instance_index)` in `Vertex.instance_index` (`render/forward_io.wesl:3`).

### Shader assets: embedded and imports (WESL)

- `embedded_asset!(app, "file.wesl")` — `bevy_asset-0.20.0/src/io/embedded/mod.rs:352` (`bevy::asset::embedded_asset`). Path is relative to the file the macro is invoked from; registered at `embedded://<crate_name>/<path relative to src/>`. Three-arg form `embedded_asset!(app, "src_prefix", "file")` replaces the default `src` prefix (`:331-338`). Helpers: `embedded_path!` `:228`, `load_embedded_asset!(asset_server, "file")` `:205` (returns `Handle<A>`, same path rule).
- Example in Bevy: `bevy_post_process-0.20.0/src/bloom/mod.rs:48` `embedded_asset!(app, "bloom.wesl");` and the pipeline uses `load_embedded_asset!(asset_server.as_ref(), "dof.wesl")` (`dof/mod.rs:526`).
- Material shader reference: `ShaderRef::Path(AssetPath::parse("embedded://bevy_elements/terrain/terrain.wesl"))` (plan format holds; extension must be `.wesl` for any shader that has imports/`@if`). `ShaderRef::from("embedded://bevy_elements/terrain/terrain.wesl")` also works.
- `load_internal_asset!(app, handle, "path", Shader::from_wgsl|from_wesl)` still exists — `embedded/mod.rs:388` — for a hard-coded `Handle<Shader>` (`Shader::from_wesl(source, path)` `shader.rs:176`).
- Importing a library module: there is NO `#define_import_path`. A shader loaded from `embedded://bevy_foo/bar.wesl` is importable as `bevy_foo::bar`; for `embedded://bevy_elements/noise.wesl` the import is `import bevy_elements::noise::{hash, perlin};`. Library shaders must be loaded (kept alive) before dependents compile: use `bevy::shader::load_shader_library!(app, "noise.wesl");` — `bevy_shader-0.20.0/src/lib.rs:28` (it does `embedded_asset!` + `load_embedded_asset!` + `mem::forget(handle)`; "works around a limitation of the shader loader not properly loading dependencies"). Bevy uses it for e.g. `bevy_pbr-0.20.0/src/atmosphere/mod.rs:101-102`. Dependents that fail with `ShaderImportNotYetAvailable` retry until the library arrives (`pipeline_cache.rs:911-917`).
- WESL syntax (from migration guide and Bevy sources): imports end in `;` and come first; `import package::{a::b, c};` inside the same crate (`package` = this crate); `@if(DEF)` / `@elif` / `@else` attach to declarations/members/params/statements; `Int`/`UInt` shader defs read as `constants::NAME`; `ShaderDefVal::{Bool, Int, UInt}(name, value)` (`bevy_shader-0.20.0/src/shader_cache.rs:115`). `shader_defs` are only honoured for `.wesl` (`shader.rs:288`).

### Bevy WESL module paths (all under `bevy_pbr-0.20.0/src/`)

0.19 naga_oil name -> 0.20 WESL import path (module = file path inside the crate; `bevy_pbr::render::...`):

| item | 0.20 import | file:line |
|---|---|---|
| `view` (uniform `View`), `lights`, `globals`, `fog` | `bevy_pbr::render::mesh_view_bindings::{view, lights, globals, fog, ..}` | `render/mesh_view_bindings.wesl:11,12,39,43` (group 0: view @0, lights @1, globals @11) |
| `View` struct | `bevy_render::view::View` | `bevy_render-0.20.0/src/view.wesl:40` |
| `Globals` | `bevy_render::globals::Globals` | `bevy_render-0.20.0/src/globals.wesl` |
| `Vertex`, `VertexOutput`, `FragmentOutput` (forward) | `bevy_pbr::render::forward_io::{Vertex, VertexOutput, FragmentOutput}` | `render/forward_io.wesl:30` (Vertex) |
| prepass `VertexOutput`, `FragmentOutput`, `Vertex` | `bevy_pbr::prepass::io::{Vertex, VertexOutput, FragmentOutput}` | `prepass/io.wesl:30,107,140` |
| prepass bindings (depth/normal/motion textures) | `bevy_pbr::prepass::bindings`; utils `bevy_pbr::prepass::utils` | `prepass/bindings.wesl`, `prepass/utils.wesl` |
| `pbr_input_from_standard_material`, `pbr_input_from_vertex_output` | `bevy_pbr::render::pbr_fragment::{pbr_input_from_standard_material, pbr_input_from_vertex_output}` | `render/pbr_fragment.wesl:71,34` |
| `apply_pbr_lighting`, `main_pass_post_lighting_processing`, `alpha_discard` | `bevy_pbr::render::pbr_functions::{..}` | used in `render/pbr.wesl:2-18` |
| deferred output | `bevy_pbr::deferred::functions::deferred_output` | `render/pbr.wesl:12` |
| `mesh_position_local_to_world`, `get_world_from_local`, `mesh_normal_local_to_world`, `mesh_position_local_to_clip`, `get_tag` | `bevy_pbr::render::mesh_functions::{..}` | `render/mesh_functions.wesl:15,52,59,65,155` |
| `mesh` storage array | `bevy_pbr::render::mesh_bindings::{mesh, metadata}` | imported in `mesh_functions.wesl:1-9` |
| `position_world_to_clip`, `position_ndc_to_world`, `depth_ndc_to_view_z`, `frag_coord_to_uv`, ... | `bevy_pbr::render::view_transformations::{..}` | `render/view_transformations.wesl:35-233` |
| `STANDARD_MATERIAL_FLAGS_*`, `PbrInput` | `bevy_pbr::render::pbr_types::{..}` | `render/pbr_types.wesl` |
| atmosphere types/functions | `bevy_pbr::atmosphere::{types, functions}` | `atmosphere/types.wesl`, `functions.wesl` (loaded by `load_shader_library!`, `atmosphere/mod.rs:101-102`) |

The `package::` prefix inside Bevy's own files means "this crate"; from outside use the crate name `bevy_pbr::`. Import paths for `bevy_core_pipeline` fullscreen vertex: `bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput` [INFERENCE from the `fullscreen_vertex_shader.wesl` file name; not opened].

WESL fragment-extension skeleton (shape of Bevy's own fragments, `render/pbr.wesl:1-40`):
```wesl
import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
import bevy_pbr::render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing};
import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};
// extension bindings start at 100:
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<storage, read> height: array<f32>;
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput { ... }
```
(Prepass and deferred variants must be gated by `@if(PREPASS_PIPELINE)` as in `pbr.wesl:10-20`, or provided as separate `prepass_*`/`deferred_*` shaders.)

### lux constants

`bevy::light::light_consts::lux` — `bevy_light-0.20.0/src/lib.rs:130` (inner `pub mod lux`; all `pub const _: f32`): `MOONLESS_NIGHT = 0.0001`, `FULL_MOON_NIGHT = 0.05`, `CIVIL_TWILIGHT = 3.4`, `LIVING_ROOM = 50.`, `HALLWAY = 80.`, `DARK_OVERCAST_DAY = 100.`, `OFFICE = 320.`, `CLEAR_SUNRISE = 400.`, `OVERCAST_DAY = 1000.`, `AMBIENT_DAYLIGHT = 10_000.` (default `DirectionalLight` illuminance), `FULL_DAYLIGHT = 20_000.`, `DIRECT_SUNLIGHT = 100_000.`, `RAW_SUNLIGHT = 130_000.`. Lines `lib.rs:131-157`. The plan's `lux::RAW_SUNLIGHT` exists. No `lux::FULL_MOON_NIGHT` conflict: it is 0.05 lx, while the plan's moon uses 0.32 lx peak (Krisciunas & Schaefer), so do not use that constant for the moon.

## Custom post-process / render passes

### Where systems go

- Registering (`examples/shader_advanced/custom_post_processing.rs:62-71`):
  ```rust
  render_app
      .add_systems(RenderStartup, init_post_process_pipeline)
      .add_systems(Render, prepare_bind_groups.in_set(RenderSystems::PrepareBindGroups))
      .add_systems(Core3d, post_process_system.in_set(Core3dSystems::PostProcess));
  ```
  Imports `bevy::core_pipeline::{schedule::Core3d, Core3dSystems, FullscreenShader}`, `bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery}`, `bevy::render::view::ViewTarget`.
- Ordering for this crate (all are system-ordering constraints inside `Core3d`):
  - Clouds / ground effects drawn into the scene colour AFTER opaque + atmosphere sky and BEFORE transparent: `.after(main_opaque_pass_3d).before(main_transparent_pass_3d)` (same as `render_sky`; note `render_sky` is also `.after(main_opaque_pass_3d).before(main_transparent_pass_3d)`, so add `.after(render_sky)` if the sky must be underneath; `render_sky` is declared `pub fn` at `bevy_pbr-0.20.0/src/atmosphere/node.rs:147` but its module is private (`atmosphere/mod.rs:40` `mod node;`), so it cannot be named; order via the set instead: put clouds in `.in_set(Core3dSystems::MainPass).after(main_transparent_pass_3d)` which is guaranteed after both).
  - Precipitation / lightning overlay after the whole main pass: `.after(main_transparent_pass_3d).before(Core3dSystems::EarlyPostProcess)` (or `.in_set(Core3dSystems::MainPass).after(main_transparent_pass_3d)`; the OIT resolve is `.after(main_transparent_pass_3d).in_set(MainPass)`, `cp/src/oit/mod.rs:114-117`, so also add `.after(oit_resolve)` is unnecessary unless OIT is used).
  - Pure post effects before tonemapping: `.in_set(Core3dSystems::PostProcess).before(tonemapping)` where `tonemapping` is `bevy::core_pipeline::tonemapping::tonemapping`.
  - Volumetric fog (Bevy's own) runs `.after(Core3dSystems::MainPass).before(Core3dSystems::EarlyPostProcess)` (`volumetric_fog/mod.rs:100-103`); to composite after Bevy's fog use `.after(volumetric_fog)` (the fn is in `bevy_pbr::volumetric_fog`, check visibility) or `.in_set(EarlyPostProcess)`.
  - Because the main target at that time is HDR scene-linear (`Hdr` camera => `Rgba16Float`, `ViewTarget::main_texture_format()` `view/mod.rs:~980`), passes placed before tonemapping blend in linear HDR. The example hard-codes `Rgba8UnormSrgb` and warns to change it with `Hdr` ("Make sure you change the TextureFormat of the ColorTargetState if you enable Hdr", example L285); use `view_target.main_texture_format()` when specialising.
- Camera/view gating: query components that only exist on the view (e.g. your extracted `ElementsCamera` marker); `ViewQuery` fails silently (system skips) if the view lacks a component. The post-process system is skipped on views where the query does not match.

### ViewTarget ping-pong

`ViewTarget` — `render/src/view/mod.rs:785` (`#[derive(Component, Clone)]`). Relevant API:
- `fn post_process_write(&self) -> PostProcessWrite<'_>` — `view/mod.rs:1034`; `PostProcessWrite { pub source: &TextureView, pub source_texture: &Texture, pub destination: &TextureView, pub destination_texture: &Texture }` (`:802`). It FLIPS the main texture to `destination` as a side effect (`fetch_xor` at `:1035`), so after calling it you must write `source` (modified) into `destination`. Calling it twice flips back; do it once per effect.
- `fn main_texture_view(&self) -> &TextureView` `:948`, `fn main_texture_other_view(&self)` (to precache bind groups for both, as the example does, L118-137), `fn main_texture(&self) -> &Texture`, `fn main_texture_format(&self) -> TextureFormat`, `fn get_color_attachment(&self) -> RenderPassColorAttachment<'_>` `:907` (load on later calls; clears on the first call of the frame; resolves MSAA), `fn get_unsampled_color_attachment(&self)` `:916`, `fn out_texture(&self)`.
- To DRAW ON TOP of the scene colour without reading it (clouds composite, precipitation, bolts) do NOT call `post_process_write`; render directly into the current main texture with `LoadOp::Load` using `view_target.main_texture_view()` as the colour view (Bevy's volumetric fog does exactly this, `volumetric_fog/render.rs:395-410`) or `target.get_color_attachment()` (as `main_transparent_pass_3d`). Use `post_process_write` only when the shader must READ the scene colour (tone-map style effects, e.g. a cloud composite that needs `scene_color`).

### Depth access

- Main depth: `ViewDepthStencilTexture { pub attachment: DepthStencilAttachment }` — `render/src/view/mod.rs:1151` (component on each 3D view; renamed from `ViewDepthTexture`). `.texture() -> &Texture`, `.get_attachment(StoreOp) -> RenderPassDepthStencilAttachment<'_>` (`:1170`; LOAD after the first use, used by `main_transparent_pass_3d`, `cp/src/core_3d/main_transparent_pass_3d_node.rs`). Depth format `CORE_3D_DEPTH_FORMAT = TextureFormat::Depth32Float` (`cp/src/core_3d/mod.rs:5`); depth is reverse-Z (`depth_ndc_to_view_z` helper `view_transformations.wesl:169`).
- Views: `view_depth.attachment.depth_stencil_views()` -> `&DepthStencilViews` (`render/src/texture/texture_attachment.rs:206`), with `.depth_only_view() -> Option<&TextureView>` (`:131`, bindable as `texture_depth_2d`, used by volumetric fog `render.rs:332-337`), `.attachment_view() -> &TextureView` (`:149`, the combined view for a render-pass attachment; the combined view cannot be bound as a texture).
- Binding the MAIN depth texture as a sampled texture needs `TEXTURE_BINDING`, which is NOT in the default: `Camera3d::default().depth_texture_usages = TextureUsages::RENDER_ATTACHMENT` (`bevy_camera-0.20.0/src/components.rs:36`). Set `Camera3d { depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING).into(), .. }` (field `components.rs:29`, `Camera3dDepthTextureUsage: From<TextureUsages>` `:45`) if you want to read main depth directly (also needed as `depth_stencil` read-only in the same pass as sampling [INFERENCE: wgpu permits a read-only depth attachment plus a texture binding of the same depth view; not run]).
- Prepass depth (recommended for reading in a shader): add `DepthPrepass` to the camera (`bevy::core_pipeline::prepass::DepthPrepass`, `cp/src/prepass/mod.rs:65`; `DepthPrepassDoubleBuffer` `:91`; main-world component, marker; `DEPTH_PREPASS_TEXTURE_SUPPORTED` is `false` only on WebGL2 wasm without webgpu, `core_3d/mod.rs:18,31`). Render-world component `ViewPrepassTextures` — `cp/src/prepass/mod.rs:126`:
  ```rust
  pub struct ViewPrepassTextures {
      pub depth: Option<DepthStencilAttachment>,   // Some only with DepthPrepass; was Option<ColorAttachment> in 0.19
      pub normal: Option<ColorAttachment>, pub motion_vectors: Option<ColorAttachment>,
      pub deferred: Option<ColorAttachment>, pub deferred_lighting_pass_id: Option<ColorAttachment>,
      pub size: Extent3d,
  }
  fn depth_only_view(&self) -> Option<&TextureView>   // :145
  ```
  The prepass depth texture is a separate texture with `COPY_DST | TEXTURE_BINDING | ..` usage that Bevy fills by copying (`core_3d/mod.rs:~840`, usage block `:~843-845`, and the main depth gets `COPY_SRC` when `DepthPrepass` is present, `core_3d/mod.rs:683-686`), so it can be bound with `texture_depth_2d()` in any pass without special camera settings. It is multisampled if MSAA is on; the plan sets `Msaa::Off` (a component: `bevy::render::view::Msaa`, `view/mod.rs:248`, default `Sample4`, variants `Off=1, Sample2, Sample4, Sample8`).
- `Hdr` marker: `bevy::camera::Hdr` — `bevy_camera-0.20.0/src/components.rs:89`.

### View uniforms and bind group

- `ViewUniforms { pub uniforms: DynamicUniformBuffer<ViewUniform> }` (`Resource`, `view/mod.rs:761`) and `ViewUniformOffset { pub offset: u32 }` (`Component`, `:780`). Bind `view_uniforms.uniforms.binding()` (`Option<BindingResource>`) as `uniform_buffer::<ViewUniform>(true)` (dynamic offset) at group 0 of your own layout and call `pass.set_bind_group(0, &bg, &[view_offset.offset])`. In WESL declare `@group(0) @binding(0) var<uniform> view: bevy_render::view::View;` (`View` struct `view.wesl:40`).
- Alternative that reuses Bevy's group-0 (view, lights, globals, fog, atmosphere ...): `bevy_pbr::MeshViewBindGroup { pub main: BindGroup, pub main_offsets: ArrayVec<u32,7>, pub binding_array: BindGroup, pub empty: BindGroup }` (`bevy_pbr-0.20.0/src/render/mesh_view_bindings.rs:625`, `Component` on each view), bound as `render_pass.set_bind_group(0, &view_bind_group.main, &view_bind_group.main_offsets)` (volumetric fog does this, `render.rs:~435`), with layout `MeshPipeline.main_layout` descriptor (`mesh_view_bindings.rs:76`). Use this only if the shader needs lights/shadow maps/fog; its layout depends on specialisation keys.
- Globals: `bevy::render::globals::GlobalsBuffer` (time) [`render/src/globals.rs`, not opened].

### Procedural (no vertex buffer) instanced-quad pass

There is no vendored example that does exactly this (closest: `custom_phase_item.rs` with a vertex buffer, `fullscreen_material.rs`, and `custom_post_processing.rs` for the fullscreen triangle). Descriptor types are `material/src/descriptor.rs`: `RenderPipelineDescriptor` (`:11`; fields `label, layout, immediate_size, vertex: VertexState, primitive: PrimitiveState, depth_stencil: Option<DepthStencilState>, multisample: MultisampleState, fragment: Option<FragmentState>, zero_initialize_workgroup_memory`), `VertexState { shader, shader_defs, entry_point, buffers: Vec<VertexBufferLayout>, constants }` (`:68`), `FragmentState { shader, shader_defs, entry_point, targets: Vec<Option<ColorTargetState>>, constants }` (`:83`). `queue_render_pipeline(RenderPipelineDescriptor) -> CachedRenderPipelineId`; `get_render_pipeline(id) -> Option<&RenderPipeline>` (as in `custom_post_processing.rs:156,244`). Pattern assembled from those APIs (not run):

```rust
// RenderStartup
let pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
    label: Some("rain".into()),
    layout: vec![layout.clone()],                 // 0: view uniform(dynamic) + particles storage(read_only, VERTEX) + depth texture + params
    vertex: VertexState { shader: shader.clone(), entry_point: Some("vertex".into()), buffers: vec![], ..default() }, // no vertex buffers
    fragment: Some(FragmentState {
        shader,
        entry_point: Some("fragment".into()),
        targets: vec![Some(ColorTargetState {
            format: TextureFormat::Rgba16Float,    // = view_target.main_texture_format() for an Hdr camera
            blend: Some(BlendState::ALPHA_BLENDING),
            write_mask: ColorWrites::ALL,
        })],
        ..default()
    }),
    primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList, cull_mode: None, ..default() },
    depth_stencil: None,       // depth test done in the fragment shader against the prepass depth texture (soft particles)
    multisample: MultisampleState::default(), // Msaa::Off (plan); count must equal the view's sample count
    ..default()
});

// Core3d system, ordered .in_set(Core3dSystems::MainPass).after(main_transparent_pass_3d)
fn draw_rain(view: ViewQuery<(&ViewTarget, &ViewUniformOffset, &ViewPrepassTextures)>, /* res */ mut ctx: RenderContext) {
    let (target, view_offset, prepass) = view.into_inner();
    let Some(depth) = prepass.depth_only_view() else { return };
    // bind group: ViewUniforms binding + particle ShaderBuffer + depth (texture_depth_2d) (create or cache per view)
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("rain"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: target.main_texture_view(), depth_slice: None, resolve_target: None,
            ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
        })],
        depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
    });
    pass.set_render_pipeline(pipeline);               // TrackedRenderPass API
    pass.set_bind_group(0, &bind_group, &[view_offset.offset]);
    pass.draw(0..6, 0..particle_count);               // vertex_index 0..6 builds a quad from @builtin(vertex_index)/(instance_index)
}
```
- Pass descriptor fields in 0.20 include `multiview_mask: None` (see `custom_post_processing.rs:193`, `main_transparent_pass_3d_node.rs`).
- To use hardware depth testing instead, build the depth attachment from the main depth: `RenderPassDepthStencilAttachment { view: view_depth.attachment.depth_stencil_views().attachment_view(), depth_ops: None /* read-only */, stencil_ops: None }` plus `DepthStencilState { format: CORE_3D_DEPTH_FORMAT, depth_write_enabled: false, depth_compare: CompareFunction::GreaterEqual, .. }` (reverse-Z) [INFERENCE: not run]. Reading the same texture in the shader at the same time additionally needs `TEXTURE_BINDING` on the main depth (see above); the prepass-depth approach avoids it.
- `TrackedRenderPass` methods used by Bevy passes: `set_render_pipeline`, `set_bind_group`, `draw`, `set_camera_viewport(&Viewport)` (`main_transparent_pass_3d_node.rs`); when the camera has a viewport, apply it: `Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)` (`bevy_camera::Viewport`, `MainPassResolutionOverride`). `ctx.command_encoder().begin_render_pass(..)` returns the raw wgpu pass (as in the example) with `set_pipeline`.
- Compute inside `Core3d` (per-view cloud raymarch) is allowed: use `ViewQuery` plus `ctx.command_encoder().begin_compute_pass(..)`; the encoder is shared across systems of the same schedule submission order.

## Diagnostics

- `bevy::render::diagnostic::RenderDiagnosticsPlugin` — `render/src/diagnostic/mod.rs:64` (`#[derive(Default)] pub struct RenderDiagnosticsPlugin;`). Add to the MAIN app (`app.add_plugins(RenderDiagnosticsPlugin)`; Bevy example: `examples/3d/solari.rs:57`). `build` inserts `RenderDiagnosticsMutex` and a `PreUpdate` system `sync_diagnostics`; `finish` creates `DiagnosticsRecorder` in the render app and schedules `begin_diagnostics_frame` (after `ExtractCommands`, before `PrepareAssets`), `resolve_encoder` (`RenderGraphSystems::Render`..`Submit`) and `finish_diagnostics_frame` (`RenderGraphSystems::Finish`) (`mod.rs:77-100`). `DefaultPlugins` does NOT include it (only `bevy_diagnostic::DiagnosticsPlugin` is in the group).
- Recording: `RenderContext::diagnostic_recorder() -> Option<Res<DiagnosticsRecorder>>` (`render_context.rs:254`) returns `None` without the plugin. `RecordDiagnostics` (`mod.rs:139`) is implemented for `DiagnosticsRecorder`, `Option<Arc<T>>` (`:245`) and `Option<&T>` (`:289`), so call through `.as_deref()` (`Option<Res<_>>` -> `Option<&DiagnosticsRecorder>`) as Bevy does (`main_transparent_pass_3d_node.rs`):
  ```rust
  fn time_span<E: WriteTimestamp, N: Into<Cow<'static, str>>>(&self, encoder: &mut E, name: N) -> TimeSpanGuard<'_, Self, E>   // :143
  fn pass_span<P: Pass, N: Into<Cow<'static, str>>>(&self, pass: &mut P, name: N) -> PassSpanGuard<'_, Self, P>             // :159
  // guards: .end(encoder) / .end(pass)  (:210, :233); a guard that is dropped without `end` PANICS (:217,:240 drop impls)
  fn record_f32 / record_u32(&self, encoder, &BufferSlice, name)                                                              // :176,:183
  ```
  `WriteTimestamp` is implemented for `CommandEncoder`, `RenderPass`, `ComputePass` (`internal.rs:753,766,772`); `Pass` for `RenderPass` and `ComputePass` (`internal.rs:808,812`).
- Timing a compute pass:
  ```rust
  fn run_sim(mut ctx: RenderContext, /* .. */) {
      let diag = ctx.diagnostic_recorder();          // Option<Res<DiagnosticsRecorder>>
      let diag = diag.as_deref();                    // Option<&DiagnosticsRecorder>, implements RecordDiagnostics
      let mut pass = ctx.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some("wind"), ..default() });
      let span = diag.pass_span(&mut pass, "wind");  // GPU timestamps around the pass (+ compute_shader_invocations where supported)
      /* set_pipeline / set_bind_group / dispatch_workgroups */
      span.end(&mut pass);
  }
  // or around several passes / a whole system:
  let span = diag.time_span(ctx.command_encoder(), "elements_sim");
  /* ... passes ... */
  span.end(ctx.command_encoder());
  ```
  Bevy's own use: `diagnostics.pass_span(&mut render_pass, "main_transparent_pass_3d")` and `diagnostics.time_span(ctx.command_encoder(), "volumetric_lighting")` (`volumetric_fog/render.rs:~334`).
- Platform: "Timestamp queries and pipeline statistics are currently supported only on Vulkan and DX12. On other platforms (Metal, WebGPU, WebGL2) only CPU time will be recorded" (`mod.rs:55-58`). So on Metal/wasm the `elapsed_gpu` diagnostic is absent and only `elapsed_cpu` exists (`internal.rs:536-546` CPU-only branch; GPU branch `:636`).
- Reading the measurement (main world): measurements are copied into `bevy_diagnostic::DiagnosticsStore` by `sync_diagnostics` in `PreUpdate`. Path: `DiagnosticPath::from_components(["render", <span name components...>, <field>])` (`internal.rs:515-520`), i.e. `render/<name>/elapsed_gpu` and `render/<name>/elapsed_cpu` (unit suffix `ms`, values in milliseconds, `internal.rs:636-640`). Nested spans (a span opened while another is open and the parent path is still current) become `render/<parent>/<child>/elapsed_gpu` (`internal.rs:374-385`). Also `.../compute_shader_invocations` for compute passes, and `.../vertex_shader_invocations`, `clipper_invocations`, `clipper_primitives_out`, `fragment_shader_invocations` for render passes (`:660-690`). Value diagnostics from `record_f32/u32` appear as `render/<name>`.
  ```rust
  fn print_ms(store: Res<DiagnosticsStore>) {
      let p = DiagnosticPath::new("render/wind/elapsed_gpu");   // bevy::diagnostic::{DiagnosticPath, DiagnosticsStore}
      if let Some(d) = store.get(&p) { if let Some(ms) = d.smoothed() { info!("wind {ms:.3} ms"); } }
  }
  ```
  `LogDiagnosticsPlugin` prints them too (doc `mod.rs:46`).
- Related plugins in the same module (`mod.rs:29-33`): `RenderAssetDiagnosticPlugin`, `ErasedRenderAssetDiagnosticPlugin`, `MeshAllocatorDiagnosticPlugin`.

# Bevy 0.20.0 render facts, part B: atmosphere, fog and lighting, misc, WebGPU limits

Paths are relative to `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. Facts below were read from the vendored 0.20.0 source, not from bevy.org.

Three plan-breaking findings first:

1. `DirectionalLight::shadows_enabled` is now `shadow_maps_enabled` (`bevy_light-0.20.0/src/directional_light.rs:93`). `PointLight` likewise (`point_light.rs`). The plan's `shadows_enabled: true` will not compile.
2. Observers: `On<Add, T>` is gone. Use `On<Add<T>>` (`bevy_ecs-0.20.0/src/lifecycle.rs:352`; `bevy-0.20.0/_release-content/migration-guides/observer_event_matching.md`; `bevy-0.20.0/examples/ecs/observers.rs:142`).
3. Shaders are WESL. Plain `.wgsl` with no preprocessor directives still works. `#import`, `#ifdef` and `#define_import_path` are gone (`bevy-0.20.0/_release-content/migration-guides/shaders_use_wesl.md`). Imports use `import a::b::c;` at the top of the file, `@if(DEF)` replaces `#ifdef`, and the module path of an embedded shader is `<crate>::<path under src, no extension>`. Bevy's own modules moved too: `bevy_pbr::render::mesh_view_bindings`, `bevy_pbr::prepass::utils`, etc. So `bevy_elements::noise` is `embedded://bevy_elements/<...>/noise.wesl` imported as `bevy_elements::<...>::noise`. The plan's `bevy_elements::noise` name needs a file at `src/noise.wesl` (or write the helpers as `.wgsl` without imports and `include_str!`-concatenate them).

## Atmosphere

Gating: `Atmosphere`, `ScatteringMedium`, `AtmosphereEnvironmentMapLight`, `SunDisk`, `FogVolume`, `VolumetricFog`, `VolumetricLight` live in `bevy_light` (cargo feature `bevy_light`, re-exported as `bevy::light`). `AtmosphereSettings`, `AtmosphereMode`, `AtmospherePlugin` (added by `PbrPlugin`, `bevy_pbr-0.20.0/src/lib.rs:246`), `DistanceFog`, `FogFalloff` live in `bevy_pbr` (re-exported as `bevy::pbr`). Features to enable: `bevy_pbr` and `bevy_light` (`bevy-0.20.0/Cargo.toml:2809,2814`). `bevy_post_process` (`Bloom`, `AutoExposure`) is `bevy-0.20.0/Cargo.toml:2816`. The `3d` feature bundles them.

`AtmospherePlugin` is silently skipped, with a `warn!`, if the adapter lacks compute shaders, `Rgba16Float` `STORAGE_BINDING`, or `max_storage_textures_per_shader_stage > 0` (`bevy_pbr-0.20.0/src/atmosphere/mod.rs:127-152`). It is therefore unavailable on WebGL2 and the plan's wasm target is fine.

### `Atmosphere` (component, on its own entity, NOT on the camera)

`bevy_light-0.20.0/src/atmosphere.rs:34-58`. Path `bevy::light::Atmosphere`.

```rust
#[derive(Clone, Component, FromTemplate)]
#[require(GlobalTransform)]
#[component(on_add = set_default_transform)]
pub struct Atmosphere {
    pub inner_radius: f32,      // m, planet radius
    pub outer_radius: f32,      // m, from planet centre
    pub ground_albedo: Vec3,    // multiscattering only
    pub medium: Handle<ScatteringMedium>,
}
```

- `pub fn earth(medium: Handle<ScatteringMedium>) -> Self` at `atmosphere.rs:74`: inner 6_360_000, outer 6_460_000, albedo `Vec3::splat(0.3)`.
- `pub fn mars(medium) -> Self` at `atmosphere.rs:91`.
- The entity's `GlobalTransform` is the planet centre. If it is still default when `Atmosphere` is added, the hook places it at `-Vec3::Y * inner_radius` (`atmosphere.rs:60-70`). The scale of `GlobalTransform` rescales the planet in world units (`atmosphere.rs:31`). Rotation is ignored.
- Each camera with `AtmosphereSettings` uses the nearest `Atmosphere` by world distance (`bevy_pbr-0.20.0/src/atmosphere/mod.rs:205-262`).
- SNIPPET (`bevy-0.20.0/examples/3d/atmosphere.rs:137-147`):

```rust
let earth_medium = scattering_mediums.add(ScatteringMedium::earth(256, 256)); // ResMut<Assets<ScatteringMedium>>
commands.spawn(Atmosphere::earth(earth_medium));
```

- Replaces 0.19.1: `Atmosphere` used to be a camera component with `bottom_radius/top_radius/ground_albedo/rayleigh_*`. In 0.20 it is a planet entity plus a `ScatteringMedium` asset. There is no `scene_units_to_m` field in 0.20 (grep over `bevy_pbr`, `bevy_light`: no hit). Scene scale is the `GlobalTransform` scale of the `Atmosphere` entity, plus `inner_radius`/`outer_radius` (m).

### `ScatteringMedium` (asset)

`bevy_light-0.20.0/src/atmosphere.rs:134-148`, `Default` = `earth(256, 256)` at `:150`.

```rust
#[derive(TypePath, Asset, Clone)]
pub struct ScatteringMedium {
    pub label: Option<Cow<'static, str>>,
    pub falloff_resolution: u32,
    pub phase_resolution: u32,
    pub terms: SmallVec<[ScatteringTerm; 1]>,
}
pub fn new(falloff_resolution: u32, phase_resolution: u32, terms: impl IntoIterator<Item = ScatteringTerm>) -> Self   // :159
pub fn with_label(self, label: impl Into<Cow<'static, str>>) -> Self                                                 // :173
pub fn with_density_multiplier(self, multiplier: f32) -> Self                                                        // :182
pub fn earth(falloff_resolution: u32, phase_resolution: u32) -> Self                                                 // :197
pub fn mars(falloff_resolution: u32, phase_resolution: u32, dust_phase: Handle<Image>) -> Self                       // :240
```

`ScatteringTerm { absorption: Vec3 /*m^-1*/, scattering: Vec3 /*m^-1*/, falloff: Falloff, phase: PhaseFunction }` (`atmosphere.rs:285-303`). `Falloff::{Linear, Exponential{scale}, Tent{center,width}, Curve(Arc<dyn Curve<f32>>)}` (`:322`). `PhaseFunction` at `:421` (`Isotropic`, `Rayleigh`, `Mie{asymmetry}`, chromatic texture). The Earth medium (`:197-229`): Rayleigh `(5.802e-6, 13.558e-6, 33.100e-6)` with exp scale 8/60; Mie absorption 3.996e-6, scattering 0.444e-6, asymmetry 0.8, scale 1.2/60; ozone absorption `(0.650e-6, 1.881e-6, 0.085e-6)` tent(0.75, 0.3). Mutating the asset re-creates the GPU LUTs (`:128`). This is how a dust storm or haze could tint the sky: edit the medium's terms (e.g. `with_density_multiplier`, or push a dust term).

### `AtmosphereSettings` (camera component)

`bevy_pbr-0.20.0/src/atmosphere/mod.rs:294-362`. Path `bevy::pbr::AtmosphereSettings`. `#[require(Hdr)]` (line 296), so adding it forces HDR. It is a `Reflect` + `Component` and is synced to the render world as `GpuAtmosphereSettings` (`:367`).

| field | default (`mod.rs:346-361`) |
|---|---|
| `transmittance_lut_size: UVec2` | 256×128 |
| `transmittance_lut_samples: u32` | 40 |
| `multiscattering_lut_size: UVec2` | 32×32 |
| `multiscattering_lut_dirs: u32` | 64 |
| `multiscattering_lut_samples: u32` | 20 |
| `sky_view_lut_size: UVec2` | 400×200 |
| `sky_view_lut_samples: u32` | 16 |
| `aerial_view_lut_size: UVec3` | 32×32×32 |
| `aerial_view_lut_samples: u32` | 10 |
| `aerial_view_lut_max_distance: f32` (m, slices distributed linearly, `:330-336`) | 3.2e4 |
| `sky_max_samples: u32` | 16 |
| `rendering_method: AtmosphereMode` | `LookupTexture` (`:423`; other: `Raymarched`, which gives sharp volumetric shadows) |

Note: the aerial-view max distance is in world units after the `Atmosphere` transform scale is applied (the shader uses it with `world_to_atmosphere`; `:268-273`). The atmosphere does not need `DepthPrepass`: the sky pass binds the main depth texture and `configure_camera_depth_usages` adds `TEXTURE_BINDING` to `Camera3d::depth_texture_usages` (`mod.rs:411-417`). `Msaa` is supported (`render_sky_msaa` layout, `resources.rs:44`). On web the transmittance blending is simplified (`mod.rs:31-33` doc).

### `AtmosphereEnvironmentMapLight` (camera or `LightProbe` component)

`bevy_light-0.20.0/src/probe.rs:296-327`. Path `bevy::light::AtmosphereEnvironmentMapLight`.

```rust
#[derive(Component, Clone)]
pub struct AtmosphereEnvironmentMapLight {
    pub intensity: f32,                        // default 1.0
    pub affects_lightmapped_mesh_diffuse: bool, // default true
    pub size: UVec2,                           // default 128x128, power of two (corrected with a warn otherwise)
}
```

An observer (`bevy_pbr-0.20.0/src/atmosphere/environment.rs:207-266`) creates a 6-layer `Rgba16Float` cube `Image` (`TEXTURE_BINDING | STORAGE_BINDING | COPY_SRC`) and inserts `AtmosphereEnvironmentMap` and `GeneratedEnvironmentMapLight { environment_map, intensity, rotation: Quat::IDENTITY, affects_lightmapped_mesh_diffuse }`. The environment is recomputed each frame in `Core3d`, `.after(Core3dSystems::Prepass).before(Core3dSystems::MainPass)` (`mod.rs:~190`, `node.rs:23`, `environment.rs:283`). Replaces 0.19.1: the same name existed on the camera. In 0.20 it is also valid on a `LightProbe`.

### Sun disk

- Component `SunDisk`, `bevy_light-0.20.0/src/directional_light.rs:268-275`, `bevy::light::SunDisk`:

```rust
#[derive(Component, Clone)]
#[require(DirectionalLight)]
pub struct SunDisk {
    pub angular_size: f32, // rad, diameter
    pub intensity: f32,    // 0 disables, 1 default physical, >1 overexposes
}
impl SunDisk { pub const EARTH: SunDisk /*angular_size 0.00930842, intensity 1.0, :283*/; pub const MARS: SunDisk /*0.00615, 1.0, :292*/; pub const OFF: SunDisk /*0.0, 0.0, :300*/; }
```

- Yes: every `DirectionalLight` draws an Earth-sized disk by default. A light without `SunDisk` falls back to `SunDisk::EARTH` via `sun_disk.unwrap_or_default()` (`bevy_pbr-0.20.0/src/render/light.rs:885-886`, `Default for &SunDisk` at `directional_light.rs:312`). The shader loop is in `bevy_pbr-0.20.0/src/atmosphere/functions.wesl:225-246` (`sample_sun_radiance`): it iterates `lights.n_directional_lights`, draws a disk when `sun_angular_size > 0 && sun_intensity > 0`, with radiance `light.color / solid_angle * sun_intensity`.
- To disable it per light, put `SunDisk::OFF` on that light entity. Scattering and illumination are unchanged (`directional_light.rs:296-305` doc). The plan's contingency ("render the moon light without a disk component and skip the moon in the atmosphere") is not needed: the moon light spawns as `(DirectionalLight{..}, SunDisk::OFF)`. Do not use a moon `SunDisk` with `intensity` below 1 for the billboard moon; the plan draws its own disc, so `OFF` is right.
- The disk size is `SunDisk::angular_size` in radians (sun 0.52° is 0.00907 rad; Bevy's 0.0093 = 32 arcmin).
- Glow around the disk needs a `Bloom` on the camera (doc, `directional_light.rs:263-264`).

### Multiple directional lights and the sky

- The atmosphere loops over all directional lights in the view's `Lights` uniform for in-scattering, in the sky-view LUT, aerial-view LUT and multiscattering (`functions.wesl:200-221` `sample_local_inscattering`; `:475` `raymarch_atmosphere`). A moon `DirectionalLight` therefore gets its own sky contribution scaled by its `color` and `illuminance`, and its own `transmittance_to_light` and ground-occlusion term. Cost scales with the number of lights (`mod.rs:15-17` doc).
- Limits: `MAX_DIRECTIONAL_LIGHTS = 10` on native and on wasm with `webgpu`; 1 only for `webgl` without `webgpu` (`bevy_pbr-0.20.0/src/render/light.rs:229-237`). Extra lights are dropped with a one-time `warn!` (`:1182-1190`). `MAX_CASCADES_PER_LIGHT = 4` (1 on webgl-only) (`:241-245`).
- Shadow-casting directional lights: counted among the first 10 lights, capped by `max_texture_array_layers / MAX_CASCADES_PER_LIGHT` (`light.rs:1234-1241`). With the wgpu default `max_texture_array_layers = 256` that is 64. In practice two shadowed lights (sun and moon) are fine. Each costs cascades × shadow-map size (`DirectionalLightShadowMap::size`, default 2048, `directional_light.rs:193-205`).
- The light colour already carries illuminance. The in-scattering uses `(*light).color.rgb` (in the GPU light struct, colour is premultiplied by illuminance), so a moon at 0.3 lux gives a sky about 4e5 times dimmer than the sun. With no exposure adaptation it will be invisible; the plan's auto-exposure covers that.

### Atmosphere LUTs from a custom pass

- The textures exist per view as a render-world component: `bevy::pbr::resources::AtmosphereTextures { transmittance_lut: CachedTexture, multiscattering_lut: CachedTexture, sky_view_lut: CachedTexture, aerial_view_lut: CachedTexture }` (`bevy_pbr-0.20.0/src/atmosphere/resources.rs:389-394`, `#[derive(Component)]`, `pub mod resources` at `mod.rs:41`; `mod atmosphere` is private and re-exported by `pub use atmosphere::*` at `lib.rs:27,59`, so the Rust path is `bevy::pbr::resources::…`, while the WESL modules stay `bevy_pbr::atmosphere::…`). Also public: `AtmosphereSampler` (Resource, `Deref` to `Sampler`, `resources.rs:232`), `GpuAtmosphere` (`:477`), `GpuAtmosphereSettings` (`mod.rs:367`), `AtmosphereTransforms` (`:501`), `AtmosphereBuffer` (Component per camera, `:813`, changed in 0.20: `migration-guides/atmosphere_multicam.md`). A custom render-world system can query `&AtmosphereTextures` on the view entity and bind `.<lut>.default_view`.
- NOT public: `AtmosphereBindGroups` is `pub(crate)` (`resources.rs:596`). Build your own bind group with a layout copied from `AtmosphereBindGroupLayouts`/`bindings.wesl`.
- WESL side: LUT bindings are declared in `bevy_pbr-0.20.0/src/atmosphere/bindings.wesl` at `@group(0)`: 0 `atmosphere: Atmosphere` uniform, 1 `settings: AtmosphereSettings` uniform, 2 `atmosphere_transforms` uniform, 3 `view: View`, 4 `lights: Lights`, 5/6/7 medium density LUT, scattering LUT, sampler, 8 `transmittance_lut: texture_2d<f32>`, 9 `multiscattering_lut`, 10 `sky_view_lut`, 11 `aerial_view_lut: texture_3d<f32>`, 12 `atmosphere_lut_sampler`. Importable functions (module `bevy_pbr::atmosphere::functions`, `functions.wesl`): `sample_transmittance_lut(r, mu)` `:119`, `sample_multiscattering_lut(r, mu)` `:124`, `sample_sky_view_lut(r, ray_dir_as)` `:129`, `sample_aerial_view_lut(uv, t)` `:147`, `sample_sun_radiance(ray_dir_ws)` `:225`, `get_view_position()` `:293`, `raymarch_atmosphere(...)` `:407`. They reference the module-level `bindings::{atmosphere, settings, ...}` globals, so importing them forces the whole `group(0)` layout above onto your shader. Practical route for clouds: write a small WESL module that imports `bevy_pbr::atmosphere::functions::sample_transmittance_lut` and supply a matching group-0 bind group. The 4-storage-texture-per-stage and binding-count pressure is the reason the plan's fallback (analytic Rayleigh/Mie in the cloud shader) may still be simpler.
- Schedule slots: LUTs are built in `Core3d` between `Core3dSystems::Prepass` and `Core3dSystems::MainPass`; the sky draws `.after(main_opaque_pass_3d).before(main_transparent_pass_3d)` (`mod.rs:~185-195`). `Core3dSystems = { Prepass, MainPass, EarlyPostProcess, PostProcess }` (`bevy_core_pipeline-0.20.0/src/schedule.rs:50-55`).

### Camera requirements

- `AtmosphereSettings` requires `Hdr` (`mod.rs:296`). `Hdr` is a unit component `bevy::camera::Hdr` (`bevy_camera-0.20.0/src/components.rs:89`; 0.19 `Camera::hdr` bool is gone).
- `Exposure { ev100: f32 }` is required in practice because raw sunlight is 130 000 lx. The atmosphere example uses `Exposure { ev100: 13.0 }`, `Tonemapping::AcesFitted`, `Bloom::NATURAL`, `AtmosphereEnvironmentMapLight::default()`, `Msaa::Off`, `VolumetricFog { ambient_intensity: 0.0, .. }` (`bevy-0.20.0/examples/3d/atmosphere.rs:150-179`). `GlobalAmbientLight::NONE` is inserted as a resource there (`:48`).
- Light inputs: use `light_consts::lux::RAW_SUNLIGHT` (130 000) for the sun, not `DIRECT_SUNLIGHT`, since the atmosphere does the filtering (`atmosphere.rs` example comment `:205-208`; `bevy_light-0.20.0/src/lib.rs:157`).
- Scale factors: ground albedo `Atmosphere::ground_albedo` (`atmosphere.rs:53`; default 0.3), `aerial_view_lut_max_distance` default 32 km (`AtmosphereSettings`), and the `GlobalTransform` scale of the `Atmosphere` entity (no `scene_units_to_m`).
- The atmosphere and `VolumetricFog` together are "untested" per the module doc (`mod.rs:19-22`), but `examples/3d/atmosphere.rs` does combine them, including `FogVolume::default()` at `:208-211`; the fog shader has an `ATMOSPHERE` define that samples sun transmittance (`volumetric_fog.wesl:324`, `pbr_lighting.wesl:927,940`).

## Fog and lighting

### `VolumetricFog` (camera component)

`bevy_light-0.20.0/src/volumetric.rs:25-55`, `bevy::light::VolumetricFog`. Requires WebGPU on wasm.

```rust
#[derive(Clone, Copy, Component, Debug, Reflect)]
pub struct VolumetricFog {
    pub ambient_color: Color,    // default WHITE
    pub ambient_intensity: f32,  // default 0.1; 0 if no EnvironmentMapLight
    pub jitter: f32,             // m, default 0.0, for TAA
    pub step_count: u32,         // default 64
}
```

### `FogVolume` (entity component)

`bevy_light-0.20.0/src/volumetric.rs:78-125`, `#[require(Transform, Visibility)]`. A unit cube at the origin, shaped by `Transform`; only seen by cameras with `VolumetricFog` and lit by a `VolumetricLight` directional light.

```rust
pub struct FogVolume {
    pub fog_color: Color,                    // WHITE
    pub density_factor: f32,                 // 0.1
    pub density_texture: Option<Handle<Image>>, // None
    pub density_texture_offset: Vec3,        // ZERO, UVW offset, wrap with ImageAddressMode::Repeat
    pub absorption: f32,                     // 0.3
    pub scattering: f32,                     // 0.3
    pub scattering_asymmetry: f32,           // default impl 0.5 (the doc comment says 0.8; the code at volumetric.rs:~118 says 0.5)
    pub light_tint: Color,                   // WHITE
    pub light_intensity: f32,                // 1.0
}
```

- `density_texture` still exists in 0.20; no replacement needed, so the plan's froxel fallback is unused.
- Sampling (`bevy_pbr-0.20.0/src/volumetric_fog/volumetric_fog.wesl:262-276` and `:356-368`): `density = density_factor * textureSampleLevel(density_texture, density_sampler, P_uvw + density_texture_offset, 0.0).r` where `P_uvw` is inside `[0,1]^3`, else density 0. Only the `.r` channel is read. Binding layout: `texture_3d(TextureSampleType::Float { filterable: true })` at binding 2 and `sampler(Filtering)` at binding 3, both fragment stage (`render.rs:216-224`). The bind group uses the `GpuImage`'s own `texture_view` and `sampler` (`render.rs:390-394`), so the image must be `TextureDimension::D3`, a filterable float format (`Rgba16Float`, `Rgba8Unorm`, `R16Float`; NOT `R32Float` on a device without `FLOAT32_FILTERABLE`), and its `ImageSampler` set (use `ImageSampler::linear()` plus `address_mode_*` Repeat if scrolling). To have compute write it: `texture_descriptor.usage |= STORAGE_BINDING`. `Rgba16Float` storage is legal on baseline WebGPU (see limits below).
- Local-to-UVW: `UVW_FROM_LOCAL * local_from_world` (`render.rs:~662`): the transform's unit cube maps to UVW `[0,1]^3`. So an 800×120×800 m volume is `Transform::from_scale(Vec3::new(800.0,120.0,800.0))` centred where wanted.
- The density texture is only bound if it is already loaded in `Assets<Image>`; otherwise the non-textured pipeline runs (`render.rs:355-364`).
- `light_intensity`, `light_tint`, `fog_color`, `absorption`, `scattering`, `density_factor` feed `VolumetricFogUniform` (`render.rs:664-680`).
- SNIPPET: `commands.spawn((FogVolume::default(), Transform::from_scale(Vec3::new(10.0, 1.0, 10.0)).with_translation(Vec3::Y * 0.5)));` (`examples/3d/atmosphere.rs:208-211`).

### `VolumetricLight`

`pub struct VolumetricLight;` (`volumetric.rs:16`), a marker for a `DirectionalLight` with `shadow_maps_enabled: true`. Without a shadow map it has no light shafts.

### `DistanceFog` and `FogFalloff`

`bevy_pbr-0.20.0/src/fog.rs:56-...`, `bevy::pbr::{DistanceFog, FogFalloff}`, `#[derive(Component, ExtractComponent, Reflect)]`, camera component.

```rust
pub struct DistanceFog {
    pub color: Color,
    pub directional_light_color: Color,
    pub directional_light_exponent: f32,
    pub falloff: FogFalloff,
}
// default (fog.rs:469-481): color WHITE, falloff Linear{start 0.0, end 100.0}, directional_light_color Color::NONE, directional_light_exponent 8.0

pub enum FogFalloff {                        // fog.rs:103
    Linear { start: f32, end: f32 },         // :132
    Exponential { density: f32 },            // :187
    ExponentialSquared { density: f32 },     // :235
    Atmospheric { extinction: Vec3, inscattering: Vec3 }, // :284
}
impl FogFalloff {
    pub fn from_visibility(visibility: f32) -> FogFalloff                                  // :313
    pub fn from_visibility_colors(visibility: f32, extinction_color: Color, inscattering_color: Color) -> FogFalloff // :371
    // also from_visibility_contrast, _squared, _contrast_squared, _color, _contrast_color, _contrast_colors
}
```

`from_visibility_colors` uses `REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD`. It returns `Atmospheric`. Colour alpha modulates the effect. `Color::WHITE` or `NONE` extinction disables extinction; `BLACK` or `NONE` inscattering disables inscattering (`fog.rs:~366-369`). Visibility is in world units.

### `Exposure`

`bevy_camera-0.20.0/src/camera.rs:233-284`, `bevy::camera::Exposure`, `#[derive(Component, Clone, Copy, Reflect)]`.

```rust
pub struct Exposure { pub ev100: f32 }
impl Exposure {
    pub const SUNLIGHT: Self (ev100 15.0), OVERCAST (12.0), INDOOR (7.0), BLENDER (9.7);   // :239-253
    pub const EV100_SUNLIGHT/OVERCAST/INDOOR/BLENDER: f32;                                  // :256-264
    pub fn from_physical_camera(PhysicalCameraParameters) -> Self                          // :266
    pub fn exposure(&self) -> f32 { ops::exp2(-self.ev100) / 1.2 }                         // :275
}
impl Default for Exposure { fn default() -> Self { Self::BLENDER } }                       // :280
```

Relation: scene linear colour is multiplied by `exposure() = 2^(-ev100)/1.2`. A surface luminance L (cd/m²) maps to display value about `L * 2^(-ev100) / 1.2` (Filament; `camera.rs:273` links the derivation). The plan's `ev100 = log2(E*100/250)` is a valid illuminance-metering form (K = 12.5 reflected light gives `ev100 = log2(E*S/(C))`; Bevy's `from_physical_camera` is the other route), but note Bevy divides by 1.2, so the plan's formula should be checked against the sun case: 130 000 lx gives ev100 ≈ 19.0, vs the example's 13.0 for the (scattered, dimmer) atmosphere scene. Treat the plan's constant 100/250 as a tunable.

### `Hdr`

Component `bevy::camera::Hdr` (unit struct, `bevy_camera-0.20.0/src/components.rs:87-89`). It replaces `Camera::hdr`. `AtmosphereSettings` and `AutoExposure` both `#[require(Hdr)]` (`mod.rs:296`, `bevy_post_process-0.20.0/src/auto_exposure/settings.rs:29`).

### `DepthPrepass`, `Msaa`, `Tonemapping`, `Bloom`

- `DepthPrepass`: `pub struct DepthPrepass;`, `bevy_core_pipeline-0.20.0/src/prepass/mod.rs:65`, `bevy::core_pipeline::prepass::DepthPrepass`.
- `Msaa`: a camera Component (`bevy_render-0.20.0/src/view/mod.rs:248`, `bevy::render::view::Msaa` and in the prelude, `bevy_render-0.20.0/src/lib.rs:105`): `enum Msaa { Off = 1, Sample2, #[default] Sample4, Sample8 }`, `fn samples(&self) -> u32`. Default is `Sample4`, so set `Msaa::Off` explicitly. A depth prepass read in a custom pass is not possible with MSAA in WebGL2 but is with WebGPU (`core_3d/mod.rs:12-28` doc).
- `Tonemapping`: Component enum at `bevy_render-0.20.0/src/view/mod.rs:283`, imported as `bevy::core_pipeline::tonemapping::Tonemapping` (example). Variants include `None`, `Linear`, `Reinhard`, ..., `AcesFitted`, `TonyMcMapface` (default). `Camera3d` registers it as required (`bevy_core_pipeline-0.20.0/src/core_3d/mod.rs:104-108`).
- `Bloom`: `bevy_post_process-0.20.0/src/bloom/settings.rs:33`, `bevy::post_process::bloom::Bloom`, `Bloom::NATURAL` at `:132`. Replaces 0.19's `bevy::core_pipeline::bloom`.

### `AutoExposure` (built in)

Yes, exists in 0.20: `bevy::post_process::auto_exposure::AutoExposure`, plugin `AutoExposurePlugin` (`bevy_post_process-0.20.0/src/auto_exposure/mod.rs:40`), `#[require(Hdr)]`, requires compute shaders (not WebGL2; `settings.rs:26`).

```rust
pub struct AutoExposure {                         // settings.rs:31
    pub range: RangeInclusive<f32>,               // EV, default -8.0..=8.0
    pub filter: RangeInclusive<f32>,              // histogram percentile, default 0.10..=0.90
    pub speed_brighten: f32,                      // 3.0
    pub speed_darken: f32,                        // 1.0
    pub exponential_transition_distance: f32,     // 1.5
    pub metering_mask: Handle<Image>,
    pub compensation_curve: Handle<AutoExposureCompensationCurve>,
}
```

(defaults `settings.rs:94-106`). It is a histogram-based luminance average that writes a GPU exposure; it ignores the `Exposure` component on that camera and range is EV relative to a fixed pivot, so the huge dynamic range from 130 000 lx to 0.0001 lx (about 30 stops) exceeds `range` ±8. The plan's CPU-driven `Exposure::ev100` stays the right choice for the sun-to-night span, optionally with `AutoExposure` only for fine adaptation. (`AutoExposureCompensationCurve`: `compensation_curve.rs:23`.)

### Lights

- `DirectionalLight` (`bevy_light-0.20.0/src/directional_light.rs:73-145`), `#[require(Cascades, CascadesFrusta, CascadeShadowConfig, CascadesVisibleEntities, Transform, Visibility, VisibilityClass)]`. Fields: `color: Color`, `illuminance: f32` (lux, default `AMBIENT_DAYLIGHT` 10 000, `:151`), `shadow_maps_enabled: bool` (default false), `contact_shadows_enabled: bool` (false), `soft_shadow_size: Option<f32>` (only with feature `experimental_pbr_pcss`), `affects_lightmapped_mesh_diffuse: bool` (true), `shadow_depth_bias: f32` (0.02), `shadow_normal_bias: f32` (1.8). Replaces 0.19.1: `shadows_enabled` is now `shadow_maps_enabled`.
- `CascadeShadowConfig { bounds: Vec<f32>, overlap_proportion: f32, minimum_distance: f32 }` (`cascade.rs:26-32`), built via `CascadeShadowConfigBuilder { num_cascades, minimum_distance, maximum_distance, first_cascade_far_bound, overlap_proportion }` (`:60`), defaults (`:136-158`): 4 cascades (1 on webgl-only), min 0.1, max 150.0, first far bound 10.0, overlap 0.2. `.build() -> CascadeShadowConfig` (`:~91`), `From<Builder> for CascadeShadowConfig`. Lives at `bevy::light::{CascadeShadowConfig, CascadeShadowConfigBuilder}`.
- `PointLight` (`bevy_light-0.20.0/src/point_light.rs:49-...`): `color`, `intensity: f32` (lumens, `:53-54`, default `VERY_LARGE_CINEMA_LIGHT` 1 000 000 lm), `range: f32` (20.0), `radius: f32` (0.0), `shadow_maps_enabled` (false), `contact_shadows_enabled`, `affects_lightmapped_mesh_diffuse`, `shadow_depth_bias`, `shadow_normal_bias`, `shadow_map_near_z`. Defaults at `:129-145`. The plan's `1e8 lm` at range 5 km is legal (no clamp). Shadow cubemaps are capped by `max_texture_cubes` (`render/light.rs:1228-1232`); leave `shadow_maps_enabled` false for the flash.
- Ambient: renamed in 0.20. `AmbientLight` is now a per-camera Component (`#[require(Camera)]`, fields `color`, `brightness`, `affects_lightmapped_meshes`, `bevy_light-0.20.0/src/ambient_light.rs:12`). The global one is `GlobalAmbientLight` Resource (`ambient_light.rs:62`) with `Default` `{ color WHITE, brightness 80.0, affects_lightmapped_meshes true }` and `GlobalAmbientLight::NONE` (brightness 0). Replaces 0.19.1's `AmbientLight` resource. In 0.20 the resource is `GlobalAmbientLight`.
- `light_consts::lux` (`bevy_light-0.20.0/src/lib.rs:95-158`, `bevy::light::light_consts::lux`): `MOONLESS_NIGHT 0.0001`, `FULL_MOON_NIGHT 0.05`, `CIVIL_TWILIGHT 3.4`, `LIVING_ROOM 50`, `HALLWAY 80`, `DARK_OVERCAST_DAY 100`, `OFFICE 320`, `CLEAR_SUNRISE 400`, `OVERCAST_DAY 1000`, `AMBIENT_DAYLIGHT 10_000`, `FULL_DAYLIGHT 20_000`, `DIRECT_SUNLIGHT 100_000`, `RAW_SUNLIGHT 130_000`. Also `light_consts::lumens` (`:99`). Note `FULL_MOON_NIGHT` = 0.05 lx versus the plan's 0.32 lx peak (Krisciunas & Schaefer / astronomical value ≈ 0.3 lux). Either is just a number; the plan's source-based 0.32 wins.
- Night environment: `EnvironmentMapLight { diffuse_map, specular_map, intensity, rotation, .. }` (`probe.rs:107`), `GeneratedEnvironmentMapLight { environment_map, intensity, rotation, affects_lightmapped_mesh_diffuse }` (`probe.rs:269`), `Skybox { image: Option<Handle<Image>>, brightness, rotation }` (`probe.rs:235`), `LightProbe` (`probe.rs:75`). For a night sky, `AtmosphereEnvironmentMapLight` already follows the atmosphere (moon in-scatter included), so no separate probe is required. A starry `Skybox` can be drawn under the atmosphere because the sky pass blends over it (`mod.rs:6-9` doc), but the plan's star pass is separate.

## Misc render facts needed

- `Camera::order: isize` (`bevy_camera-0.20.0/src/camera.rs:389`), default `0` (`:420`). `Camera3d` is `pub struct Camera3d { pub depth_load_op: Camera3dDepthLoadOp, pub depth_texture_usages: Camera3dDepthTextureUsage }`, `#[require(Camera, Projection)]` (`components.rs:22-30`). Also registered as requiring `Tonemapping` and `DebandDither` by `Core3dPlugin` (`core_3d/mod.rs:104-108`).
- Projection: `Projection` enum default `Projection::Perspective(PerspectiveProjection::default())` (`projection.rs:275-279`). `PerspectiveProjection { fov: f32 (PI/4), aspect_ratio: f32 (1.0), near: f32 (0.1), far: f32 (1000.0), near_clip_plane: Vec4 }` (`:284-334`, default `:417-427`). The projection matrix is infinite reverse-Z: it builds `Mat4::from_cols(.., Vec4::new(0,0,near,0))` with no far term (`projection.rs:~376-380`) and `far` is used only for frustum culling (`fn far(&self)`, `:394`; `ViewFrustum::from_clip_from_world_custom_far`, `:73`). So depth is 0 at infinity and `depth == 0.0` is the sky (`render_sky.wesl:49`). The plan's moon billboard at "0.9 × far plane" still works for culling: set `far` ≥ the billboard distance, or set `NoFrustumCulling`.
- `Mesh3d(pub Handle<Mesh>)`, `#[require(Transform)]`, `bevy_mesh-0.20.0/src/components.rs:103`. `MeshMaterial3d<M: Material>(pub Handle<M>)`, `bevy_pbr-0.20.0/src/mesh_material.rs:41`.
- `Aabb { pub center: Vec3A, pub half_extents: Vec3A }`, `bevy_camera-0.20.0/src/primitives.rs:62`, `Aabb::from_min_max(Vec3, Vec3)` (`:~68`). `NoFrustumCulling` (unit struct, `bevy_camera-0.20.0/src/visibility/mod.rs:318`). `Visibility { Inherited (default), Hidden, Visible }` (`visibility/mod.rs:83`).
- `MeshTag { pub value: u32, .. }` (`bevy_mesh-0.20.0/src/components.rs:205-211`, `Deref<Target=u32>`). Note it carries an optional second field (opaque type id, see its doc), so construct with `MeshTag::new(..)` or `MeshTag { value, ..default() }` after reading `:195-260`.
- Screenshot: `pub struct Screenshot(pub RenderTarget)` (`bevy_render-0.20.0/src/view/window/screenshot.rs:84`), `Screenshot::primary_window() -> Self` (`:102`), `Screenshot::image(Handle<Image>) -> Self` (`:107`), `pub fn save_to_disk(path: impl AsRef<Path>) -> impl FnMut(On<ScreenshotCaptured>)` (`:145`), `ScreenshotCaptured { entity: Entity, image: Image }` (`:53`, EntityEvent). Module path `bevy::render::view::window::screenshot` (via `pub use window::*` at `view/mod.rs:12`; `pub mod screenshot` at `view/window/mod.rs:22`). Usage: `commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));`.
- `AppExit` is an enum `AppExit { Success, Error(NonZero<u8>) }` and a `Message` (`bevy_app-0.20.0/src/app.rs:1550`). Write with `world.write_message(AppExit::Success)` or `MessageWriter<AppExit>`.
- `TimeUpdateStrategy` (`bevy_time-0.20.0/src/lib.rs:113-129`): `Automatic` (default), `ManualInstant(Instant)`, `ManualDuration(Duration)`, `FixedTimesteps(u32)`. Insert it as a Resource.
- `Time<Virtual>` (`bevy_time-0.20.0/src/virt.rs:75`): `set_relative_speed(f32)` (`:192`), `set_relative_speed_f64` (`:205`), `relative_speed()` (`:148`), `pause()` (`:219`), `set_max_delta(Duration)` (`:140`); default `max_delta` 250 ms (`virt.rs:86`), so at low frame rates `delta` is clamped. `Time::delta_secs_f64()` at `bevy_time-0.20.0/src/time.rs:290`, `elapsed_secs_f64()` `:312`. Use `Res<Time<Virtual>>`.
- Headless GPU app (`bevy-0.20.0/examples/app/headless_renderer.rs:86-110`): `DefaultPlugins.set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() }).disable::<WinitPlugin>()` plus `ScheduleRunnerPlugin::run_loop(Duration)`. `WinitPlugin` would panic without a display server. `ExitCondition::{OnPrimaryClosed, OnAllClosed, DontExit}` (`bevy_window-0.20.0/src/lib.rs:160-175`). For a test that calls `app.update()` by hand (plan's GPU tests), `app.run()` is not used, so the runner is irrelevant, but `app.finish(); app.cleanup();` must be called once before the first `update()` for the render plugins to finish (call `app.finish()` then `app.cleanup()`; `RenderPlugin` builds the device in `Plugin::build` and in `finish` the sub-app is wired; read `examples/app/externally_driven_headless_renderer.rs:49` for the manual-update pattern before writing the test harness). `bevy::window::ExitCondition`, `bevy::winit::WinitPlugin`, `bevy::app::{AppExit, ScheduleRunnerPlugin}`.
- `RenderPlugin { render_creation: RenderCreation, synchronous_pipeline_compilation: bool, debug_flags: RenderDebugFlags }` (`bevy_render-0.20.0/src/lib.rs:160-166`). `RenderCreation::{Manual(Box<RenderResources>), Automatic(Box<WgpuSettings>)}` (`settings.rs:222-227`; `RenderCreation::manual(..)` at `:231`), and `impl From<WgpuSettings> for RenderCreation` exists. `WgpuSettings` (`settings.rs:39-67`): `device_label`, `backends: Option<Backends>`, `power_preference`, `priority: WgpuSettingsPriority`, `features: WgpuFeatures`, `disabled_features`, `limits: WgpuLimits` (alias of `wgpu::Limits`, `settings.rs:15`), `constrained_limits: Option<WgpuLimits>`, `dx12_shader_compiler`, `gles3_minor_version`, `instance_flags`, `memory_hints`, `instance_memory_budget_thresholds`, `force_fallback_adapter`, `adapter_name`. Use `synchronous_pipeline_compilation: true` in tests so compute kernels are ready on the first frame (otherwise pipelines compile asynchronously and `ready()` returns None for a few frames).
- Raising `max_storage_buffers_per_shader_stage`: with the default `WgpuSettingsPriority::Functionality`, `limits` is REPLACED by `adapter.limits()` (the adapter's maximum), then `constrained_limits` is applied as an upper bound (`bevy_render-0.20.0/src/renderer/mod.rs:318-350`). So on a real adapter you already get the adapter's full value (RTX 3090 Vulkan reports ≥ 16 storage buffers per stage) and need no request. Only with `priority: WgpuSettingsPriority::WebGPU` (`WGPU_SETTINGS_PRIO=webgpu`) is `options.limits` honoured as the request: set `WgpuSettings { priority: WgpuSettingsPriority::WebGPU, limits: Limits { max_storage_buffers_per_shader_stage: 16, ..Limits::defaults() }, .. }`. `WGPU_SETTINGS_PRIO` accepts `webgpu`, `functionality`, `webgl2` (`settings.rs:321-331`). `Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` is requested by default (`settings.rs:~141`).
- Messages: `App::add_message::<M: Message>(&mut self) -> &mut Self` (`bevy_app-0.20.0/src/app.rs:427`); `MessageWriter<'w, M>` with `write(&mut self, M) -> MessageId<M>` (`bevy_ecs-0.20.0/src/message/message_writer.rs:62,74`) and `write_batch`; `MessageReader<'w, 's, M>` with `read(&mut self) -> MessageIterator` (`message_reader.rs:34,44`); `World::write_message<M: Message>(&mut self, M) -> Option<MessageId<M>>` (`bevy_ecs-0.20.0/src/world/mod.rs:3041`). Derive `#[derive(Message)]`.
- Observers: `On<'w,'t,E: EventPattern>` (`bevy_ecs-0.20.0/src/observer/system_param.rs:25`). Lifecycle triggers are `Add<B>`, `Insert<B>` (and `Discard`, `Remove`, `Despawn`), `bevy_ecs-0.20.0/src/lifecycle.rs:352,379`. So `app.add_observer(|add: On<Add<ElementsCamera>>, mut commands: Commands| { let e = add.entity; .. })`. `Add<(A, B)>` is an OR filter (`lifecycle.rs:347-349`). `#[require(...)]` unchanged: `#[derive(Component)] #[require(Hdr)] struct ElementsCamera`. `Single<'w,'s, D: IterQueryData, F: QueryFilter = ()>` at `bevy_ecs-0.20.0/src/system/query.rs:2995`.
- Images, including 3D: `Image::new(size: Extent3d, dimension: TextureDimension, data: Vec<u8>, format: TextureFormat, asset_usage: RenderAssetUsages) -> Image` (`bevy_image-0.20.0/src/image.rs:1123`), `Image::new_uninit(..)` (`:1143`), `Image::new_fill(size, dimension, pixel: &[u8], format, asset_usage)` (`:1206`). Public fields: `data: Option<Vec<u8>>` (`:639`), `texture_descriptor: TextureDescriptor<Option<&'static str>, &'static [TextureFormat]>` (`:652`), `sampler: ImageSampler` (`:654`), `texture_view_descriptor` (`:662`), `asset_usage: RenderAssetUsages` (`:664`). 3D storage texture:

```rust
let mut img = Image::new_fill(Extent3d { width: 128, height: 32, depth_or_array_layers: 128 }, TextureDimension::D3, &[0u8; 8], TextureFormat::Rgba16Float, RenderAssetUsages::RENDER_WORLD);
img.texture_descriptor.usage = TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
img.sampler = ImageSampler::linear();
```

  (`environment.rs:225-246` does the same for a cube.) `RenderAssetUsages` is a bitflag `MAIN_WORLD | RENDER_WORLD` (`bevy_asset-0.20.0/src/render_asset.rs:32-37`), default both (`:40`). Set `RENDER_WORLD` only when the CPU never reads it (a main-world-less asset cannot be mutated afterwards).
- `embedded_asset!` (`bevy_asset-0.20.0/src/io/embedded/mod.rs:352-365`): `embedded_asset!(app, "file.wesl")` registers `file.wesl` relative to the calling source file, with the `src` directory prefix stripped. `embedded_asset!(app, "src_dir", "file")` overrides the prefix. Load path `embedded://<crate_name>/<path under src>`. `load_embedded_asset!` is the loader macro. A shader that must be importable by others is loaded with `bevy_shader::load_shader_library!(app, "file.wesl")` (`bevy_shader-0.20.0/src/lib.rs:28-39`: registers the asset, loads it and `mem::forget`s the handle); WESL names it `<crate>::<path>`.

## WebGPU limits

wgpu 30.0.1, `wgpu-types-30.0.1/src/limits.rs`. `Limits::default()` calls `Limits::defaults()` (`:344-347`).

| limit | `defaults()` (`:421-460`) | `downlevel_defaults()` (`:557-575`) | `downlevel_webgl2_defaults()` (`:644-665`) |
|---|---|---|---|
| `max_texture_dimension_3d` | 2048 | 256 | 256 (inherited) |
| `max_texture_dimension_2d` | 8192 | 2048 | 2048 |
| `max_bind_groups` | 4 | 4 | 4 |
| `max_bindings_per_bind_group` | 1000 | 1000 | 1000 |
| `max_storage_buffers_per_shader_stage` | 8 | 4 | 0 |
| `max_storage_textures_per_shader_stage` | 4 | 4 | 0 |
| `max_storage_buffer_binding_size` | 128 MiB | 128 MiB | 0 |
| `max_buffer_size` | 256 MiB | 256 MiB | 256 MiB |
| `max_compute_workgroup_storage_size` | 16384 | 16352 | 0 |
| `max_compute_invocations_per_workgroup` | 256 | 256 | 0 |
| `max_compute_workgroup_size_x / y / z` | 256 / 256 / 64 | 256 / 256 / 64 | 0 / 0 / 0 |
| `max_compute_workgroups_per_dimension` | 65535 | 65535 | 0 |

Consequences for the plan: 8×8×1 and 4×4×4 workgroups are inside the baseline (64 invocations each, limit 256). A 3D grid larger than 2048 per axis is impossible; the plan's 96×24×96 and 128×32×128 textures are fine. Baseline allows 8 storage buffers per stage; the plan's "8 buffers per stage" splitting rule applies even native if a device reports only the downlevel 4. On wasm, `WgpuSettings::default()` selects `Backends::BROWSER_WEBGPU` with `webgpu` and `target_arch = "wasm32"`, and `Limits::downlevel_webgl2_defaults()` only if the `webgl` feature without `webgpu`, or `WgpuSettingsPriority::WebGL2` (`bevy_render-0.20.0/src/settings.rs:71-100`). Otherwise `Limits::default()` (`:101-117`; `ci_limits` feature clamps to 4 storage textures and 1024 3D dimension). Then, because the default priority is `Functionality`, `renderer/mod.rs:322-326` swaps `limits` for `adapter.limits()`, which in a browser is whatever WebGPU reports (normally the spec defaults above). Bevy's `default_platform` feature list contains `webgl2` (`bevy-0.20.0/Cargo.toml:2894`): the plan's wasm dev-dependency must use `default-features = false, features = ["web", "webgpu"]` as already planned, or `bevy` will pick the GL backend list. When both `webgl` and `webgpu` are enabled, `webgpu` wins for backend and limits.

Storage texture formats (`wgpu-types-30.0.1/src/texture/format.rs:908-1000`, `guaranteed_format_features(device_features)`):

- `Rgba8Unorm`: `STORAGE_READ_ONLY | STORAGE_WRITE_ONLY` only (`:975`). Write-only OK. Read-write needs `Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` and adapter support.
- `Rgba16Float`: `STORAGE_READ_ONLY | STORAGE_WRITE_ONLY` (`:991`), usage includes `STORAGE_BINDING`. So `rgba16float` 3D storage textures are allowed: format feature flags are per format, not per dimension, and WebGPU only restricts 3D for compressed formats.
- `Rgba32Float`, `Rg32Float`: read-only and write-only (`:988,994`). Allowed but not filterable without `FLOAT32_FILTERABLE` (`:1106`).
- `R32Float`: `msaa | STORAGE_READ_ONLY | STORAGE_WRITE_ONLY | STORAGE_READ_WRITE` (`:971`, `s_all`). The only float with baseline read-write along with `R32Uint` and `R32Sint` (`:969-970`). Not filterable by default (`:1106`).
- `Rgba8Snorm`: `storage` only. `Rgba8Unorm`'s sRGB sibling has no storage.
- `Bgra8Unorm` storage needs `Features::BGRA8UNORM_STORAGE` (`:~945`, `features.rs:1716`).
- `Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` (`features.rs:679`) lets `Rgba8Unorm`/`Rgba16Float`/etc. gain `STORAGE_READ_WRITE` if the adapter reports it. Bevy requests it by default (`settings.rs:~141`) but it is a native-only feature; browsers do not expose it. Plan decision "write-only `rgba16float` or `rgba8unorm`, ping-pong, no read_write textures" is correct for wasm.
- Storage-texture count: `max_storage_textures_per_shader_stage` = 4 in `defaults()` (`limits.rs:435`) and `downlevel_defaults`; the atmosphere plugin only needs ≥ 1 (`atmosphere/mod.rs:142-148`). Compute passes that bind more than 4 storage textures must be split.
