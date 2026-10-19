/// AST optimization policy, independent of the compiler executable's build mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptimizeConfig {
    pub constant_folding: bool,
    pub dead_code_elimination: bool,
    pub unused_variable_elimination: bool,
    pub unused_function_elimination: bool,
}

impl OptimizeConfig {
    pub const fn disabled() -> Self {
        Self {
            constant_folding: false,
            dead_code_elimination: false,
            unused_variable_elimination: false,
            unused_function_elimination: false,
        }
    }

    pub const fn enabled() -> Self {
        Self {
            constant_folding: true,
            dead_code_elimination: true,
            unused_variable_elimination: true,
            unused_function_elimination: true,
        }
    }
}

impl Default for OptimizeConfig {
    fn default() -> Self {
        Self {
            constant_folding: true,
            dead_code_elimination: true,
            ..Self::disabled()
        }
    }
}
