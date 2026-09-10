#![allow(clippy::missing_errors_doc)]

use crate::{
    eval::{EvalError, Map, Value},
    map,
};

#[must_use]
pub fn create_map() -> Map<'static> {
    map! {
        all: Value::Function(all),
        any: Value::Function(any),
        equals: Value::Function(equals),
        first: Value::Function(first),
        is_empty: Value::Function(is_empty),
        last: Value::Function(last),
        len: Value::Function(len),
    }
}

pub fn any<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::List(values)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    let mut result = false;
    for value in values {
        match value {
            Value::Bool(value) => result |= *value,
            _ => return Err(EvalError::TypeMismatch),
        }
    }

    Ok(Value::Bool(result))
}

pub fn equals<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::List(left), Value::List(right)] => Ok(Value::Bool(left == right)),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn all<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::List(values)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    let mut result = true;
    for value in values {
        match value {
            Value::Bool(value) => result &= *value,
            _ => return Err(EvalError::TypeMismatch),
        }
    }

    Ok(Value::Bool(result))
}

pub fn first<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::List(values)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    values.first().cloned().ok_or(EvalError::TypeMismatch)
}

pub const fn is_empty<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::List(values)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    Ok(Value::Bool(values.is_empty()))
}

pub fn last<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::List(values)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    values.last().cloned().ok_or(EvalError::TypeMismatch)
}

pub fn len<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::List(values)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    let Ok(length) = i64::try_from(values.len()) else {
        return Err(EvalError::Overflow);
    };
    Ok(Value::Int(length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        eval::{evaluate_ast, stdlib},
        parser::Parser,
        value,
    };

    fn evaluate(input: &str) -> Result<Value<'_>, EvalError> {
        let ast = Parser::new(input).parse().expect("valid input");
        let scope = stdlib::new();
        evaluate_ast(&ast, &scope)
    }

    #[test]
    fn evaluates_all_lists() {
        assert_eq!(evaluate("[true, true].all()"), Ok(value!(true)));
        assert_eq!(evaluate("[true, false].all()"), Ok(value!(false)));
        assert_eq!(evaluate("[].all()"), Ok(value!(true)));
    }

    #[test]
    fn evaluates_list_equality() {
        assert_eq!(
            evaluate("[1, [true]].equals([1, [true]])"),
            Ok(value!(true))
        );
        assert_eq!(evaluate("[1, 2].equals([1, 3])"), Ok(value!(false)));
    }

    #[test]
    fn methods_can_be_invoked() {
        let cases = [
            ("[false, true].any()", value!(true)),
            ("[].is_empty()", value!(true)),
            ("[1].is_empty()", value!(false)),
            ("[1, 2, 3].len()", value!(3)),
            ("[1, 2].first()", value!(1)),
            ("[1, 2].last()", value!(2)),
        ];
        for (expression, expected) in cases {
            assert_eq!(evaluate(expression), Ok(expected), "{expression}");
        }
    }

    #[test]
    fn expect_errors() {
        let cases = [
            ("[true, 1].all()", Err(EvalError::TypeMismatch)),
            ("[true, 1].any()", Err(EvalError::TypeMismatch)),
            ("[].first()", Err(EvalError::TypeMismatch)),
            ("[].last()", Err(EvalError::TypeMismatch)),
            ("[1].equals(1)", Err(EvalError::TypeMismatch)),
        ];
        for (expression, expected) in cases {
            assert_eq!(evaluate(expression), expected, "{expression}");
        }
    }
}
