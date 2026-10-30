//! The crate's WESL validator: compiles every shader module to WGSL and parses the result with
//! naga, without a GPU and in a second.
//!
//! A WESL error is **silent at build time**: the shaders are embedded with `embedded_asset!`, so
//! a struct constructed with the wrong number of fields, a name used from another module
//! without an import, a local that shadows another local, or a `pow(x, p)` with `p` in a `let`
//! all compile, are embedded, and then fail in the GPU driver at run time. The symptom is a
//! surface that renders as a flat dark grey, and the only clue is a `failed to process shader`
//! line in the run's log — which a capture script throws away.
//!
//! ```sh
//! cargo run --release --manifest-path tools/wesl-check/Cargo.toml -- src
//! ```
//!
//! ```sh
//! cargo run --release --manifest-path tools/wesl-check/Cargo.toml -- src
//! ```
//!
//! The path is the crate's `src` directory, where the modules live. Bevy's own shader modules
//! are not on disk, so they are stubbed with the *interface* the crate's shaders use (Bevy's own
//! struct fields and function signatures), so a wrong call into Bevy fails here and not in the
//! driver. A Bevy module the shaders import without a stub is a failure, never "not checked".
//!
//! # Why the stubs are mounted the way they are
//!
//! A WESL import's path is not the text that was written. `import bevy_pbr::render::view;` inside
//! a module of the crate's own package arrives at the resolver as
//! `PathOrigin::Package("bevy_elements/bevy_pbr")` with components `["render", "view"]`: WESL
//! joins the import against the importing module's path, and a `Package` joined onto a `Package`
//! takes the sub-package form `parent/child`, a spelling no source file can write (see
//! `join_path` in `wgsl-parse`). A router dispatches on an *origin equality*, so each Bevy
//! namespace is mounted twice, as itself and as `bevy_elements/<it>`, and the virtual modules
//! inside a mount are keyed by `Absolute` paths, because that is the form the router hands on.

use std::borrow::Cow;
use std::path::Path;

use wesl::error::ResolveError;
use wesl::resolver::{FileResolver, Resolver, Router, VirtualResolver};
use wgsl_parse::syntax::{ModulePath, PathOrigin};

/// The crate's own modules. Every one of them: a WESL error is silent at build time, so a
/// module nobody compiles is a module that fails in the GPU driver.
const MODULES: &[&str] = &[
    "bevy_elements::celestial::moon",
    "bevy_elements::celestial::stars",
    "bevy_elements::climate::snow",
    "bevy_elements::clouds::common",
    "bevy_elements::clouds::composite",
    "bevy_elements::clouds::generate",
    "bevy_elements::clouds::march",
    "bevy_elements::clouds::resolve",
    "bevy_elements::clouds::shadow",
    "bevy_elements::clouds::weather",
    "bevy_elements::cover::common",
    "bevy_elements::cover::readback",
    "bevy_elements::cover::shelter",
    "bevy_elements::cover::snow",
    "bevy_elements::dust::aeolian",
    "bevy_elements::dust::surface_wind",
    "bevy_elements::fog::density",
    "bevy_elements::grid",
    "bevy_elements::heat_haze::haze",
    "bevy_elements::hydrology::readback",
    "bevy_elements::hydrology::source_map",
    "bevy_elements::hydrology::substep",
    "bevy_elements::hydrology::thermal",
    "bevy_elements::hydrology::water",
    "bevy_elements::hydrology::water_normals",
    "bevy_elements::noise",
    "bevy_elements::paths",
    "bevy_elements::precipitation::common",
    "bevy_elements::precipitation::draw",
    "bevy_elements::precipitation::precip",
    "bevy_elements::surface_cover",
    "bevy_elements::terrain::common",
    "bevy_elements::terrain::flow",
    "bevy_elements::terrain::generate",
    "bevy_elements::terrain::ground",
    "bevy_elements::terrain::heightmap",
    "bevy_elements::terrain::pack",
    "bevy_elements::terrain::render",
    "bevy_elements::terrain::textures",
    "bevy_elements::trails::surface",
    "bevy_elements::trails::trails",
    "bevy_elements::waves::waves",
    "bevy_elements::weathered::weathered",
    "bevy_elements::wind",
    "bevy_elements::wind::sim",
];

