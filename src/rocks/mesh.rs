//! The rocks' shapes (`docs/research/rock-scatter.md` Model 5): a block cut by a few joint planes
//! and bounded by a sphere, its edges rounded by a soft minimum (spheroidal weathering erodes
//! the edges first), roughened by a little noise, and squashed to the talus's axis ratios.
//! Twelve shapes, two families: the talus's fresh angular blocks and the meadow's rounded ones.

use bevy::asset::RenderAssetUsages;
use bevy::math::{IVec3, Quat, Vec3};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::platform::collections::HashMap;

use super::model::{SHAPES_PER_FAMILY, mix64, unit};

/// Shapes in all: the angular family first.
pub(crate) const SHAPES: usize = 2 * SHAPES_PER_FAMILY;
/// The near mesh's and the far mesh's icosphere subdivisions: 1280 and 320 triangles. The two
/// switch abruptly, so the far mesh is fine enough that its silhouette barely changes.
pub(crate) const NEAR_SUBDIVISIONS: u32 = 3;
pub(crate) const FAR_SUBDIVISIONS: u32 = 2;

/// The rock palette (`rock-palette.md`, `ROCK_TONE_*` in `common.wesl`): basalt, sandstone,
/// limestone, varnish, each a multiplier on the rock's albedo; their mean is exactly 1.
const TONES: [Vec3; 4] = [
    Vec3::new(0.848, 1.015, 1.309),
    Vec3::new(1.223, 1.083, 0.832),
    Vec3::new(1.214, 1.270, 1.256),
    Vec3::new(0.715, 0.632, 0.603),
];
/// How much of a tone a whole rock wears. The cliff mixes its palette through its beds and
/// fragments, so no large area wears one tone at full strength; a whole rock in the basalt tone
/// read as a painted blue-grey against the warm ground (`/tmp/i42/r2/crop_rocks.png`). A lerp
/// towards 1 keeps the palette's mean exactly 1.
const TONE_STRENGTH: f32 = 0.6;

/// The axis ratios each family draws from (Verdian 2021: b:a 0.57–0.64 and c:a 0.26–0.36
/// measured on talus, 0.67 and 0.45 in theory).
const B_RANGE: [f32; 2] = [0.55, 0.85];
const C_ANGULAR: [f32; 2] = [0.30, 0.45];
const C_ROUNDED: [f32; 2] = [0.38, 0.55];
/// The soft minimum's width: the edges' rounding, in units of the joint planes' distance. A
/// fresh talus block's edges are sharp; an old meadow rock's are worn round.
const ROUND_ANGULAR: f32 = 0.015;
const ROUND_ROUNDED: f32 = 0.11;
/// The bounding sphere: far outside the joint planes for a fresh block, so the planes alone make
/// it a polyhedron; at the planes' reach for a weathered rock, so its corners are worn away.
const SPHERE_ANGULAR: f32 = 1.6;
const SPHERE_ROUNDED: f32 = 1.0;
/// The joint planes' distances from the centre, and each plane's own tilt off its set.
const JOINTS_ANGULAR: [f32; 2] = [0.58, 0.80];
const JOINTS_ROUNDED: [f32; 2] = [0.72, 0.92];
const JOINT_JITTER: f32 = 0.18;
/// The chips: extra planes in any direction that knock off a corner or an edge, and their
/// distances. A fresh block has many and they cut deep, so its outline is an irregular polygon.
const CHIPS_ANGULAR: usize = 6;
const CHIPS_ROUNDED: usize = 2;
const CHIP_ANGULAR: [f32; 2] = [0.70, 0.92];
/// The surface's roughness: a relative displacement of the radius.
const ROUGH_ANGULAR: f32 = 0.035;
const ROUGH_ROUNDED: f32 = 0.05;
/// The base's darkening (the contact): the colour at the buried line, rising to 1 over this
/// share of the height above the lowest point.
const BASE_DARK: f32 = 0.55;
const BASE_RISE: [f32; 2] = [0.18, 0.55];

/// What the placement needs of a shape: its family, its axes (the long one is 1) and its tone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShapeInfo {
    pub angular: bool,
    /// The middle axis over the long one.
    pub b: f32,
    /// The short (vertical) axis over the long one: the rock's height for a length of 1.
    pub c: f32,
}

