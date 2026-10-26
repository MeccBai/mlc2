mod cache;
mod formatting;
mod frontend;
mod initialize;
mod ir_dump;
mod link;
mod progress;
mod setup;
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
        Commands::Setup => Some("Setup"),
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
        Commands::Setup => return setup::run(&cli, &root, progress),
        Commands::Init { project } => return initialize::run(&root, project),
        Commands::Fmt { input } => return formatting::run(&root, input.as_deref(), progress),
        Commands::Ll { input } => {
            vec![(input.clone(), None, TargetKind::Bin)]
        }
        Commands::Symbols { input, module } => {
            let input = root.join(input);
            if !input.is_file() {
                return Err(format!(
                    "Declaration input not found\nExpected path: {}",
                    input.display()
                ));
            }
            let output = cli
                .output
                .clone()
                .unwrap_or_else(|| input.with_extension("sym"));
            match input.extension().and_then(|extension| extension.to_str()) {
                Some("h") => {
                    let name = module
                        .as_deref()
                        .or_else(|| input.file_stem().and_then(|stem| stem.to_str()))
                        .ok_or("Header has no valid module name")?;
                    let target = cli
                        .triplet
                        .as_deref()
                        .unwrap_or(mlc_builder::paths::HOST_TRIPLET);
                    mlc_builder::artifacts::convert_c_header(&input, &output, name, target)?;
                }
                Some("toml") => mlc_builder::artifacts::convert_toml(&input, &output)?,
                _ => return Err("symbols accepts only .h or .toml input".into()),
            }
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
            let project = ProjectFile::load(&root)?.ok_or_else(|| {
                format!(
                    "Project.toml not found\nExpected paths:\n  {}\n  {}",
                    root.join("Project.toml").display(),
                    root.join("project.toml").display()
                )
            })?;
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
        Some(path) => {
            let path = root.join(path);
            if !path.is_file() {
                return Err(format!(
                    "Configuration file not found\nExpected path: {}",
                    path.display()
                ));
            }
            GlobalConfig::load(&path)?
        }
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
            false,
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
    objects_only: bool,
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
    }
    if !cli.lib_dirs.is_empty() {
        paths.imports.lib_dirs = cli
            .lib_dirs
            .iter()
            .map(|p| root.join(p).join(&paths.triplet))
            .collect();
    }
    let output_root = cli
        .output
        .as_ref()
        .map(|p| root.join(p))
        .unwrap_or_else(|| paths.build_root.clone());
    let mode = cli.build_mode(paths.mode);
    paths.configure_output(output_root.clone(), mode);
    let output = output_root.join(mode.directory());
    let plan = BuildPlan::discover_with_paths(entry, &paths)?;
    let debug = mode.debug_info() && (!objects_only || paths.triplet.contains("windows"));
    if debug && !paths.triplet.contains("windows") {
        return Err("Debug line tables currently require a Windows target".into());
    }
    let optimized = if cli.no_optimize {
        false
    } else {
        cli.optimize || mode.optimized()
    };
    let mut optimize = if optimized {
        mlc_optimizer::OptimizeConfig::enabled()
    } else {
        mlc_optimizer::OptimizeConfig::disabled()
    };
    if objects_only {
        optimize.unused_function_elimination = false;
    }
    let std_mode = paths.std_mode();
    let module_configs: HashMap<_, _> = plan
        .targets
        .iter()
        .filter(|target| target.artifact_stems.len() > 1)
        .map(|target| {
            let library_mode = if PathResolver::is_std_module(&target.module_name) {
                std_mode
            } else {
                mode
            };
            let library_optimized = library_mode.optimized();
            let library_debug = library_mode.debug_info() && paths.triplet.contains("windows");
            let mut optimize = if library_optimized {
                mlc_optimizer::OptimizeConfig::enabled()
            } else {
                mlc_optimizer::OptimizeConfig::disabled()
            };
            // A shared library object must retain helpers independently of this consumer.
            optimize.unused_function_elimination = false;
            (
                target.id.0,
                (
                    optimize,
                    mlc_codegen::gens::CodegenConfig {
                        debug_info: library_debug,
                        optimized: library_optimized,
                    },
                ),
            )
        })
        .collect();
    progress.stage("Analyzing and generating IR", 0);
    let generated = frontend::generate_with_module_configs(
        &plan,
        &paths.triplet,
        kind,
        optimize,
        mlc_codegen::gens::CodegenConfig {
            debug_info: debug,
            optimized,
        },
        &module_configs,
    )?;
    if matches!(cli.command, Commands::Ll { .. }) {
        let directory = output_root
            .join("ll")
            .join(mode.directory())
            .join(&paths.triplet);
        let module_directories = module_configs
            .keys()
            .map(|id| {
                (
                    *id,
                    output_root
                        .join("ll")
                        .join(
                            if PathResolver::is_std_module(&plan.targets[*id].module_name) {
                                std_mode
                            } else {
                                mode
                            }
                            .directory(),
                        )
                        .join(&paths.triplet),
                )
            })
            .collect();
        ir_dump::write(&plan, generated, &directory, &module_directories)?;
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
    let module_options = module_configs
        .iter()
        .map(|(id, (optimize, codegen))| {
            (
                plan.targets[*id].module_name.clone(),
                mlc_builder::schedule::ModuleOptions {
                    optimize: codegen.optimized,
                    compiler_options: vec![
                        format!("library-v2={},{}", codegen.optimized, codegen.debug_info),
                        format!("optimizer={optimize:?}"),
                        format!("debug-lines-v3={}", codegen.debug_info),
                    ],
                },
            )
        })
        .collect();
    let options = BuildOptions {
        module_options,
        optimize: optimized,
        output: paths.output.clone(),
        target: paths.triplet.clone(),
        compiler_id: format!("mlc-{}-cli-v2", env!("CARGO_PKG_VERSION")),
        compiler_options: vec![
            format!("kind={kind:?}"),
            format!("optimizer={optimize:?}"),
            format!("debug-lines-v3={debug}"),
        ],
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
    if objects_only {
        println!(
            "Prepared {} ({}/{})",
            entry.display(),
            mode.directory(),
            paths.triplet
        );
        return Ok(None);
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
            optimized,
        ))?;
        let hit = cache::matches(&object, &key);
        if !hit {
            mlc_builder::llvm::IrCompiler::with_optimization(&paths.triplet, optimized)
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
        debug,
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
    let link_hit = tool.is_some()
        && cli.link_args.is_empty()
        && cache::matches(&artifact, &link_key)
        && (!debug
            || kind == TargetKind::Static
            || cache::matches(&artifact.with_extension("pdb"), &link_key));
    if !link_hit {
        link::link(cli, &objects, &artifact, kind, &paths.triplet, debug)?;
        if debug && kind != TargetKind::Static {
            cache::record(&artifact.with_extension("pdb"), &link_key)?;
        }
        cache::record(&artifact, &link_key)?;
    }
    progress.link(link_hit);
    progress.advance();
    progress.clear();
    println!("Built {}", artifact.display());
    Ok(Some(artifact))
}
