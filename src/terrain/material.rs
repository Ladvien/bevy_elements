//! The terrain's materials: `TerrainMaterial` (the owned render's displaced, layer-blended
//! ground) and `GroundExtension` (the same cloud shadows and water darkening on a consumer's own
//! meshes). Both read the packed `terrain_cells` buffer; the crate keeps their uniforms and buffer
//! handle current.

use bevy::asset::{AssetPath, RenderAssetUsages, uuid_handle};
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;

use super::lod::{LodConfig, MAX_LEVELS};
use super::{TerrainParamsGpu, TerrainState};
use crate::ElementsUnits;
use crate::clouds::CloudShadows;
use crate::cover::{CoverState, WetnessSettings};
use crate::trails::TrailState;
use crate::trails::model::MARCH_BOUND_M;
use crate::weathered::ElementsGlobals;

/// A one-cell `terrain_cells` stand-in for shaders that bind the terrain before one exists. Its
/// uniform says "no terrain" (`TerrainParams::misc.z = 0`).
pub(crate) const NO_TERRAIN: Handle<ShaderBuffer> =
    uuid_handle!("2f3c9a40-6a44-4b3a-9a67-0d1b5e7c4a11");

/// Insert the placeholder buffer asset. A zero cell: height 0, no sediment, no water, flat.
pub(crate) fn insert_placeholder(buffers: &mut Assets<ShaderBuffer>) {
    let mut buffer = ShaderBuffer::new(vec![[0f32; 4]], RenderAssetUsages::RENDER_WORLD);
    buffer.label = "elements_terrain_none".into();
    if let Err(error) = buffers.insert(NO_TERRAIN.id(), buffer) {
        error!("could not create the terrain placeholder buffer: {error}");
    }
}

/// Uniform of the owned terrain material. Mirrors `TerrainMaterialParams` of `render.wesl`.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType, Reflect)]
#[reflect(opaque)]
pub struct TerrainMaterialParams {
    pub(crate) terrain: TerrainParamsGpu,
    /// Per level: x morph start, y range (m).
    pub(crate) lod: [Vec4; MAX_LEVELS],
    /// x: coarsest level.
    pub(crate) top: Vec4,
    /// Cloud shadow map origin xy (metre frame), extent, the lights' mean transmittance.
    pub(crate) shadow: Vec4,
    /// Unit direction toward the light; w = 1 with a map.
    pub(crate) light: Vec4,
    /// The water's absorption σ (m⁻¹) in xyz and its refractive index in w (0: no sun-leg
    /// attenuation under water).
    pub(crate) absorption: Vec4,
    /// Ground cover: x 1 when the cover maps are bound, y rain intensity `min(1, R_liquid / 20)`,
    /// z puddle start depth (m), w full-puddle depth (m).
    pub(crate) cover: Vec4,
    /// Phase of each of the four ripple layers, 0–1 (zero while it does not rain).
    pub(crate) ripple: Vec4,
    /// The trail field's window: x texel edge (m), y texels a side, z 1 when the field is bound,
    /// w the parallax march's bound on |h| (m).
    pub(crate) trail: Vec4,
    /// x: the share of the light on a horizontal surface carried by the sun or moon's beam, which
    /// a trail's self-shadow removes. yz: the window's centre (`TrailState::centre`).
    pub(crate) trail_light: Vec4,
    /// The ground's detail textures ([`GroundTextures`]): xyz one tile's size (m) for the sand,
    /// the soil and the grass, zero for a layer left procedural (all zero without a set); w the
    /// procedural grain's albedo share under a textured layer.
    pub(crate) ground: Vec4,
    /// The game's grade of the bare ground ([`TerrainLook`]): xyz the tint, w the saturation.
    pub(crate) look: Vec4,
    /// World units to the metre ([`ElementsUnits`](crate::ElementsUnits)): the vertex and
    /// fragment stages divide the positions Bevy gives them by it and multiply the ones they
    /// hand back.
    pub(crate) units: f32,
}

