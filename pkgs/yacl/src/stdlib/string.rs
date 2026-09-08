#![allow(clippy::missing_errors_doc)]

use crate::{
    eval::{EvalError, Value},
    scope::{Scope, Symbol},
};

#[must_use]
pub fn create_scope() -> Scope<'static> {
    Scope::from_symbols([
        ("len_chars", Symbol::Function(len_chars)),
        ("len_bytes", Symbol::Function(len_bytes)),
        ("is_empty", Symbol::Function(is_empty)),
        ("is_ascii", Symbol::Function(is_ascii)),
        ("contains", Symbol::Function(contains)),
        ("starts_with", Symbol::Function(starts_with)),
        ("ends_with", Symbol::Function(ends_with)),
        ("count", Symbol::Function(count)),
    ])
}

pub fn len_chars<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => {
            let length = i64::try_from(value.chars().count()).map_err(|_| EvalError::Overflow)?;
            Ok(Value::Int(length))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn len_bytes<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => {
            let Ok(length) = i64::try_from(value.len()) else {
                return Err(EvalError::Overflow);
            };
            Ok(Value::Int(length))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn is_empty<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Bool(value.is_empty())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn is_ascii<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Bool(value.is_ascii())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn contains<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(needle)] => Ok(Value::Bool(value.contains(needle))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn starts_with<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(prefix)] => Ok(Value::Bool(value.starts_with(prefix))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn ends_with<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(suffix)] => Ok(Value::Bool(value.ends_with(suffix))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn count<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(needle)] => {
            let count = if needle.is_empty() {
                0
            } else {
                value.matches(needle).count()
            };
            let count = i64::try_from(count).map_err(|_| EvalError::Overflow)?;
            Ok(Value::Int(count))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{eval::evaluate_ast, parser::Parser};

    fn evaluate(input: &str) -> Result<Value<'_>, EvalError> {
        let ast = Parser::new(input).parse().expect("valid input");
        let scope = crate::stdlib::new();
        evaluate_ast(&ast, &scope)
    }

    #[test]
    fn evaluates_string_methods() {
        let value = evaluate(
            "char_length = \"héllo\".len_chars()\nbyte_length = \"héllo\".len_bytes()\nempty = \"\".is_empty()\nascii = \"hello\".is_ascii()\ncontains = \"hello\".contains(\"ell\")\nstarts = \"hello\".starts_with(\"he\")\nends = \"hello\".ends_with(\"lo\")\ncount = \"hello hello\".count(\"hello\")",
        )
        .expect("string methods should evaluate");

        let Value::Map(document) = value else {
            panic!("expected map")
        };
        assert_eq!(document.get("char_length"), Some(&Value::Int(5)));
        assert_eq!(document.get("byte_length"), Some(&Value::Int(6)));
        assert_eq!(document.get("empty"), Some(&Value::Bool(true)));
        assert_eq!(document.get("ascii"), Some(&Value::Bool(true)));
        assert_eq!(document.get("contains"), Some(&Value::Bool(true)));
        assert_eq!(document.get("starts"), Some(&Value::Bool(true)));
        assert_eq!(document.get("ends"), Some(&Value::Bool(true)));
        assert_eq!(document.get("count"), Some(&Value::Int(2)));
    }

    #[test]
    fn rejects_invalid_string_method_arguments() {
        assert_eq!(
            evaluate("1.len_chars()"),
            Err(EvalError::UnknownFunction("len_chars".to_owned()))
        );
        assert_eq!(
            evaluate("\"hello\".contains(1)"),
            Err(EvalError::TypeMismatch)
        );
    }
}
