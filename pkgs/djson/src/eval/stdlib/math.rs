#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::missing_errors_doc
)]

use crate::{
    eval::{EvalError, Map, Value},
    map,
};

pub const PI: Value = Value::Float(std::f64::consts::PI);
pub const INFINITY: Value = Value::Float(f64::INFINITY);
pub const NAN: Value = Value::Float(f64::NAN);

#[must_use]
pub fn create_map() -> Map {
    map! {
        PI: PI,
        INFINITY: INFINITY,
        NAN: NAN,
        acos: Value::Function(acos),
        asin: Value::Function(asin),
        atan: Value::Function(atan),
        atan2: Value::Function(atan2),
        sin: Value::Function(sin),
        cos: Value::Function(cos),
        tan: Value::Function(tan),
    }
}

pub fn sin(arguments: &[Value]) -> Result<Value, EvalError> {
    unary_float(arguments, f64::sin)
}

pub fn cos(arguments: &[Value]) -> Result<Value, EvalError> {
    unary_float(arguments, f64::cos)
}

pub fn tan(arguments: &[Value]) -> Result<Value, EvalError> {
    unary_float(arguments, f64::tan)
}

pub fn asin(arguments: &[Value]) -> Result<Value, EvalError> {
    unary_float(arguments, f64::asin)
}

pub fn acos(arguments: &[Value]) -> Result<Value, EvalError> {
    unary_float(arguments, f64::acos)
}

pub fn atan(arguments: &[Value]) -> Result<Value, EvalError> {
    unary_float(arguments, f64::atan)
}

pub fn atan2(arguments: &[Value]) -> Result<Value, EvalError> {
    match numeric_pair(arguments) {
        Some((left, right)) => Ok(Value::Float(left.atan2(right))),
        None => Err(EvalError::TypeMismatch),
    }
}

fn unary_float(arguments: &[Value], operation: fn(f64) -> f64) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float(operation(*value as f64))),
        [Value::Float(value)] => Ok(Value::Float(operation(*value))),
        _ => Err(EvalError::TypeMismatch),
    }
}

fn numeric_pair(arguments: &[Value]) -> Option<(f64, f64)> {
    match arguments {
        [left, right] => Some((as_float(left)?, as_float(right)?)),
        _ => None,
    }
}

const fn as_float(value: &Value) -> Option<f64> {
    match value {
        Value::Int(value) => Some(*value as f64),
        Value::Float(value) => Some(*value),
        _ => None,
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
    fn stdlib_members_can_be_used() {
        use std::f64::consts::*;

        let import_stdlib = "let std = import('std')\n";
        let cases = [
            ("std.math.tan(std.math.PI / 4)", value!((PI / 4.0).tan())),
            ("std.math.asin(1)", value!(1f64.asin())),
            ("std.math.acos(0)", value!(0f64.acos())),
            ("std.math.atan(1)", value!(1f64.atan())),
            ("std.math.atan2(1, 1)", value!(1f64.atan2(1f64))),
            ("std.math.PI", value!(PI)),
            ("std.math.INFINITY", value!(INFINITY)),
        ];
        for (expression, expected) in cases {
            let expression = format!("{import_stdlib}{expression}");
            assert_eq!(evaluate(&expression), Ok(expected), "{expression}");
        }

        // Check NAN
        let expression = format!("{import_stdlib}std.math.NAN");
        assert!(matches!(
            evaluate(&expression),
            Ok(Value::Float(value)) if value.is_nan()
        ));
    }

    #[test]
    fn expect_errors() {
        let import_stdlib = "let std = import('std')\n";
        let cases = [
            ("result: std.math.cos(true)", Err(EvalError::TypeMismatch)),
            ("result: std.math.sin(1, 2)", Err(EvalError::TypeMismatch)),
        ];
        for (expression, expected) in cases {
            let expression = format!("{import_stdlib}{expression}");
            assert_eq!(evaluate(&expression), expected, "{expression}");
        }
    }
}
