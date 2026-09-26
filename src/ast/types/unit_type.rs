use chumsky::primitive::todo;

use crate::ast::arena::{GenericArena, InterfaceIndex, get_ident};
use crate::ast::config::Config;
use crate::ast::generic::{Constraints, InsFailed};
use crate::ast::types::CompileType::{Generic, Unit};
use crate::ast::types::resolve_type;
use crate::ast::{GenericIndex, SymbolTable, TypeArena, TypeIndex, stmt::Statement};
use crate::error::ice::ice;
use crate::parser::Visibility::{self, Export, Private};
use crate::parser::out::{TempType, TempUnit};
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum UnitError {
    GenericNotFound,
    MemberTypeNotFound,
    MemberTypeIsGeneric,
}

impl UnitType {
    pub fn new(
        config: &Config,
        prototype: TempUnit,
        symbols: &mut SymbolTable,
    ) -> Result<UnitType, UnitError> {
        let mut this = UnitType {
            name: prototype.name,
            attributes: prototype.attributes,
            members: Vec::new(),
            generics: Vec::new(),
            exported: match prototype.visibility {
                Export => true,
                Private => false,
                _ => ice("Unexpected visibility for unit type."),
            },
        };

        let mut generics = HashMap::<String, GenericIndex>::new();

        this.generics = prototype
            .generics
            .into_iter()
            .map(|generic| {
                let constraint = generic
                    .constraint
                    .unwrap_or_else(|| ice("Generic constraint not found."))
                    .join();
                let ident = get_ident(&constraint);
                let index = symbols
                    .generics
                    .requires
                    .get_by_ident(ident)
                    .unwrap_or_else(|| ice("Generic constraint type not found."));
                generics.insert(generic.name, index);
                index
            })
            .collect::<Vec<_>>();

        this.members = prototype
            .members
            .into_iter()
            .map(|member| {
                let ty_index = resolve_type(config, member.ty.0, symbols).unwrap();
                UnitMember {
                    name: member.name,
                    member_type: ty_index,
                    public: member.public,
                }
            })
            .collect::<Vec<_>>();

        Ok(this)
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
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut SymbolTable,
        actives: Option<&mut HashMap<String, TypeIndex>>,
    ) -> Result<TypeIndex, InsFailed> {
        if self.generics.len() != params.len() {
            return Err(InsFailed::CountMismatch);
        } else if self.generics.len() == 0 {
            return Err(InsFailed::NoGenerics);
        }

        let instance_name = self.generic_instance_name(&symbols.types, params);
        let ident = get_ident(&instance_name);

        if let Some(instance_index) = symbols.types.get_by_ident(ident) {
            return Ok(instance_index);
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
                        params,
                        symbols,
                        Some(temp_actives),
                    )?;
                    Ok(UnitMember {
                        name: member.name.clone(),
                        member_type: new_type,
                        public: member.public,
                    })
                } else {
                    Ok(member.clone())
                }
            })
            .collect::<Result<Vec<UnitMember>, InsFailed>>()?;

        let instance = UnitType {
            name: instance_name,
            members: new_members,
            attributes: self.attributes.clone(),
            generics: Vec::new(),
            exported: self.exported,
        };

        symbols.types.insert(ident, Unit(instance));

        Ok(symbols.types.get_by_ident(ident).unwrap())
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
