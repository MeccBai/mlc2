//! Macro expansion is a syntax-to-syntax phase, before runtime semantic analysis.
use super::*;
use mlc_core::parser::out::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct Definition {
    owner: usize,
    value: TempMacroFunc,
}
struct Expander<'a> {
    plan: &'a BuildPlan,
    definitions: HashMap<String, Definition>,
    aliases: HashMap<String, (String, bool)>,
    visible: HashMap<usize, HashSet<usize>>,
    generated: Vec<(usize, TempGlobalStmt)>,
    serial: usize,
}
fn path(name: &str) -> TempPath {
    TempPath {
        segments: name.split("::").map(str::to_owned).collect(),
    }
}
fn ty(name: &str, span: Span) -> Spanned<TempType> {
    (TempType::Path(path(name)), span)
}
fn expr(name: &str, span: Span) -> Spanned<TempExpr> {
    (TempExpr::Path(path(name)), span)
}

pub(super) fn expand(
    plan: &BuildPlan,
    modules: &mut HashMap<usize, TempModule>,
) -> Result<(), String> {
    let mut ex = Expander {
        plan,
        definitions: HashMap::new(),
        aliases: HashMap::new(),
        visible: HashMap::new(),
        generated: vec![],
        serial: 0,
    };
    for target in &plan.targets {
        for (item, _) in &modules[&target.id.0] {
            match item {
                TempGlobalStmt::MacroFunc(value) => {
                    let name = format!("{}::{}", target.module_name, value.name);
                    if ex
                        .definitions
                        .insert(
                            name.clone(),
                            Definition {
                                owner: target.id.0,
                                value: value.clone(),
                            },
                        )
                        .is_some()
                    {
                        return Err(format!("Duplicate macro {name}!"));
                    }
                }
                TempGlobalStmt::Using(value) => {
                    if let TempType::Path(p) = &value.target.0 {
                        ex.aliases.insert(
                            format!("{}::{}", target.module_name, value.name),
                            (
                                p.segments.join("::"),
                                value.visibility == TempVisibility::Export,
                            ),
                        );
                    }
                }
                _ => {}
            }
        }
        let mut visible = HashSet::from([target.id.0]);
        let mut pending: Vec<_> = target.requires.iter().map(|id| id.0).collect();
        while let Some(id) = pending.pop() {
            if !visible.insert(id) {
                continue;
            }
            for (item, _) in &modules[&id] {
                if let TempGlobalStmt::Import(import) = item {
                    if import.exported() {
                        let name = import.path().join("::");
                        for dependency in &plan.targets[id].requires {
                            if plan.targets[dependency.0].module_name == name {
                                pending.push(dependency.0);
                            }
                        }
                    }
                }
            }
        }
        ex.visible.insert(target.id.0, visible);
    }
    for id in &plan.order {
        let mut module = modules.remove(&id.0).unwrap();
        module.retain(|(item, _)| !matches!(item, TempGlobalStmt::MacroFunc(_)));
        let mut constants = HashMap::new();
        for (item, _) in &module {
            if let TempGlobalStmt::Variable(variable) = item {
                if let Some(value) = constant_string(&variable.initializer, &constants) {
                    if variable.ty.is_none() || ex.is_str_type(variable.ty.as_ref(), id.0) {
                        constants.insert(variable.name.clone(), value);
                    }
                }
            }
        }
        for (item, _) in &mut module {
            match item {
                TempGlobalStmt::Func(function) => {
                    if let Some(scope) = &mut function.body {
                        ex.scope(scope, id.0, &constants, 0)?;
                    }
                }
                TempGlobalStmt::Interface(function) => {
                    if let Some(scope) = &mut function.body {
                        ex.scope(scope, id.0, &constants, 0)?;
                    }
                }
                TempGlobalStmt::Variable(variable) => {
                    ex.expression(&mut variable.initializer, id.0, &constants, 0)?
                }
                _ => {}
            }
        }
        modules.insert(id.0, module);
    }
    for (id, item) in ex.generated {
        modules.get_mut(&id).unwrap().push((item, (0..0).into()));
    }
    Ok(())
}
impl Expander<'_> {
    fn is_str_type(&self, ty: Option<&Spanned<TempType>>, owner: usize) -> bool {
        let Some((TempType::Path(path), _)) = ty else {
            return false;
        };
        let mut name = path.segments.join("::");
        for _ in 0..64 {
            if name == "str" {
                return true;
            }
            if !name.contains("::") {
                name = format!("{}::{name}", self.plan.targets[owner].module_name);
            }
            let Some((target, _)) = self.aliases.get(&name) else {
                return false;
            };
            name = target.clone();
        }
        false
    }

    fn scope(
        &mut self,
        scope: &mut TempScope,
        owner: usize,
        constants: &HashMap<String, Spanned<TempExpr>>,
        depth: usize,
    ) -> Result<(), String> {
        let mut constants = constants.clone();
        for (statement, _) in &mut scope.statements {
            match statement {
                TempStmt::Variable(v) => {
                    self.expression(&mut v.initializer, owner, &constants, depth)?;
                    if v.value_type == mlc_core::ast::types::ValueType::Constant
                        || ((v.ty.is_none() || self.is_str_type(v.ty.as_ref(), owner))
                            && constant_string(&v.initializer, &constants).is_some())
                    {
                        constants.insert(v.name.clone(), v.initializer.clone());
                    } else {
                        constants.remove(&v.name);
                    }
                }
                TempStmt::Assignment { target, value } => {
                    self.expression(target, owner, &constants, depth)?;
                    self.expression(value, owner, &constants, depth)?;
                }
                TempStmt::Expression(v) => self.expression(v, owner, &constants, depth)?,
                TempStmt::Return(Some(v)) => self.expression(v, owner, &constants, depth)?,
                TempStmt::If {
                    condition,
                    then_scope,
                    else_scope,
                } => {
                    self.expression(condition, owner, &constants, depth)?;
                    self.scope(then_scope, owner, &constants, depth)?;
                    if let Some(scope) = else_scope {
                        self.scope(scope, owner, &constants, depth)?;
                    }
                }
                TempStmt::While { condition, scope } => {
                    self.expression(condition, owner, &constants, depth)?;
                    self.scope(scope, owner, &constants, depth)?;
                }
                TempStmt::For {
                    start, end, scope, ..
                } => {
                    self.expression(start, owner, &constants, depth)?;
                    self.expression(end, owner, &constants, depth)?;
                    self.scope(scope, owner, &constants, depth)?;
                }
                TempStmt::Anonymous(scope) => self.scope(scope, owner, &constants, depth)?,
                TempStmt::Match { value, branches } => {
                    self.expression(value, owner, &constants, depth)?;
                    for (pattern, scope) in branches {
                        if let TempMatchPattern::Expression(v) = pattern {
                            self.expression(v, owner, &constants, depth)?;
                        }
                        self.scope(scope, owner, &constants, depth)?;
                    }
                }
                TempStmt::VariantMatch {
                    value, branches, ..
                } => {
                    self.expression(value, owner, &constants, depth)?;
                    for (_, scope) in branches {
                        self.scope(scope, owner, &constants, depth)?;
                    }
                }
                TempStmt::MacroFor { .. } => {
                    return Err("macro for is only valid inside a macro function".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn expression(
        &mut self,
        node: &mut Spanned<TempExpr>,
        owner: usize,
        constants: &HashMap<String, Spanned<TempExpr>>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err("Macro expansion exceeded 64 nested calls".into());
        }
        let span = node.1;
        match &mut node.0 {
            TempExpr::Group(v) | TempExpr::Unary { value: v, .. } => {
                self.expression(v, owner, constants, depth)?
            }
            TempExpr::Binary { operands, .. }
            | TempExpr::Array(operands)
            | TempExpr::Init {
                values: operands, ..
            } => {
                for v in operands {
                    self.expression(v, owner, constants, depth)?;
                }
            }
            TempExpr::Member { base, .. } => self.expression(base, owner, constants, depth)?,
            TempExpr::Call { callee, args } => {
                if let TempCallee::Expr(v) = callee {
                    self.expression(v, owner, constants, depth)?;
                }
                for v in args {
                    self.expression(v, owner, constants, depth)?;
                }
            }
            TempExpr::MacroCall { path: call, args } => {
                for arg in args.iter_mut() {
                    self.expression(arg, owner, constants, depth + 1)?;
                }
                let mut name = call.segments.join("::");
                let mut bridge_owner = None;
                if call.segments.len() == 1 {
                    name = format!("{}::{name}", self.plan.targets[owner].module_name);
                }
                for _ in 0..64 {
                    let Some((target, exported)) = self.aliases.get(&name) else {
                        break;
                    };
                    if !exported
                        && !name.starts_with(&format!("{}::", self.plan.targets[owner].module_name))
                    {
                        return Err(format!("Private macro alias {name}"));
                    }
                    bridge_owner = self
                        .plan
                        .targets
                        .iter()
                        .find(|module| name.starts_with(&format!("{}::", module.module_name)))
                        .map(|module| module.id.0);
                    name = target.clone();
                }
                let def = self
                    .definitions
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| format!("Unknown macro {name}!"))?;
                if def.owner != owner
                    && (def.value.visibility != TempVisibility::Export
                        || !self.visible[&owner].contains(&def.owner))
                {
                    return Err(format!("Macro {name}! is not imported or exported"));
                }
                if def.value.params.is_empty() {
                    return Err(format!("Macro {name}! requires an Args parameter"));
                }
                let pack = def.value.params.last().unwrap();
                let Some((
                    TempType::Generic {
                        base,
                        args: requirements,
                    },
                    _,
                )) = &pack.ty
                else {
                    return Err("Macro pack must have type std::macro::Args<Requirement>".into());
                };
                if base.segments.join("::") != "std::macro::Args" || requirements.len() != 1 {
                    return Err("Macro pack must have type std::macro::Args<Requirement>".into());
                }
                let TempType::Path(requirement) = &requirements[0].0 else {
                    return Err("Args requires a named generic requirement".into());
                };
                let fixed = def.value.params.len() - 1;
                if name == "std::fmt::println" && args.is_empty() {
                    args.push((
                        TempExpr::Literal {
                            kind: TempLiteralKind::String,
                            text: "\"\"".into(),
                        },
                        span,
                    ));
                }
                let standard_format = matches!(
                    name.as_str(),
                    "std::fmt::format" | "std::fmt::print" | "std::fmt::println"
                );
                if args.len() < fixed {
                    return Err(format!("Macro {name}! expects at least {fixed} arguments"));
                }
                for (i, param) in def.value.params.iter().take(fixed).enumerate() {
                    if self.is_str_type(param.ty.as_ref(), def.owner) {
                        constant_string(&args[i], constants).ok_or_else(|| {
                            format!(
                                "{name}!: str parameter {} requires a compile-time string",
                                param.name
                            )
                        })?;
                    }
                }
                if standard_format {
                    let mut pattern = args[0].clone();
                    for _ in 0..64 {
                        match &pattern.0 {
                            TempExpr::Group(v) => pattern = (**v).clone(),
                            TempExpr::Path(p) if p.segments.len() == 1 => {
                                if let Some(v) = constants.get(&p.segments[0]) {
                                    pattern = v.clone();
                                } else {
                                    break;
                                }
                            }
                            _ => break,
                        }
                    }
                    let TempExpr::Literal {
                        kind: TempLiteralKind::String,
                        text,
                    } = &pattern.0
                    else {
                        return Err("format! requires a compile-time string pattern".into());
                    };
                    let slots = count_slots(text)?;
                    if slots != args.len() - fixed {
                        return Err(format!(
                            "format!: {slots} placeholders but {} arguments",
                            args.len() - fixed
                        ));
                    }
                }
                self.serial += 1;
                let helper = format!("__mlc_macro_{}", self.serial);
                let mut substitutions = HashMap::new();
                let mut parameters = vec![];
                let mut generics = vec![];
                for (i, parameter) in def.value.params.iter().take(fixed).enumerate() {
                    let ident = format!("__macro_arg_{i}");
                    substitutions.insert(parameter.name.clone(), ident.clone());
                    parameters.push(TempParam {
                        name: ident,
                        name_span: span,
                        ty: parameter.ty.clone(),
                    });
                }
                let expanded_args = args.clone();
                for i in fixed..args.len() {
                    let ident = format!("__macro_arg_{i}");
                    let generic = format!("__MacroType{i}");
                    generics.push(TempGenericParam {
                        name: generic.clone(),
                        name_span: span,
                        constraint: Some((requirement.clone(), span)),
                    });
                    parameters.push(TempParam {
                        name: ident,
                        name_span: span,
                        ty: Some(ty(&generic, span)),
                    });
                }
                let mut body = def.value.body.clone();
                substitute_scope(&mut body, &substitutions);
                unroll(&mut body, &pack.name, fixed, args.len(), span)?;
                let helper_constants = def
                    .value
                    .params
                    .iter()
                    .take(fixed)
                    .enumerate()
                    .filter_map(|(i, p)| {
                        if self.is_str_type(p.ty.as_ref(), def.owner) {
                            constant_string(&args[i], constants)
                                .map(|value| (format!("__macro_arg_{i}"), value))
                        } else {
                            None
                        }
                    })
                    .collect();
                self.scope(&mut body, def.owner, &helper_constants, depth + 1)?;
                let function = TempFunc {
                    symbol: TempFuncSymbol {
                        visibility: TempVisibility::Export,
                        name: helper.clone(),
                        name_span: span,
                        generics,
                        params: parameters,
                        return_type: def.value.return_type.clone(),
                        attributes: vec![],
                    },
                    body: Some(body),
                };
                let mut call_owner = def.owner;
                if let Some(bridge) = bridge_owner.filter(|bridge| *bridge != def.owner) {
                    let mut forwarding = function.clone();
                    let arguments = forwarding
                        .symbol
                        .params
                        .iter()
                        .map(|p| expr(&p.name, span))
                        .collect();
                    let call = (
                        TempExpr::Call {
                            callee: TempCallee::Expr(Box::new(expr(
                                &format!("{}::{helper}", self.plan.targets[def.owner].module_name),
                                span,
                            ))),
                            args: arguments,
                        },
                        span,
                    );
                    let statement = if forwarding.symbol.return_type.is_some() {
                        TempStmt::Return(Some(call))
                    } else {
                        TempStmt::Expression(call)
                    };
                    forwarding.body = Some(TempScope {
                        statements: vec![(statement, span)],
                    });
                    self.generated
                        .push((bridge, TempGlobalStmt::Func(forwarding)));
                    call_owner = bridge;
                }
                self.generated
                    .push((def.owner, TempGlobalStmt::Func(function)));
                node.0 = TempExpr::Call {
                    callee: TempCallee::Expr(Box::new(expr(
                        &format!("{}::{helper}", self.plan.targets[call_owner].module_name),
                        span,
                    ))),
                    args: expanded_args,
                };
            }
            _ => {}
        }
        Ok(())
    }
}
fn count_slots(text: &str) -> Result<usize, String> {
    let mut chars = text.chars().peekable();
    let mut count = 0;
    while let Some(ch) = chars.next() {
        match ch {
            '{' => match chars.next() {
                Some('{') => {}
                Some('}') => count += 1,
                _ => return Err("format!: expected {} or escaped {{".into()),
            },
            '}' => {
                if chars.next() != Some('}') {
                    return Err("format!: unmatched }; use }} for a literal brace".into());
                }
            }
            _ => {}
        }
    }
    Ok(count)
}
fn substitute_expr(node: &mut Spanned<TempExpr>, names: &HashMap<String, String>) {
    match &mut node.0 {
        TempExpr::Path(p) if p.segments.len() == 1 => {
            if let Some(name) = names.get(&p.segments[0]) {
                p.segments[0] = name.clone();
            }
        }
        TempExpr::Group(v)
        | TempExpr::Unary { value: v, .. }
        | TempExpr::Member { base: v, .. } => substitute_expr(v, names),
        TempExpr::Binary { operands, .. }
        | TempExpr::Array(operands)
        | TempExpr::Init {
            values: operands, ..
        } => {
            for v in operands {
                substitute_expr(v, names)
            }
        }
        TempExpr::Call { callee, args } => {
            if let TempCallee::Expr(v) = callee {
                substitute_expr(v, names)
            }
            for v in args {
                substitute_expr(v, names)
            }
        }
        TempExpr::MacroCall { args, .. } => {
            for v in args {
                substitute_expr(v, names)
            }
        }
        _ => {}
    }
}
fn substitute_scope(scope: &mut TempScope, names: &HashMap<String, String>) {
    for (statement, _) in &mut scope.statements {
        match statement {
            TempStmt::Variable(v) => substitute_expr(&mut v.initializer, names),
            TempStmt::Expression(v) | TempStmt::Return(Some(v)) => substitute_expr(v, names),
            TempStmt::Assignment { target, value } => {
                substitute_expr(target, names);
                substitute_expr(value, names)
            }
            TempStmt::MacroFor { scope, .. } | TempStmt::Anonymous(scope) => {
                substitute_scope(scope, names)
            }
            TempStmt::If {
                condition,
                then_scope,
                else_scope,
            } => {
                substitute_expr(condition, names);
                substitute_scope(then_scope, names);
                if let Some(scope) = else_scope {
                    substitute_scope(scope, names)
                }
            }
            TempStmt::While { condition, scope } => {
                substitute_expr(condition, names);
                substitute_scope(scope, names)
            }
            TempStmt::For {
                start, end, scope, ..
            } => {
                substitute_expr(start, names);
                substitute_expr(end, names);
                substitute_scope(scope, names)
            }
            TempStmt::Match { value, branches } => {
                substitute_expr(value, names);
                for (_, scope) in branches {
                    substitute_scope(scope, names)
                }
            }
            TempStmt::VariantMatch {
                value, branches, ..
            } => {
                substitute_expr(value, names);
                for (_, scope) in branches {
                    substitute_scope(scope, names)
                }
            }
            _ => {}
        }
    }
}
fn unroll(
    scope: &mut TempScope,
    pack: &str,
    start: usize,
    end: usize,
    span: Span,
) -> Result<(), String> {
    let mut out = vec![];
    for (statement, location) in std::mem::take(&mut scope.statements) {
        match statement {
            TempStmt::MacroFor {
                binding,
                pack: source,
                scope: body,
            } => {
                if source.0 != pack {
                    return Err(format!("Unknown macro argument pack {}", source.0));
                }
                for i in start..end {
                    let mut expanded = body.clone();
                    substitute_scope(
                        &mut expanded,
                        &HashMap::from([(binding.0.clone(), format!("__macro_arg_{i}"))]),
                    );
                    unroll(&mut expanded, pack, start, end, span)?;
                    out.push((TempStmt::Anonymous(expanded), location));
                }
            }
            mut statement => {
                match &mut statement {
                    TempStmt::Anonymous(scope)
                    | TempStmt::While { scope, .. }
                    | TempStmt::For { scope, .. } => unroll(scope, pack, start, end, span)?,
                    TempStmt::If {
                        then_scope,
                        else_scope,
                        ..
                    } => {
                        unroll(then_scope, pack, start, end, span)?;
                        if let Some(scope) = else_scope {
                            unroll(scope, pack, start, end, span)?
                        }
                    }
                    _ => {}
                }
                out.push((statement, location));
            }
        }
    }
    scope.statements = out;
    Ok(())
}

fn constant_string(
    value: &Spanned<TempExpr>,
    constants: &HashMap<String, Spanned<TempExpr>>,
) -> Option<Spanned<TempExpr>> {
    let mut value = value.clone();
    for _ in 0..64 {
        match &value.0 {
            TempExpr::Literal {
                kind: TempLiteralKind::String,
                ..
            } => return Some(value),
            TempExpr::Group(inner) => value = (**inner).clone(),
            TempExpr::Path(path) if path.segments.len() == 1 => {
                value = constants.get(&path.segments[0])?.clone()
            }
            _ => return None,
        }
    }
    None
}
