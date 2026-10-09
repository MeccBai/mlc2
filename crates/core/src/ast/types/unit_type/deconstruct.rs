use super::UnitType;
use crate::ast::{TypeIndex, types::TypeLookup};

/// 解析阶段生成的内部析构体；不允许用户提供实现。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deconstruct {
    pub name: String,
    pub members: Vec<DropMember>,
}

impl Deconstruct {
    pub const VISIBILITY: crate::parser::out::TempVisibility =
        crate::parser::out::TempVisibility::Public;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropMember {
    pub index: usize,
    pub ty: TypeIndex,
}

impl UnitType {
    pub(super) fn generate_deconstruct(&mut self, types: &(impl TypeLookup + ?Sized)) {
        if self.has_generic() {
            return;
        }
        let members = self
            .members
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, member)| member.member_type.has_res(types))
            .map(|(index, member)| DropMember {
                index,
                ty: member.member_type,
            })
            .collect::<Vec<_>>();
        if !members.is_empty() {
            self.deconstruct = Some(Deconstruct {
                name: crate::ast::symbol_name::SymbolName::deconstruct(&self.name),
                members,
            });
        }
    }
}

/// 等全部声明完成后补齐前向引用；已有具体析构体不重复构建。
pub(crate) fn prepare_deconstructs(symbols: &mut dyn crate::ast::symbols::Resolution) {
    fn visit(
        ty: TypeIndex,
        symbols: &mut dyn crate::ast::symbols::Resolution,
        active: &mut std::collections::HashSet<TypeIndex>,
    ) {
        if !active.insert(ty) {
            return;
        }
        if let crate::ast::types::CompileType::List(list) = symbols.get_type(ty).unqualified() {
            let element = list.element_type;
            visit(element, symbols, active);
            return;
        }
        let crate::ast::types::CompileType::Unit(mut unit) =
            symbols.get_type(ty).unqualified().clone()
        else {
            return;
        };
        if unit.has_generic() || unit.deconstruct.is_some() {
            return;
        }
        for member in &unit.members {
            visit(member.member_type, symbols, active);
        }
        unit.generate_deconstruct(symbols);
        if ty.file_id() == symbols.local().types.file_id() {
            symbols
                .local_mut()
                .types
                .set(&ty, crate::ast::types::CompileType::Unit(unit));
        }
    }
    let types = symbols
        .local()
        .types
        .entries()
        .map(|(_, index, _)| index)
        .collect::<Vec<_>>();
    let mut active = std::collections::HashSet::new();
    for ty in types {
        visit(ty, symbols, &mut active);
    }
}
