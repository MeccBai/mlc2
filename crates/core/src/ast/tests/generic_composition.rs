use super::*;
use crate::ast::generic::{Constraints, GenericTypeRequire};
use crate::diagnostic::error::{ConstraintError, IllegalUseError, ResolveError};

#[test]
fn composition_flattens_deduplicates_and_accepts_all_operand_orders() {
    for terms in [
        "Integer && Signed && Wide",
        "Integer && Wide && Signed",
        "Signed && Integer && Wide",
        "Signed && Wide && Integer",
        "Wide && Integer && Signed",
        "Wide && Signed && Integer",
        "(Integer && Wide) && (Signed && Integer)",
    ] {
        let source = format!(
            "generic Combined = {terms};generic Again = Combined && Integer && Combined;generic Wide {{std::generic::min_bits<32>;}};generic Signed {{std::generic::is_signed;}};generic Integer {{std::generic::is_integer;}};func<T:Again> identity(value:T)->T{{return value;}}func main(){{var value=identity(42);}}"
        );
        let ast = valid(&source);
        let index = ast
            .symbols
            .generics
            .requires
            .get_by_name(&"Again".into())
            .unwrap();
        let rules = &ast.symbols.generics.requires.get(index).requires;
        assert_eq!(rules.len(), 3);
        for rule in [
            GenericTypeRequire::Integer,
            GenericTypeRequire::Signed,
            GenericTypeRequire::MinBits(32),
        ] {
            assert!(rules.contains(&Constraints::Type(rule)));
        }
    }
}

#[test]
fn every_merged_requirement_is_enforced_on_instantiation() {
    for call in [
        "identity(cast<u32>(42))",
        "identity(cast<i16>(42))",
        "identity(1.5)",
    ] {
        let source = format!(
            "generic SignedInteger=Integer && Signed && Wide;generic Integer{{std::generic::is_integer;}};generic Signed{{std::generic::is_signed;}};generic Wide{{std::generic::min_bits<32>;}};func<T:SignedInteger> identity(value:T)->T{{return value;}}func main(){{var value={call};}}"
        );
        let ast = invalid_semantics(&source);
        assert!(matches!(
            ast.config.error_handle().first_error(),
            Some(CompileError::IllegalUse(
                IllegalUseError::RequirementUnmet { .. }
            ))
        ));
    }
}

#[test]
fn conflicting_compositions_fail_without_instantiation() {
    for declarations in [
        "generic Integer{std::generic::is_integer;}generic Float{std::generic::is_float;}generic Impossible=Integer && Float;",
        "generic Integer{std::generic::is_integer;}generic Float{std::generic::is_float;}generic Impossible=Float && Integer;",
        "generic Low{std::generic::min_bits<32>;}generic High{std::generic::max_bits<16>;}generic Impossible=High && Low;",
        "generic Number{std::generic::is_float;}generic Small{std::generic::max_bits<16>;}generic Impossible=Small && Number;",
        "generic Number{std::generic::is_integer;}generic Range{std::generic::min_bits<9>;std::generic::max_bits<15>;}generic Impossible=Number && Range;",
        "generic First{pub func get(self)->i32;}generic Second{pub func get(self)->f64;}generic Impossible=First && Second;",
        "generic First{pub func get(self,x:i32);}generic Second{pub func get(self,x:f64);}generic Impossible=First && Second;",
        "generic First{pub func get(self);}generic Second{pub func get(mut self);}generic Impossible=First && Second;",
        "unit A{};unit B{};generic First{pub func get(self,x:A);}generic Second{pub func get(self,x:B);}generic Impossible=First && Second;",
        "generic Impossible{std::generic::is_integer;std::generic::is_float;}",
        "generic Low{std::generic::min_bits<9>;}generic High{std::generic::max_bits<15>;}generic Impossible=Low && High;",
    ] {
        let ast = invalid_semantics(declarations);
        assert!(
            matches!(
                ast.config.error_handle().first_error(),
                Some(CompileError::Resolve(ResolveError::Constraint(
                    ConstraintError::ConflictingRequirements { .. }
                )))
            ),
            "{:?}",
            ast.config.error_handle()
        );
    }
}

#[test]
fn cycles_and_unknown_references_report_errors() {
    for declarations in ["generic A=A;", "generic A=B && C;generic B=A;generic C{};"] {
        let ast = invalid_semantics(declarations);
        assert!(matches!(
            ast.config.error_handle().first_error(),
            Some(CompileError::Resolve(ResolveError::Constraint(
                ConstraintError::CyclicRequirements { .. }
            )))
        ));
    }
    invalid_semantics("generic A=Missing && Other;generic Other{};");
    invalid_semantics("unit NotAConstraint{};generic A=NotAConstraint;");
}

#[test]
fn compatible_interface_requirements_merge_and_ignore_parameter_names() {
    let ast = valid(
        "generic Both=First && Second;generic First{pub func get(self,x:i32)->i32;}generic Second{pub func get(self,y:i32)->i32;}unit P{};pub P::func get(self,value:i32)->i32{return value;}func<T:Both> accept(value:T){}func main(){accept(P{});}",
    );
    let index = ast
        .symbols
        .generics
        .requires
        .get_by_name(&"Both".into())
        .unwrap();
    assert_eq!(ast.symbols.generics.requires.get(index).requires.len(), 1);
    valid(
        "generic Both=Readonly && Mutable;generic Readonly{pub func use(self,p:$i32);}generic Mutable{pub func use(self,p:$mut i32);}unit P{};pub P::func use(self,p:$mut i32){}func<T:Both> accept(value:T){}func main(){accept(P{});}",
    );
}

#[test]
fn aliases_and_qualified_local_names_resolve_during_lowering() {
    valid(
        "using Alias=Base;generic Derived=Alias && Base;generic Base{std::generic::is_integer;}func<T:Derived> accept(value:T){}func main(){accept(42);}",
    );
}