/// One shape's geometry: positions centred on the bounding box (long axis along X, 1 long;
/// height along Y, `c` tall), outward normals, vertex colours (tone, mottle, the base's
/// darkening) and triangles.
pub(crate) struct RockGeometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colours: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub info: ShapeInfo,
}

/// An icosphere: the icosahedron's 12 vertices, each triangle split into four `subdivisions`
/// times, every vertex on the unit sphere and shared (no seams).
pub(crate) fn icosphere(subdivisions: u32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let mut v: Vec<Vec3> = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .iter()
    .map(|p| Vec3::from_array(*p).normalize())
    .collect();
    let mut faces: Vec<[u32; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..subdivisions {
        let mut midpoints: HashMap<(u32, u32), u32> = HashMap::default();
        let mut midpoint = |a: u32, b: u32, v: &mut Vec<Vec3>| -> u32 {
            let key = (a.min(b), a.max(b));
            *midpoints.entry(key).or_insert_with(|| {
                v.push(((v[a as usize] + v[b as usize]) * 0.5).normalize());
                (v.len() - 1) as u32
            })
        };
        let mut next = Vec::with_capacity(faces.len() * 4);
        for [a, b, c] in faces {
            let ab = midpoint(a, b, &mut v);
            let bc = midpoint(b, c, &mut v);
            let ca = midpoint(c, a, &mut v);
            next.extend_from_slice(&[[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = next;
    }
    (v, faces)
}

/// A smooth 3D value noise, 0–1.
fn value_noise3(p: Vec3, seed: u64) -> f32 {
    let base = p.floor();
    let f = p - base;
    let w = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let c = base.as_ivec3();
    let v = |d: IVec3| {
        let q = c + d;
        unit(mix64(
            seed ^ (q.x as i64 as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
                ^ (q.y as i64 as u64).wrapping_mul(0xA076_1D64_78BD_642F)
                ^ (q.z as i64 as u64).wrapping_mul(0xE703_7ED1_A0B4_28DB),
        ))
    };
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = lerp(v(IVec3::new(0, 0, 0)), v(IVec3::new(1, 0, 0)), w.x);
    let x10 = lerp(v(IVec3::new(0, 1, 0)), v(IVec3::new(1, 1, 0)), w.x);
    let x01 = lerp(v(IVec3::new(0, 0, 1)), v(IVec3::new(1, 0, 1)), w.x);
    let x11 = lerp(v(IVec3::new(0, 1, 1)), v(IVec3::new(1, 1, 1)), w.x);
    lerp(lerp(x00, x10, w.y), lerp(x01, x11, w.y), w.z)
}

/// Two octaves of [`value_noise3`], 0–1.
fn fbm3(p: Vec3, seed: u64) -> f32 {
    (2.0 * value_noise3(p, seed) + value_noise3(p * 2.07 + 11.3, seed ^ 0x3F)) / 3.0
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The geometry of shape `shape` (0..[`SHAPES`]) on an icosphere of `subdivisions`. The same
/// shape at two subdivisions has the same silhouette: the near and far meshes of one rock.
pub(crate) fn rock_geometry(shape: usize, subdivisions: u32) -> RockGeometry {
    let seed = mix64(0x5EED_0000 + shape as u64);
    let draw = |k: u64| unit(mix64(seed ^ k.wrapping_mul(0x2545_F491_4F6C_DD1D)));
    let range = |r: [f32; 2], k: u64| r[0] + (r[1] - r[0]) * draw(k);
    let angular = shape < SHAPES_PER_FAMILY;
    let (joints, round, rough, c_range, sphere, chips, chip) = if angular {
        (
            JOINTS_ANGULAR,
            ROUND_ANGULAR,
            ROUGH_ANGULAR,
            C_ANGULAR,
            SPHERE_ANGULAR,
            CHIPS_ANGULAR,
            CHIP_ANGULAR,
        )
    } else {
        (
            JOINTS_ROUNDED,
            ROUND_ROUNDED,
            ROUGH_ROUNDED,
            C_ROUNDED,
            SPHERE_ROUNDED,
            CHIPS_ROUNDED,
            [JOINTS_ROUNDED[1], JOINTS_ROUNDED[1] + 0.1],
        )
    };
    // The joint planes: three sets, near-orthogonal (a block breaks along joint sets), turned by
    // up to 20° so no two shapes share a frame, each plane tilted a little off its set and at its
    // own distance.
    let turn = Quat::from_axis_angle(
        Vec3::new(draw(1) - 0.5, draw(2) - 0.5, draw(3) - 0.5).normalize_or(Vec3::Y),
        (draw(4) - 0.5) * 0.7,
    );
    let unit_vector = |k: u64| {
        let z = draw(k) * 2.0 - 1.0;
        let phi = draw(k + 1) * std::f32::consts::TAU;
        let r = (1.0 - z * z).max(0.0).sqrt();
        Vec3::new(r * phi.cos(), z, r * phi.sin())
    };
    let mut planes: Vec<(Vec3, f32)> = [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ]
    .iter()
    .enumerate()
    .map(|(i, axis)| {
        let i = i as u64;
        let n = (turn * *axis + unit_vector(60 + 2 * i) * JOINT_JITTER).normalize();
        (n, range(joints, 10 + i))
    })
    .collect();
    for i in 0..chips as u64 {
        planes.push((unit_vector(20 + 2 * i), range(chip, 40 + i)));
    }
    let (directions, faces) = icosphere(subdivisions);
    // The radius along each direction: the soft minimum of the bounding sphere and every plane
    // facing it, `min − w·ln Σ exp(−(t − min)/w)`, then the roughness. The surface's own normal is the
    // same soft minimum's blend of the planes' normals (the sphere's is the direction): exactly a
    // plane's on a face, turning only within the rounding's width at an edge.
    let (mut points, mut analytic): (Vec<Vec3>, Vec<Vec3>) = directions
        .iter()
        .map(|&d| {
            let mut terms = vec![(sphere, d)];
            for &(n, offset) in &planes {
                let facing = d.dot(n);
                if facing > 1e-3 {
                    terms.push((offset / facing, n));
                }
            }
            let min = terms.iter().map(|t| t.0).fold(f32::MAX, f32::min);
            let mut sum = 0.0;
            let mut normal = Vec3::ZERO;
            for &(t, n) in &terms {
                let w = (-(t - min) / round).exp();
                sum += w;
                normal += n * w;
            }
            let r = min - round * sum.ln();
            let lumps = 1.0 + rough * (fbm3(d * 2.2, seed) - 0.5) * 2.0;
            (d * r * lumps, normal.normalize_or(d))
        })
        .unzip();
    // Squash to the drawn axis ratios: the long axis along X, 1 long; the middle along Z; the
    // short one up, so the rock lies on its largest face.
    let (lo, hi) = points.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
        (lo.min(*p), hi.max(*p))
    });
    let centre = (lo + hi) * 0.5;
    let extent = (hi - lo).max(Vec3::splat(1e-3));
    let b = range(B_RANGE, 50);
    let c = range(c_range, 51).min(b * 0.9);
    let scale = Vec3::new(1.0 / extent.x, c / extent.y, b / extent.z);
    for p in &mut points {
        *p = (*p - centre) * scale;
    }
    // A normal under a scale goes by its inverse.
    for n in &mut analytic {
        *n = (*n / scale).normalize_or(Vec3::Y);
    }
    // The geometric normals (area-weighted) carry the roughness's lumps; the analytic ones keep
    // the faces flat and the edges crisp. Weighted two to one towards the analytic.
    let mut normals = vec![Vec3::ZERO; points.len()];
    for &[i, j, k] in &faces {
        let (a, b, c) = (points[i as usize], points[j as usize], points[k as usize]);
        let n = (b - a).cross(c - a);
        normals[i as usize] += n;
        normals[j as usize] += n;
        normals[k as usize] += n;
    }
    for (n, a) in normals.iter_mut().zip(&analytic) {
        *n = 2.0 * *a + n.normalize_or(*a);
    }
    // Colour: the shape's tone, a mottle, and the base's darkening above the buried line.
    let tone = Vec3::ONE.lerp(TONES[shape % TONES.len()], TONE_STRENGTH);
    let colours = points
        .iter()
        .map(|p| {
            let mottle = 0.88 + 0.24 * fbm3(*p * 3.0 + 5.0, seed ^ 0xC0);
            let height = (p.y + c * 0.5) / c;
            let base =
                BASE_DARK + (1.0 - BASE_DARK) * smoothstep(BASE_RISE[0], BASE_RISE[1], height);
            let rgb = tone * mottle * base;
            [rgb.x, rgb.y, rgb.z, 1.0]
        })
        .collect();
    RockGeometry {
        positions: points.iter().map(|p| p.to_array()).collect(),
        normals: normals
            .iter()
            .map(|n| n.normalize_or(Vec3::Y).to_array())
            .collect(),
        colours,
        indices: faces.iter().flatten().copied().collect(),
        info: ShapeInfo { angular, b, c },
    }
}

/// The geometry as a mesh (kept in the main world too, so a game can build a collider from it).
pub(crate) fn rock_mesh(geometry: RockGeometry) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, geometry.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, geometry.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, geometry.colours)
    .with_inserted_indices(Indices::U32(geometry.indices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shapes_are_slabs_and_blocks_not_spheres() {
        for shape in 0..SHAPES {
            let g = rock_geometry(shape, NEAR_SUBDIVISIONS);
            let (lo, hi) = g
                .positions
                .iter()
                .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
                    let p = Vec3::from_array(*p);
                    (lo.min(p), hi.max(p))
                });
            let e = hi - lo;
            assert!((e.x - 1.0).abs() < 1e-3, "shape {shape}: long axis {}", e.x);
            assert!(
                (0.55..=0.85).contains(&(e.z / e.x)),
                "shape {shape}: b:a {}",
                e.z
            );
            assert!(
                (0.30..=0.55).contains(&(e.y / e.x)),
                "shape {shape}: c:a {}",
                e.y
            );
            assert!(e.y < e.z, "shape {shape} does not lie on its largest face");
            // The placement's axes are the mesh's.
            assert!((g.info.b - e.z).abs() < 1e-4 && (g.info.c - e.y).abs() < 1e-4);
        }
    }

    #[test]
    fn the_meshes_are_closed_and_face_out() {
        for shape in 0..SHAPES {
            for subdivisions in [FAR_SUBDIVISIONS, NEAR_SUBDIVISIONS] {
                let g = rock_geometry(shape, subdivisions);
                let p = |i: u32| Vec3::from_array(g.positions[i as usize]);
                // The divergence theorem: the signed volume is positive only for outward faces.
                let volume: f32 = g
                    .indices
                    .chunks(3)
                    .map(|t| p(t[0]).dot(p(t[1]).cross(p(t[2]))) / 6.0)
                    .sum();
                assert!(
                    volume > 0.0,
                    "shape {shape} at {subdivisions}: volume {volume}"
                );
                // Every vertex normal points away from the centre.
                for (q, n) in g.positions.iter().zip(&g.normals) {
                    assert!(Vec3::from_array(*q).dot(Vec3::from_array(*n)) > 0.0);
                }
            }
        }
    }

    #[test]
    fn the_angular_family_has_flat_faces_and_the_rounded_one_does_not() {
        // A flat face is a run of parallel normals: a shape's "flatness" is the share of its
        // neighbouring vertices whose normals agree to 3°, high on a faceted block and low on a
        // rounded one.
        let flatness = |shape: usize| {
            let g = rock_geometry(shape, NEAR_SUBDIVISIONS);
            let mut agree = 0;
            let mut total = 0;
            for t in g.indices.chunks(3) {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    let na = Vec3::from_array(g.normals[a as usize]);
                    let nb = Vec3::from_array(g.normals[b as usize]);
                    total += 1;
                    if na.dot(nb) > 3f32.to_radians().cos() {
                        agree += 1;
                    }
                }
            }
            agree as f32 / total as f32
        };
        let angular: f32 =
            (0..SHAPES_PER_FAMILY).map(flatness).sum::<f32>() / SHAPES_PER_FAMILY as f32;
        let rounded: f32 =
            (SHAPES_PER_FAMILY..SHAPES).map(flatness).sum::<f32>() / SHAPES_PER_FAMILY as f32;
        assert!(
            angular > rounded + 0.1,
            "angular {angular} vs rounded {rounded}"
        );
    }

    #[test]
    fn the_palette_is_the_rocks_mean() {
        // Every tone is used by the same number of shapes, so the rocks' mean colour is the
        // rock's albedo.
        let mean = (0..SHAPES).map(|s| TONES[s % TONES.len()]).sum::<Vec3>() / SHAPES as f32;
        assert!((mean - Vec3::ONE).abs().max_element() < 1e-3, "{mean}");
    }
}
