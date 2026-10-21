mod cache;
mod formatting;
mod frontend;
mod ir_dump;
mod link;
mod progress;
use crate::cli::{Cli, Commands};
use mlc_builder::{
    config::GlobalConfig,
    paths::PathResolver,
    plan::BuildPlan,
    project::{ProjectFile, TargetKind},
    schedule::{self, BuildOptions, CompileOutput},
};
use progress::Progress;
use std::{collections::HashMap, path::Path};

pub fn run(cli: Cli) -> Result<i32, String> {
    let operation = match &cli.command {
        Commands::Build { .. } => Some("Build"),
        Commands::Fmt { .. } => Some("Format"),
        _ => None,
    };
    let progress = Progress::new(operation);
    let result = run_inner(cli, &progress);
    progress.finish(result.is_ok());
    result
}

fn run_inner(cli: Cli, progress: &Progress) -> Result<i32, String> {
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    let targets = match &cli.command {
        Commands::Fmt { input } => return formatting::run(&root, input.as_deref(), progress),
        Commands::Ll { input } => {
            vec![(input.clone(), None, TargetKind::Bin)]
        }
        Commands::Symbols { input } => {
            let output = cli
                .output
                .clone()
                .unwrap_or_else(|| input.with_extension("sym"));
            mlc_builder::artifacts::convert_toml(input, &output)?;
            println!("Written {}", output.display());
            return Ok(0);
        }
        Commands::Example { name } => {
            let output = mlc_examples::render(name)?;
            use std::io::Write;
            return std::io::stdout()
                .lock()
                .write_all(output.as_bytes())
                .map(|()| 0)
                .map_err(|error| error.to_string());
        }
        Commands::Build {
            target: Some(target),
        }
        | Commands::Run { target }
            if Path::new(target).extension().is_some_and(|ext| ext == "m2") =>
        {
            vec![(root.join(target), None, TargetKind::Bin)]
        }
        Commands::Build { .. } | Commands::Run { .. } => {
            let target = match &cli.command {
                Commands::Build { target } => target.as_deref(),
                Commands::Run { target } => Some(target.as_str()),
                _ => unreachable!(),
            };
            let project = ProjectFile::load(&root)?.ok_or("Project.toml not found")?;
            let selected: Vec<_> = project
                .project
                .targets
                .into_iter()
                .filter(|t| target.is_none_or(|name| name == t.name))
                .collect();
            if selected.is_empty() {
                return Err("No matching project target".into());
            }
            selected
                .into_iter()
                .map(|t| (root.join(t.entry), Some(t.name), t.kind))
                .collect::<Vec<_>>()
        }
    };
    let global = match &cli.config {
        Some(path) => GlobalConfig::load(path)?,
        None => GlobalConfig::load_default()?,
    };
    for (entry, name, kind) in targets {
        let kind = cli.kind.map(Into::into).unwrap_or(kind);
        if matches!(cli.command, Commands::Run { .. }) && kind != TargetKind::Bin {
            return Err(
                "mlc run requires a bin target; static and shared libraries cannot be run".into(),
            );
        }
        let artifact = build(
            &cli,
            &global,
            &root,
            &entry,
            name.as_deref(),
            kind,
            progress,
        )?;
        if matches!(cli.command, Commands::Run { .. }) {
            let artifact = artifact.ok_or("Build did not produce an executable")?;
            let status = std::process::Command::new(&artifact)
                .status()
                .map_err(|e| format!("Cannot run {}: {e}", artifact.display()))?;
            return status
                .code()
                .ok_or_else(|| "Program terminated without an exit code".into());
        }
    }
    Ok(0)
}

fn build(
    cli: &Cli,
    global: &GlobalConfig,
    root: &Path,
    entry: &Path,
    name: Option<&str>,
    kind: TargetKind,
    progress: &Progress,
) -> Result<Option<std::path::PathBuf>, String> {
    progress.stage("Discovering sources", 0);
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut paths = if name.is_none() {
        PathResolver::standalone(&executable, root, global)?
    } else {
        PathResolver::new(&executable, root, name, global)?
    };
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
    let optimize = if cli.no_optimize {
        mlc_optimizer::OptimizeConfig::disabled()
    } else if cli.optimize {
        mlc_optimizer::OptimizeConfig::enabled()
    } else {
        mlc_optimizer::OptimizeConfig::default()
    };
    progress.stage("Analyzing and generating IR", 0);
    let generated = frontend::generate_with_options(&plan, &paths.triplet, kind, optimize)?;
    if matches!(cli.command, Commands::Ll { .. }) {
        let directory = if cli.output.is_some() {
            output
        } else {
            output.join("ll")
        };
        ir_dump::write(&plan, generated, &directory)?;
        return Ok(None);
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
        compiler_options: vec![format!("kind={kind:?}"), format!("optimizer={optimize:?}")],
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
    progress.stage("Compiling modules", plan.targets.len());
    let observer = progress.clone();
    let report = schedule::build_with_progress(
        plan,
        options,
        move |request| {
            Ok(CompileOutput {
                global_init: global_inits.get(&request.target.id.0).cloned(),
                ir: Some(
                    modules
                        .get(&request.target.id.0)
                        .ok_or("Missing generated module")?
                        .clone(),
                ),
            })
        },
        move |_, result| observer.node(result),
    )?;
    progress.clear();
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if !report.succeeded() {
        return Err(format!("Build failed: {:?}", report.results));
    }
    std::fs::create_dir_all(&paths.output).map_err(|e| e.to_string())?;
    let mut objects = report.objects;
    if let Some(ir) = generated.supplement {
        progress.stage("Compiling generic instances", 1);
        let object = paths.output.join("__mlc_instances.obj");
        let key = mlc_builder::artifacts::semantic_hash(&(
            &paths.triplet,
            env!("CARGO_PKG_VERSION"),
            &ir,
        ))?;
        let hit = cache::matches(&object, &key);
        if !hit {
            mlc_builder::llvm::IrCompiler::init(&paths.triplet)
                .map_err(|e| e.to_string())?
                .emit(ir, &object)
                .map_err(|e| e.to_string())?;
            cache::record(&object, &key)?;
        }
        progress.object(hit);
        progress.advance();
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
    let linker = if kind == TargetKind::Static {
        link::archiver_path(cli)?
    } else {
        link::linker_path(cli, &paths.triplet)?
    };
    let tool = cache::tool_identity(&linker);
    let lld = if kind == TargetKind::Static {
        None
    } else {
        Some(link::runtime_identity(&paths.triplet)?)
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
        "native-lld-table-v2",
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
    progress.stage("Linking", 1);
    let link_hit =
        tool.is_some() && cli.link_args.is_empty() && cache::matches(&artifact, &link_key);
    if !link_hit {
        link::link(cli, &objects, &artifact, kind, &paths.triplet)?;
        cache::record(&artifact, &link_key)?;
    }
    progress.link(link_hit);
    progress.advance();
    progress.clear();
    println!("Built {}", artifact.display());
    Ok(Some(artifact))
}
