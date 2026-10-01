use super::setup;
use crate::ast::{
    EnumBool, TypeIndex,
    expression::Expression,
    function::InterfaceSymbol,
    statement::Variable,
    symbols::StatementContext,
    types::{CompileType, UnitType, unit_type::UnitMember},
};
use crate::parser::out::{TempExpr, TempPath};
use std::{collections::HashMap, rc::Rc};

#[test]
fn fields_and_interfaces_enforce_receiver_and_public_rules() {
    for interface in [false, true] {
        for receiver in [false, true] {
            for public in [false, true] {
                for indirect in [false, true] {
                    let (mut config, mut symbols) = setup();
                    let mut unit = UnitType::empty();
                    unit.name = "Owner".into();
                    unit.members.push(UnitMember {
                        name: "field".into(),
                        member_type: symbols.get_base(
                            crate::ast::types::base_type::DataType::Integer,
                            32,
                            true,
                        ),
                        public,
                    });
                    let owner = symbols
                        .types
                        .insert("Owner".into(), CompileType::Unit(unit));
                    let method = symbols.interfaces.insert(
                        "Owner::method".into(),
                        InterfaceSymbol {
                            public,
                            has_self: true,
                            mutable: false,
                            exported: false,
                            owner,
                            attributes: Default::default(),
                            generics: vec![],
                            generic_map: HashMap::new(),
                            name: "Owner::method".into(),
                            params: vec![],
                            ret_type: None,
                        },
                    );
                    let mut context = StatementContext::new(EnumBool::True(method));
                    let name = if receiver { "self" } else { "other" };
                    let variable = Rc::new(Variable {
                        name: name.into(),
                        var_type: owner,
                        init_val: Box::new(Expression::null()),
                    });
                    let span = (0..1).into();
                    if receiver {
                        assert!(context.insert_self(&mut config, variable, span));
                    } else {
                        assert!(context.insert_parameter(&mut config, variable, span));
                    }
                    let member = (
                        TempExpr::Member {
                            base: Box::new((
                                TempExpr::Path(TempPath {
                                    segments: vec![name.into()],
                                }),
                                span,
                            )),
                            indirect,
                            name: if interface { "method" } else { "field" }.into(),
                            name_span: span,
                        },
                        span,
                    );
                    if interface {
                        Expression::search_interface(
                            &mut config,
                            member,
                            &mut symbols,
                            Some(&context),
                        );
                    } else {
                        Expression::new(&mut config, member, &mut symbols, Some(&context));
                    }
                    let allowed = if receiver {
                        indirect
                    } else {
                        public && !indirect
                    };
                    assert_eq!(
                        config.error_handle().errors.is_empty(),
                        allowed,
                        "interface={interface}, receiver={receiver}, public={public}, indirect={indirect}"
                    );
                }
            }
        }
    }
}
