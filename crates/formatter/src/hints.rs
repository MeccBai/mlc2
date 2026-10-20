//! AST-derived delimiter roles: comparisons must never be mistaken for generics.
use crate::layout::Style;
use mlc_syntax::{
    lexer::{Lexed, token::Token},
    parser::out::*,
};
use std::collections::HashMap;

pub(super) struct Hints<'a> {
    lexed: &'a Lexed,
    pub roles: HashMap<usize, Style>,
}

impl<'a> Hints<'a> {
    pub fn collect(lexed: &'a Lexed, module: &TempModule) -> HashMap<usize, Style> {
        let mut this = Self {
            lexed,
            roles: HashMap::new(),
        };
        for (item, _) in module {
            match item {
                TempGlobalStmt::Func(f) => {
                    this.signature(
                        f.symbol.name_span,
                        &f.symbol.generics,
                        &f.symbol.params,
                        f.symbol.return_type.as_ref(),
                    );
                    if let Some(body) = &f.body {
                        this.scope(body);
                    }
                }
                TempGlobalStmt::Interface(f) => {
                    this.interface(&f.symbol);
                    if let Some(body) = &f.body {
                        this.scope(body);
                    }
                }
                TempGlobalStmt::MacroFunc(f) => {
                    this.signature(f.name_span, &[], &f.params, f.return_type.as_ref());
                    this.scope(&f.body);
                }
                TempGlobalStmt::Unit(unit) => {
                    this.generics(&unit.generics);
                    if unit.is_union {
                        this.after(unit.name_span.end, Token::LeftBracket, Style::Members);
                    }
                    for member in &unit.members {
                        this.ty(&member.ty);
                    }
                }
                TempGlobalStmt::Using(using) => this.ty(&using.target),
                TempGlobalStmt::Variable(var) => this.variable(var),
                TempGlobalStmt::Generic(generic) => {
                    for (requirement, _) in &generic.requirements {
                        if let TempConstraints::Interface(symbol) = requirement {
                            this.interface(symbol);
                        }
                    }
                }
                TempGlobalStmt::Enum(_) | TempGlobalStmt::Import(_) => (),
            }
        }
        this.roles
    }

    fn after(&mut self, offset: usize, kind: Token, style: Style) {
        self.within(offset, usize::MAX, kind, style);
    }

    fn within(&mut self, offset: usize, end: usize, kind: Token, style: Style) {
        let first = self
            .lexed
            .tokens
            .partition_point(|(_, span)| span.start < offset);
        if let Some(relative) = self.lexed.tokens[first..]
            .iter()
            .take_while(|(_, span)| span.start < end)
            .position(|(token, _)| crate::render::kind(token) == kind)
        {
            self.roles.insert(first + relative, style);
        }
    }

    fn before(&mut self, offset: usize, kind: Token, style: Style) {
        if let Some(index) = self
            .lexed
            .tokens
            .iter()
            .rposition(|(token, span)| span.end <= offset && crate::render::kind(token) == kind)
        {
            self.roles.insert(index, style);
        }
    }

    fn generics(&mut self, params: &[TempGenericParam]) {
        if let Some(first) = params.first() {
            self.before(first.name_span.start, Token::LAngle, Style::Vertical);
        }
    }

    fn signature(
        &mut self,
        name: Span,
        generics: &[TempGenericParam],
        params: &[TempParam],
        returns: Option<&Spanned<TempType>>,
    ) {
        self.generics(generics);
        self.after(name.end, Token::LParen, Style::Vertical);
        for param in params {
            if let Some(ty) = &param.ty {
                self.ty(ty);
            }
        }
        if let Some(ty) = returns {
            self.ty(ty);
        }
    }

    fn interface(&mut self, symbol: &TempInterfaceSymbol) {
        self.signature(
            symbol.name_span,
            &symbol.generics,
            &symbol.params,
            symbol.return_type.as_ref(),
        );
    }

    fn variable(&mut self, var: &TempVar) {
        if let Some(ty) = &var.ty {
            self.ty(ty);
        }
        self.expr(&var.initializer);
    }

