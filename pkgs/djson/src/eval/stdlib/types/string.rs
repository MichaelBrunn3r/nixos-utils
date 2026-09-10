#![allow(clippy::missing_errors_doc)]

use crate::{
    eval::{EvalError, Map, Value},
    map,
};

#[must_use]
pub fn create_map() -> Map<'static> {
    map! {
        len_chars: Value::Function(len_chars),
        len_bytes: Value::Function(len_bytes),
        is_empty: Value::Function(is_empty),
        is_blank: Value::Function(is_blank),
        is_ascii: Value::Function(is_ascii),
        contains: Value::Function(contains),
        starts_with: Value::Function(starts_with),
        ends_with: Value::Function(ends_with),
        remove_prefix: Value::Function(remove_prefix),
        remove_suffix: Value::Function(remove_suffix),
        count: Value::Function(count),
        find: Value::Function(find),
        split: Value::Function(split),
        lines: Value::Function(lines),
        trim: Value::Function(trim),
        trim_start: Value::Function(trim_start),
        trim_end: Value::Function(trim_end),
        uppercase: Value::Function(uppercase),
        lowercase: Value::Function(lowercase),
        replace: Value::Function(replace),
        repeat: Value::Function(repeat),
        pad_start: Value::Function(pad_start),
        pad_end: Value::Function(pad_end),
        reverse: Value::Function(reverse),
    }
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

pub fn is_empty<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Bool(value.is_empty())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn is_blank<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Bool(value.trim().is_empty())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn is_ascii<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Bool(value.is_ascii())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn contains<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(needle)] => Ok(Value::Bool(value.contains(needle.as_ref()))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn starts_with<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(prefix)] => {
            Ok(Value::Bool(value.starts_with(prefix.as_ref())))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn ends_with<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(suffix)] => {
            Ok(Value::Bool(value.ends_with(suffix.as_ref())))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn remove_prefix<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(prefix)] => Ok(Value::Str(
            value
                .strip_prefix(prefix.as_ref())
                .unwrap_or(value)
                .to_owned()
                .into(),
        )),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn remove_suffix<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(suffix)] => Ok(Value::Str(
            value
                .strip_suffix(suffix.as_ref())
                .unwrap_or(value)
                .to_owned()
                .into(),
        )),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn count<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(needle)] => {
            let count = if needle.is_empty() {
                0
            } else {
                value.matches(needle.as_ref()).count()
            };
            let count = i64::try_from(count).map_err(|_| EvalError::Overflow)?;
            Ok(Value::Int(count))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn find<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(needle)] => {
            let index = value
                .find(needle.as_ref())
                .map(|index| value[..index].chars().count())
                .map(i64::try_from)
                .transpose()
                .map_err(|_| EvalError::Overflow)?
                .unwrap_or(-1);
            Ok(Value::Int(index))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn split<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(delimiter)] => Ok(Value::List(
            value
                .split(delimiter.as_ref())
                .map(|part| Value::Str(part.to_owned().into()))
                .collect(),
        )),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn lines<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::List(
            value
                .lines()
                .map(|line| Value::Str(line.to_owned().into()))
                .collect(),
        )),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn uppercase<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Str(value.to_uppercase().into())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn trim<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Str(value.trim().to_owned().into())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn trim_start<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Str(value.trim_start().to_owned().into())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn trim_end<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Str(value.trim_end().to_owned().into())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn lowercase<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Str(value.to_lowercase().into())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn replace<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Str(from), Value::Str(to)] => {
            Ok(Value::Str(value.replace(from.as_ref(), to.as_ref()).into()))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn repeat<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value), Value::Int(count)] if *count >= 0 => {
            let count = usize::try_from(*count).map_err(|_| EvalError::Overflow)?;
            value.len().checked_mul(count).ok_or(EvalError::Overflow)?;
            Ok(Value::Str(value.repeat(count).into()))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn pad_start<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::Str(value), Value::Int(width), Value::Str(padding)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };
    if *width < 0 || padding.is_empty() {
        return Err(EvalError::TypeMismatch);
    }

    let width = usize::try_from(*width).map_err(|_| EvalError::Overflow)?;
    let length = value.chars().count();
    let missing = width.saturating_sub(length);
    let mut result = padding.chars().cycle().take(missing).collect::<String>();
    result.push_str(value);
    Ok(Value::Str(result.into()))
}

pub fn pad_end<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::Str(value), Value::Int(width), Value::Str(padding)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };
    if *width < 0 || padding.is_empty() {
        return Err(EvalError::TypeMismatch);
    }

    let width = usize::try_from(*width).map_err(|_| EvalError::Overflow)?;
    let length = value.chars().count();
    let missing = width.saturating_sub(length);
    let mut result = value.to_string();
    result.extend(padding.chars().cycle().take(missing));
    Ok(Value::Str(result.into()))
}

pub fn reverse<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(value)] => Ok(Value::Str(value.chars().rev().collect::<String>().into())),
        _ => Err(EvalError::TypeMismatch),
    }
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
    fn methods_can_be_invoked() {
        let cases = [
            ("'héllo'.len_chars()", value!(5)),
            ("'héllo'.len_bytes()", value!(6)),
            ("''.is_empty()", value!(true)),
            ("' \\t\\n'.is_blank()", value!(true)),
            ("'hello'.is_ascii()", value!(true)),
            ("'hello'.contains('ell')", value!(true)),
            ("'hello'.starts_with('he')", value!(true)),
            ("'hello'.ends_with('lo')", value!(true)),
            ("'hello'.remove_prefix('he')", value!("llo")),
            ("'hello'.remove_suffix('lo')", value!("hel")),
            ("'hello hello'.count('hello')", value!(2)),
            ("'héllo'.find('ll')", value!(2)),
            ("'a,b,c'.split(',')", value!(["a", "b", "c"])),
            ("'one\\ntwo'.lines()", value!(["one", "two"])),
            ("'  hello  '.trim()", value!("hello")),
            ("'  hello  '.trim_start()", value!("hello  ")),
            ("'  hello  '.trim_end()", value!("  hello")),
            ("'héllo'.uppercase()", value!("HÉLLO")),
            ("'HÉLLO'.lowercase()", value!("héllo")),
            ("'hello hello'.replace('hello', 'hi')", value!("hi hi")),
            ("'ha'.repeat(3)", value!("hahaha")),
            ("'7'.pad_start(3, '0')", value!("007")),
            ("'7'.pad_end(3, '0')", value!("700")),
            ("'héllo'.reverse()", value!("olléh")),
        ];
        for (expression, expected) in cases {
            assert_eq!(evaluate(expression), Ok(expected), "{expression}");
        }
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
        assert_eq!(
            evaluate("\"hello\".repeat(-1)"),
            Err(EvalError::TypeMismatch)
        );
    }
}
