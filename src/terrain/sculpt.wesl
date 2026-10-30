// The terrain's hand edit: one brush stroke over the bedrock layer (docs/research/terrain-sculpt.md).
//
//   terrain_sculpt   per cell inside the brush, bedrock += delta(kind, weight)
//
// `cells` is the *previous* pack (surface height in `.x`, m above the terrain's y0), read only, so
// the smooth and flatten kinds have no read-after-write race on the layer they write. The pack
// runs after this kernel in the same compute pass and carries the edit to everything that reads it.

import bevy_elements::grid::{GridParams, grid_index};
import bevy_elements::noise::perlin3;

struct SculptParams {
    grid: GridParams,
    // xy: brush centre, world XZ (m). z: radius (m). w: hardness, the fraction of the radius at full weight.
    area: vec4<f32>,
    // x: kind (0 raise, 1 lower, 2 smooth, 3 flatten, 4 noise). y: amount (raise, lower: m; smooth,
    // flatten: blend; noise: amplitude, m). z: flatten target above y0 (m) | noise feature size (m).
    // w: noise seed.
    shape: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: SculptParams;
@group(0) @binding(1) var<storage, read_write> bedrock: array<f32>;
@group(0) @binding(2) var<storage, read> cells: array<vec4<f32>>;

/// The brush weight at distance `d` from the centre: 1 inside the hardness plateau, a smoothstep
/// down to 0 at the radius, 0 beyond. Mirrored by `sculpt_weight` in `sculpt.rs`.
fn sculpt_weight(d: f32, r: f32, hardness: f32) -> f32 {
    let t = clamp((d / r - hardness) / (1.0 - hardness), 0.0, 1.0);
    let s = 1.0 - t;
    return s * s * (3.0 - 2.0 * s);
}

fn surface(c: vec2<i32>) -> f32 {
    return cells[grid_index(params.grid, c)].x;
}

@compute @workgroup_size(8, 8, 1)
fn terrain_sculpt(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = params.grid.size;
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let c = vec2<i32>(id.xy);
    let p = params.grid.origin + (vec2<f32>(id.xy) + 0.5) * params.grid.cell;
    let d = length(p - params.area.xy);
    if d >= params.area.z {
        return;
    }
    let weight = sculpt_weight(d, params.area.z, params.area.w);
    let k = id.y * size.x + id.x;
    let kind = u32(params.shape.x + 0.5);
    let amount = params.shape.y;
    var delta = 0.0;
    if kind == 0u {
        delta = amount * weight;
    } else if kind == 1u {
        delta = -amount * weight;
    } else if kind == 2u || kind == 3u {
        let here = surface(c);
        if kind == 2u {
            var sum = 0.0;
            for (var j = -1; j <= 1; j += 1) {
                for (var i = -1; i <= 1; i += 1) {
                    if i != 0 || j != 0 {
                        sum += surface(c + vec2<i32>(i, j));
                    }
                }
            }
            delta = amount * weight * (sum / 8.0 - here);
        } else {
            delta = amount * weight * (params.shape.z - here);
        }
    } else {
        let q = vec3<f32>(p / params.shape.z, params.shape.w);
        let n = (perlin3(q) + 0.5 * perlin3(q * 2.0 + vec3<f32>(5.2, 1.3, 0.0)) + 0.25 * perlin3(q * 4.0 + vec3<f32>(9.7, 8.3, 0.0))) / 1.75;
        delta = amount * weight * n;
    }
    bedrock[k] += delta;
}