    fn ty(&mut self, (ty, span): &Spanned<TempType>) {
        match ty {
            TempType::Generic { args, .. } => {
                self.after(span.start, Token::LAngle, Style::Vertical);
                for arg in args {
                    self.ty(arg);
                }
            }
            TempType::Function {
                params, returns, ..
            } => {
                self.after(span.start, Token::LParen, Style::Vertical);
                for param in params {
                    self.ty(param);
                }
                if let Some(ty) = returns {
                    self.ty(ty);
                }
            }
            TempType::Reference { inner, .. } | TempType::Resource { inner } => self.ty(inner),
            TempType::Array { element, .. } => self.ty(element),
            TempType::Path(_) | TempType::InferredResource => (),
        }
    }

    fn expr(&mut self, (expr, span): &Spanned<TempExpr>) {
        match expr {
            TempExpr::Init { target, values } => {
                self.after(
                    target.as_ref().map_or(span.start, |(_, span)| span.end),
                    Token::LBrace,
                    Style::Packed,
                );
                if let Some(ty) = target {
                    self.ty(ty);
                }
                for value in values {
                    self.expr(value);
                }
            }
            TempExpr::Array(values) => {
                for value in values {
                    self.expr(value);
                }
            }
            TempExpr::Call { callee, args } => {
                match callee {
                    TempCallee::Expr(expr) => {
                        self.expr(expr);
                        let start = if expr.1.end >= span.end {
                            expr.1.start
                        } else {
                            expr.1.end
                        };
                        self.within(start, span.end, Token::LParen, Style::Vertical);
                    }
                    TempCallee::GenericPath { args: types, .. } => {
                        self.after(span.start, Token::LAngle, Style::Vertical);
                        for ty in types {
                            self.ty(ty);
                        }
                        // The first argument follows the call's opening parenthesis.
                        let offset = types.last().map_or(span.start, |(_, span)| span.end);
                        self.within(offset, span.end, Token::LParen, Style::Vertical);
                    }
                }
                for arg in args {
                    self.expr(arg);
                }
            }
            TempExpr::MacroCall { args, .. } => {
                self.after(span.start, Token::LParen, Style::Vertical);
                for arg in args {
                    self.expr(arg);
                }
            }
            TempExpr::Group(inner) | TempExpr::Unary { value: inner, .. } => self.expr(inner),
            TempExpr::Binary { operands, .. } => {
                for value in operands {
                    self.expr(value);
                }
            }
            TempExpr::Member { base, .. } => self.expr(base),
            TempExpr::Literal { .. } | TempExpr::Path(_) => (),
        }
    }

    fn scope(&mut self, scope: &TempScope) {
        for (stmt, _) in &scope.statements {
            match stmt {
                TempStmt::Variable(var) => self.variable(var),
                TempStmt::Assignment { target, value } => {
                    self.expr(target);
                    self.expr(value);
                }
                TempStmt::Expression(expr) | TempStmt::Return(Some(expr)) => self.expr(expr),
                TempStmt::If {
                    condition,
                    then_scope,
                    else_scope,
                } => {
                    self.expr(condition);
                    self.scope(then_scope);
                    if let Some(scope) = else_scope {
                        self.scope(scope);
                    }
                }
                TempStmt::While { condition, scope } => {
                    self.expr(condition);
                    self.scope(scope);
                }
                TempStmt::For {
                    start, end, scope, ..
                } => {
                    self.expr(start);
                    self.expr(end);
                    self.scope(scope);
                }
                TempStmt::Match { value, branches } => {
                    self.expr(value);
                    for (pattern, scope) in branches {
                        if let TempMatchPattern::Expression(expr) = pattern {
                            self.expr(expr);
                        }
                        self.scope(scope);
                    }
                }
                TempStmt::VariantMatch {
                    value, branches, ..
                } => {
                    self.expr(value);
                    for (ty, scope) in branches {
                        self.ty(ty);
                        self.scope(scope);
                    }
                }
                TempStmt::Anonymous(scope) | TempStmt::MacroFor { scope, .. } => self.scope(scope),
                TempStmt::Continue | TempStmt::Break | TempStmt::Return(None) => (),
            }
        }
    }
}
