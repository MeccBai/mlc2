use super::*;
use crate::ast::arena::TypeIndex;
use crate::ast::expression::Expression;
use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::warning::WarningHandle;

fn variable(name: &str) -> Rc<Variable> {
    Rc::new(Variable {
        read_count: Default::default(),
        declaration_span: (0..0).into(),
        name: name.into(),
        var_type: TypeIndex::empty(),
        init_val: Box::new(Expression::null()),
    })
}

#[test]
fn shadowing_is_rejected_and_existing_binding_survives() {
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    );
    let span = (3..8).into();
    let mut context = StatementContext::new(EnumBool::False(FuncIndex::empty()));
    let outer = variable("value");
    assert!(context.declare(&mut config, Rc::clone(&outer), span));
    context.push_frame();
    let inner = variable("value");
    assert!(!context.declare(&mut config, Rc::clone(&inner), span));
    let ast_reference = Rc::clone(context.lookup("value").unwrap());
    assert!(Rc::ptr_eq(&ast_reference, &outer));
    assert!(config.is_poisoned());
    assert_eq!(config.error_handle().errors.len(), 1);
    let exited = context.pop_frame().unwrap();
    assert!(Rc::ptr_eq(context.lookup("value").unwrap(), &outer));
    drop(exited);
    assert_eq!(ast_reference.name, "value");
    assert!(context.pop_frame().is_none());
}

#[test]
fn lookup_includes_receiver_generics_and_parameters_and_blocks_shadowing() {
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    );
    let mut context = StatementContext::new(EnumBool::False(FuncIndex::empty()));
    let span = (0..1).into();
    assert!(context.insert_self(&mut config, variable("self"), span));
    assert!(context.insert_generic_parameter(
        &mut config,
        "generic".into(),
        GenericIndex::empty(),
        span
    ));
    assert!(context.insert_parameter(&mut config, variable("argument"), span));
    assert!(context.declare(&mut config, variable("local"), span));
    context.push_frame();
    for name in ["self", "generic", "argument", "local"] {
        assert!(context.resolve(name).is_some());
    }
    assert!(!context.declare(&mut config, variable("argument"), span));
    assert!(context.frames().last().unwrap().locals().is_empty());
}

#[test]
fn duplicate_does_not_replace_binding_or_change_cleanup_order() {
    let mut frame = ScopeFrame::new();
    let first = variable("first");
    frame.declare(Rc::clone(&first)).unwrap();
    frame.declare(variable("second")).unwrap();
    let existing = frame.declare(variable("first")).unwrap_err();
    assert!(Rc::ptr_eq(&existing, &first));
    assert_eq!(frame.locals().len(), 2);
    assert_eq!(
        frame
            .cleanup_order()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        vec!["second", "first"]
    );
}
#[test]
fn scope_stack_retains_nearest_loop_and_restores_parent() {
    use super::SupperScopeType;
    let mut context = super::StatementContext::new(crate::ast::symbols::EnumBool::False(
        crate::ast::arena::FuncIndex::empty(),
    ));
    assert_eq!(context.supper_scope, (0, SupperScopeType::Function));
    assert_eq!(context.loop_scope(), None);
    context.push_scope(SupperScopeType::For);
    let outer = context.supper_scope;
    context.push_scope(SupperScopeType::If);
    context.push_frame();
    assert_eq!(context.loop_scope(), Some(outer));
    context.push_scope(SupperScopeType::While);
    let inner = context.supper_scope;
    context.push_scope(SupperScopeType::Match);
    assert_eq!(context.loop_scope(), Some(inner));
    context.pop_frame();
    context.pop_frame();
    assert_eq!(context.loop_scope(), Some(outer));
    context.pop_frame();
    context.pop_frame();
    context.pop_frame();
    assert_eq!(context.supper_scope, (0, SupperScopeType::Function));
    assert_eq!(context.loop_scope(), None);
    assert!(context.pop_frame().is_none());
}
