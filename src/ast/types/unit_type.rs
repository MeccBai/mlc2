use crate::ast::arena::get_ident;
use crate::ast::config::Config;
use crate::ast::types::CompileType::Unit;
use crate::ast::types::resolve_type;
use crate::ast::{GenericIndex, SymbolTable, TypeArena, TypeIndex};
use crate::error::ice::ice;
use crate::error::{CompileError, IllegalUseError, ResolveError};
use crate::parser::out::{Span, TempUnit};
use std::collections::HashMap;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UnitMember {
    pub name: String,
    pub member_type: TypeIndex,
    pub public: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UnitType {
    pub name: String,
    pub members: Vec<UnitMember>,
    pub attributes: Vec<String>,
    pub generics: Vec<GenericIndex>,
    pub exported: bool,
}

impl UnitType {
    pub fn new(config: &mut Config, prototype: TempUnit, symbols: &mut SymbolTable) -> Self {
        let mut this = UnitType {
            name: prototype.name,
            attributes: prototype.attributes,
            members: Vec::new(),
            generics: Vec::new(),
            exported: prototype.visibility.normal_export(),
        };

        let mut generics = HashMap::<String, GenericIndex>::new();

        this.generics = prototype
            .generics
            .into_iter()
            .filter_map(|generic| {
                let (path, span) = generic.constraint?;
                let constraint = path.join();
                let ident = get_ident(&constraint);
                let Some(index) = symbols.generics.requires.get_by_ident(ident) else {
                    config
                        .submit_error(CompileError::Resolve(ResolveError::UnknownConstraint), span);
                    return None;
                };
                generics.insert(generic.name, index);
                Some(index)
            })
            .collect::<Vec<_>>();

        this.members = prototype
            .members
            .into_iter()
            .filter_map(|member| {
                let ty_index = resolve_type(config, member.ty, symbols)?;
                Some(UnitMember {
                    name: member.name,
                    member_type: ty_index,
                    public: member.public,
                })
            })
            .collect::<Vec<_>>();

        this
    }

    pub fn empty() -> Self {
        Self {
            name: String::new(),
            members: Vec::new(),
            attributes: Vec::new(),
            generics: Vec::new(),
            exported: false,
        }
    }

    pub fn format(&self) -> String {
        self.name.clone()
    }

    pub fn size(&self, arena: &TypeArena) -> usize {
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

    pub fn align(&self, arena: &TypeArena) -> usize {
        let mut max_align = 1;

        self.members.iter().for_each(|member| {
            let align = member.member_type.align(arena);
            if align > max_align {
                max_align = align;
            }
        });

        max_align
    }

    pub fn dump(&self, arena: &TypeArena) -> String {
        let unit_name = self.format();
        format!("unit:{},members: {:?}", unit_name, self.members)
    }

    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut SymbolTable,
        actives: Option<&mut HashMap<String, TypeIndex>>,
        span: Span,
    ) -> Option<TypeIndex> {
        if self.generics.len() != params.len() {
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

        for generic in &self.generics {
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

        let instance_name = self.generic_instance_name(&symbols.types, params);
        let ident = get_ident(&instance_name);

        if let Some(instance_index) = symbols.types.get_by_name(&ident) {
            return Some(instance_index);
        }

        let temp_instance = UnitType::empty();

        let holder_index = symbols.types.insert(ident, Unit(temp_instance));

        let mut binding = HashMap::<String, TypeIndex>::new();
        let temp_actives = actives.unwrap_or(&mut binding);

        temp_actives.insert(instance_name.clone(), holder_index);

        let new_members = self
            .members
            .iter()
            .map(|member| {
                if member.member_type.is_generic(&symbols.types) {
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
            .collect::<Option<Vec<UnitMember>>>()?;

        let instance = UnitType {
            name: instance_name,
            members: new_members,
            attributes: self.attributes.clone(),
            generics: Vec::new(),
            exported: self.exported,
        };

        symbols.types.set(&holder_index, Unit(instance));
        Some(holder_index)
    }

    pub fn has_generic(&self) -> bool {
        !self.generics.is_empty()
    }

    pub fn generic_instance_name(
        &self,
        arena: &TypeArena,
        params: &HashMap<GenericIndex, TypeIndex>,
    ) -> String {
        let elements = self
            .generics
            .iter()
            .map(|generic| {
                params
                    .get(generic)
                    .unwrap_or_else(|| ice("Generic param not found."))
                    .format(arena)
            })
            .collect::<Vec<_>>()
            .join(",");

        format!("{}<{}>", self.name, elements)
    }
}