/// The Bevy namespaces the crate's shaders import from.
const BEVY_NAMESPACES: &[&str] = &["bevy_pbr", "bevy_render", "bevy_core_pipeline", "constants"];

/// The only error that is not the crate's shaders' fault: a module that *re-exports* another
/// module's `package`-visible item, which Bevy's own loader resolves the other way.
const BENIGN: &[&str] = &["is declared with `package` visibility"];

/// Bevy's own shader modules, stubbed. Every module the crate's shaders import has a stub in
/// `stub_source`; an import without one fails the check (`grep -rh "^import bevy_" src`).
const BEVY_MODULES: &[&str] = &[
    "bevy_core_pipeline::fullscreen_vertex_shader",
    "bevy_pbr::decal::clustered",
    "bevy_pbr::deferred::functions",
    "bevy_pbr::prepass::io",
    "bevy_pbr::prepass::utils",
    "bevy_pbr::render::forward_io",
    "bevy_pbr::render::mesh_functions",
    "bevy_pbr::render::mesh_view_bindings",
    "bevy_pbr::render::pbr_fragment",
    "bevy_pbr::render::pbr_functions",
    "bevy_pbr::render::pbr_types",
    "bevy_pbr::render::utils",
    "bevy_pbr::render::view_transformations",
    "bevy_render::view",
];

/// The path a virtual module is registered under: `Absolute` components, the form the router
/// strips a mount's prefix down to.
fn virtual_path(name: &str) -> ModulePath {
    ModulePath::new(PathOrigin::Absolute, name.split("::").map(str::to_string).collect())
}

/// Bevy's `View` uniform (`bevy_render-0.20.0/src/view.wesl`), whole: it is declared in
/// `bevy_render::view` and bound in `bevy_pbr::render::mesh_view_bindings`.
const VIEW: &str = "\
struct ColorGrading {
    balance: mat3x3<f32>,
    saturation: vec3<f32>,
    contrast: vec3<f32>,
    gamma: vec3<f32>,
    gain: vec3<f32>,
    lift: vec3<f32>,
    midtone_range: vec2<f32>,
    exposure: f32,
    hue: f32,
    post_saturation: f32,
}
struct View {
    clip_from_world: mat4x4<f32>,
    unjittered_clip_from_world: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    view_from_clip: mat4x4<f32>,
    world_position: vec3<f32>,
    exposure: f32,
    viewport: vec4<f32>,
    main_pass_viewport: vec4<f32>,
    frustum: array<vec4<f32>, 6>,
    lod_view_world_position: vec3<f32>,
    color_grading: ColorGrading,
    mip_bias: f32,
    frame_count: u32,
};
";

