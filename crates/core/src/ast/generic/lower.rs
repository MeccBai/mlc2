//! Resolve compositions during AST lowering. The result is the ordinary flat
//! GenericRequire::requires list; no composition survives into semantic checking.
use super::*;
use crate::ast::symbols::ExportSymbol;
use std::collections::{HashSet, VecDeque};

impl GenericRequire {
    fn reference_name(config: &Config, path: &TempPath, symbols: &dyn Resolution) -> String {
        let name = path.segments.join("::");
        symbols.using_target(config, &name).unwrap_or(name)
    }

    fn local_name<'a>(config: &Config, name: &'a str) -> &'a str {
        name.strip_prefix(&format!("{}::", config.symbol_prefix()))
            .unwrap_or(name)
    }

    pub(crate) fn find(
        config: &Config,
        path: &TempPath,
        symbols: &dyn Resolution,
    ) -> Option<GenericIndex> {
        let name = Self::reference_name(config, path, symbols);
        symbols
            .local()
            .generics
            .requires
            .get_by_name(&Self::local_name(config, &name).to_owned())
            .or_else(|| symbols.local().generics.requires.get_by_name(&name))
            .or_else(|| match symbols.named_export(config, &name) {
                Some(ExportSymbol::GenericRequire(index)) => Some(index),
                _ => None,
            })
    }

    pub(crate) fn lower_all(
        config: &mut Config,
        prototypes: Vec<TempGeneric>,
        symbols: &mut dyn Resolution,
    ) {
        let names: HashMap<_, _> = prototypes
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.clone(), i))
            .collect();
        let mut remaining = vec![0; prototypes.len()];
        let mut users = vec![Vec::new(); prototypes.len()];
        for (i, prototype) in prototypes.iter().enumerate() {
            let mut dependencies = HashSet::new();
            for (requirement, _) in &prototype.requirements {
                let TempConstraints::Type {
                    path,
                    argument: None,
                } = requirement
                else {
                    continue;
                };
                if path.segments.starts_with(&["std".into(), "generic".into()]) {
                    continue;
                }
                let name = Self::reference_name(config, path, symbols);
                if let Some(&dependency) = names.get(Self::local_name(config, &name)) {
                    if dependencies.insert(dependency) {
                        remaining[i] += 1;
                        users[dependency].push(i);
                    }
                }
            }
        }
        let mut ready: VecDeque<_> = remaining
            .iter()
            .enumerate()
            .filter_map(|(i, &n)| (n == 0).then_some(i))
            .collect();
        while let Some(i) = ready.pop_front() {
            if config.is_poisoned() {
                return;
            }
            let (requirement, _) = Self::new(config, prototypes[i].clone(), symbols);
            if config.is_poisoned() {
                return;
            }
            symbols
                .local_mut()
                .generics
                .requires
                .insert(requirement.name.clone(), requirement);
            for &user in &users[i] {
                remaining[user] -= 1;
                if remaining[user] == 0 {
                    ready.push_back(user);
                }
            }
        }
        if let Some(i) = remaining.iter().position(|&n| n != 0) {
            config.submit_error(
                CompileError::Resolve(ResolveError::Constraint(
                    ConstraintError::CyclicRequirements {
                        name: prototypes[i].name.clone(),
                    },
                )),
                prototypes[i].name_span,
            );
        }
    }

    pub(super) fn merge(
        requires: &mut Vec<Constraints>,
        additions: Vec<Constraints>,
        symbols: &dyn Resolution,
    ) -> Result<(), String> {
        for rule in additions {
            if let Constraints::Interface(interface) = &rule {
                let mut duplicate = false;
                for existing in requires.iter() {
                    let Constraints::Interface(previous) = existing else {
                        continue;
                    };
                    if previous.name != interface.name {
                        continue;
                    }
                    if !interface_compatible(previous, interface, symbols, false) {
                        return Err(format!(
                            "incompatible signatures for interface {}",
                            interface.name
                        ));
                    }
                    duplicate |= interface_compatible(previous, interface, symbols, true);
                }
                if duplicate {
                    continue;
                }
            } else if requires.contains(&rule) {
                continue;
            }
            requires.push(rule);
            validate_types(requires)?;
        }
        Ok(())
    }
}

