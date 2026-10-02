use super::*;
use crate::gens::instruction::LlvmType;

fn push(stack: &mut VariableStack, name: &str, reg: usize) -> StackPosition {
    stack.push(
        name.into(),
        TypeIndex::empty(),
        LlvmValue {
            code: vec![],
            ty: LlvmType::Int(32),
            reg: Some(reg),
            in_reg: false,
        },
    )
}

#[test]
fn sibling_scope_positions_remain_distinct_and_replayable() {
    let mut stack = VariableStack::default();
    let root = stack.position();
    let outer = push(&mut stack, "outer", 0);
    let first = push(&mut stack, "then", 1);
    stack.restore(outer);
    let second = push(&mut stack, "else", 2);
    assert_ne!(first, second);
    assert_eq!(
        stack
            .between(first, root)
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        vec!["then", "outer"]
    );
    assert_eq!(stack.between(second, outer)[0].name, "else");
    stack.restore(root);
    assert_eq!(stack.variables().len(), 3);
    assert_eq!(stack.position(), root);
}