/// A stub's source: the declarations the crate's shaders reach for, with Bevy 0.20's own
/// signatures (`bevy_pbr-0.20.0/src`, `bevy_render-0.20.0/src`).
///
/// A stub is not Bevy's implementation: it is the shape of the interface, so that a call with the
/// wrong arity, a field that does not exist, a wrong type or an unimported name fails here and not
/// in the GPU driver. Bodies return placeholders. Bevy's `@if` fields are declared unconditionally
/// (the crate's own `@if` blocks decide which pass reads them). The struct fields are Bevy's in
/// full except `DirectionalLight` and `Lights`, which omit the cascade and rect-light arrays that
/// need Bevy's `constants` sizes and that no shader of the crate reads.
fn stub_source(name: &str) -> String {
    let source = match name {
        "constants" => "const MATERIAL_BIND_GROUP: u32 = 1u;\nconst MESH_BIND_GROUP: u32 = 0u;\n".to_string(),
        // `bevy_pbr::render` exists for its children, which the shaders import through it.
        "bevy_pbr::render" => "\
import bevy_pbr::render::forward_io;
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::mesh_view_bindings;
import bevy_pbr::render::pbr_fragment;
import bevy_pbr::render::pbr_functions;
import bevy_pbr::render::pbr_types;
import bevy_pbr::render::utils;
import bevy_pbr::render::view_transformations;
"
        .to_string(),
        "bevy_render::view" => VIEW.to_string(),
        "bevy_core_pipeline::fullscreen_vertex_shader" => "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
"
        .to_string(),
        "bevy_pbr::render::forward_io" => "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv_b: vec2<f32>,
    @location(4) world_tangent: vec4<f32>,
    @location(5) color: vec4<f32>,
    @location(6) @interpolate(flat) instance_index: u32,
    @location(7) @interpolate(flat) visibility_range_dither: i32,
}
struct FragmentOutput {
    @location(0) color: vec4<f32>,
}
"
        .to_string(),
        "bevy_pbr::prepass::io" => "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) uv_b: vec2<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) world_tangent: vec4<f32>,
    @location(4) world_position: vec4<f32>,
    @location(5) previous_world_position: vec4<f32>,
    @location(6) unclipped_depth: f32,
    @location(7) @interpolate(flat) instance_index: u32,
    @location(8) color: vec4<f32>,
}
struct FragmentOutput {
    @location(0) normal: vec4<f32>,
    @location(1) motion_vector: vec2<f32>,
    @location(2) deferred: vec4<u32>,
    @location(3) deferred_lighting_pass_id: u32,
    @builtin(frag_depth) frag_depth: f32,
}
"
        .to_string(),
        "bevy_pbr::prepass::utils" => "\
fn prepass_depth(frag_coord: vec4<f32>, sample_index: u32) -> f32 {
    return frag_coord.z;
}
"
        .to_string(),
        "bevy_pbr::deferred::functions" => "\
import bevy_pbr::prepass::io::{VertexOutput, FragmentOutput};
import bevy_pbr::render::pbr_types::PbrInput;
fn deferred_output(in: VertexOutput, pbr_input: PbrInput) -> FragmentOutput {
    var out: FragmentOutput;
    return out;
}
"
        .to_string(),
        "bevy_pbr::decal::clustered" => "\
import bevy_pbr::render::pbr_types::PbrInput;
fn apply_decals(pbr_input: ptr<function, PbrInput>) {
}
"
        .to_string(),
        "bevy_pbr::render::mesh_functions" => "\
fn get_world_from_local(instance_index: u32) -> mat4x4<f32> {
    return mat4x4<f32>();
}
fn get_tag(instance_index: u32) -> u32 {
    return 0u;
}
"
        .to_string(),
        "bevy_pbr::render::mesh_view_bindings" => format!(
            "{VIEW}\
struct DirectionalLight {{
    color: vec4<f32>,
    direction_to_light: vec3<f32>,
    flags: u32,
    soft_shadow_size: f32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    num_cascades: u32,
    cascades_overlap_proportion: f32,
    depth_texture_base_index: u32,
    decal_index: u32,
    sun_disk_angular_size: f32,
    sun_disk_intensity: f32,
}};
struct Lights {{
    directional_lights: array<DirectionalLight, 10u>,
    ambient_color: vec4<f32>,
    cluster_dimensions: vec4<u32>,
    cluster_factors: vec4<f32>,
    n_directional_lights: u32,
    spot_light_shadowmap_offset: i32,
    ambient_light_flags: u32,
    n_rect_lights: u32,
}};
struct Globals {{
    time: f32,
    delta_time: f32,
    frame_count: u32,
}};
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> lights: Lights;
@group(0) @binding(9) var<uniform> globals: Globals;
"
        ),
        "bevy_pbr::render::pbr_types" => "\
struct StandardMaterial {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    attenuation_color: vec4<f32>,
    uv_transform: mat3x3<f32>,
    reflectance: vec3<f32>,
    perceptual_roughness: f32,
    metallic: f32,
    diffuse_transmission: f32,
    specular_transmission: f32,
    thickness: f32,
    ior: f32,
    attenuation_distance: f32,
    clearcoat: f32,
    clearcoat_perceptual_roughness: f32,
    anisotropy_strength: f32,
    anisotropy_rotation: vec2<f32>,
    flags: u32,
    alpha_cutoff: f32,
    parallax_depth_scale: f32,
    max_parallax_layer_count: f32,
    lightmap_exposure: f32,
    max_relief_mapping_search_steps: u32,
    deferred_lighting_pass_id: u32,
};
const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32 = 1u << 5u;
const STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT: u32 = 1u << 8u;
struct PbrInput {
    material: StandardMaterial,
    diffuse_occlusion: vec3<f32>,
    specular_occlusion: f32,
    frag_coord: vec4<f32>,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    lightmap_light: vec3<f32>,
    clearcoat_N: vec3<f32>,
    anisotropy_strength: f32,
    anisotropy_T: vec3<f32>,
    anisotropy_B: vec3<f32>,
    is_orthographic: bool,
    flags: u32,
    directional_shadow_factor: f32,
};
"
        .to_string(),
        "bevy_pbr::render::pbr_fragment" => "\
import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::pbr_types::PbrInput;
fn pbr_input_from_standard_material(in: VertexOutput, is_front: bool) -> PbrInput {
    var pbr_input: PbrInput;
    return pbr_input;
}
"
        .to_string(),
        "bevy_pbr::render::pbr_functions" => "\
import bevy_pbr::render::pbr_types::PbrInput;
fn visibility_range_dither(frag_coord: vec4<f32>, dither: i32) {
}
fn alpha_discard(material_flags: u32, alpha_cutoff: f32, output_color: vec4<f32>) -> vec4<f32> {
    return output_color;
}
fn apply_pbr_lighting(in: PbrInput) -> vec4<f32> {
    return in.material.base_color;
}
fn main_pass_post_lighting_processing(pbr_input: PbrInput, input_color: vec4<f32>) -> vec4<f32> {
    return input_color;
}
"
        .to_string(),
        "bevy_pbr::render::utils" => "\
fn interleaved_gradient_noise(pixel_coordinates: vec2<f32>, frame: u32) -> f32 {
    return 0.0;
}
fn sample_cube_dir(uv: vec2f, face: u32) -> vec3f {
    return vec3f(uv, f32(face));
}
"
        .to_string(),
        "bevy_pbr::render::view_transformations" => "\
fn position_world_to_view(world_pos: vec3<f32>) -> vec3<f32> {
    return world_pos;
}
fn position_world_to_clip(world_pos: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(world_pos, 1.0);
}
fn depth_ndc_to_view_z(ndc_depth: f32) -> f32 {
    return ndc_depth;
}
"
        .to_string(),
        other => panic!("no stub for the Bevy module `{other}`; add it to `stub_source`"),
    };
    source
}

