use crate::ast::{FuncIndex, TypeIndex, function::FuncSymbol, types::TypeLookup};

/// 函数名不是类型身份；signature 保存相同调用 ABI 的具体符号。
#[derive(Debug, Clone, Eq)]
pub struct FunctionType {
    pub params: Vec<TypeIndex>,
    pub returns: Option<TypeIndex>,
    pub c_abi: bool,
    pub variadic: bool,
    pub signature: FuncIndex,
}

impl PartialEq for FunctionType {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params
            && self.returns == other.returns
            && self.c_abi == other.c_abi
            && self.variadic == other.variadic
    }
}

impl FunctionType {
    /// A declared pointer type needs an ABI signature, not a function body.
    pub fn declare(
        params: Vec<TypeIndex>,
        returns: Option<TypeIndex>,
        variadic: bool,
        symbols: &mut dyn crate::ast::symbols::Resolution,
    ) -> TypeIndex {
        let mut ty = Self {
            params,
            returns,
            variadic,
            c_abi: false,
            signature: FuncIndex::empty(),
        };
        let key = ty.format(symbols);
        if let Some(index) = symbols.local().types.get_by_name(&key) {
            return index;
        }
        let name = crate::ast::symbol_name::SymbolName::function_signature(&key);
        let mut params = ty
            .params
            .iter()
            .enumerate()
            .map(|(i, ty)| (*ty, format!("arg{i}")))
            .collect::<Vec<_>>();
        if variadic {
            params.push((TypeIndex::empty(), "...".into()));
        }
        ty.signature = symbols.local_mut().functions.insert(
            name.clone(),
            FuncSymbol {
                name,
                params,
                ret_type: returns,
                generics: Default::default(),
                generic_map: Default::default(),
                attributes: Default::default(),
                exported: false,
            },
        );
        symbols
            .local_mut()
            .types
            .insert(key, super::CompileType::Function(ty))
    }

    pub fn is_generic(&self, types: &(impl TypeLookup + ?Sized)) -> bool {
        self.params
            .iter()
            .chain(self.returns.iter())
            .any(|ty| ty.is_generic(types))
    }

    pub fn instantiation(
        self,
        config: &mut crate::ast::config::Config,
        bindings: &std::collections::HashMap<crate::ast::GenericIndex, TypeIndex>,
        symbols: &mut dyn crate::ast::symbols::Resolution,
        actives: Option<&crate::ast::function::InstantiationActives>,
        span: crate::parser::out::Span,
    ) -> Option<TypeIndex> {
        let params = self
            .params
            .into_iter()
            .map(|ty| ty.instantiation(config, bindings, symbols, actives, span))
            .collect::<Option<Vec<_>>>()?;
        let returns = match self.returns {
            Some(ty) => Some(ty.instantiation(config, bindings, symbols, actives, span)?),
            None => None,
        };
        Some(Self::declare(params, returns, self.variadic, symbols))
    }
    pub fn new(signature: FuncIndex, symbol: &FuncSymbol) -> Self {
        Self {
            params: symbol
                .params
                .iter()
                .filter(|(_, name)| name != "...")
                .map(|(ty, _)| *ty)
                .collect(),
            returns: symbol.ret_type,
            c_abi: symbol
                .attributes
                .contains(&crate::ast::attribute::FuncAttibute::Cabi),
            variadic: symbol.params.last().is_some_and(|(_, name)| name == "..."),
            signature,
        }
    }

    pub fn format(&self, types: &(impl TypeLookup + ?Sized)) -> String {
        self.format_with(|ty| ty.format(types))
    }

    pub fn generic_instance_name(
        &self,
        types: &(impl TypeLookup + ?Sized),
        bindings: &std::collections::HashMap<crate::ast::GenericIndex, TypeIndex>,
    ) -> String {
        self.format_with(|ty| ty.generic_instance_name(types, bindings))
    }

    fn format_with(&self, format: impl Fn(TypeIndex) -> String) -> String {
        let mut params = self.params.iter().map(|ty| format(*ty)).collect::<Vec<_>>();
        if self.variadic {
            params.push("...".into());
        }
        format!(
            "{}func({})->{}",
            if self.c_abi { "c_abi " } else { "" },
            params.join(","),
            self.returns.map(format).unwrap_or_else(|| "void".into())
        )
    }
}

impl TypeIndex {
    pub(crate) fn contains_function_pointer(self, types: &(impl TypeLookup + ?Sized)) -> bool {
        fn visit(
            ty: TypeIndex,
            types: &(impl TypeLookup + ?Sized),
            visited: &mut std::collections::HashSet<TypeIndex>,
        ) -> bool {
            if ty.is_empty() || !visited.insert(ty) {
                return false;
            }
            match types.get_type(ty).unqualified() {
                super::CompileType::Function(_) => true,
                super::CompileType::List(list) => visit(list.element_type, types, visited),
                super::CompileType::Unit(unit) => unit
                    .members
                    .iter()
                    .any(|member| visit(member.member_type, types, visited)),
                _ => false,
            }
        }
        visit(self, types, &mut std::collections::HashSet::new())
    }
}
