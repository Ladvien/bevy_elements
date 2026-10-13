// bevy_elements::noise — hashing and procedural noise shared by every domain.
//
// A supported import for consumers: `import bevy_elements::noise::{perlin3, fbm3, curl3};`.
// All functions are deterministic for a given input on a given GPU; they are not bit-identical
// across vendors.

/// PCG3D hash (Jarzynski & Olano 2020, "Hash Functions for GPU Rendering"): three 32-bit outputs.
fn pcg3d(v_in: vec3<u32>) -> vec3<u32> {
    var v = v_in * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> vec3<u32>(16u);
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return v;
}

/// Hash of an integer lattice point to three floats in [0, 1).
fn hash33(p: vec3<i32>) -> vec3<f32> {
    return vec3<f32>(pcg3d(bitcast<vec3<u32>>(p)) >> vec3<u32>(8u)) * (1.0 / 16777216.0);
}

/// Hash of an integer lattice point to one float in [0, 1).
fn hash31(p: vec3<i32>) -> f32 {
    return hash33(p).x;
}

fn quintic(t: vec3<f32>) -> vec3<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

fn gradient(cell: vec3<i32>, offset: vec3<f32>) -> f32 {
    let g = normalize(hash33(cell) * 2.0 - 1.0 + vec3<f32>(1e-6));
    return dot(g, offset);
}

/// Gradient (Perlin) noise, roughly in [-1, 1], period 1 lattice cell.
fn perlin3(p: vec3<f32>) -> f32 {
    let i = vec3<i32>(floor(p));
    let f = fract(p);
    let u = quintic(f);
    let n000 = gradient(i, f);
    let n100 = gradient(i + vec3<i32>(1, 0, 0), f - vec3<f32>(1.0, 0.0, 0.0));
    let n010 = gradient(i + vec3<i32>(0, 1, 0), f - vec3<f32>(0.0, 1.0, 0.0));
    let n110 = gradient(i + vec3<i32>(1, 1, 0), f - vec3<f32>(1.0, 1.0, 0.0));
    let n001 = gradient(i + vec3<i32>(0, 0, 1), f - vec3<f32>(0.0, 0.0, 1.0));
    let n101 = gradient(i + vec3<i32>(1, 0, 1), f - vec3<f32>(1.0, 0.0, 1.0));
    let n011 = gradient(i + vec3<i32>(0, 1, 1), f - vec3<f32>(0.0, 1.0, 1.0));
    let n111 = gradient(i + vec3<i32>(1, 1, 1), f - vec3<f32>(1.0, 1.0, 1.0));
    let x00 = mix(n000, n100, u.x);
    let x10 = mix(n010, n110, u.x);
    let x01 = mix(n001, n101, u.x);
    let x11 = mix(n011, n111, u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z) * 1.1547;
}

/// Fractal sum of `octaves` Perlin octaves, lacunarity 2, gain 0.5, normalised to roughly [-1, 1].
fn fbm3(p: vec3<f32>, octaves: u32) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var norm = 0.0;
    var q = p;
    for (var o = 0u; o < octaves; o += 1u) {
        sum += amplitude * perlin3(q);
        norm += amplitude;
        amplitude *= 0.5;
        q = q * 2.0 + vec3<f32>(17.13, 31.71, 5.37);
    }
    return sum / max(norm, 1e-6);
}

/// Worley (cellular) noise: distance to the nearest feature point, period 1 lattice cell,
/// roughly in [0, 1]. `period` > 0 wraps the lattice so the result tiles.
fn worley3(p: vec3<f32>, period: i32) -> f32 {
    let i = vec3<i32>(floor(p));
    let f = fract(p);
    var best = 1e9;
    for (var z = -1; z <= 1; z += 1) {
        for (var y = -1; y <= 1; y += 1) {
            for (var x = -1; x <= 1; x += 1) {
                let o = vec3<i32>(x, y, z);
                var c = i + o;
                if period > 0 {
                    c = ((c % period) + period) % period;
                }
                let d = vec3<f32>(o) + hash33(c) - f;
                best = min(best, dot(d, d));
            }
        }
    }
    return sqrt(best);
}

/// Divergence-free noise (Bridson, Hourihan & Nordenstam 2007): the curl of three decorrelated
/// Perlin potentials, by central differences of step `h` lattice cells.
fn curl3(p: vec3<f32>, h: f32) -> vec3<f32> {
    let a = vec3<f32>(0.0, 0.0, 0.0);
    let b = vec3<f32>(31.416, 47.853, 12.679);
    let c = vec3<f32>(-23.17, 9.31, 71.02);
    let dx = vec3<f32>(h, 0.0, 0.0);
    let dy = vec3<f32>(0.0, h, 0.0);
    let dz = vec3<f32>(0.0, 0.0, h);
    // ψ = (ψ1, ψ2, ψ3); curl ψ = (∂ψ3/∂y − ∂ψ2/∂z, ∂ψ1/∂z − ∂ψ3/∂x, ∂ψ2/∂x − ∂ψ1/∂y)
    let d3dy = perlin3(p + c + dy) - perlin3(p + c - dy);
    let d2dz = perlin3(p + b + dz) - perlin3(p + b - dz);
    let d1dz = perlin3(p + a + dz) - perlin3(p + a - dz);
    let d3dx = perlin3(p + c + dx) - perlin3(p + c - dx);
    let d2dx = perlin3(p + b + dx) - perlin3(p + b - dx);
    let d1dy = perlin3(p + a + dy) - perlin3(p + a - dy);
    return vec3<f32>(d3dy - d2dz, d1dz - d3dx, d2dx - d1dy) / (2.0 * h);
}