/// Serves the crate's own `.wesl` files; every package also has an empty root module.
struct Files(FileResolver);

impl Resolver for Files {
    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, ResolveError> {
        if path.is_root() {
            return Ok(String::new().into());
        }
        self.0.resolve_source(path)
    }
}

/// Shader-def sets the crate's pipelines specialise with. Each is compiled in full, because
/// `@if` prunes the other branches before naga sees them: the base set (every def off) checks
/// the forward path only, and the prepass and deferred paths are otherwise never compiled
/// before a GPU runs them.
const VARIANTS: &[(&str, &[&str])] = &[
    ("forward", &[]),
    ("prepass", &["PREPASS_PIPELINE"]),
    (
        "prepass+normal+motion",
        &["PREPASS_PIPELINE", "NORMAL_PREPASS_OR_DEFERRED_PREPASS", "MOTION_VECTOR_PREPASS"],
    ),
    (
        "deferred",
        &["PREPASS_PIPELINE", "DEFERRED_PREPASS", "NORMAL_PREPASS_OR_DEFERRED_PREPASS"],
    ),
    (
        "forward+optional",
        &[
            "VISIBILITY_RANGE_DITHER",
            "VERTEX_OUTPUT_INSTANCE_INDEX",
            "GROUND_TEXTURES",
            "GROUND_COLOURS",
            "PATHS",
            "BLEND_MULTIPLY",
        ],
    ),
    (
        "prepass+ortho",
        &["PREPASS_PIPELINE", "UNCLIPPED_DEPTH_ORTHO_EMULATION", "VISIBILITY_RANGE_DITHER"],
    ),
    // `terrain/heightmap.wesl` is built twice, with and without this def (`terrain/gpu.rs`).
    ("uint-source", &["UINT_SOURCE"]),
];