impl Default for TerrainMaterialParams {
    fn default() -> Self {
        Self {
            terrain: TerrainParamsGpu::default(),
            lod: [Vec4::ZERO; MAX_LEVELS],
            top: Vec4::ZERO,
            shadow: Vec4::ZERO,
            light: Vec4::ZERO,
            absorption: Vec4::ZERO,
            cover: Vec4::ZERO,
            ripple: Vec4::ZERO,
            trail: Vec4::ZERO,
            trail_light: Vec4::ZERO,
            ground: Vec4::W,
            look: Vec4::ONE,
            units: 1.0,
        }
    }
}

/// The extension that turns a `StandardMaterial` into the terrain's displaced, layer-blended
/// ground. The crate fills it.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(TerrainExtensionKey)]
pub struct TerrainExtension {
    #[uniform(100)]
    pub(crate) params: TerrainMaterialParams,
    #[storage(101, read_only)]
    pub(crate) cells: Handle<ShaderBuffer>,
    /// The cover pack (snow depth, wetness, SWE): read by the vertex stage for the snow
    /// displacement too.
    #[storage(102, read_only)]
    pub(crate) cover: Handle<ShaderBuffer>,
    #[texture(103)]
    #[sampler(104)]
    pub(crate) cloud_shadow: Option<Handle<Image>>,
    /// The trail field (`TrailState::field`), sampled by world XZ.
    #[texture(105)]
    #[sampler(106)]
    pub(crate) trail: Option<Handle<Image>>,
    /// The ground's detail textures ([`GroundTextures::image`]), a 2D array of six layers.
    #[texture(107, dimension = "2d_array")]
    #[sampler(108)]
    pub(crate) ground: Option<Handle<Image>>,
}

/// The pipeline key of [`TerrainExtension`]: whether a [`GroundTextures`] set is bound. The
/// textured path is compiled only into the pipeline that has one (the `GROUND_TEXTURES` shader
/// def), so a ground without a set pays nothing for it.
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct TerrainExtensionKey {
    /// A ground texture set is bound.
    pub ground_textures: bool,
}

impl From<&TerrainExtension> for TerrainExtensionKey {
    fn from(extension: &TerrainExtension) -> Self {
        Self {
            ground_textures: extension.ground.is_some(),
        }
    }
}

const RENDER_SHADER: &str = "embedded://bevy_elements/terrain/render.wesl";
const GROUND_SHADER: &str = "embedded://bevy_elements/terrain/ground.wesl";

impl MaterialExtension for TerrainExtension {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::parse(RENDER_SHADER))
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::parse(RENDER_SHADER))
    }

    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::parse(RENDER_SHADER))
    }

    fn deferred_vertex_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::parse(RENDER_SHADER))
    }

    fn deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::parse(RENDER_SHADER))
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.ground_textures
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push("GROUND_TEXTURES".into());
        }
        Ok(())
    }
}

/// The terrain's ground material: a `StandardMaterial` extended with [`TerrainExtension`].
pub type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainExtension>;

/// The terrain's ground **detail textures**: physically based maps of the sand, the soil and the
/// grass, laid over the procedural layers of the owned render
/// (`docs/research/ground-textures.md`).
///
/// `image` is a 2D array texture of six layers, two per ground layer in the order sand, soil,
/// grass, in linear `Rgba8Unorm` with its mip chain:
///
/// - layer `2k`: RGB the albedo **detail**, a multiplier of the layer's own colour with mean 1,
///   stored as `value · 64 / 255` (range 0 to 4); A the relief's height, 0–1 with mean ½;
/// - layer `2k + 1`: RG the tangent normal's u and v, `(c · 127 + 128) / 255`, with u along world
///   X and v along world Z; B the roughness detail and A the ambient occlusion detail, both
///   multipliers with mean 1 stored as `value · 128 / 255`.
///
/// Because the detail is a multiplier with mean 1, the ground's colour at a distance (where the
/// mips average it out) is the procedural one, so a set changes the near field and not the
/// landscape. `tools/ground-textures/prep.py` packs albedos into this format as a KTX2 file with
/// every mip level; a set made from another style's albedos is a stylistic variant of the
/// ground. Load one with [`GroundTextures::load`]. The textures are hex-tiled (Mikkelsen 2022),
/// so a tile never repeats visibly, and where two layers meet the taller texel's layer shows.
/// A layer whose tile is zero is not sampled and stays procedural, so a set may texture some
/// layers only. Without an image (the default) the ground is the procedural one alone, exactly.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct GroundTextures {
    /// The texture set; `None` for the procedural ground alone. Until the image has loaded the
    /// ground is the procedural one.
    pub image: Option<Handle<Image>>,
    /// One tile's size on the ground, m: sand, soil, grass. Zero leaves that layer procedural.
    pub tile_m: Vec3,
    /// The share of the crate's procedural grain's albedo mottle kept under a textured layer,
    /// 0–1: the textures carry the near field's colour (an untextured layer keeps all of it).
    /// The grain's relief is kept whole, because the turf's bumps are what a low sun shows
    /// across the mid field, where the textures' normals have averaged flat.
    pub grain: f32,
}

