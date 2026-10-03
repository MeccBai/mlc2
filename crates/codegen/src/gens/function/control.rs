use super::*;
use crate::ast::{
    expression::{CompAtom, Composite, operators::Operator},
    statement::{ForStatement, IfStatement, MatchPattern, MatchStatement, WhileStatement},
};

impl FunctionGenerator<'_> {
    pub(super) fn if_statement(&mut self, statement: &IfStatement) {
        if let Some(condition) =
            constant::value(&statement.condition, self.package).and_then(constant::Scalar::boolean)
        {
            self.anonymous_scope(if condition {
                &statement.then_branch
            } else {
                statement.else_branch.as_deref().unwrap_or(&[])
            });
            return;
        }
        let yes = self.label("if.then");
        let no = self.label("if.else");
        let end = self.label("if.end");
        self.condition(&statement.condition, yes.clone(), no.clone());
        self.block(yes);
        let then_falls = self.scope(&statement.then_branch, end.clone());
        self.block(no);
        let else_falls = self.scope(statement.else_branch.as_deref().unwrap_or(&[]), end.clone());
        if then_falls || else_falls {
            self.block(end);
        }
    }

    pub(super) fn while_statement(&mut self, statement: &WhileStatement) {
        if constant::value(&statement.condition, self.package).and_then(constant::Scalar::boolean)
            == Some(false)
        {
            return;
        }
        let condition = self.label("while.condition");
        let body = self.label("while.body");
        let end = self.label("while.end");
        let mark = self.variables.position();
        self.branch(condition.clone());
        self.block(condition.clone());
        self.condition(&statement.condition, body.clone(), end.clone());
        self.loops.push(LoopTarget {
            break_label: end.clone(),
            continue_label: condition.clone(),
            break_to: mark,
            continue_to: mark,
        });
        self.block(body);
        self.scope(&statement.stmts, condition);
        self.loops.pop();
        self.block(end);
    }

    pub(super) fn for_statement(&mut self, statement: &ForStatement) {
        if constant::empty_for(statement, self.package) {
            return;
        }
        let outer = self.variables.position();
        if let Some(init) = &statement.init {
            self.statements(std::slice::from_ref(&**init));
        }
        if self.terminated {
            fail("For initializer terminated its block");
        }
        let iteration = self.variables.position();
        let condition = self.label("for.condition");
        let body = self.label("for.body");
        let step = self.label("for.step");
        let exit = self.label("for.exit");
        let end = self.label("for.end");
        self.branch(condition.clone());
        self.block(condition.clone());
        self.condition(&statement.condition, body.clone(), exit.clone());
        self.loops.push(LoopTarget {
            break_label: end.clone(),
            continue_label: step.clone(),
            break_to: outer,
            continue_to: iteration,
        });
        self.block(body);
        self.scope(&statement.stmts, step.clone());
        self.block(step);
        if let Some(execution) = &statement.execution {
            self.statements(std::slice::from_ref(&**execution));
        }
        if !self.terminated {
            self.branch(condition);
        }
        self.loops.pop();
        self.block(exit);
        self.exit(ExitKind::Scope, outer, end.clone());
        let names: Vec<_> = self
            .variables
            .between(self.variables.position(), outer)
            .iter()
            .map(|v| v.name.clone())
            .collect();
        for name in names {
            self.bindings.remove(&name);
        }
        self.variables.restore(outer);
        self.block(end);
    }

    pub(super) fn match_statement(&mut self, statement: &MatchStatement) {
        match constant::select_match(statement, self.package) {
            constant::MatchSelection::Scope(body) => {
                self.anonymous_scope(body);
                return;
            }
            constant::MatchSelection::Skip => return,
            constant::MatchSelection::Dynamic => {}
        }
        // Evaluate the scrutinee once. Pattern expressions are evaluated only along
        // the unmatched path; default is the fallback regardless of source position.
        let matched = self.expression(&statement.value);
        let operand = self.load(&matched);
        let mut binding = matched.clone();
        binding.reg = match operand {
            IrValue::Reg(reg) => Some(reg),
            _ => unreachable!(),
        };
        binding.in_reg = true;
        let temporary = format!("$match.{}", self.next_label);
        let temporary_type = self
            .expression_type(&statement.value)
            .unwrap_or_else(|| fail("Missing match scrutinee type"));
        self.bindings.insert(temporary.clone(), binding);
        let variable = std::rc::Rc::new(crate::ast::statement::Variable {
            read_count: Default::default(),
            declaration_span: (0..0).into(),
            name: temporary.clone(),
            var_type: temporary_type,
            init_val: Box::new(Expression::Poison),
        });
        let end = self.label("match.end");
        let mut falls = false;
        let mut default = None;
        for (pattern, body) in &statement.branches {
            let MatchPattern::Value(pattern) = pattern else {
                default = Some(body);
                continue;
            };
            let yes = self.label("match.arm");
            let no = self.label("match.next");
            let comparison = Expression::CompositeE(Composite {
                members: vec![
                    CompAtom::VarValueA(variable.clone()),
                    CompAtom::from_expr(pattern.clone()),
                ],
                operators: vec![Operator::Equal],
            });
            self.condition(&comparison, yes.clone(), no.clone());
            self.block(yes);
            falls |= self.scope(body, end.clone());
            self.block(no);
        }
        if let Some(body) = default {
            falls |= self.scope(body, end.clone());
        } else {
            self.branch(end.clone());
            falls = true;
        }
        self.bindings.remove(&temporary);
        if falls {
            self.block(end);
        }
    }

    fn expression_type(&self, expr: &Expression) -> Option<TypeIndex> {
        IrGenerator::expression_type(expr, self.package)
    }
}
