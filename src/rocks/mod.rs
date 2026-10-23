//! Rocks on the ground: boulders and stones scattered over the terrain as meshes, so the ground
//! has silhouettes and cast shadows and not only a painted texture
//! (`docs/research/rock-scatter.md`).
//!
//! Where rocks lie comes from the terrain readback (the slope and the sediment), how big they are
//! from Golombek's exponential rock-abundance law, where exactly from a jittered lattice per size
//! class thinned by the local cover, and how far they are drawn from their size. Each rock is
//! seated on the surface the terrain's level-of-detail patches actually draw, and drawn with the
//! crate's [`WeatheredMaterial`] (snow, wetness, cloud shadows) with a stone grain.

mod mesh;
pub(crate) mod model;

use bevy::camera::visibility::VisibilityRange;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::plugin::{ElementsQuality, ElementsSystems, ElementsUnits, ElementsView};
use crate::query::EnvironmentSnapshot;
use crate::terrain::{TerrainRuntime, TerrainSnapshot, TerrainState};
use crate::weathered::{WeatheredExtension, WeatheredMaterial, WeatheredPlugin};
use mesh::{FAR_SUBDIVISIONS, NEAR_SUBDIVISIONS, SHAPES, ShapeInfo, rock_geometry, rock_mesh};
use model::{
    CLASSES, ClassLattice, CoverParams, Lattices, PatchLevels, Placed, TILT_TO_SLOPE, tile_rocks,
};

/// How many rocks lie on the ground and where. Insert it before the plugin to override the
/// defaults; a change rebuilds the scatter.
///
/// The covers are Golombek's `k`: the fraction of the ground covered by rocks of every size
/// (Golombek et al. 2021, `docs/research/rock-scatter.md`). Rocks smaller than 25 cm are not
/// drawn as meshes (the terrain material's scree and turf carry them), so a cover of 0.30 shows
/// as 17 % of the ground under rocks of 25 cm and up.
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RockSettings {
    /// Cover of a talus apron: the band of slope at a cliff's foot where the terrain material
    /// draws its scree (16°–40°, densest at 26°). Default 0.30.
    pub talus_cover: f32,
    /// Cover of the meadow where it is rocky (about a quarter of it, in patches a few hundred
    /// metres across, and more of it the steeper the hillside). Default 0.05 (Golombek's rocky
    /// ground, 4–5 %).
    pub meadow_cover: f32,
    /// Cover of the flat meadow away from the rocky patches. Default 0.015 (Golombek's plains,
    /// 1–2 %).
    pub meadow_floor: f32,
    /// A multiplier on how far rocks are drawn ([`ElementsQuality::rock_range`] metres per metre
    /// of a rock's diameter). Default 1; 0 draws none.
    pub range_scale: f32,
    /// Seed of the scatter.
    pub seed: u64,
}

impl Default for RockSettings {
    fn default() -> Self {
        Self {
            talus_cover: 0.30,
            meadow_cover: 0.05,
            meadow_floor: 0.015,
            range_scale: 1.0,
            seed: 0,
        }
    }
}

impl RockSettings {
    fn cover(&self) -> CoverParams {
        CoverParams {
            talus_cover: self.talus_cover.max(0.0),
            meadow_floor: self.meadow_floor.max(0.0),
            meadow_cover: self.meadow_cover.max(0.0),
        }
    }
}

/// One scattered rock: the entity that carries its transform. Its two children draw it (a near
/// and a far mesh, crossfaded by [`VisibilityRange`]); the near child's mesh is kept in the main
/// world, so a game can build a collider from it when this component is added.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct ScatteredRock {
    /// Its diameter in plan, m (the mean of its long and middle axes).
    pub diameter_m: f32,
    /// Its height, m, of which a quarter to a third is below the ground.
    pub height_m: f32,
}

/// How many rocks are on the ground now, and in how many tiles.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub struct RockStats {
    /// Rocks spawned.
    pub rocks: u32,
    /// Tiles spawned, over every size class.
    pub tiles: u32,
}