impl Default for GroundTextures {
    fn default() -> Self {
        Self {
            image: None,
            tile_m: Vec3::new(1.5, 2.0, 2.0),
            grain: 0.3,
        }
    }
}

impl GroundTextures {
    /// Load a texture set (see the type's documentation for the format) with the settings it
    /// needs: linear (not sRGB), repeating, trilinear and anisotropic. The image format's loader
    /// must be enabled (Bevy's `ktx2` and `zstd_rust` features for the prep tool's output).
    pub fn load<'a>(asset_server: &AssetServer, path: impl Into<AssetPath<'a>>) -> Handle<Image> {
        asset_server
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.is_srgb = false;
                settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    label: Some("elements_ground_textures".into()),
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    mag_filter: ImageFilterMode::Linear,
                    min_filter: ImageFilterMode::Linear,
                    mipmap_filter: ImageFilterMode::Linear,
                    anisotropy_clamp: 16,
                    ..default()
                });
            })
            .load(path)
    }
}

/// A game's colour grade of the owned terrain's bare ground (rock, soil, grass, sand, scree) and
/// of the scattered rocks lying on it, so a palette can grade the ground the way the game grades
/// its own materials (`docs/research/terrain-look.md`). The saturation is applied first, around
/// the colour's own linear Rec. 709 luminance — 0 renders the ground at its luminance, 1 leaves
/// it, above 1 oversaturates (negative is taken as 0) — and then the tint multiplies the result,
/// so a cooling tint survives a desaturation. The grade applies to the textured albedo too, and
/// before the ground cover: snow and standing water keep their own colours, and wet grass keeps
/// behaving as grass. The default renders the ground as without it, within a float's rounding.
/// A change is a uniform write (no pipeline is specialised or compiled), so a palette blend may
/// write it every frame. [`TerrainRender::External`](crate::TerrainRender::External) grounds and
/// a game's own [`WeatheredMaterial`](crate::WeatheredMaterial)s are the game's to grade.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TerrainLook {
    /// Linear multiplier on the bare ground's albedo. Default `Vec3::ONE`.
    pub tint: Vec3,
    /// 1 keeps the ground's colours, 0 renders it at its own luminance (grey), above 1
    /// oversaturates. Default 1.
    pub saturation: f32,
}

impl Default for TerrainLook {
    fn default() -> Self {
        Self {
            tint: Vec3::ONE,
            saturation: 1.0,
        }
    }
}

impl TerrainLook {
    /// The uniform's lane: the tint, and the saturation clamped at zero.
    pub(crate) fn packed(&self) -> Vec4 {
        self.tint.max(Vec3::ZERO).extend(self.saturation.max(0.0))
    }
}

/// The ground textures part of the terrain's uniform, and the image to bind: nothing until the
/// set's image exists, so the material is never left waiting for it.
fn ground_inputs(textures: &GroundTextures, images: &Assets<Image>) -> (Vec4, Option<Handle<Image>>) {
    match &textures.image {
        Some(image) if images.contains(image) => (
            textures
                .tile_m
                .max(Vec3::ZERO)
                .extend(textures.grain.clamp(0.0, 1.0)),
            Some(image.clone()),
        ),
        _ => (Vec4::W, None),
    }
}

