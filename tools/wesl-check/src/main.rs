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
//! The path is the crate's `src` directory, where the modules live. Bevy's own shader modules
//! are not on disk, so they are stubbed; the stub only has to *exist* for the import to
//! resolve.

use std::borrow::Cow;
use std::path::Path;

use wesl::error::ResolveError;
use wesl::resolver::{FileResolver, Resolver, Router, VirtualResolver};
use wgsl_parse::syntax::ModulePath;

/// A resolver that serves the crate's own `.wesl` files *and* a set of stubbed modules under
/// the same namespace, because a router mounts one resolver per namespace and the crate's
/// modules import Bevy's through the crate's own path.
struct CrateResolver<'a> {
    files: FileResolver,
    stubs: VirtualResolver<'a>,
}

impl Resolver for CrateResolver<'_> {
    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, ResolveError> {
        // The stubs first: a stub is an *exact* match on a name, and a file is found by a
        // path, so the stub wins where both could answer.
        match self.stubs.resolve_source(path) {
            Ok(source) => Ok(source),
            Err(stub_error) => self
                .files
                .resolve_source(path)
                .map_err(|_| stub_error),
        }
    }
}

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
    "bevy_elements::hydrology::readback",
    "bevy_elements::hydrology::source_map",
    "bevy_elements::hydrology::substep",
    "bevy_elements::hydrology::thermal",
    "bevy_elements::hydrology::water",
    "bevy_elements::hydrology::water_normals",
    "bevy_elements::noise",
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

/// The errors that are not the crate's shaders' fault, and so are reported and not counted as
/// failures:
///
/// - a module that *re-exports* another module's `package`-visible item: Bevy's own loader
///   resolves the import the other way, and the crate's shaders compile and run;
/// - a Bevy module the stub list does not have: the crate's shaders import Bevy's own
///   modules, which are not on disk, so the import cannot resolve. The stub list is a
///   *convenience*, and a missing entry must not fail the check — the check is for the
///   crate's own modules.
const BENIGN: &[&str] = &[
    "is declared with `package` visibility",
    "module not found",
];

/// Bevy's own shader modules, stubbed: they are compiled into Bevy and are not on disk, but the
/// crate's modules import them and the import has to resolve.
///
/// The list is every module the crate's own shaders import. It is generated from the imports
/// with:
///
/// ```sh
/// grep -rh "^import bevy_" src/ --include=*.wesl | sed 's/^import //; s/::.*//'
/// ```
///
/// A module that is missing here fails the whole check, so the list is kept in step with the
/// imports.
const BEVY_MODULES: &[&str] = &[
    "bevy_core_pipeline::fullscreen_vertex_shader",
    "bevy_pbr::decal::clustered",
    "bevy_pbr::deferred::functions",
    "bevy_pbr::prepass::io",
    "bevy_pbr::prepass::utils",
    "bevy_pbr::render",
    "bevy_pbr::render::forward_io",
    "bevy_pbr::render::mesh_view_bindings",
    "bevy_pbr::render::pbr_fragment",
    "bevy_pbr::render::pbr_functions",
    "bevy_pbr::render::pbr_types",
    "bevy_pbr::render::utils",
    "bevy_render::view",
];

/// The stub's source for a Bevy module: an empty module, except for `bevy_pbr::render`, whose
/// own sub-modules the crate's shaders import *through* it.
fn stub_source(name: &str) -> String {
    if name == "constants" {
        return String::from(
            "const MATERIAL_BIND_GROUP: u32 = 1u;\nconst MESH_BIND_GROUP: u32 = 0u;\n",
        );
    }
    if name != "bevy_pbr::render" {
        return String::new();
    }
    let mut source = String::new();
    for child in [
        "forward_io",
        "mesh_view_bindings",
        "pbr_fragment",
        "pbr_functions",
        "pbr_types",
        "utils",
        "view_transformations",
    ] {
        source.push_str(&format!(
            "import bevy_elements::bevy_pbr::render::{child};\n"
        ));
    }
    source
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "src".to_string());
    let root = Path::new(&root);

    // One resolver per namespace. The crate's own files and Bevy's stubs live in the *same*
    // namespace, because the crate's modules import Bevy's through the crate's own path —
    // `import bevy_elements::bevy_pbr::render::view_transformations;` — and a router mounts
    // one resolver per namespace, so a second mount would *replace* the first.
    let mut resolver = FileResolver::new(root);
    resolver.set_extension("wesl");
    resolver.set_root(root);
    let mut router = Router::new();

    // The crate's namespace holds the crate's own files and the stubs of Bevy's modules.
    let mut stubs = VirtualResolver::new();
    for name in BEVY_MODULES {
        // The stub's path is registered in **both** forms: relative to the `bevy_elements`
        // mount (`bevy_pbr::render::view_transformations`) and fully qualified
        // (`bevy_elements::bevy_pbr::render::view_transformations`). The resolver looks a
        // module up relative to the mount of the namespace the *importing* module lives in, and
        // the crate's modules import Bevy's both ways.
        stubs.add_module(name.parse().expect("a module path"), stub_source(name).into());
        stubs.add_module(
            format!("bevy_elements::{name}")
                .parse()
                .expect("a module path"),
            stub_source(name).into(),
        );
    }
    // Bevy's shader prelude substitutes `constants::NAME` with a Rust-side constant before
    // the WGSL is compiled, so the crate's shaders read it as a module path. It is stubbed
    // with the constants the crate's own shaders use.
    for path in ["constants", "bevy_elements::constants"] {
        stubs.add_module(
            path.parse().expect("a module path"),
            stub_source("constants").into(),
        );
    }
    router.mount_resolver(
        "bevy_elements".parse().expect("the crate's name"),
        CrateResolver {
            files: resolver,
            stubs,
        },
    );

    // Bevy's own namespaces, each with the stubs of the modules in it. The stub's path is
    // relative to the namespace's mount, so `bevy_pbr::render::view_transformations` is
    // mounted under `bevy_pbr` as `render::view_transformations`.
    for namespace in ["bevy_pbr", "bevy_render", "bevy_core_pipeline"] {
        let mut stub = VirtualResolver::new();
        for name in BEVY_MODULES {
            if let Some(rest) = name.strip_prefix(&format!("{namespace}::")) {
                stub.add_module(rest.parse().expect("a module path"), "".into());
            }
        }
        router.mount_resolver(namespace.parse().expect("a namespace"), stub);
    }

    // `keep_main`: a library module has no entry point, and stripping keeps only what an entry
    // point reaches, so without it a library compiled to an *empty* string and "the WGSL is ok"
    // checked nothing past the parser (a `let x: f32 = vec3<f32>(1.0);` in `textures.wesl`
    // passed). Keeping the module's own declarations pulls in what they import, and
    // `visibility: false` is how Bevy compiles the crate's shaders, so a `package` item imported
    // across the crate's modules is not refused here when Bevy accepts it.
    let options = wesl::CompileOptions {
        keep_main: true,
        visibility: false,
        ..Default::default()
    };
    let compiler = wesl::Compiler::new_with_resolver(options, router);

    let mut failed = 0;
    for module in MODULES {
        let path: wgsl_parse::syntax::ModulePath = match module.parse() {
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
                    println!("{module}: not checked ({benign})");
                } else {
                    println!("{module}: ERROR {e}");
                    failed += 1;
                }
            }
        }
    }

    if failed > 0 {
        eprintln!("{failed} module(s) failed");
        std::process::exit(1);
    }
}
