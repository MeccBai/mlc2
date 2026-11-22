use super::*;
use crate::ast::function::{InstantiationActives, instantiate::InstanceIndex};

impl GenericIndex {
    pub(crate) fn check_bound(
        self,
        subject: TypeIndex,
        bindings: &HashMap<GenericIndex, TypeIndex>,
        config: &mut Config,
        symbols: &mut dyn Resolution,
        actives: &InstantiationActives,
        span: Span,
    ) -> bool {
        let application = symbols.get_generic(self).application.clone();
        let Some((template, arguments)) = application else {
            return self.check_and_submit(subject, symbols, config, span);
        };
        let Some(arguments) = arguments
            .into_iter()
            .map(|ty| ty.instantiation(config, bindings, symbols, Some(actives), span))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        template
            .instantiate_requirement(&arguments, config, symbols, actives, span)
            .is_some_and(|instance| instance.check_and_submit(subject, symbols, config, span))
    }

    fn instantiate_requirement(
        self,
        arguments: &[TypeIndex],
        config: &mut Config,
        symbols: &mut dyn Resolution,
        actives: &InstantiationActives,
        span: Span,
    ) -> Option<GenericIndex> {
        let template = symbols.get_generic(self).clone();
        if template.parameters.len() != arguments.len() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                span,
            );
            return None;
        }
        let arguments_names = arguments
            .iter()
            .map(|argument| argument.format(symbols))
            .collect::<Vec<_>>();
        let name = SymbolName::generic_instance(&format!("$require.{self:?}"), &arguments_names);
        let key = (self.file_id(), name.clone());
        if actives.borrow().contains_key(&key) {
            config.submit_error(
                CompileError::Resolve(ResolveError::Constraint(
                    ConstraintError::CyclicRequirements {
                        name: template.name.clone(),
                    },
                )),
                span,
            );
            return None;
        }
        if let Some(index) = symbols.local().generics.requires.get_by_name(&name) {
            return Some(index);
        }
        actives
            .borrow_mut()
            .insert(key.clone(), InstanceIndex::Requirement);
        let result = (|| {
            let bindings = template
                .parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied())
                .collect::<HashMap<_, _>>();
            for (&parameter, &argument) in template.parameters.iter().zip(arguments) {
                if !parameter.check_bound(argument, &bindings, config, symbols, actives, span) {
                    return None;
                }
            }
            let mut instance = GenericRequire::empty(name.clone());
            instance.requires = template.requires;
            for rule in &mut instance.requires {
                if let Constraints::Interface(interface) = rule {
                    for (param, _) in &mut interface.params {
                        *param =
                            param.instantiation(config, &bindings, symbols, Some(actives), span)?;
                    }
                    if let Some(ty) = &mut interface.ret_type {
                        *ty = ty.instantiation(config, &bindings, symbols, Some(actives), span)?;
                    }
                }
            }
            Some(symbols.local_mut().generics.requires.insert(name, instance))
        })();
        actives.borrow_mut().remove(&key);
        result
    }
}
