mod board;
use crate::cli::{Cli, Commands};
use mlc_builder::{
    llvm::IrCompiler,
    paths::validate_triplet,
    plan::{BuildPlan, ImportResolver},
    project::{BuildMode, Project, TargetKind},
};
use mlc_core::diagnostic::Report;
use std::{collections::HashMap, fs, path::Path, process::Command};

pub(super) fn run(cli: &Cli, root: &Path, project: &Project) -> Result<i32, Report> {
    let Commands::Build { target } = &cli.command else {
        return Err("Use mlc build to compile firmware".into());
    };
    let triplet = project
        .triplet
        .as_deref()
        .ok_or("Embedded projects require Project.Triplet")?;
    validate_triplet(triplet)?;
    if project
        .targets
        .iter()
        .any(|target| target.triplet.is_some())
        || cli.triplet.as_deref().is_some_and(|value| value != triplet)
    {
        return Err("Embedded targets share Project.Triplet".into());
    }
    let cpu = project
        .cpu
        .as_deref()
        .filter(|cpu| !cpu.is_empty())
        .ok_or("Embedded projects require Project.Cpu")?;
    let board_path = project
        .board
        .as_ref()
        .map(|path| root.join(path))
        .or_else(|| {
            ["Board.toml", "board.toml"]
                .into_iter()
                .map(|name| root.join(name))
                .find(|path| path.is_file())
        });
    let board = board_path.as_deref().map(board::Board::read).transpose()?;
    if board.is_some() && !matches!(triplet, "thumbv7m-none-eabi" | "thumbv7em-none-eabi") {
        return Err(
            "Generated startup currently supports Cortex-M ARMv7-M soft-float triplets".into(),
        );
    }
    let selected: Vec<_> = project
        .targets
        .iter()
        .filter(|item| target.as_ref().is_none_or(|name| *name == item.name))
        .collect();
    if selected.is_empty() {
        return Err("No matching project target".into());
    }
    let mut names = std::collections::HashSet::new();
    for item in &selected {
        if !super::valid_target_name(&item.name) || !names.insert(item.name.to_lowercase()) {
            return Err(format!("Invalid or duplicate project target name: {}", item.name).into());
        }
        if cli.kind.map(Into::into).unwrap_or(item.kind) != TargetKind::Bin {
            return Err("Embedded firmware targets require Type = \"bin\"".into());
        }
    }
    let staging = tempfile::tempdir_in(root).map_err(|e| e.to_string())?;
    let startup_object = staging.path().join("startup.o");
    let script = match (&project.linker_script, &board) {
        (Some(path), _) => root.join(path),
        (None, Some(board)) => {
            let path = staging.path().join("board.ld");
            fs::write(&path, board.linker_script()?).map_err(|e| e.to_string())?;
            path
        }
        (None, None) => {
            return Err("Embedded projects require Project.LinkerScript or Board.toml".into());
        }
    };
    if !script.is_file() {
        return Err(format!("Embedded input not found: {}", script.display()).into());
    }
    if board.is_none() {
        let startup = root.join(
            project
                .startup
                .as_ref()
                .ok_or("Embedded projects require Project.Startup or Board.toml")?,
        );
        if !startup.is_file() {
            return Err(format!("Embedded input not found: {}", startup.display()).into());
        }
        invoke(
            Command::new("clang")
                .current_dir(root)
                .arg(format!("--target={triplet}"))
                .arg(format!("-mcpu={cpu}"))
                .arg("-c")
                .arg(&startup)
                .arg("-o")
                .arg(&startup_object),
        )?;
    }

    for item in selected {
        let mode = cli.build_mode(
            item.profile
                .or(item.mode)
                .or(project.mode)
                .unwrap_or(BuildMode::Default),
        );
        let optimize = !cli.no_optimize && (cli.optimize || mode.optimized());
        let compiler =
            IrCompiler::for_embedded(triplet, cpu, optimize).map_err(|e| e.to_string())?;
        let plan = BuildPlan::discover(
            &root.join(&item.entry),
            &ImportResolver { lib_dirs: vec![] },
        )?;
        let mut optimization = if optimize {
            mlc_optimizer::OptimizeConfig::enabled()
        } else {
            mlc_optimizer::OptimizeConfig::disabled()
        };
        // IRQ roots are referenced from startup assembly, outside the MLC call graph.
        optimization.unused_function_elimination = false;
        let generated = super::frontend::generate_with_module_configs(
            &plan,
            triplet,
            TargetKind::Bin,
            optimization,
            mlc_codegen::gens::CodegenConfig {
                debug_info: false,
                optimized: optimize,
            },
            &HashMap::new(),
            Some(staging.path().join("ctfe")),
            std::time::Duration::from_secs(3),
            Some(&compiler),
        )?;
        if let Some(board) = &board {
            let main_return = generated
                .entry_return
                .as_ref()
                .ok_or("Missing firmware main")?;
            let initializer = generated
                .global_inits
                .iter()
                .find(|(id, _)| *id == plan.entry.0)
                .map(|(_, name)| name)
                .ok_or("Missing firmware initializer")?;
            let startup = board.startup(&main_return.to_string(), initializer)?;
            compiler
                .emit(startup, &startup_object)
                .map_err(|e| e.to_string())?;
        }
        let mut objects = vec![startup_object.clone()];
        for (index, ir) in generated
            .modules
            .into_iter()
            .map(|(_, ir)| ir)
            .chain(generated.supplement)
            .enumerate()
        {
            let object = staging.path().join(format!("module-{index}.o"));
            compiler.emit(ir, &object).map_err(|e| e.to_string())?;
            objects.push(object);
        }
        let directory = root
            .join(cli.output.as_ref().unwrap_or(&project.output_dir))
            .join(mode.directory());
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let artifact = directory.join(format!("{}.elf", item.name));
        let temporary = tempfile::NamedTempFile::new_in(&directory)
            .map_err(|e| e.to_string())?
            .into_temp_path();
        let linker = cli
            .linker
            .as_ref()
            .map(|p| root.join(p).into_os_string())
            .unwrap_or_else(|| format!("ld.lld{}", std::env::consts::EXE_SUFFIX).into());
        invoke(
            Command::new(linker)
                .current_dir(root)
                .arg("-T")
                .arg(&script)
                .arg("--gc-sections")
                .arg("-o")
                .arg(&temporary)
                .args(&objects)
                .args(&cli.link_args),
        )?;
        temporary.persist(&artifact).map_err(|e| e.to_string())?;
        crate::output::artifact("linked", &artifact);
        crate::output::info(format!("Built {}", artifact.display()));
    }
    Ok(0)
}

fn invoke(command: &mut Command) -> Result<(), Report> {
    let program = command.get_program().to_string_lossy().into_owned();
    let output = command
        .output()
        .map_err(|e| format!("Cannot run {program}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed ({}):\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    crate::output::warning(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    Ok(())
}
