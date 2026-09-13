use crate::ast::arena::{get_ident, GenericArena};
use crate::ast::{GenericIndex, TypeArena, TypeIndex, stmt::Statement};
use crate::parser::out::{TempType, TempUnit};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UnitMember {
    pub name: String,
    pub member_type: TypeIndex,
    pub public: bool,
}

pub struct Interface {
    pub name: String,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
    pub body: Vec<Statement>,
    pub attributes: Vec<String>,
    pub generics: Vec<GenericIndex>,
    pub public: bool,
    pub immutable: bool,
    pub exported: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UnitType {
    pub name: String,
    pub members: Vec<UnitMember>,
    pub attributes: Vec<String>,
    pub generics: Vec<GenericIndex>,
    pub exported: bool,
}

pub struct UnitResult {
    pub unit: UnitType,
    pub generic: bool,
    pub need_finalize: bool,
}

impl UnitType {
    pub fn new(prototype: TempUnit, arena: &TypeArena,generics:&GenericArena) -> UnitResult {
        let name = prototype.name;
        let attributes = prototype.attributes;

        let mut need_finalize = false;

        let mut members = Vec::<UnitMember>::new();

        prototype.members.into_iter().for_each(|member| {
            let name = member.name;
            let public = member.public;
            match member.ty.0 {
                TempType::Path(temp) => {
                    let ty = temp.join();
                    let ident = get_ident(&ty);
                    let ty_index = arena.get_by_name(ident);
                    if let Some(ty_index) = ty_index {
                        members.push(UnitMember {
                            name,
                            member_type: ty_index,
                            public,
                        });
                    } else {
                        need_finalize = true;
                    }
                }
                TempType::Generic { base, args } => {

                }
                TempType::Reference(reference) => {

                }
            }
        });

        todo!()
    }

    pub fn finalize(&mut self) {}
    pub fn format(&self) -> String {
        self.name.clone()
    }
}