/// Rocks scattered on the [`Terrain`](crate::Terrain): see [`RockSettings`].
///
/// Needs the [`TerrainPlugin`](crate::TerrainPlugin) and an
/// [`ElementsCamera`](crate::ElementsCamera); adds the [`WeatheredPlugin`] if it is missing.
pub struct RocksPlugin;

impl Plugin for RocksPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WeatheredPlugin>() {
            app.add_plugins(WeatheredPlugin);
        }
        app.init_resource::<RockSettings>()
            .init_resource::<RockStats>()
            .init_resource::<RockField>()
            .add_systems(Startup, create_assets)
            .add_systems(
                Update,
                (update_levels, scatter, seat_rocks)
                    .chain()
                    .after(ElementsSystems::Readback),
            );
    }
}

/// Tiles built per frame at most, nearest first: a camera that jumps fills its rings over a few
/// frames instead of stalling one.
const TILES_PER_FRAME: usize = 24;
/// The camera moves this far, m, before the rocks are seated again (the morph follows it).
const SEAT_STEP_M: f32 = 0.25;
/// A seat that moved less than this, m, is left alone (no change to the transform).
const SEAT_TOLERANCE_M: f32 = 0.001;
/// The near mesh is drawn to this many metres per metre of diameter (Deviation 4).
const NEAR_PER_M: f32 = 22.0;
/// A rock shrinks to nothing over the last fifth of its range, and its two meshes switch
/// abruptly. Bevy's dithered `VisibilityRange` crossfade left black holes in every fading rock
/// under the crate's depth prepass, mask or not (`AGENTS.md`), and a rock a few pixels wide that
/// shrinks away does not pop.
const FADE: f32 = 0.2;
/// Diameter bins per class for the draw ranges: few distinct [`VisibilityRange`]s, so Bevy's
/// table (64 entries where storage buffers are scarce) holds them all.
const RANGE_BINS: usize = 6;

/// The meshes and the material every rock shares.
#[derive(Resource)]
struct RockAssets {
    /// Per shape: the near and the far mesh.
    meshes: Vec<[Handle<Mesh>; 2]>,
    shapes: Vec<ShapeInfo>,
    material: Handle<WeatheredMaterial>,
}

/// The terrain rock's albedo (`ROCK` in `render.wesl`); the shapes' vertex colours carry the
/// palette's tone.
const ROCK_ALBEDO: [f32; 3] = [0.076, 0.071, 0.066];
/// The stone grain's feature size, m (Parameters).
const GRAIN_M: f32 = 0.30;

fn create_assets(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<WeatheredMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let mut handles = Vec::with_capacity(SHAPES);
    let mut shapes = Vec::with_capacity(SHAPES);
    for shape in 0..SHAPES {
        let near = rock_geometry(shape, NEAR_SUBDIVISIONS);
        shapes.push(near.info);
        let far = rock_geometry(shape, FAR_SUBDIVISIONS);
        handles.push([meshes.add(rock_mesh(near)), meshes.add(rock_mesh(far))]);
    }
    let material = materials.add(WeatheredMaterial {
        base: StandardMaterial {
            base_color: Color::linear_rgb(ROCK_ALBEDO[0], ROCK_ALBEDO[1], ROCK_ALBEDO[2]),
            perceptual_roughness: 0.9,
            reflectance: 0.35,
            ..default()
        },
        extension: WeatheredExtension::default().with_grain(GRAIN_M),
    });
    commands.insert_resource(RockAssets {
        meshes: handles,
        shapes,
        material,
    });
}

/// What the scatter was built for: a change rebuilds it.
#[derive(Clone, Debug, PartialEq)]
struct FieldKey {
    grid_origin: Vec2,
    grid_cell: f32,
    grid_size: UVec2,
    grid_y0: f32,
    cover: CoverParams,
    seed: u64,
    range_per_m: f32,
    units: f32,
}

