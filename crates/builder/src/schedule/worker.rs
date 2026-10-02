use super::*;
use crate::{
    artifacts::{
        self, ArtifactPaths, DependencyHash, FORMAT_VERSION, GenericBundle, Manifest, Metadata,
        ObjectMode, Symbols,
    },
    plan::TargetInput,
};
use mlc_syntax::parser::out::TempGlobalStmt;
use std::{collections::BTreeMap, fs};

pub(super) fn run(
    target: &Target,
    dependencies: &[Artifact],
    options: &BuildOptions,
    compile: impl FnOnce(CompileRequest<'_>) -> Result<CompileOutput, String>,
) -> Result<NodeResult, String> {
    let hashes: BTreeMap<String, DependencyHash> = dependencies
        .iter()
        .map(|artifact| {
            Ok((
                artifact.manifest.config.module_name.clone(),
                artifact.manifest.dependency_hash()?,
            ))
        })
        .collect::<Result<_, String>>()?;
    if let TargetInput::Declaration(_) = &target.input {
        let artifact = artifacts::load(&target.file)?;
        if artifact.manifest.config.object == ObjectMode::Source
            && artifact.manifest.config.dependencies != hashes
        {
            return Err(
                "Prebuilt declarations have changed dependencies; rebuild their source".into(),
            );
        }
        if artifact.object.is_some() && artifact.manifest.config.target != options.target {
            return Err("Prebuilt object target does not match build target".into());
        }
        return Ok(NodeResult::Declaration(artifact));
    }
    let TargetInput::Source { text, module } = &target.input else {
        unreachable!()
    };
    let file_hash = artifacts::content_hash(text.as_bytes());
    let build_key = artifacts::semantic_hash(&(
        &options.target,
        &options.compiler_id,
        &options.compiler_options,
        &file_hash,
        &hashes,
    ))?;
    let stems = if target.artifact_stems.is_empty() {
        vec![super::safe_stem(&options.output, &target.module_name)?]
    } else {
        target.artifact_stems.clone()
    };
    for stem in &stems {
        let paths = ArtifactPaths::new(stem, &options.target);
        if let Ok(artifact) = artifacts::load(&paths.manifest) {
            if artifact.manifest.config.build_key == build_key
                && artifact.manifest.config.object == ObjectMode::Source
                && artifact.manifest.config.source_file == target.file.to_string_lossy()
            {
                return Ok(NodeResult::Cached(artifact));
            }
        }
    }
    let (stem, warnings) = super::storage::writable_stem(&stems)?;
    let paths = ArtifactPaths::new(&stem, &options.target);
    let compiled = compile(CompileRequest {
        target,
        dependencies,
        options,
    })?;
    let symbols = Symbols::from_module(module);
    let templates = artifacts::has_templates(module).then(|| GenericBundle {
        format_version: FORMAT_VERSION,
        source_file: target.file.to_string_lossy().into_owned(),
        module_name: target.module_name.clone(),
        module: module.clone(),
    });
    let generic_hash = templates
        .as_ref()
        .map(|bundle| artifacts::semantic_hash(&bundle.module))
        .transpose()?
        .unwrap_or_default();
    let mut manifest = Manifest {
        config: Metadata {
            format_version: FORMAT_VERSION,
            source_file: target.file.to_string_lossy().into_owned(),
            module_name: target.module_name.clone(),
            file_hash,
            symbols_hash: symbols.public_hash()?,
            generic_hash,
            build_key,
            target: options.target.clone(),
            object: ObjectMode::Source,
            object_file: None,
            object_hash: None,
            generic: None,
            generic_file_hash: None,
            dependencies: hashes,
        },
        symbols,
        requires: module
            .iter()
            .filter_map(|(item, _)| match item {
                TempGlobalStmt::Import(import) => Some(import.clone()),
                _ => None,
            })
            .collect(),
    };
    manifest.config.symbols_hash = manifest.computed_symbols_hash()?;
    let object = if let Some(ir) = compiled.ir {
        let root = paths.object.parent().ok_or("Object has no parent")?;
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        // The LLVM machine is created and destroyed on this worker, never shared.
        let path = tempfile::NamedTempFile::new_in(root)
            .map_err(|e| e.to_string())?
            .into_temp_path();
        crate::llvm::IrCompiler::init(&options.target)
            .map_err(|e| e.to_string())?
            .emit(ir, &path)
            .map_err(|e| e.to_string())?;
        Some(fs::read(&path).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let mut artifact = super::storage::publish(
        &stems,
        &stem,
        manifest,
        templates,
        object.as_deref(),
        &options.target,
    )?;
    artifact.warnings.splice(0..0, warnings);
    Ok(NodeResult::Built(artifact))
}