fn validate_types(requires: &[Constraints]) -> Result<(), String> {
    let (mut integer, mut float, mut signed) = (false, false, false);
    let (mut minimum, mut maximum) = (0, usize::MAX);
    for rule in requires {
        let Constraints::Type(rule) = rule else {
            continue;
        };
        match rule {
            GenericTypeRequire::Integer => integer = true,
            GenericTypeRequire::Float => float = true,
            GenericTypeRequire::Signed => signed = true,
            GenericTypeRequire::MinBits(bits) => minimum = minimum.max(*bits),
            GenericTypeRequire::MaxBits(bits) => maximum = maximum.min(*bits),
        }
    }
    if integer && float {
        return Err("is_integer and is_float are mutually exclusive".into());
    }
    if minimum > maximum {
        return Err(format!("min_bits<{minimum}> exceeds max_bits<{maximum}>"));
    }
    if minimum / 8 + usize::from(minimum % 8 != 0) > maximum / 8 {
        return Err(format!(
            "no byte-aligned type satisfies the bit-width range {minimum}..={maximum}"
        ));
    }
    if integer || float || signed {
        let widths: &[usize] = if float { &[32, 64] } else { &[8, 16, 32, 64] };
        if !widths
            .iter()
            .any(|&width| minimum <= width && width <= maximum)
        {
            return Err(format!(
                "no supported numeric type satisfies the bit-width range {minimum}..={maximum}"
            ));
        }
    }
    Ok(())
}

fn interface_compatible(
    a: &InterfaceRequire,
    b: &InterfaceRequire,
    symbols: &dyn Resolution,
    exact: bool,
) -> bool {
    a.has_self == b.has_self
        && a.mutable == b.mutable
        && a.params.len() == b.params.len()
        && match (a.ret_type, b.ret_type) {
            (None, None) => true,
            (Some(a), Some(b)) => types_compatible(a, b, symbols, exact),
            _ => false,
        }
        && a.params
            .iter()
            .zip(&b.params)
            .all(|((a, _), (b, _))| types_compatible(*a, *b, symbols, exact))
}

fn types_compatible(a: TypeIndex, b: TypeIndex, symbols: &dyn Resolution, exact: bool) -> bool {
    if a == b {
        return true;
    }
    match (
        symbols.get_type(a).unqualified(),
        symbols.get_type(b).unqualified(),
    ) {
        (CompileType::Ref(a), CompileType::Ref(b)) => {
            a.level == b.level
                && (!exact || (a.mut_base == b.mut_base && a.ownership == b.ownership))
                && types_compatible(a.base, b.base, symbols, exact)
        }
        (CompileType::List(a), CompileType::List(b)) => {
            a.length == b.length && types_compatible(a.element_type, b.element_type, symbols, exact)
        }
        // Local units are still declaration shells at this point. Distinct shell
        // indices must not compare equal merely because both names are empty.
        (CompileType::Unit(a), CompileType::Unit(b)) => !a.name.is_empty() && a.name == b.name,
        (CompileType::Function(a), CompileType::Function(b)) => {
            a.c_abi == b.c_abi
                && a.variadic == b.variadic
                && a.params.len() == b.params.len()
                && a.params
                    .iter()
                    .zip(&b.params)
                    .all(|(a, b)| types_compatible(*a, *b, symbols, true))
                && match (a.returns, b.returns) {
                    (None, None) => true,
                    (Some(a), Some(b)) => types_compatible(a, b, symbols, true),
                    _ => false,
                }
        }
        (a, b) => a == b,
    }
}
