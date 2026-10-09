mod cache;
mod frontend;
mod ir_dump;
mod link;
use crate::cli::{Cli, Commands};
use mlc_builder::{
    config::GlobalConfig,
    paths::PathResolver,
    plan::BuildPlan,
    project::{ProjectFile, TargetKind},
    schedule::{self, BuildOptions, CompileOutput},
};
use std::{collections::HashMap, path::Path};

pub fn run(cli: Cli) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    let targets = match &cli.command {
        Some(Commands::Ll { input }) => {
            if cli.entry.is_some() {
                return Err("Do not combine ll with an entry file".into());
            }
            vec![(input.clone(), None, TargetKind::Bin)]
        }
        Some(Commands::Symbols { input }) => {
            if cli.entry.is_some() {
                return Err("Do not combine symbols with an entry file".into());
            }
            let output = cli
                .output
                .clone()
                .unwrap_or_else(|| input.with_extension("sym"));
            mlc_builder::artifacts::convert_toml(input, &output)?;
            println!("Written {}", output.display());
            return Ok(());
        }
        Some(Commands::Example { name }) => {
            if cli.entry.is_some() {
                return Err("Do not combine example with an entry file".into());
            }
            let output = mlc_examples::render(name)?;
            use std::io::Write;
            return std::io::stdout()
                .lock()
                .write_all(output.as_bytes())
                .map_err(|error| error.to_string());
        }
        Some(Commands::Build { target }) => {
            if cli.entry.is_some() {
                return Err("Do not combine build with an entry file".into());
            }
            let project = ProjectFile::load(&root)?.ok_or("Project.toml not found")?;
            let selected: Vec<_> = project
                .project
                .targets
                .into_iter()
                .filter(|t| target.as_ref().is_none_or(|name| name == &t.name))
                .collect();
            if selected.is_empty() {
                return Err("No matching project target".into());
            }
            selected
                .into_iter()
                .map(|t| (root.join(t.entry), Some(t.name), t.kind))
                .collect::<Vec<_>>()
        }
        None => vec![(
            cli.entry
                .clone()
                .ok_or("Specify an entry file or use 'mlc build'; see --help")?,
            None,
            TargetKind::Bin,
        )],
    };
    let global = match &cli.config {
        Some(path) => GlobalConfig::load(path)?,
        None => GlobalConfig::load_default()?,
    };
    for (entry, name, kind) in targets {
        build(
            &cli,
            &global,
            &root,
            &entry,
            name.as_deref(),
            cli.kind.map(Into::into).unwrap_or(kind),
        )?;
    }
    Ok(())
}

