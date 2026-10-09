use super::*;
use crate::diagnostic::error::IllegalUseError;

fn failure(source: &str, ty: &str, rule: &str, reason: &str, call: &str) {
    let ast = invalid_semantics(source);
    let span = source.rfind(call).unwrap();
    let error = CompileError::IllegalUse(IllegalUseError::RequirementUnmet {
        ty: ty.into(),
        requirement: rule.into(),
        reason: reason.into(),
    });
    assert!(
        ast.config.error_handle().errors.contains(&ErrorInfo::new(
            error.clone(),
            (span..span + call.len()).into()
        )),
        "{:?}",
        ast.config.error_handle()
    );
    let message = error.to_string();
    assert!(message.contains(ty) && message.contains(rule) && message.contains(reason));
}

#[test]
fn function_reports_the_first_failed_numeric_rule_at_the_call() {
    let source = "generic Number { std::generic::is_integer; std::generic::max_bits<16>; }; func<T:Number> f(x:T) {} func main() { f<f64>(1.0); }";
    failure(
        source,
        "f64",
        "std::generic::is_integer",
        "an integer type is required",
        "f<f64>(1.0)",
    );
    let source = "generic Small { std::generic::max_bits<16>; }; func<T:Small> f(x:T) {} func main() { f<i32>(1); }";
    failure(
        source,
        "i32",
        "std::generic::max_bits(16)",
        "requires at most 16 bits, but the type has 32 bits",
        "f<i32>(1)",
    );
}

#[test]
fn unit_and_interface_instantiation_also_report_the_specific_rule() {
    let source = "generic Integer { std::generic::is_integer; }; unit Box<T:Integer> { value:T; }; func main() { var x = Box<f64>{1.0}; }";
    let ast = invalid_semantics(source);
    assert!(
        ast.config
            .error_handle()
            .errors
            .iter()
            .any(|error| format!("{error:?}").contains("std::generic::is_integer"))
    );
    let source = "generic Integer { std::generic::is_integer; }; unit P {}; api P::func<T:Integer> f(x:T) {} func main() { P::f<f64>(1.0); }";
    failure(
        source,
        "f64",
        "std::generic::is_integer",
        "an integer type is required",
        "P::f<f64>(1.0)",
    );
}

#[test]
fn interface_requirements_report_missing_and_mismatched_signatures() {
    for (method, reason) in [
        ("", "was not found"),
        ("P::func get(self) -> i32 { return 1; }", "is not public"),
        (
            "pub P::func get(self) -> bool { return true; }",
            "return type must be i32, but found bool",
        ),
        (
            "pub P::func get(mut self) -> i32 { return 1; }",
            "receiver mutability",
        ),
        (
            "pub P::func get(self,x:i32) -> i32 { return x; }",
            "expected 0 parameters, but found 1",
        ),
    ] {
        let source = format!(
            "generic HasGet {{ func get(self) -> i32; }}; unit P {{}}; {method} func<T:HasGet> f(x:T) {{}} func main() {{ f<P>(P{{}}); }}"
        );
        let ast = invalid_semantics(&source);
        let diagnostic = format!("{:?}", ast.config.error_handle().errors);
        assert!(diagnostic.contains("func get(self) -> i32"), "{diagnostic}");
        assert!(diagnostic.contains(reason), "{diagnostic}");
    }
}