/// Uniform of [`GroundExtension`]. Mirrors `GroundParams` of `ground.wesl`.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType, Reflect)]
#[reflect(opaque)]
pub struct GroundParams {
    pub(crate) terrain: TerrainParamsGpu,
    /// Cloud shadow map origin xy (metre frame), extent, the lights' mean transmittance.
    pub(crate) shadow: Vec4,
    /// Unit direction toward the light; w = 1 with a map.
    pub(crate) light: Vec4,
    /// The water's absorption σ (m⁻¹) in xyz and its refractive index in w.
    pub(crate) absorption: Vec4,
    /// As `TerrainMaterialParams::cover`.
    pub(crate) cover: Vec4,
    /// As `TerrainMaterialParams::ripple`.
    pub(crate) ripple: Vec4,
    /// x: `column_gate_m`, the depth below the column top where snow, wetness and puddles stop
    /// (m); `NO_COLUMN_GATE` for none.
    pub(crate) gate: Vec4,
    /// As `TerrainMaterialParams::trail`.
    pub(crate) trail: Vec4,
    /// As `TerrainMaterialParams::trail_light`.
    pub(crate) trail_light: Vec4,
    /// As `TerrainMaterialParams::units`.
    pub(crate) units: f32,
}

/// `GroundParams::gate.x` for a ground that is not gated by depth below the column top.
pub(crate) const NO_COLUMN_GATE: f32 = -1.0e30;

impl Default for GroundParams {
    fn default() -> Self {
        Self {
            terrain: TerrainParamsGpu::default(),
            shadow: Vec4::ZERO,
            light: Vec4::ZERO,
            absorption: Vec4::ZERO,
            cover: Vec4::ZERO,
            ripple: Vec4::ZERO,
            gate: Vec4::new(NO_COLUMN_GATE, 0.0, 0.0, 0.0),
            trail: Vec4::ZERO,
            trail_light: Vec4::ZERO,
            units: 1.0,
        }
    }
}

/// Shades a consumer's own mesh as the terrain's ground
/// ([`TerrainRender::External`](crate::TerrainRender::External)): snow covers it, it darkens and
/// glosses when wet, water standing on it shows as a puddle with ripples while it rains, and
/// cloud shadows fall on it, all by world XZ. It does not displace or blend layers; the mesh
/// keeps its own geometry and vertex shader (so snow whitens it but does not raise it).
///
/// Use it as `ExtendedMaterial<StandardMaterial, GroundExtension>` with
/// `GroundExtension::default()`; the crate keeps its buffer, grid and shadow map current every
/// frame, and until a terrain exists it shades as plain `StandardMaterial` (no water, no
/// terrain) with the cloud shadows.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct GroundExtension {
    #[uniform(100)]
    pub(crate) params: GroundParams,
    #[storage(101, read_only)]
    pub(crate) terrain: Handle<ShaderBuffer>,
    #[storage(102, read_only)]
    pub(crate) cover: Handle<ShaderBuffer>,
    #[texture(103)]
    #[sampler(104)]
    pub(crate) cloud_shadow: Option<Handle<Image>>,
    /// The trail field (`TrailState::field`), sampled by world XZ.
    #[texture(105)]
    #[sampler(106)]
    pub(crate) trail: Option<Handle<Image>>,
}

impl Default for GroundExtension {
    fn default() -> Self {
        Self {
            params: GroundParams::default(),
            terrain: NO_TERRAIN,
            cover: NO_TERRAIN,
            cloud_shadow: None,
            trail: None,
        }
    }
}

impl GroundExtension {
    /// Snow, wetness and puddles shade only a fragment within `metres` of the column top or above
    /// it: the terrain's surface height at the fragment's XZ, minus `metres`. A fragment deeper
    /// than that (the walls and floor of a tunnel under a snowy hill) is left as the material
    /// made it. The maps are heightfields, so without the gate a voxel volume's tunnels are
    /// shaded white wherever the map above them has snow.
    ///
    /// Set it on the ground material of a [`VoxelGround`](crate::VoxelGround) volume to the
    /// volume's voxel cell size; the slab of snow sits above `top − cell`. The default, no gate,
    /// is right for a ground that is itself a heightfield.
    #[must_use]
    pub fn with_column_gate_m(mut self, metres: f32) -> Self {
        self.params.gate.x = metres;
        self
    }
}

impl MaterialExtension for GroundExtension {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::parse(GROUND_SHADER))
    }
}