/// The live scatter.
#[derive(Resource, Default)]
struct RockField {
    key: Option<FieldKey>,
    lattices: Vec<ClassLattice>,
    /// Per (class, tile): the tile's entity and its rock count.
    tiles: HashMap<(usize, IVec2), (Entity, u32)>,
    /// The patch levels the terrain draws, for the seat.
    levels: Option<PatchLevels>,
    /// Set when `levels` changed since the last seat.
    levels_changed: bool,
}

/// Where a rock sits, for re-seating it when the camera or the terrain moves.
#[derive(Component, Clone, Copy, Debug)]
struct RockSeat {
    /// Metre-frame XZ.
    xz: Vec2,
    /// The rock's up (its short axis), unit.
    up: Vec3,
    /// From the surface along `up` to the mesh's centre, m.
    lift_m: f32,
    /// The mesh's scale, m (its long axis's length).
    scale_m: f32,
    /// Where it has shrunk away, m from the eye.
    far_m: f32,
    /// The two surfaces the patches draw here, the camera's distance morphing between them;
    /// read at the surface `version`, from the patch levels when `patched`.
    lattices: Lattices,
    version: u32,
    patched: bool,
}

/// The tile entity of one class: the parent of its rocks.
#[derive(Component)]
struct RockTile;

/// Mirror the terrain's selected patches into a level per leaf, for the seat.
fn update_levels(
    runtime: Option<Res<TerrainRuntime>>,
    state: Option<Res<TerrainState>>,
    mut field: ResMut<RockField>,
) {
    let levels = runtime.zip(state).and_then(|(runtime, state)| {
        let lod = runtime.lod?;
        if !runtime.draws_ground() {
            return None;
        }
        let leaves = lod.nodes(0);
        let mut level = vec![u8::MAX; (leaves.x * leaves.y) as usize];
        for patch in &runtime.selected {
            let span = 1u32 << patch.level;
            for z in patch.iz * span..((patch.iz + 1) * span).min(leaves.y) {
                for x in patch.ix * span..((patch.ix + 1) * span).min(leaves.x) {
                    level[(z * leaves.x + x) as usize] = patch.level as u8;
                }
            }
        }
        Some(PatchLevels {
            cell_m: state.grid.cell_m,
            origin: state.grid.origin,
            top: lod.levels.saturating_sub(1),
            morph: (0..lod.levels)
                .map(|k| (lod.morph_start_m(k), lod.range_m(k)))
                .collect(),
            leaf_m: lod.leaf_m,
            leaves,
            level,
        })
    });
    if field.levels != levels {
        field.levels = levels;
        field.levels_changed = true;
    }
}

/// The rock's draw range, m: binned per class so few distinct ranges exist.
fn binned_diameter(class: usize, diameter_m: f32) -> f32 {
    let (a, b) = CLASSES[class];
    let t = ((diameter_m / a).ln() / (b / a).ln()).clamp(0.0, 0.999);
    let bin = (t * RANGE_BINS as f32).floor();
    a * (b / a).powf((bin + 0.5) / RANGE_BINS as f32)
}