/// Every name an `@if(...)` in a `.wesl` file under `dir` tests, with the file that tests it.
fn feature_names(dir: &Path, found: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            feature_names(&path, found);
        } else if path.extension().is_some_and(|e| e == "wesl")
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            let mut rest = text.as_str();
            while let Some(at) = rest.find("@if(") {
                rest = &rest[at + 4..];
                let Some(end) = rest.find(')') else { break };
                for name in rest[..end].split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                    if !name.is_empty() {
                        found.push((name.to_string(), path.display().to_string()));
                    }
                }
            }
        }
    }
}

/// A def no variant turns on is compiled disabled, so its branch reaches naga never.
fn untested_defs(root: &Path) -> usize {
    let mut found = Vec::new();
    feature_names(root, &mut found);
    found.sort();
    found.dedup();
    let mut failed = 0;
    for (name, file) in found {
        if !VARIANTS.iter().any(|(_, defs)| defs.contains(&name.as_str())) {
            println!("{file}: `@if({name})` is in no variant of VARIANTS, so its branch is never compiled");
            failed += 1;
        }
    }
    failed
}

fn router(root: &Path) -> Router {

    let mut files = FileResolver::new(root);
    files.set_extension("wesl");
    files.set_root(root);

    let mut router = Router::new();
    router.mount_resolver("bevy_elements".parse().expect("the crate's name"), Files(files));

    for namespace in BEVY_NAMESPACES {
        for spelling in [namespace.to_string(), format!("bevy_elements/{namespace}")] {
            let mut stubs = VirtualResolver::new();
            // `constants::NAME` is the *root* of the `constants` package, not a path component;
            // every other package gets an empty root.
            if *namespace == "constants" {
                stubs.add_module(ModulePath::new_root(), stub_source("constants").into());
            } else {
                stubs.add_module(ModulePath::new_root(), String::new().into());
            }
            if *namespace == "bevy_pbr" {
                stubs.add_module(virtual_path("render"), stub_source("bevy_pbr::render").into());
            }
            for name in BEVY_MODULES {
                if let Some(rest) = name.strip_prefix(&format!("{namespace}::")) {
                    stubs.add_module(virtual_path(rest), stub_source(name).into());
                }
            }
            router.mount_resolver(spelling.parse().expect("a namespace"), stubs);
        }
    }

    router
}

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| "src".to_string());
    let root = Path::new(&root);
    let mut failed = 0;
    for (name, defs) in VARIANTS {
        println!("== variant {name} ({})", defs.join(", "));
        failed += check(root, defs);
    }
    failed += untested_defs(root);
    if failed > 0 {
        eprintln!("{failed} module(s) failed");
        std::process::exit(1);
    }
}

fn check(root: &Path, defs: &[&str]) -> usize {
    // `keep_main`: a library module has no entry point, and stripping keeps only what an entry
    // point reaches, so without it a library compiled to an *empty* string and the check passed
    // nothing past the parser. `visibility: false` is how Bevy compiles the crate's shaders.
    let options = wesl::CompileOptions {
        keep_main: true,
        visibility: false,
        ..Default::default()
    };
    let mut options = options;
    for def in defs {
        options.features.set(*def, true);
    }
    let compiler = wesl::Compiler::new_with_resolver(options, router(root));

    let mut failed = 0;
    for module in MODULES {
        let path: ModulePath = match module.parse() {
            Ok(path) => path,
            Err(e) => {
                println!("{module}: the module path is wrong: {e}");
                failed += 1;
                continue;
            }
        };
        match compiler.compile_module(&path) {
            Ok(result) => {
                let wgsl = result.to_string();
                match naga::front::wgsl::parse_str(&wgsl) {
                    Ok(_) => println!("{module}: the WGSL is ok"),
                    Err(e) => {
                        println!("{module}: the WGSL has an error:\n{}", e.emit_to_string(&wgsl));
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                let text = e.to_string();
                if let Some(benign) = BENIGN.iter().find(|b| text.contains(**b)) {
                    println!("{module}: not checked ({benign}): {text}");
                } else {
                    println!("{module}: ERROR {e}");
                    failed += 1;
                }
            }
        }
    }

    failed
}
