use crate::{
    eval::{EvalError, Map, Value},
    map,
};

#[must_use]
pub fn create_map() -> Map {
    map! {
        assert: Value::Function(assert),
    }
}
pub const fn assert(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Bool(true)] => Ok(Value::Bool(true)),
        [Value::Bool(false)] => Err(EvalError::AssertionFailed),
        _ => Err(EvalError::TypeMismatch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        eval::{Scope, evaluate_ast, stdlib},
        parser::Parser,
        value,
    };

    fn evaluate(input: &str) -> Result<Value, EvalError> {
        let ast = Parser::new(input).parse_stmnts().expect("valid input");
        let mut scope = Scope::child(stdlib::new());
        evaluate_ast(&ast, &mut scope)
    }

    #[test]
    fn assertions_can_be_invoked() {
        assert_eq!(
            evaluate("let std = import('std')\nstd.assert.assert(true)"),
            Ok(value!(true))
        );
    }

    #[test]
    fn failed_assertions_return_an_error() {
        assert_eq!(
            evaluate("let std = import('std')\nstd.assert.assert(false)"),
            Err(EvalError::AssertionFailed)
        );
    }

    #[test]
    fn assertions_require_a_boolean() {
        assert_eq!(
            evaluate("let std = import('std')\nstd.assert.assert(1)"),
            Err(EvalError::TypeMismatch)
        );
    }
}