/// Build the tiles that entered each class's ring and drop the ones that left it.
#[allow(clippy::too_many_arguments)]
fn scatter(
    mut commands: Commands,
    settings: Res<RockSettings>,
    quality: Res<ElementsQuality>,
    units: Res<ElementsUnits>,
    view: Res<ElementsView>,
    snapshot: Res<EnvironmentSnapshot>,
    assets: Option<Res<RockAssets>>,
    mut field: ResMut<RockField>,
    mut stats: ResMut<RockStats>,
) {
    let Some(assets) = assets else {
        return;
    };
    let Some(snap) = snapshot.terrain.as_ref() else {
        clear(&mut commands, &mut field, &mut stats);
        field.key = None;
        return;
    };
    if view.entity.is_none() {
        return;
    }
    let cover = settings.cover();
    let range_per_m = quality.rock_range() * settings.range_scale.max(0.0);
    let u = units.per_metre();
    let key = FieldKey {
        grid_origin: snap.grid.origin,
        grid_cell: snap.grid.cell_m,
        grid_size: snap.grid.size,
        grid_y0: snap.grid.y0,
        cover,
        seed: settings.seed,
        range_per_m,
        units: u,
    };
    if field.key.as_ref() != Some(&key) {
        clear(&mut commands, &mut field, &mut stats);
        field.lattices = (0..CLASSES.len())
            .map(|class| ClassLattice::new(class, cover.max_cover(), range_per_m))
            .collect();
        field.key = Some(key);
    }
    if cover.max_cover() <= 0.0 || range_per_m <= 0.0 {
        return;
    }
    let eye = view.eye;
    let eye_xz = eye.xz();
    // Drop the tiles that left their ring by more than a tile (the margin keeps a camera on a
    // tile's edge from rebuilding it every frame).
    let lattices = field.lattices.clone();
    field.tiles.retain(|&(class, tile), &mut (entity, rocks)| {
        let lattice = &lattices[class];
        if lattice.tile_distance(tile, eye_xz) > lattice.radius_m + lattice.tile_m() {
            commands.entity(entity).despawn();
            stats.rocks = stats.rocks.saturating_sub(rocks);
            stats.tiles = stats.tiles.saturating_sub(1);
            false
        } else {
            true
        }
    });
    let mut wanted: Vec<(f32, usize, IVec2)> = lattices
        .iter()
        .flat_map(|lattice| {
            lattice
                .tiles_near(eye_xz, 0.0)
                .into_iter()
                .map(move |tile| (lattice.tile_distance(tile, eye_xz), lattice.class, tile))
        })
        .filter(|(_, class, tile)| !field.tiles.contains_key(&(*class, *tile)))
        .collect();
    wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
    for &(_, class, tile) in wanted.iter().take(TILES_PER_FRAME) {
        let lattice = &lattices[class];
        let rocks = tile_rocks(snap, &cover, lattice, tile, settings.seed);
        let count = rocks.len() as u32;
        let parent = commands
            .spawn((RockTile, Transform::IDENTITY, Visibility::default()))
            .id();
        for rock in &rocks {
            let child = spawn_rock(
                &mut commands,
                &assets,
                class,
                rock,
                range_per_m,
                u,
                eye,
                snap,
                field.levels.as_ref(),
            );
            commands.entity(parent).add_child(child);
        }
        field.tiles.insert((class, tile), (parent, count));
        stats.rocks += count;
        stats.tiles += 1;
    }
}

/// Spawn one rock and its two meshes, seated on the surface the patches draw for an eye at `eye`.
#[allow(clippy::too_many_arguments)]
fn spawn_rock(
    commands: &mut Commands,
    assets: &RockAssets,
    class: usize,
    rock: &Placed,
    range_per_m: f32,
    units: f32,
    eye: Vec3,
    snap: &TerrainSnapshot,
    levels: Option<&PatchLevels>,
) -> Entity {
    let info = assets.shapes[rock.shape];
    // The diameter in plan is the mean of the long and middle axes.
    let scale_m = 2.0 * rock.diameter_m / (1.0 + info.b);
    let height_m = info.c * scale_m;
    let up = Quat::from_euler(EulerRot::XYZ, rock.jitter.x, 0.0, rock.jitter.y)
        * Vec3::Y.lerp(rock.normal, TILT_TO_SLOPE).normalize();
    let rotation = Quat::from_rotation_arc(Vec3::Y, up) * Quat::from_rotation_y(rock.yaw);
    let d = binned_diameter(class, rock.diameter_m);
    let seat = RockSeat {
        xz: rock.xz,
        up,
        lift_m: height_m * (0.5 - rock.bury),
        scale_m,
        far_m: range_per_m * d,
        lattices: Lattices::at(snap, levels, rock.xz),
        version: snap.version,
        patched: levels.is_some(),
    };
    let surface = seat.lattices.height(levels, eye, rock.xz);
    let (translation, scale) = seat_transform(&seat, surface, eye, units);
    let near = NEAR_PER_M * d * units;
    let far = seat.far_m * units;
    let [near_mesh, far_mesh] = assets.meshes[rock.shape].clone();
    commands
        .spawn((
            ScatteredRock {
                diameter_m: rock.diameter_m,
                height_m,
            },
            seat,
            Transform {
                translation,
                rotation,
                scale: Vec3::splat(scale),
            },
            Visibility::default(),
        ))
        .with_children(|r| {
            r.spawn((
                Mesh3d(near_mesh),
                MeshMaterial3d(assets.material.clone()),
                VisibilityRange::abrupt(0.0, near),
            ));
            r.spawn((
                Mesh3d(far_mesh),
                MeshMaterial3d(assets.material.clone()),
                VisibilityRange::abrupt(near, far),
            ));
        })
        .id()
}

