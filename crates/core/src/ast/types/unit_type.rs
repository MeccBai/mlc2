use crate::ast::arena::get_ident;
use crate::ast::attribute::UnitAttribute;
use crate::ast::config::Config;
use crate::ast::function::bindings;
use crate::ast::generic::GenericRequire;
use crate::ast::symbol_name::SymbolName;
use crate::ast::symbols::Resolution;
use crate::ast::types::CompileType::Unit;
use crate::ast::types::resolve_type;
use crate::ast::{
    GenericIndex, TypeArena, TypeIndex,
    symbols::{EnumBool, SymbolTable},
};
use crate::diagnostic::error::{CAbiError, CompileError, IllegalUseError, ResolveError};
use crate::diagnostic::ice::ice;
use crate::parser::out::{Span, TempUnit};
use crate::visibility::TempVisibilityExt;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

mod application;
pub use application::UnitApplication;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UnitMember {
    pub name: String,
    pub member_type: TypeIndex,
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitType {
    pub application: Option<UnitApplication>,
    pub name: String,
    pub members: Vec<UnitMember>,
    pub attributes: HashSet<UnitAttribute>,
    pub generics: Vec<String>,
    pub generic_map: HashMap<String, GenericIndex>,
    pub exported: bool,
}

impl UnitType {
    fn member_resort(&mut self, symbols: &dyn Resolution) {
        self.members.sort_by(|a, b| {
            a.member_type
                .size(symbols)
                .cmp(&b.member_type.size(symbols))
        });
    }

    pub fn finalize(
        config: &mut Config,
        prototype: TempUnit,
        symbols: &mut dyn Resolution,
    ) -> (Self, Span) {
        let name_span = prototype.name_span;
        let mut this = UnitType {
            application: None,
            name: prototype.name,
            attributes: UnitAttribute::parse(config, prototype.attributes, name_span),
            members: Vec::new(),
            generics: Vec::new(),
            generic_map: HashMap::new(),
            exported: prototype.visibility.normal_export(config, &name_span),
        };
        if this.attributes.contains(&UnitAttribute::Cabi) && !prototype.generics.is_empty() {
            crate::ast::attribute::validate::reject(config, CAbiError::GenericUnit, name_span);
            return (this, name_span);
        }
        if !this.attributes.contains(&UnitAttribute::Cabi) {
            this.member_resort(symbols);
        }
        let (generics, generic_map, _bindings) =
            bindings::generics(config, "unit", &this.name, prototype.generics, symbols);
        this.generics = generics;
        this.generic_map = generic_map;

        // Publish the generic header before resolving recursive member types.
        if !this.generics.is_empty() {
            if let Some(index) = symbols.local().generics.units.get_by_name(&this.name) {
                symbols.local_mut().generics.units.set(&index, this.clone());
            }
        }

        this.members = prototype
            .members
            .into_iter()
            .filter_map(|member| {
                let ty_index = resolve_type(config, member.ty, symbols, Some(&this))?;
                Some(UnitMember {
                    name: member.name,
                    member_type: ty_index,
                    public: member.public,
                })
            })
            .collect::<Vec<_>>();

        (this, name_span)
    }

    pub fn empty() -> Self {
        Self {
            application: None,
            name: String::new(),
            members: Vec::new(),
            attributes: HashSet::new(),
            generics: Vec::new(),
            generic_map: HashMap::new(),
            exported: false,
        }
    }

    pub fn format(&self) -> String {
        self.name.clone()
    }

    pub fn size(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> usize {
        let mut current_offset = 0;
        let mut max_align = 1;

        for member in &self.members {
            let m_size = member.member_type.size(arena);
            let m_align = member.member_type.align(arena).max(1);
            current_offset = (current_offset + m_align - 1) & !(m_align - 1);
            current_offset += m_size;
            if m_align > max_align {
                max_align = m_align;
            }
        }
        (current_offset + max_align - 1) & !(max_align - 1)
    }

    pub fn align(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> usize {
        let mut max_align = 1;

        self.members.iter().for_each(|member| {
            let align = member.member_type.align(arena);
            if align > max_align {
                max_align = align;
            }
        });

        max_align
    }

    pub fn dump(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> String {
        let unit_name = self.format();
        format!("unit:{},members: {:?}", unit_name, self.members)
    }

    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut dyn Resolution,
        actives: Option<&crate::ast::function::InstantiationActives>,
        span: Span,
    ) -> Option<TypeIndex> {
        let owner = self
            .application
            .as_ref()
            .map(|app| app.template.file_id())
            .or_else(|| {
                self.generic_map
                    .values()
                    .next()
                    .map(|index| index.file_id())
            })
            .unwrap_or(config.file_id());
        let exported = self.exported;
        crate::ast::symbols::owner::in_owner(
            config,
            symbols,
            owner,
            exported,
            span,
            |config, symbols| self.instantiate_local(config, params, symbols, actives, span),
        )
        .flatten()
    }

    fn instantiate_local(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut dyn Resolution,
        actives: Option<&crate::ast::function::InstantiationActives>,
        span: Span,
    ) -> Option<TypeIndex> {
        if let Some(application) = self.application {
            return application.instantiation(config, params, symbols, actives, span);
        }
        if self
            .generics
            .iter()
            .any(|name| !params.contains_key(&self.generic_map[name]))
        {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                span,
            );
            return None;
        } else if self.generics.len() == 0 {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::NonGenericInstantiation),
                span,
            );
            return None;
        }

        let arguments = self
            .generics
            .iter()
            .map(|name| params[&self.generic_map[name]])
            .collect::<Vec<_>>();
        if arguments.iter().any(|ty| ty.is_generic(symbols)) {
            let template = symbols
                .local()
                .generics
                .units
                .get_by_name(&self.name)
                .unwrap_or_else(|| {
                    symbols
                        .local_mut()
                        .generics
                        .units
                        .insert(self.name.clone(), self.clone())
                });
            return Some(Self::pending_application(template, arguments, symbols));
        }

        for generic in &self.generics {
            let generic = self
                .generic_map
                .get(generic)
                .unwrap_or_else(|| ice("Generic not found."));
            let Some(&argument) = params.get(generic) else {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                    span,
                );
                return None;
            };
            if !generic.check(argument, symbols) {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::RequirementUnmet),
                    span,
                );
                return None;
            }
        }

        let instance_name = self.generic_instance_name(symbols, params);
        let active_key = (config.file_id(), instance_name.clone());
        let ident = get_ident(&instance_name);

        if let Some(instance_index) = symbols.local().types.get_by_name(&ident) {
            return Some(instance_index);
        }

        let mut temp_instance = UnitType::empty();
        temp_instance.name = instance_name.clone();

        let holder_index = symbols.local_mut().types.insert(ident, Unit(temp_instance));

        let binding = crate::ast::function::InstantiationActives::default();
        let temp_actives = actives.unwrap_or(&binding);

        temp_actives.borrow_mut().insert(
            active_key.clone(),
            crate::ast::function::instantiate::InstanceIndex::Type(holder_index),
        );

        let new_members = self
            .members
            .iter()
            .map(|member| {
                if member.member_type.is_generic(symbols) {
                    let new_type = member.member_type.clone().instantiation(
                        config,
                        params,
                        symbols,
                        Some(temp_actives),
                        span,
                    )?;
                    Some(UnitMember {
                        name: member.name.clone(),
                        member_type: new_type,
                        public: member.public,
                    })
                } else {
                    Some(member.clone())
                }
            })
            .collect::<Option<Vec<UnitMember>>>();
        let Some(new_members) = new_members else {
            temp_actives.borrow_mut().remove(&active_key);
            symbols.local_mut().types.forget_name(&instance_name);
            return None;
        };

        let instance = UnitType {
            application: None,
            name: instance_name,
            members: new_members,
            attributes: self.attributes.clone(),
            generics: Vec::new(),
            generic_map: HashMap::new(),
            exported: self.exported,
        };

        temp_actives.borrow_mut().remove(&active_key);
        symbols.local_mut().types.set(&holder_index, Unit(instance));
        Some(holder_index)
    }

    pub fn has_generic(&self) -> bool {
        self.application.is_some() || !self.generics.is_empty()
    }

    pub fn generic_instance_name(
        &self,
        arena: &(impl crate::ast::types::TypeLookup + ?Sized),
        params: &HashMap<GenericIndex, TypeIndex>,
    ) -> String {
        if let Some(application) = &self.application {
            let arguments = application
                .arguments
                .iter()
                .map(|ty| ty.generic_instance_name(arena, params))
                .collect::<Vec<_>>();
            return SymbolName::generic_instance(&application.template_name, &arguments);
        }
        let elements = self
            .generics
            .iter()
            .map(|generic| {
                params
                    .get(
                        self.generic_map
                            .get(generic)
                            .unwrap_or_else(|| ice("Generic not found")),
                    )
                    .unwrap_or_else(|| ice("Generic param not found."))
                    .format(arena)
            })
            .collect::<Vec<_>>();

        SymbolName::generic_instance(&self.name, &elements)
    }

    pub fn get_member(&self, name: &str) -> Option<&UnitMember> {
        self.members.iter().find(|member| member.name == name)
    }
}

#[cfg(test)]
mod tests;
