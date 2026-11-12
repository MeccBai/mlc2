use super::positions;
use mlc_builder::{
    artifacts,
    config::GlobalConfig,
    paths::PathResolver,
    plan::{BuildPlan, TargetInput},
};
use mlc_core::{
    ast::{
        AbstractSyntaxTree,
        config::{Config, FileId},
        semantic::Occurrence,
        symbols::PackageSymbolTable,
    },
    diagnostic::{error::ErrorHandle, warning::WarningHandle},
    parser::out::*,
};
use mlc_syntax::lexer::{self, Lexed, TokenPack};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};
#[allow(unused_imports)]
#[path = "../driver/frontend/macros.rs"]
mod macros;
#[derive(Clone)]
struct Declaration {
    file: usize,
    span: Span,
    kind: &'static str,
    ty: String,
}
struct File {
    path: PathBuf,
    text: String,
    lexed: Lexed,
    module: String,
    params: Vec<(Span, TempParam)>,
}
pub struct Snapshot {
    stamps: Vec<(PathBuf, std::time::SystemTime, u64)>,
    file: usize,
    files: HashMap<usize, File>,
    occurrences: HashMap<usize, Vec<Occurrence>>,
    declarations: HashMap<String, Declaration>,
}
fn declaration(
    map: &mut HashMap<String, Declaration>,
    file: usize,
    module: &str,
    name: &str,
    span: Span,
    kind: &'static str,
) {
    map.insert(
        format!("{module}::{name}"),
        Declaration {
            file,
            span,
            kind,
            ty: name.into(),
        },
    );
}
pub fn analyze(entry: &Path) -> Result<Snapshot, String> {
    let _recovery = mlc_core::diagnostic::ice::recoverable();
    let entry = entry.canonicalize().map_err(|e| e.to_string())?;
    let root = entry
        .parent()
        .unwrap()
        .ancestors()
        .find(|p| p.join("Project.toml").is_file() || p.join("project.toml").is_file())
        .unwrap_or(entry.parent().unwrap());
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let paths = PathResolver::new(
        &exe,
        root,
        None,
        &GlobalConfig::load_default().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let plan = BuildPlan::discover_with_paths(&entry, &paths).map_err(|e| e.to_string())?;
    let layout = mlc_builder::llvm::IrCompiler::init(&paths.triplet)
        .map_err(|e| e.to_string())?
        .target_layout();
    let mut files = HashMap::new();
    let mut parsed_modules = HashMap::new();
    let mut declarations = HashMap::new();
    for id in &plan.order {
        let target = &plan.targets[id.0];
        let (module, text, path) = match &target.input {
            TargetInput::Source { module, text, .. } => {
                (module.clone(), text.clone(), target.file.clone())
            }
            TargetInput::Declaration(manifest) => {
                let artifact = artifacts::load(&target.file)?;
                let module = artifact
                    .templates
                    .map(|b| b.module)
                    .unwrap_or_else(|| manifest.symbols.declarations());
                let path = if manifest.config.source_file.is_empty() {
                    target.file.clone()
                } else {
                    PathBuf::from(&manifest.config.source_file)
                };
                (
                    module,
                    std::fs::read_to_string(&path).unwrap_or_default(),
                    path,
                )
            }
        };
        let mut params = vec![];
        for (item, span) in &module {
            match item {
                TempGlobalStmt::Unit(unit) => {
                    declaration(
                        &mut declarations,
                        id.0,
                        &target.module_name,
                        &unit.name,
                        unit.name_span,
                        if unit.is_union { "type" } else { "class" },
                    );
                    for member in &unit.members {
                        declaration(
                            &mut declarations,
                            id.0,
                            &target.module_name,
                            &format!("{}::{}", unit.name, member.name),
                            member.name_span,
                            "property",
                        );
                    }
                }
                TempGlobalStmt::Enum(e) => {
                    declaration(
                        &mut declarations,
                        id.0,
                        &target.module_name,
                        &e.name,
                        e.name_span,
                        "enum",
                    );
                    for (name, span) in &e.variants {
                        declaration(
                            &mut declarations,
                            id.0,
                            &target.module_name,
                            &format!("{}::{name}", e.name),
                            *span,
                            "enumMember",
                        );
                    }
                }
                TempGlobalStmt::Func(f) => {
                    declaration(
                        &mut declarations,
                        id.0,
                        &target.module_name,
                        &f.symbol.name,
                        f.symbol.name_span,
                        "function",
                    );
                    params.extend(f.symbol.params.iter().cloned().map(|p| (*span, p)));
                }
                TempGlobalStmt::Interface(f) => {
                    params.extend(f.symbol.params.iter().cloned().map(|p| (*span, p)));
                }
                _ => {}
            }
        }
        let lexed = if text.is_empty() {
            lexer::tokenize("").unwrap()
        } else {
            lexer::tokenize(&text).map_err(|e| e.message())?
        };
        files.insert(
            id.0,
            File {
                path,
                text,
                lexed,
                module: target.module_name.clone(),
                params,
            },
        );
        parsed_modules.insert(id.0, module);
    }
    macros::expand(&plan, &mut parsed_modules)?;
    let mut package = PackageSymbolTable::new();
    let mut asts = vec![];
    let mut indexes = HashMap::new();
    for id in &plan.order {
        let target = &plan.targets[id.0];
        let file = &files[&id.0];
        let index = Rc::new(RefCell::new(vec![]));
        let mut config = Config::new(
            FileId::new(id.0),
            vec![],
            target.module_name.clone(),
            file.path.to_string_lossy().into(),
            ErrorHandle::new(file.path.to_string_lossy().into()),
            WarningHandle::new(file.path.to_string_lossy().into()),
        );
        config.set_target_layout(layout);
        config.set_source_lines(file.lexed.lines.clone());
        config.semantic = Some(index.clone());
        indexes.insert(id.0, index);
        package.set_imports(
            FileId::new(id.0),
            target.requires.iter().map(|id| FileId::new(id.0)),
        );
        let mut ast = AbstractSyntaxTree::new(config, parsed_modules.remove(&id.0).unwrap());
        ast.export(&mut package);
        if ast.config.is_poisoned() {
            return Err("Declaration analysis failed".into());
        }
        asts.push((id.0, ast));
    }
    // There is deliberately no constant executor, optimizer or IR generation here.
    for (_, ast) in &mut asts {
        ast.analysis(&mut package);
        if ast.config.is_poisoned() {
            return Err("Semantic analysis failed".into());
        }
    }
    let occurrences = indexes
        .into_iter()
        .map(|(id, index)| (id, index.borrow().clone()))
        .collect();
    let stamps = plan
        .targets
        .iter()
        .map(|t| t.file.clone())
        .chain([root.join("Project.toml"), root.join("project.toml")])
        .filter_map(|path| {
            let metadata = std::fs::metadata(&path).ok()?;
            Some((path, metadata.modified().ok()?, metadata.len()))
        })
        .collect();
    Ok(Snapshot {
        stamps,
        file: plan.entry.0,
        files,
        occurrences,
        declarations,
    })
}
impl Snapshot {
    pub fn fresh(&self) -> bool {
        self.stamps.iter().all(|(path, time, len)| {
            std::fs::metadata(path)
                .ok()
                .is_some_and(|m| m.modified().ok() == Some(*time) && m.len() == *len)
        })
    }
    pub fn affected(&self, dirty: &std::collections::HashSet<String>) -> bool {
        self.files
            .values()
            .any(|file| dirty.contains(&positions::uri(&file.path)))
    }

    fn occurrence(&self, start: usize, end: usize) -> Option<&Occurrence> {
        self.occurrences
            .get(&self.file)?
            .iter()
            .filter(|o| o.span.start <= start && o.span.end >= end)
            .min_by_key(|o| {
                (
                    o.span.end - o.span.start,
                    usize::from(o.definition.is_none()),
                )
            })
    }
    fn named(&self, index: usize) -> Option<&Declaration> {
        let file = &self.files[&self.file];
        let tokens = &file.lexed.tokens;
        let mut start = index;
        while start >= 2
            && tokens[start - 1].0.to_string() == "::"
            && matches!(tokens[start - 2].0, TokenPack::Ident(_))
        {
            start -= 2;
        }
        let name = tokens[start..=index]
            .iter()
            .map(|t| t.0.to_string())
            .collect::<String>();
        self.declarations
            .get(&format!("{}::{name}", file.module))
            .or_else(|| self.declarations.get(&name))
    }
    fn definition(&self, index: usize, o: Option<&Occurrence>) -> Option<Value> {
        let file = &self.files[&self.file];
        let token = &file.lexed.tokens[index];
        if let Some(d) = self.named(index).or_else(|| {
            o.and_then(|o| o.symbol.as_ref())
                .and_then(|name| self.declarations.get(name))
        }) {
            let target = self.files.get(&d.file)?;
            if target.text.is_empty() {
                return None;
            }
            return Some(
                json!({"uri":positions::uri(&target.path),"range":positions::range(&target.text,d.span.start,d.span.end)}),
            );
        }
        let (id, mut span) = o?.definition?;
        let target = self.files.get(&id.index())?;
        if span.end == 0 {
            let name = token.0.to_string();
            span = target
                .params
                .iter()
                .find(|(scope, p)| {
                    scope.start <= token.1.start && scope.end >= token.1.end && p.name == name
                })?
                .1
                .name_span;
        }
        if target.text.is_empty() || span.end > target.text.len() {
            return None;
        }
        Some(
            json!({"uri":positions::uri(&target.path),"range":positions::range(&target.text,span.start,span.end)}),
        )
    }
    pub fn query(&self, method: &str, params: &Value) -> Value {
        let file = &self.files[&self.file];
        if method == "textDocument/inlayHint" {
            let mut hints = vec![];
            let range_start = positions::offset(&file.text, &params["range"]["start"]).unwrap_or(0);
            let range_end =
                positions::offset(&file.text, &params["range"]["end"]).unwrap_or(file.text.len());
            for o in self.occurrences.get(&self.file).into_iter().flatten() {
                for (span, name) in &o.parameters {
                    if span.start > file.text.len() {
                        continue;
                    }
                    let position = positions::position(&file.text, span.start);
                    if let (Some(start), Some(end)) = (
                        positions::offset(&file.text, &params["range"]["start"]),
                        positions::offset(&file.text, &params["range"]["end"]),
                    ) {
                        if span.start < start || span.start > end {
                            continue;
                        }
                    }
                    let hint = json!({"position":position,"label":format!("{name}:"),"kind":2,"paddingRight":true});
                    if !hints.contains(&hint) {
                        hints.push(hint);
                    }
                }
            }
            for o in self.occurrences.get(&self.file).into_iter().flatten() {
                if o.span.end < range_start
                    || o.span.end > range_end
                    || o.kind != "variable"
                    || o.span.end > file.text.len()
                    || o.definition != Some((FileId::new(self.file), o.span))
                    || file.text[o.span.end..].trim_start().starts_with(':')
                {
                    continue;
                }
                let hint = json!({"position":positions::position(&file.text,o.span.end),"label":format!(": {}",o.ty),"kind":1,"paddingLeft":false});
                if !hints.contains(&hint) {
                    hints.push(hint);
                }
            }
            hints.sort_by_key(|h| {
                (
                    h["position"]["line"].as_u64(),
                    h["position"]["character"].as_u64(),
                )
            });
            return json!(hints);
        }
        if method == "textDocument/semanticTokens/full" {
            return self.tokens();
        }
        let Some(offset) = positions::offset(&file.text, &params["position"]) else {
            return Value::Null;
        };
        let Some((index, token)) = file
            .lexed
            .tokens
            .iter()
            .enumerate()
            .find(|(_, t)| t.1.start <= offset && offset < t.1.end)
        else {
            return Value::Null;
        };
        let occurrence = self.occurrence(token.1.start, token.1.end);
        if method == "textDocument/definition" {
            return self.definition(index, occurrence).unwrap_or(Value::Null);
        }
        let ty = occurrence
            .map(|o| o.ty.clone())
            .or_else(|| {
                file.params
                    .iter()
                    .find(|(_, p)| {
                        p.name_span.start <= token.1.start && p.name_span.end >= token.1.end
                    })
                    .and_then(|(_, p)| p.ty.as_ref())
                    .map(|(_, span)| file.text[span.start..span.end].to_owned())
            })
            .or_else(|| self.named(index).map(|d| d.ty.clone()));
        ty.map(|ty|json!({"contents":{"kind":"plaintext","value":ty},"range":positions::range(&file.text,token.1.start,token.1.end)})).unwrap_or(Value::Null)
    }
    fn tokens(&self) -> Value {
        let file = &self.files[&self.file];
        let mut tokens = vec![];
        for (index, (token, span)) in file.lexed.tokens.iter().enumerate() {
            let kind = match token {
                TokenPack::KeyWord(_) => "keyword",
                TokenPack::Operator(_) => "operator",
                TokenPack::Data { kind, .. } => {
                    if kind.to_string().contains("String")
                        || file.text[span.start..span.end].starts_with('"')
                    {
                        "string"
                    } else {
                        "number"
                    }
                }
                TokenPack::Ident(_) => self
                    .named(index)
                    .map(|d| d.kind)
                    .or_else(|| self.occurrence(span.start, span.end).map(|o| o.kind))
                    .unwrap_or("variable"),
            };
            tokens.push((*span, kind));
        }
        for c in &file.lexed.comments {
            tokens.push((c.span, "comment"));
        }
        tokens.sort_by_key(|t| t.0.start);
        let mut data = vec![];
        let (mut last_line, mut last_column) = (0, 0);
        for (span, kind) in tokens {
            let mut offset = span.start;
            for line in file.text[span.start..span.end].split_inclusive('\n') {
                let text = line.trim_end_matches(['\r', '\n']);
                if !text.is_empty() {
                    let pos = positions::position(&file.text, offset);
                    let row = pos["line"].as_u64().unwrap();
                    let column = pos["character"].as_u64().unwrap();
                    data.extend([
                        row - last_line,
                        if row == last_line {
                            column - last_column
                        } else {
                            column
                        },
                        text.encode_utf16().count() as u64,
                        super::LEGEND.iter().position(|k| *k == kind).unwrap_or(7) as u64,
                        0,
                    ]);
                    last_line = row;
                    last_column = column;
                }
                offset += line.len();
            }
        }
        json!({"data":data})
    }
}
