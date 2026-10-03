//! Dependency-driven execution and incremental build decisions.
mod storage;
#[cfg(test)]
mod tests;
mod worker;
use crate::{
    artifacts::{Artifact, content_hash},
    plan::{BuildPlan, Target, TargetIndex},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};
use taskflowrs::{Executor, TaskHandle, Taskflow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildOptions {
    pub output: PathBuf,
    pub target: String,
    pub compiler_id: String,
    pub compiler_options: Vec<String>,
    pub workers: usize,
}

pub struct CompileRequest<'a> {
    pub target: &'a Target,
    pub dependencies: &'a [Artifact],
    pub options: &'a BuildOptions,
}

/// Compilation belongs to the caller: no Rc AST/package crosses worker boundaries.
/// Returning no IR explicitly creates a declarations-only artifact.
pub struct CompileOutput {
    pub ir: Option<String>,
    pub global_init: Option<String>,
}

#[derive(Debug, Clone)]
pub enum NodeResult {
    Built(Artifact),
    Cached(Artifact),
    Declaration(Artifact),
    Failed {
        file: PathBuf,
        error: String,
    },
    Blocked {
        file: PathBuf,
        dependencies: Vec<TargetIndex>,
    },
}
impl NodeResult {
    pub fn artifact(&self) -> Option<&Artifact> {
        match self {
            Self::Built(a) | Self::Cached(a) | Self::Declaration(a) => Some(a),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct BuildReport {
    pub warnings: Vec<String>,
    pub results: Vec<NodeResult>,
    pub objects: Vec<PathBuf>,
    /// Compare with the previous linked output's key, including object contents.
    pub link_key: String,
}
impl BuildReport {
    pub fn succeeded(&self) -> bool {
        self.results.iter().all(|r| r.artifact().is_some())
    }
}

/// One task graph run per build. Only ready nodes are executed; no worker waits
/// for an unfinished predecessor, and Result failure is handled by this layer.
pub fn build(
    plan: BuildPlan,
    options: BuildOptions,
    compile: impl Fn(CompileRequest<'_>) -> Result<CompileOutput, String> + Send + Sync + 'static,
) -> Result<BuildReport, String> {
    validate(&plan)?;
    if plan
        .triplet
        .as_ref()
        .is_some_and(|triplet| triplet != &options.target)
    {
        return Err("Build target does not match PathResolver triplet".into());
    }
    std::fs::create_dir_all(&options.output).map_err(|e| e.to_string())?;
    let plan = Arc::new(plan);
    let options = Arc::new(options);
    let compile = Arc::new(compile);
    let results: Arc<Vec<OnceLock<NodeResult>>> =
        Arc::new((0..plan.targets.len()).map(|_| OnceLock::new()).collect());
    let mut graph = Taskflow::new();
    let mut handles: Vec<Option<TaskHandle>> = vec![None; plan.targets.len()];
    for &id in &plan.order {
        let plan_ref = plan.clone();
        let options_ref = options.clone();
        let compile_ref = compile.clone();
        let slots = results.clone();
        let handle = graph.emplace(move || {
            let target = &plan_ref.targets[id.0];
            let dependencies: Vec<_> = target
                .requires
                .iter()
                .filter_map(|&index| slots[index.0].get().and_then(NodeResult::artifact).cloned())
                .collect();
            let result = if dependencies.len() != target.requires.len() {
                NodeResult::Blocked {
                    file: target.file.clone(),
                    dependencies: target
                        .requires
                        .iter()
                        .copied()
                        .filter(|index| {
                            slots[index.0]
                                .get()
                                .and_then(NodeResult::artifact)
                                .is_none()
                        })
                        .collect(),
                }
            } else {
                match worker::run(target, &dependencies, &options_ref, |request| {
                    compile_ref(request)
                }) {
                    Ok(result) => result,
                    Err(error) => NodeResult::Failed {
                        file: target.file.clone(),
                        error,
                    },
                }
            };
            let _ = slots[id.0].set(result);
        });
        for dependency in &plan.targets[id.0].requires {
            handles[dependency.0]
                .as_ref()
                .ok_or("Invalid dependency execution order")?
                .precede(&handle);
        }
        handles[id.0] = Some(handle);
    }
    Executor::new(options.workers).run(&graph).wait();
    let results: Vec<_> = results
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            slot.get().cloned().unwrap_or_else(|| NodeResult::Failed {
                file: plan.targets[index].file.clone(),
                error: "Task completed without publishing a result".into(),
            })
        })
        .collect();
    let mut objects = Vec::new();
    let mut link_inputs = Vec::new();
    for result in &results {
        if let Some(artifact) = result.artifact() {
            if let Some(path) = &artifact.object {
                objects.push(path.clone());
                link_inputs.push((
                    artifact.manifest.config.module_name.clone(),
                    artifact.object_hash.clone(),
                ));
            }
        }
    }
    link_inputs.sort();
    let link_key = content_hash(
        &serde_json::to_vec(&(
            &options.target,
            &options.compiler_id,
            &options.compiler_options,
            link_inputs,
        ))
        .map_err(|e| e.to_string())?,
    );
    Ok(BuildReport {
        warnings: results
            .iter()
            .filter_map(NodeResult::artifact)
            .flat_map(|a| a.warnings.clone())
            .collect(),
        results,
        objects,
        link_key,
    })
}

fn validate(plan: &BuildPlan) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for index in &plan.order {
        let target = plan.targets.get(index.0).ok_or("Invalid target index")?;
        if target.id != *index || !seen.insert(*index) {
            return Err("Duplicate or mismatched target index".into());
        }
        if target
            .requires
            .iter()
            .any(|dependency| !seen.contains(dependency) || dependency == index)
        {
            return Err("Plan is not in dependency-first order".into());
        }
        safe_stem(Path::new(""), &target.module_name)?;
    }
    if seen.len() != plan.targets.len() || !seen.contains(&plan.entry) {
        return Err("Incomplete build plan".into());
    }
    Ok(())
}

fn safe_stem(root: &Path, module: &str) -> Result<PathBuf, String> {
    let mut path = root.to_owned();
    for part in module.split("::") {
        if part.is_empty()
            || part.contains(['/', '\\', ':', '.'])
            || !matches!(
                Path::new(part).components().next(),
                Some(std::path::Component::Normal(_))
            )
        {
            return Err("Invalid artifact module name".into());
        }
        path.push(part);
    }
    Ok(path)
}