fn build(
    cli: &Cli,
    global: &GlobalConfig,
    root: &Path,
    entry: &Path,
    name: Option<&str>,
    kind: TargetKind,
) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut paths = PathResolver::new(&executable, root, name, global)?;
    if let Some(triplet) = &cli.triplet {
        mlc_builder::paths::validate_triplet(triplet)?;
        paths.triplet = triplet.clone();
        for dir in &mut paths.imports.lib_dirs {
            *dir = dir.parent().ok_or("Invalid library root")?.join(triplet);
        }
        paths.output = paths
            .output
            .parent()
            .ok_or("Invalid output root")?
            .join(triplet);
        paths.lib_fallback = paths
            .lib_fallback
            .parent()
            .ok_or("Invalid cache root")?
            .join(triplet);
        if let Some(dir) = &mut paths.lib_cache {
            *dir = dir.parent().ok_or("Invalid cache root")?.join(triplet);
        }
    }
    if !cli.lib_dirs.is_empty() {
        paths.imports.lib_dirs = cli
            .lib_dirs
            .iter()
            .map(|p| root.join(p).join(&paths.triplet))
            .collect();
    }
    let output = cli
        .output
        .as_ref()
        .map(|p| root.join(p))
        .unwrap_or_else(|| {
            paths
                .output
                .parent()
                .and_then(Path::parent)
                .unwrap()
                .to_owned()
        });
    if cli.output.is_some() {
        paths.output = output.join("objects").join(&paths.triplet);
        paths.lib_fallback = output.join("objects/lib").join(&paths.triplet);
    }
    let plan = BuildPlan::discover_with_paths(entry, &paths)?;
    let generated = frontend::generate(&plan, &paths.triplet, kind)?;
    if matches!(cli.command, Some(Commands::Ll { .. })) {
        let directory = if cli.output.is_some() {
            output
        } else {
            output.join("ll")
        };
        return ir_dump::write(&plan, generated, &directory);
    }
    let module_fingerprints = generated
        .modules
        .iter()
        .map(|(id, ir)| {
            (
                plan.targets[*id].module_name.clone(),
                mlc_builder::artifacts::content_hash(ir.as_bytes()),
            )
        })
        .collect();
    let modules: HashMap<_, _> = generated.modules.into_iter().collect();
    let global_inits: HashMap<_, _> = generated.global_inits.into_iter().collect();
    let options = BuildOptions {
        output: paths.output.clone(),
        target: paths.triplet.clone(),
        compiler_id: format!("mlc-{}-cli-v2", env!("CARGO_PKG_VERSION")),
        compiler_options: vec![format!("kind={kind:?}")],
        module_fingerprints,
        workers: cli.jobs.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
        }),
    };
    if options.workers == 0 {
        return Err("--jobs must be greater than zero".into());
    }
    let report = schedule::build(plan, options, move |request| {
        Ok(CompileOutput {
            global_init: global_inits.get(&request.target.id.0).cloned(),
            ir: Some(
                modules
                    .get(&request.target.id.0)
                    .ok_or("Missing generated module")?
                    .clone(),
            ),
        })
    })?;
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if !report.succeeded() {
        return Err(format!("Build failed: {:?}", report.results));
    }
    std::fs::create_dir_all(&paths.output).map_err(|e| e.to_string())?;
    let mut objects = report.objects;
    if let Some(ir) = generated.supplement {
        let object = paths.output.join("__mlc_instances.obj");
        let key = mlc_builder::artifacts::semantic_hash(&(
            &paths.triplet,
            env!("CARGO_PKG_VERSION"),
            &ir,
        ))?;
        if !cache::matches(&object, &key) {
            mlc_builder::llvm::IrCompiler::init(&paths.triplet)
                .map_err(|e| e.to_string())?
                .emit(ir, &object)
                .map_err(|e| e.to_string())?;
            cache::record(&object, &key)?;
        }
        objects.push(object);
    }
    let stem = name
        .map(str::to_owned)
        .or_else(|| entry.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .ok_or("Entry has no filename")?;
    let artifact = link::output_path(&output, &stem, kind, &paths.triplet);
    if stem.is_empty() || stem == "." || stem == ".." || stem.contains(['/', '\\', ':']) {
        return Err("Target name must be a filename, not a path".into());
    }
    let object_hashes = objects
        .iter()
        .map(|path| {
            std::fs::read(path)
                .map(|bytes| (path.clone(), mlc_builder::artifacts::content_hash(&bytes)))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tool = cache::tool_identity(if kind == TargetKind::Static {
        &cli.archiver
    } else {
        &cli.linker
    });
    let lld = if kind == TargetKind::Static {
        None
    } else {
        mlc_builder::manifest::lld_path(
            &std::env::current_exe().map_err(|error| error.to_string())?,
            paths.triplet.contains("windows-msvc"),
        )
        .and_then(|path| cache::tool_identity(&path))
    };
    let environment = [
        "PATH",
        "LIB",
        "LIBPATH",
        "INCLUDE",
        "WindowsSdkDir",
        "VCToolsInstallDir",
    ]
    .map(|name| {
        (
            name,
            std::env::var_os(name).map(|value| value.to_string_lossy().into_owned()),
        )
    });
    let link_key = mlc_builder::artifacts::semantic_hash(&(
        &report.link_key,
        &paths.triplet,
        format!("{kind:?}"),
        &cli.linker,
        &cli.archiver,
        &cli.link_args,
        object_hashes,
        &tool,
        &lld,
        environment,
    ))?;
    // Extra linker arguments may reference external libraries/scripts that are
    // not represented in our dependency graph. Relink conservatively in that case.
    if tool.is_none() || !cli.link_args.is_empty() || !cache::matches(&artifact, &link_key) {
        link::link(cli, &objects, &artifact, kind, &paths.triplet)?;
        cache::record(&artifact, &link_key)?;
    }
    println!("Built {}", artifact.display());
    Ok(())
}
