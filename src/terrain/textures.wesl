// The ground's detail textures (docs/research/ground-textures.md): one ground layer's maps in a
// texture array, sampled without visible repetition by hex-tiling (Mikkelsen 2022, "Practical
// Real-Time Hex-Tiling", JCGT 11(3)) and decoded into an albedo multiplier, a height, a slope, a
// roughness multiplier and an occlusion.
//
// Layer `2k` of the array is the albedo detail (RGB, `value * 64 / 255`) and the height (A);
// layer `2k + 1` is the tangent normal's u and v (RG, `(c * 127 + 128) / 255`), the roughness
// detail (B, `value * 128 / 255`) and the occlusion (A). u runs along world X and v along world Z.

import bevy_elements::noise::hash33;

// The hex grid's scale in texture tiles: Mikkelsen's `2√3`, so a hex spans about a third of the
// texture and three taps from three places in it meet at every point.
const HEX_SCALE: f32 = 3.4641016;
// How much a tap's height wins the blend between the hexes, 0–1 (Mikkelsen's fall-off contrast
// β, on the height rather than the luminance): the taller grain is in front at a hex's edge.
const HEX_CONTRAST: f32 = 0.6;
const TAU: f32 = 6.2831853;
// The decoded albedo multiplier of a stored 64, and the roughness multiplier of a stored 128:
// exactly one.
const ALBEDO_DECODE: f32 = 255.0 / 64.0;
const ROUGH_DECODE: f32 = 255.0 / 128.0;
// The steepest slope a detail normal may give, so a grazing texel cannot fold the normal over.
const MAX_SLOPE: f32 = 2.0;

// The three hex-grid vertices around a point and their barycentric weights.
struct HexGrid {
    v1: vec2<i32>,
    v2: vec2<i32>,
    v3: vec2<i32>,
    w: vec3<f32>,
}

// One tap: its coordinate in the texture and its rotation.
struct HexTap {
    st: vec2<f32>,
    rot: mat2x2<f32>,
}

// One ground layer's maps at a point, decoded.
struct LayerMaps {
    // The albedo multiplier, mean 1.
    albedo: vec3<f32>,
    // The relief's height, 0–1, mean 0.5.
    height: f32,
    // The relief's slope in the world, (∂h/∂x, ∂h/∂z).
    slope: vec2<f32>,
    // The roughness multiplier, mean 1.
    rough: f32,
    // The occlusion of the ambient light, 0–1.
    occlusion: f32,
}

// The triangle of the hex grid's vertices around `st` (texture tiles), Mikkelsen's
// `TriangleGrid`: the point is skewed into a simplex grid, and its barycentric weights are the
// weights of the three hexes that meet there.
fn hex_grid(st: vec2<f32>) -> HexGrid {
    let s = st * HEX_SCALE;
    let skewed = vec2<f32>(s.x - 0.57735027 * s.y, 1.15470054 * s.y);
    let base = vec2<i32>(floor(skewed));
    let f = skewed - floor(skewed);
    let z = 1.0 - f.x - f.y;
    let up = select(0.0, 1.0, z <= 0.0);
    let flip = 2.0 * up - 1.0;
    let u = i32(up);
    var g: HexGrid;
    g.w = vec3<f32>(-z * flip, up - f.y * flip, up - f.x * flip);
    g.v1 = base + vec2<i32>(u, u);
    g.v2 = base + vec2<i32>(u, 1 - u);
    g.v3 = base + vec2<i32>(1 - u, u);
    return g;
}

// A hex vertex's centre in texture tiles (Mikkelsen's `MakeCenST`).
fn hex_centre(v: vec2<i32>) -> vec2<f32> {
    let p = vec2<f32>(v);
    return vec2<f32>(p.x + 0.5 * p.y, 0.8660254 * p.y) / HEX_SCALE;
}

// The tap of the hex at vertex `v` for ground layer `layer`: the texture turned about the hex's
// centre by a random angle and moved by a random offset, so no two hexes show the same piece of
// it the same way round.
fn hex_tap(v: vec2<i32>, layer: u32, st: vec2<f32>) -> HexTap {
    let h = hash33(vec3<i32>(v, i32(layer)));
    let angle = (h.z - 0.5) * TAU;
    let c = cos(angle);
    let s = sin(angle);
    var tap: HexTap;
    tap.rot = mat2x2<f32>(c, s, -s, c);
    let centre = hex_centre(v);
    tap.st = tap.rot * (st - centre) + centre + h.xy;
    return tap;
}