/// The ground material for a consumer's mesh: see [`GroundExtension`].
pub type GroundMaterial = ExtendedMaterial<StandardMaterial, GroundExtension>;

/// The cloud shadow part of both uniforms: map origin and extent, the mean transmittance the
/// lights carry, the light direction, and the map handle.
pub(crate) fn shadow_inputs(shadows: Option<&CloudShadows>) -> (Vec4, Vec4, Option<Handle<Image>>) {
    match shadows {
        Some(s) => (
            Vec4::new(s.origin.x, s.origin.y, s.extent, s.mean_transmittance),
            s.light_direction.extend(1.0),
            Some(s.map.clone()),
        ),
        None => (
            Vec4::new(0.0, 0.0, 1.0, 1.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            None,
        ),
    }
}

/// The per-level morph ranges and the coarsest level, as the vertex shaders read them.
pub(crate) fn lod_uniform(config: &LodConfig) -> ([Vec4; MAX_LEVELS], Vec4) {
    (
        std::array::from_fn(|k| {
            if (k as u32) < config.levels {
                Vec4::new(
                    config.morph_start_m(k as u32),
                    config.range_m(k as u32),
                    0.0,
                    0.0,
                )
            } else {
                Vec4::ZERO
            }
        }),
        Vec4::new((config.levels - 1) as f32, 0.0, 0.0, 0.0),
    )
}

/// The cover part of both uniforms: the live flag, the rain intensity and the puddle depths, and
/// the ripple phases (frozen at zero while it does not rain, so a dry scene re-prepares nothing).
fn cover_inputs(
    live: bool,
    wetness: Option<&WetnessSettings>,
    globals: Option<&ElementsGlobals>,
) -> (Vec4, Vec4) {
    let defaults = WetnessSettings::default();
    let wetness = wetness.unwrap_or(&defaults);
    let rain = globals.map_or(0.0, |g| g.rain_intensity);
    let ripple = match globals {
        Some(g) if rain > 0.0 => Vec4::from_array(g.ripple_phase),
        _ => Vec4::ZERO,
    };
    (
        Vec4::new(
            f32::from(u8::from(live)),
            rain,
            wetness.puddle_min_m,
            wetness.puddle_full_m,
        ),
        ripple,
    )
}

/// The trail part of both uniforms: the window (texel edge, texels a side, 1 when bound, the
/// march's bound on |h|), the light's direct share and the window's centre (metre-frame XZ),
/// and the field.
fn trail_inputs(trails: Option<&TrailState>, live: bool) -> (Vec4, Vec4, Option<Handle<Image>>) {
    match trails {
        Some(t) if live => (
            Vec4::new(t.texel_m, t.texels as f32, 1.0, MARCH_BOUND_M),
            Vec4::new(t.direct_share, t.centre.x, t.centre.y, 0.0),
            Some(t.field.clone()),
        ),
        _ => (Vec4::ZERO, Vec4::ZERO, None),
    }
}

/// Keep every terrain and ground material's uniform, buffer and shadow map current. Writes an
/// asset only when something changed, so a still scene re-prepares nothing.
#[allow(clippy::too_many_arguments)]
pub(super) fn update_materials(
    state: Option<Res<TerrainState>>,
    runtime: Res<super::TerrainRuntime>,
    shadows: Option<Res<CloudShadows>>,
    units: Res<ElementsUnits>,
    cover: Option<Res<CoverState>>,
    wetness: Option<Res<WetnessSettings>>,
    globals: Option<Res<ElementsGlobals>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut ground_materials: ResMut<Assets<GroundMaterial>>,
    water: Option<Res<crate::hydrology::WaterSettings>>,
    trails: Option<Res<TrailState>>,
    textures: Res<GroundTextures>,
    images: Res<Assets<Image>>,
    look: Res<TerrainLook>,
) {
    let (shadow, light, map) = shadow_inputs(shadows.as_deref());
    let absorption = water.map_or(Vec4::ZERO, |w| w.absorption.extend(w.refractive_index));
    let live = state.as_ref().is_some_and(|s| s.is_live());
    let terrain_params = state
        .as_ref()
        .map(|s| TerrainParamsGpu::new(&s.grid, 1, live))
        .unwrap_or_default();
    let cells = state.as_ref().map_or(NO_TERRAIN, |s| s.cells.clone());
    let cover_cells = cover.as_ref().map_or(NO_TERRAIN, |c| c.cover_cells.clone());
    let (cover_params, ripple) = cover_inputs(
        cover.is_some() && live,
        wetness.as_deref(),
        globals.as_deref(),
    );
    let (trail_window, trail_light, field) = trail_inputs(trails.as_deref(), live);
    let (ground, ground_maps) = ground_inputs(&textures, &images);

    if let (Some(state), Some(config)) = (state.as_ref(), runtime.lod.as_ref()) {
        let (lod, top) = lod_uniform(config);
        let params = TerrainMaterialParams {
            terrain: terrain_params,
            lod,
            top,
            shadow,
            light,
            absorption,
            cover: cover_params,
            ripple,
            trail: trail_window,
            trail_light,
            ground,
            look: look.packed(),
            units: units.per_metre(),
        };
        let ids: Vec<_> = terrain_materials.iter().map(|(id, _)| id).collect();
        for id in ids {
            let Some(material) = terrain_materials.get(id) else {
                continue;
            };
            let e = &material.extension;
            if (e.params != params
                || e.cells != state.cells
                || e.cover != cover_cells
                || e.cloud_shadow != map
                || e.trail != field
                || e.ground != ground_maps)
                && let Some(mut material) = terrain_materials.get_mut(id)
            {
                material.extension.params = params;
                material.extension.cells = state.cells.clone();
                material.extension.cover = cover_cells.clone();
                material.extension.cloud_shadow = map.clone();
                material.extension.trail = field.clone();
                material.extension.ground = ground_maps.clone();
            }
        }
    }

    let params = GroundParams {
        terrain: terrain_params,
        shadow,
        light,
        absorption,
        cover: cover_params,
        ripple,
        gate: GroundParams::default().gate,
        trail: trail_window,
        trail_light,
        units: units.per_metre(),
    };
    let ids: Vec<_> = ground_materials.iter().map(|(id, _)| id).collect();
    for id in ids {
        let Some(material) = ground_materials.get(id) else {
            continue;
        };
        let e = &material.extension;
        // The gate is the material's own: everything else is shared.
        let params = GroundParams {
            gate: e.params.gate,
            ..params
        };
        if (e.params != params
            || e.terrain != cells
            || e.cover != cover_cells
            || e.cloud_shadow != map
            || e.trail != field)
            && let Some(mut material) = ground_materials.get_mut(id)
        {
            material.extension.params = params;
            material.extension.terrain = cells.clone();
            material.extension.cover = cover_cells.clone();
            material.extension.cloud_shadow = map.clone();
            material.extension.trail = field.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::{Vec3, Vec4};

    use super::TerrainLook;

    /// terrain-look.md Model adopted 4: the default look packs to exactly (1, 1, 1, 1), the value
    /// the shader skips, and a negative tint or saturation packs to zero.
    #[test]
    fn the_default_look_packs_to_the_shaders_identity() {
        assert_eq!(TerrainLook::default().packed(), Vec4::ONE);
        let clamped = TerrainLook {
            tint: Vec3::new(-1.0, 0.5, 2.0),
            saturation: -0.3,
        }
        .packed();
        assert_eq!(clamped, Vec4::new(0.0, 0.5, 2.0, 0.0));
    }

    /// Terrain validation 9: each extension declares at most two storage buffers (terrain cells
    /// now, the cover pack with M4), so with Bevy's five in the fragment stage the WebGPU default
    /// of eight holds. Counted from the declarations: the layout itself needs a device.
    #[test]
    fn each_extension_binds_at_most_two_storage_buffers() {
        let source = include_str!("material.rs");
        for extension in ["pub struct TerrainExtension", "pub struct GroundExtension"] {
            let body = &source[source.find(extension).expect("the struct")..];
            let body = &body[..body.find("\n}").expect("the end of the struct")];
            let storage = body.matches("#[storage(").count();
            assert!(
                (1..=2).contains(&storage),
                "{extension}: {storage} storage buffers"
            );
        }
    }
}
