//! Preserve the first failed requirement instead of reducing it to a boolean.
use super::*;

impl GenericIndex {
    pub(crate) fn check_and_submit(
        self,
        param: TypeIndex,
        symbols: &dyn Resolution,
        config: &mut Config,
        span: Span,
    ) -> bool {
        let generic = symbols.get_generic(self);
        for constraint in &generic.requires {
            let (rule, reason) = match constraint {
                Constraints::Type(rule) if !rule.check(param, symbols) => {
                    let (name, reason) = match rule {
                        GenericTypeRequire::Integer => (
                            "std::generic::is_integer".into(),
                            "an integer type is required".into(),
                        ),
                        GenericTypeRequire::Float => (
                            "std::generic::is_float".into(),
                            "a floating-point type is required".into(),
                        ),
                        GenericTypeRequire::Signed => (
                            "std::generic::is_signed".into(),
                            "a signed numeric type is required".into(),
                        ),
                        GenericTypeRequire::MinBits(bits) => (
                            format!("std::generic::min_bits({bits})"),
                            format!(
                                "requires at least {bits} bits, but the type has {} bits",
                                param.size(symbols) * 8
                            ),
                        ),
                        GenericTypeRequire::MaxBits(bits) => (
                            format!("std::generic::max_bits({bits})"),
                            format!(
                                "requires at most {bits} bits, but the type has {} bits",
                                param.size(symbols) * 8
                            ),
                        ),
                    };
                    (name, reason)
                }
                Constraints::Interface(rule) => {
                    let Some(reason) = rule.failure(param, symbols) else {
                        continue;
                    };
                    let receiver = if !rule.has_self {
                        ""
                    } else if rule.mutable {
                        "mut self"
                    } else {
                        "self"
                    };
                    let mut params = vec![];
                    if !receiver.is_empty() {
                        params.push(receiver.to_owned());
                    }
                    params.extend(
                        rule.params
                            .iter()
                            .map(|(ty, name)| format!("{name}:{}", ty.format(symbols))),
                    );
                    let ret = rule
                        .ret_type
                        .map(|ty| format!(" -> {}", ty.format(symbols)))
                        .unwrap_or_default();
                    (
                        format!("func {}({}){ret}", rule.name, params.join(", ")),
                        reason,
                    )
                }
                _ => continue,
            };
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::RequirementUnmet {
                    ty: param.format(symbols),
                    requirement: rule,
                    reason,
                }),
                span,
            );
            return false;
        }
        true
    }
}

impl InterfaceRequire {
    pub(super) fn failure(&self, param: TypeIndex, symbols: &dyn Resolution) -> Option<String> {
        let param_name = symbols.get_type(param).unqualified().format(symbols);
        let name = SymbolName::callable(Some(&param_name), &self.name);
        let Some(index) = symbols.associated_interface(param, &name) else {
            return Some(format!("required interface `{name}` was not found"));
        };
        let interface = symbols.get_interface_regular(index);
        if !interface.public {
            return Some(format!("interface `{name}` is not public"));
        }
        let valid_return = match (self.ret_type, interface.ret_type) {
            (Some(expected), Some(found)) => expected.type_check(false, &found, symbols),
            (None, None) => true,
            _ => false,
        };
        if !valid_return {
            let format = |ty: Option<TypeIndex>| {
                ty.map(|ty| ty.format(symbols))
                    .unwrap_or_else(|| "void".into())
            };
            return Some(format!(
                "return type must be {}, but found {}",
                format(self.ret_type),
                format(interface.ret_type)
            ));
        }
        if self.mutable != interface.mutable {
            return Some("receiver mutability does not match the requirement".into());
        }
        if self.has_self != interface.has_self {
            return Some("self parameter does not match the requirement".into());
        }
        if self.params.len() != interface.params.len() {
            return Some(format!(
                "expected {} parameters, but found {}",
                self.params.len(),
                interface.params.len()
            ));
        }
        for (position, ((expected, _), (found, _))) in
            self.params.iter().zip(&interface.params).enumerate()
        {
            if !expected.type_check(false, found, symbols) {
                return Some(format!(
                    "parameter {} must have type {}, but found {}",
                    position + 1,
                    expected.format(symbols),
                    found.format(symbols)
                ));
            }
        }
        None
    }
}
