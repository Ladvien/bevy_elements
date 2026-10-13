//! The terrain's materials: `TerrainMaterial` (the owned render's displaced, layer-blended
//! ground) and `GroundExtension` (the same cloud shadows and water darkening on a consumer's own
//! meshes). Both read the packed `terrain_cells` buffer; the crate keeps their uniforms and buffer
//! handle current.

use bevy::asset::{AssetPath, RenderAssetUsages, uuid_handle};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;

use super::lod::{LodConfig, MAX_LEVELS};
use super::{TerrainParamsGpu, TerrainState};
use crate::clouds::CloudShadows;

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
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType, Reflect)]
#[reflect(opaque)]
pub struct TerrainMaterialParams {
    pub(crate) terrain: TerrainParamsGpu,
    /// Per level: x morph start, y range (m).
    pub(crate) lod: [Vec4; MAX_LEVELS],
    /// x: coarsest level.
    pub(crate) top: Vec4,
    /// Cloud shadow map origin xy, extent, camera transmittance.
    pub(crate) shadow: Vec4,
    /// Unit direction toward the light; w = 1 with a map.
    pub(crate) light: Vec4,
}

/// The extension that turns a `StandardMaterial` into the terrain's displaced, layer-blended
/// ground. The crate fills it.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct TerrainExtension {
    #[uniform(100)]
    pub(crate) params: TerrainMaterialParams,
    #[storage(101, read_only)]
    pub(crate) cells: Handle<ShaderBuffer>,
    #[texture(103)]
    #[sampler(104)]
    pub(crate) cloud_shadow: Option<Handle<Image>>,
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
}

/// The terrain's ground material: a `StandardMaterial` extended with [`TerrainExtension`].
pub type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainExtension>;

/// Uniform of [`GroundExtension`]. Mirrors `GroundParams` of `ground.wesl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType, Reflect)]
#[reflect(opaque)]
pub struct GroundParams {
    pub(crate) terrain: TerrainParamsGpu,
    /// Cloud shadow map origin xy, extent, camera transmittance.
    pub(crate) shadow: Vec4,
    /// Unit direction toward the light; w = 1 with a map.
    pub(crate) light: Vec4,
}

/// Shades a consumer's own mesh as the terrain's ground
/// ([`TerrainRender::External`](crate::TerrainRender::External)): the ground darkens where water
/// stands and cloud shadows fall on it, both by world XZ. It does not displace or blend layers;
/// the mesh keeps its own geometry and vertex shader.
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
    #[texture(103)]
    #[sampler(104)]
    pub(crate) cloud_shadow: Option<Handle<Image>>,
}

impl Default for GroundExtension {
    fn default() -> Self {
        Self {
            params: GroundParams::default(),
            terrain: NO_TERRAIN,
            cloud_shadow: None,
        }
    }
}

impl MaterialExtension for GroundExtension {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::parse(GROUND_SHADER))
    }
}

/// The ground material for a consumer's mesh: see [`GroundExtension`].
pub type GroundMaterial = ExtendedMaterial<StandardMaterial, GroundExtension>;

/// The cloud shadow part of both uniforms: map origin and extent, the camera's transmittance, the
/// light direction, and the map handle.
fn shadow_inputs(shadows: Option<&CloudShadows>) -> (Vec4, Vec4, Option<Handle<Image>>) {
    match shadows {
        Some(s) => (
            Vec4::new(s.origin.x, s.origin.y, s.extent, s.camera_transmittance),
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

/// Keep every terrain and ground material's uniform, buffer and shadow map current. Writes an
/// asset only when something changed, so a still scene re-prepares nothing.
pub(super) fn update_materials(
    state: Option<Res<TerrainState>>,
    runtime: Res<super::TerrainRuntime>,
    shadows: Option<Res<CloudShadows>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut ground_materials: ResMut<Assets<GroundMaterial>>,
) {
    let (shadow, light, map) = shadow_inputs(shadows.as_deref());
    let live = state.as_ref().is_some_and(|s| s.is_live());
    let terrain_params = state
        .as_ref()
        .map(|s| TerrainParamsGpu::new(&s.grid, 1, live))
        .unwrap_or_default();
    let cells = state.as_ref().map_or(NO_TERRAIN, |s| s.cells.clone());

    if let (Some(state), Some(config)) = (state.as_ref(), runtime.lod.as_ref()) {
        let config: &LodConfig = config;
        let params = TerrainMaterialParams {
            terrain: terrain_params,
            lod: std::array::from_fn(|k| {
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
            top: Vec4::new((config.levels - 1) as f32, 0.0, 0.0, 0.0),
            shadow,
            light,
        };
        let ids: Vec<_> = terrain_materials.iter().map(|(id, _)| id).collect();
        for id in ids {
            let Some(material) = terrain_materials.get(id) else {
                continue;
            };
            let e = &material.extension;
            if (e.params != params || e.cells != state.cells || e.cloud_shadow != map)
                && let Some(mut material) = terrain_materials.get_mut(id)
            {
                material.extension.params = params;
                material.extension.cells = state.cells.clone();
                material.extension.cloud_shadow = map.clone();
            }
        }
    }

    let params = GroundParams {
        terrain: terrain_params,
        shadow,
        light,
    };
    let ids: Vec<_> = ground_materials.iter().map(|(id, _)| id).collect();
    for id in ids {
        let Some(material) = ground_materials.get(id) else {
            continue;
        };
        let e = &material.extension;
        if (e.params != params || e.terrain != cells || e.cloud_shadow != map)
            && let Some(mut material) = ground_materials.get_mut(id)
        {
            material.extension.params = params;
            material.extension.terrain = cells.clone();
            material.extension.cloud_shadow = map.clone();
        }
    }
}

#[cfg(test)]
mod tests {
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