// A tap's normal map texel as a slope in the world: the tangent normal's (u, v) over its
// height, turned back by the tap's rotation (the slope is a gradient, so it turns by the
// transpose).
fn tap_slope(texel: vec4<f32>, rot: mat2x2<f32>) -> vec2<f32> {
    let n = (texel.rg * 255.0 - vec2<f32>(128.0)) / 127.0;
    let nz = sqrt(max(1.0 - dot(n, n), 0.0));
    let slope = -n / max(nz, 1.0 / MAX_SLOPE);
    return transpose(rot) * slope;
}

// Ground layer `layer`'s detail at `st` (texture tiles), with the coordinate's screen
// derivatives `dx` and `dy` (texture tiles a pixel): three hex taps of each of the layer's two
// array layers, weighted by Mikkelsen's `w^γ` (γ = 7) times the tap's height, so the hexes'
// seams are narrow and the taller grain is in front.
fn layer_maps(
    maps: texture_2d_array<f32>,
    maps_sampler: sampler,
    layer: u32,
    st: vec2<f32>,
    dx: vec2<f32>,
    dy: vec2<f32>,
) -> LayerMaps {
    let g = hex_grid(st);
    let t1 = hex_tap(g.v1, layer, st);
    let t2 = hex_tap(g.v2, layer, st);
    let t3 = hex_tap(g.v3, layer, st);
    let albedo_layer = i32(2u * layer);
    let a1 = textureSampleGrad(maps, maps_sampler, t1.st, albedo_layer, t1.rot * dx, t1.rot * dy);
    let a2 = textureSampleGrad(maps, maps_sampler, t2.st, albedo_layer, t2.rot * dx, t2.rot * dy);
    let a3 = textureSampleGrad(maps, maps_sampler, t3.st, albedo_layer, t3.rot * dx, t3.rot * dy);
    // w⁷ by multiplication: a `pow` with a computed exponent fails silently in WESL.
    let w2 = g.w * g.w;
    let w4 = w2 * w2;
    let heights = vec3<f32>(a1.a, a2.a, a3.a);
    let weighted = (w4 * w2 * g.w) * mix(vec3<f32>(1.0), 2.0 * heights, HEX_CONTRAST);
    let tap_w = weighted / max(weighted.x + weighted.y + weighted.z, 1e-12);
    let surface_layer = albedo_layer + 1;
    let b1 = textureSampleGrad(maps, maps_sampler, t1.st, surface_layer, t1.rot * dx, t1.rot * dy);
    let b2 = textureSampleGrad(maps, maps_sampler, t2.st, surface_layer, t2.rot * dx, t2.rot * dy);
    let b3 = textureSampleGrad(maps, maps_sampler, t3.st, surface_layer, t3.rot * dx, t3.rot * dy);
    var d: LayerMaps;
    d.albedo = (tap_w.x * a1.rgb + tap_w.y * a2.rgb + tap_w.z * a3.rgb) * ALBEDO_DECODE;
    d.height = dot(tap_w, heights);
    d.slope = tap_w.x * tap_slope(b1, t1.rot) + tap_w.y * tap_slope(b2, t2.rot) + tap_w.z * tap_slope(b3, t3.rot);
    d.rough = dot(tap_w, vec3<f32>(b1.b, b2.b, b3.b)) * ROUGH_DECODE;
    d.occlusion = dot(tap_w, vec3<f32>(b1.a, b2.a, b3.a));
    return d;
}

// How sharply the ground layers' heights decide their blend: a weight is scaled by
// `exp(HEIGHT_BLEND · (h − ½))`, so where two layers meet, the taller texel's layer shows
// (grass blades standing out of the soil) instead of a cross-fade.
const HEIGHT_BLEND: f32 = 8.0;
// A ground layer's textures are sampled only above this weight.
const DETAIL_GATE: f32 = 0.01;

// The ground layers' weights `w` (sand, soil, grass) blended by their heights `h`. The weights
// enter less the gate, so a layer's share is continuous where its sampling starts; their sum is
// unchanged, and equal heights change nothing but the gate's own share.
fn height_blend(w: vec3<f32>, h: vec3<f32>) -> vec3<f32> {
    let total = w.x + w.y + w.z;
    let e = max(w - vec3<f32>(DETAIL_GATE), vec3<f32>(0.0)) * exp(HEIGHT_BLEND * (h - vec3<f32>(0.5)));
    let sum = e.x + e.y + e.z;
    if sum <= 0.0 {
        return w;
    }
    return e * (total / sum);
}