/// The rock's translation and scale (world units) on a surface at `surface_m` for an eye at
/// `eye` (metre frame): it shrinks to nothing over the last fifth of its range, about its
/// buried base, so a shrinking rock stays in the ground.
fn seat_transform(seat: &RockSeat, surface_m: f32, eye: Vec3, units: f32) -> (Vec3, f32) {
    let ground = Vec3::new(seat.xz.x, surface_m, seat.xz.y);
    let t =
        ((eye.distance(ground) - seat.far_m * (1.0 - FADE)) / (seat.far_m * FADE)).clamp(0.0, 1.0);
    // Never exactly zero: a zero scale has no inverse for the normals.
    let fade = (1.0 - t * t * (3.0 - 2.0 * t)).max(1e-3);
    (
        (ground + seat.up * seat.lift_m * fade) * units,
        seat.scale_m * fade * units,
    )
}

fn clear(commands: &mut Commands, field: &mut RockField, stats: &mut RockStats) {
    for (_, (entity, _)) in field.tiles.drain() {
        commands.entity(entity).despawn();
    }
    *stats = RockStats::default();
}

/// Seat every rock again when the camera has moved (the patches' morph and the rocks' shrink
/// follow it), when the patch levels changed or when the terrain's surface changed.
fn seat_rocks(
    view: Res<ElementsView>,
    units: Res<ElementsUnits>,
    snapshot: Res<EnvironmentSnapshot>,
    mut field: ResMut<RockField>,
    mut rocks: Query<(&mut RockSeat, &mut Transform)>,
    mut last: Local<Option<(Vec3, u32)>>,
) {
    let Some(snap) = snapshot.terrain.as_ref() else {
        return;
    };
    let eye = view.eye;
    let surface_changed = last.is_none_or(|(_, version)| version != snap.version);
    let moved = last.is_none_or(|(at, _)| at.distance(eye) > SEAT_STEP_M);
    let levels_changed = std::mem::take(&mut field.levels_changed);
    if !moved && !surface_changed && !levels_changed {
        return;
    }
    *last = Some((eye, snap.version));
    let levels = field.levels.as_ref();
    let u = units.per_metre();
    let tolerance = SEAT_TOLERANCE_M * u;
    for (mut seat, mut transform) in &mut rocks {
        // The lattices depend on the surface and the covering patch's level alone: read them
        // again only when either changed, and morph between them every pass.
        let stale = seat.version != snap.version
            || seat.patched != levels.is_some()
            || (levels_changed
                && levels.is_some_and(|l| l.level_at(seat.xz).unwrap_or(0) != seat.lattices.level));
        if stale {
            seat.lattices = Lattices::at(snap, levels, seat.xz);
            seat.version = snap.version;
            seat.patched = levels.is_some();
        }
        let surface = seat.lattices.height(levels, eye, seat.xz);
        let (translation, scale) = seat_transform(&seat, surface, eye, u);
        if transform.translation.distance(translation) > tolerance
            || (transform.scale.x - scale).abs() > tolerance
        {
            transform.translation = translation;
            transform.scale = Vec3::splat(scale);
        }
    }
}
