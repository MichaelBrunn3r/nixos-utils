#![allow(
    clippy::cast_precision_loss,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation
)]

pub mod document;
pub mod scope;
pub mod stdlib;

pub use document::{Document, Value};
pub use scope::Scope;

use crate::{
    eval::document::map::Map,
    parser::ast::{AST, Expr, Identifier, InfixOp, Pattern, PrefixOp, Statement},
};

/// Evaluates a parsed document into `scope`.
///
/// Bindings declared by `let` statements are written directly into `scope`.
/// Callers that need an isolated evaluation environment, such as normal file
/// evaluation, should pass a child scope created from the standard prelude.
pub fn evaluate_ast(ast: &AST<'_>, scope: &mut Scope) -> Result<Value, EvalError> {
    let mut document = Document::new();
    let mut expression = None;

    for statement in &ast.statements {
        let value = match statement {
            Statement::Let(binding) => {
                let value = evaluate_expr(&binding.expr, scope)?;
                let mut bindings = Vec::new();
                destructure(&binding.pattern, value, &mut bindings)?;
                scope.bind_values(bindings)?;
                continue;
            }
            Statement::Expr(node) => {
                if !document.is_empty() {
                    return Err(EvalError::MixedDocumentForms);
                }
                if expression.is_some() {
                    return Err(EvalError::MultipleExpressions);
                }
                expression = Some(evaluate_expr(node, scope)?);
                continue;
            }
            Statement::KV(pair) => {
                if expression.is_some() {
                    return Err(EvalError::MixedDocumentForms);
                }
                evaluate_expr(&pair.expr, scope)?
            }
        };

        if let Statement::KV(pair) = statement
            && document.insert(pair.key, value).is_some()
        {
            return Err(EvalError::DuplicateKey(pair.key.to_owned()));
        }
    }

    Ok(expression.unwrap_or(Value::Map(document)))
}

fn destructure(
    pattern: &Pattern<'_>,
    value: Value,
    bindings: &mut Vec<(String, Value)>,
) -> Result<(), EvalError> {
    match pattern {
        Pattern::Name(name) => bindings.push(((*name).to_owned(), value)),
        Pattern::Map(patterns) => {
            let Value::Map(map) = value else {
                return Err(EvalError::TypeMismatch);
            };
            for pattern in patterns {
                let value = map
                    .get(pattern.key)
                    .cloned()
                    .ok_or_else(|| EvalError::UnknownIdentifier(pattern.key.to_owned()))?;
                destructure(&pattern.pattern, value, bindings)?;
            }
        }
        Pattern::List { patterns, rest } => {
            let Value::List(values) = value else {
                return Err(EvalError::TypeMismatch);
            };
            if patterns.len() > values.len() {
                return Err(EvalError::TypeMismatch);
            }
            for (pattern, value) in patterns.iter().zip(values.iter().cloned()) {
                destructure(pattern, value, bindings)?;
            }
            if let Some(name) = rest {
                bindings.push((
                    (*name).to_owned(),
                    Value::List(values.into_iter().skip(patterns.len()).collect()),
                ));
            }
        }
    }
    Ok(())
}

fn evaluate_expr(node: &Expr<'_>, scope: &Scope) -> Result<Value, EvalError> {
    match node {
        Expr::Bool(value) => Ok(Value::Bool(*value)),
        Expr::Int(value) => Ok(Value::Int(*value)),
        Expr::Float(value) => Ok(Value::Float(*value)),
        Expr::Str(value) => Ok(Value::Str(decode_string(value))),
        Expr::Id(Identifier::Simple("none" | "null" | "nil")) => Ok(Value::None),
        Expr::List(values) => values
            .iter()
            .map(|value| evaluate_expr(value, scope))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::List),
        Expr::Map(entries) => {
            let mut map = Map::new();
            for entry in entries {
                let value = evaluate_expr(&entry.expr, scope)?;
                if map.insert(entry.key, value).is_some() {
                    return Err(EvalError::DuplicateKey(entry.key.to_owned()));
                }
            }
            Ok(Value::Map(map))
        }
        Expr::Access { object, name } => evaluate_access(object, name, scope),
        Expr::Id(identifier) => {
            let Identifier::Simple(name) = identifier;
            let value = scope
                .resolve(name)
                .ok_or_else(|| EvalError::UnknownIdentifier((*name).to_owned()))?;
            Ok(value)
        }
        Expr::Unary { op, value } => evaluate_unary(op, evaluate_expr(value, scope)?),
        Expr::Binary { left, op, right } => evaluate_binary(
            op,
            evaluate_expr(left, scope)?,
            evaluate_expr(right, scope)?,
        ),
        Expr::Call { callee, arguments } => evaluate_call(callee, arguments, scope),
        Expr::If {
            condition,
            then: then_branch,
            r#else: else_branch,
        } => {
            let Value::Bool(condition) = evaluate_expr(condition, scope)? else {
                return Err(EvalError::TypeMismatch);
            };
            if condition {
                evaluate_expr(then_branch, scope)
            } else {
                evaluate_expr(else_branch, scope)
            }
        }
    }
}

fn decode_string(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }

        match characters.next() {
            Some('n') => decoded.push('\n'),
            Some('r') => decoded.push('\r'),
            Some('t') => decoded.push('\t'),
            Some('\\') | None => decoded.push('\\'),
            Some('"') => decoded.push('"'),
            Some('\'') => decoded.push('\''),
            Some(next) => {
                decoded.push('\\');
                decoded.push(next);
            }
        }
    }

    decoded
}

fn evaluate_access(node: &Expr<'_>, name: &str, scope: &Scope) -> Result<Value, EvalError> {
    let object = evaluate_expr(node, scope)?;
    resolve_member(&object, name).map(|(value, _)| value)
}

fn resolve_member(object: &Value, name: &str) -> Result<(Value, bool), EvalError> {
    match object {
        Value::Map(map) => map
            .get(name)
            .cloned()
            .map(|value| (value, false))
            .or_else(|| stdlib::type_member(object, name).map(|value| (value, true)))
            .ok_or_else(|| EvalError::UnknownIdentifier(name.to_owned())),
        value => stdlib::type_member(value, name)
            .map(|value| (value, true))
            .ok_or_else(|| EvalError::UnknownIdentifier(name.to_owned())),
    }
}

fn evaluate_call(
    callee: &Expr<'_>,
    arguments: &[Expr<'_>],
    scope: &Scope,
) -> Result<Value, EvalError> {
    let mut receiver = None;
    let callee = match callee {
        Expr::Access { object, name } => {
            let object = evaluate_expr(object, scope).map_err(|error| match error {
                EvalError::UnknownIdentifier(name) => EvalError::UnknownFunction(name),
                error => error,
            })?;
            let (value, binds_receiver) =
                resolve_member(&object, name).map_err(|error| match error {
                    EvalError::UnknownIdentifier(name) => EvalError::UnknownFunction(name),
                    error => error,
                })?;
            if binds_receiver {
                receiver = Some(object);
            }
            value
        }
        callee => evaluate_expr(callee, scope).map_err(|error| match error {
            EvalError::UnknownIdentifier(name) => EvalError::UnknownFunction(name),
            error => error,
        })?,
    };
    let Value::Function(function) = callee else {
        return Err(EvalError::UnknownFunction("value".to_owned()));
    };
    let mut arguments = arguments
        .iter()
        .map(|argument| evaluate_expr(argument, scope))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(receiver) = receiver {
        arguments.insert(0, receiver);
    }

    function(&arguments)
}

fn evaluate_unary(operator: &PrefixOp, value: Value) -> Result<Value, EvalError> {
    match (operator, value) {
        (PrefixOp::Positive, value @ (Value::Int(_) | Value::Float(_))) => Ok(value),
        (PrefixOp::Negative, Value::Int(value)) => value
            .checked_neg()
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (PrefixOp::Negative, Value::Float(value)) => Ok(Value::Float(-value)),
        (PrefixOp::Positive | PrefixOp::Negative, _) => Err(EvalError::TypeMismatch),
    }
}

fn evaluate_binary(op: &InfixOp, left: Value, right: Value) -> Result<Value, EvalError> {
    match (op, left, right) {
        (InfixOp::Add, Value::Int(left), Value::Int(right)) => left
            .checked_add(right)
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (InfixOp::Sub, Value::Int(left), Value::Int(right)) => left
            .checked_sub(right)
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (InfixOp::Mul, Value::Int(left), Value::Int(right)) => left
            .checked_mul(right)
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (InfixOp::Div, Value::Int(_), Value::Int(0))
        | (InfixOp::Div, Value::Float(_), Value::Float(0.0)) => Err(EvalError::DivisionByZero),
        (InfixOp::Div, Value::Int(left), Value::Int(right)) => left
            .checked_div(right)
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (InfixOp::Exp, Value::Int(left), Value::Int(right)) if right >= 0 => {
            let exponent = u32::try_from(right).map_err(|_| EvalError::Overflow)?;
            left.checked_pow(exponent)
                .map(Value::Int)
                .ok_or(EvalError::Overflow)
        }
        (InfixOp::Add, Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
        (InfixOp::Sub, Value::Float(left), Value::Float(right)) => Ok(Value::Float(left - right)),
        (InfixOp::Mul, Value::Float(left), Value::Float(right)) => Ok(Value::Float(left * right)),
        (InfixOp::Div, Value::Float(left), Value::Float(right)) => Ok(Value::Float(left / right)),
        (InfixOp::Exp, Value::Float(left), Value::Float(right)) => {
            Ok(Value::Float(left.powf(right)))
        }
        (InfixOp::Equal, left, right)
            if matches!(
                (&left, &right),
                (
                    Value::None | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Str(_),
                    Value::None | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Str(_),
                )
            ) =>
        {
            Ok(Value::Bool(left == right))
        }
        (operator, Value::Int(left), Value::Float(right)) => {
            evaluate_binary(operator, Value::Float(left as f64), Value::Float(right))
        }
        (operator, Value::Float(left), Value::Int(right)) => {
            evaluate_binary(operator, Value::Float(left), Value::Float(right as f64))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EvalError {
    AssertionFailed,
    DuplicateKey(String),
    DivisionByZero,
    MixedDocumentForms,
    MultipleExpressions,
    Overflow,
    TypeMismatch,
    UnknownIdentifier(String),
    UnknownFunction(String),
    UnknownModule(String),
    SymbolConflict(String),
}

#[cfg(test)]
mod tests {
    use super::test_utils::*;
    use super::*;
    use crate::{map, parser::Parser, value};

    fn evaluate(input: &str) -> Result<Value, EvalError> {
        let ast = Parser::new(input).parse_stmnts().expect("valid input");
        let mut scope = Scope::child(stdlib::new());
        evaluate_ast(&ast, &mut scope)
    }

    fn expect_document(label: &str, input: &str, expected: &[(&str, Value)]) {
        let value = evaluate(input)
            .unwrap_or_else(|error| panic!("{label}: expected valid document, got {error:?}"));
        let Value::Map(document) = value else {
            panic!("{label}: expected map value")
        };

        assert_entries(&document, expected);
    }

    #[test]
    fn evaluates_documents() {
        let cases = vec![
            ("literal entries", "count: 3", vec![("count", value!(3))]),
            (
                "boolean literal",
                "enabled: true",
                vec![("enabled", value!(true))],
            ),
            (
                "none aliases",
                "none_value: none\nnull_value: null\nnil_value: nil",
                vec![
                    ("none_value", value!(none)),
                    ("null_value", value!(null)),
                    ("nil_value", value!(nil)),
                ],
            ),
            (
                "nested numeric expression",
                "result: 1 + 2 * 3",
                vec![("result", value!(7))],
            ),
            (
                "unary and mixed numeric expressions",
                "negative: -2\nmixed: 1 + 2.5",
                vec![("negative", value!(-2)), ("mixed", value!(3.5))],
            ),
        ];

        for (label, input, expected) in cases {
            expect_document(label, input, &expected);
        }
    }

    #[test]
    fn evaluates_map_literals_and_member_access() {
        assert_eq!(
            evaluate("{outer: {value: 7}, \"quoted-key\": true}.outer.value"),
            Ok(Value::Int(7))
        );
        assert_eq!(
            evaluate("{\"quoted-key\": true}"),
            Ok(Value::Map(map! { "quoted-key": true }))
        );
        assert_eq!(evaluate("{a: 1,}.a"), Ok(value!(1)));
    }

    #[test]
    fn evaluates_destructuring_patterns() {
        assert_eq!(
            evaluate(
                "let {outer.{a, inner.{b}}} = {outer: {a: 1, inner: {b: 2}}}
                 result: a + b"
            ),
            Ok(Value::Map(map! { "result": 3 }))
        );
        assert_eq!(
            evaluate("let {assert} = import(\"std.assert\")\nassert(true)"),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            evaluate("let [a, b] = [1, 2]\nresult: a + b"),
            Ok(Value::Map(map! { "result": 3 }))
        );
        assert_eq!(
            evaluate("let [a] = [1, 2, 3]\nresult: a"),
            Ok(Value::Map(map! { "result": 1 }))
        );
        assert_eq!(
            evaluate("let [a, ..rest] = [1, 2, 3]\nresult: rest"),
            Ok(Value::Map(Map::from([(
                "result",
                Value::List(vec![value!(2), value!(3)]),
            )])))
        );
        assert_eq!(
            evaluate("let [..values] = [1, 2]\nresult: values"),
            Ok(Value::Map(Map::from([(
                "result",
                Value::List(vec![value!(1), value!(2)]),
            )])))
        );
        assert_eq!(
            evaluate("let [a, ..rest] = [1]\nresult: rest"),
            Ok(Value::Map(Map::from([
                ("result", Value::List(Vec::new()),)
            ])))
        );
    }

    #[test]
    fn rejects_list_patterns_longer_than_values() {
        assert_eq!(evaluate("let [a, b] = [1]"), Err(EvalError::TypeMismatch));
    }

    #[test]
    fn binds_let_from_a_map_pattern() {
        let ast = Parser::new("let {let} = {let: 7}")
            .parse_stmnts()
            .expect("valid destructuring pattern");
        let mut scope = Scope::child(stdlib::new());

        assert_eq!(evaluate_ast(&ast, &mut scope), Ok(Value::Map(map! {})));
        assert_eq!(scope.resolve("let"), Some(Value::Int(7)));

        let ast = Parser::new("let {let.{let}} = {let: {let: 9}}")
            .parse_stmnts()
            .expect("valid nested pattern");
        let mut scope = Scope::child(stdlib::new());

        assert_eq!(evaluate_ast(&ast, &mut scope), Ok(Value::Map(map! {})));
        assert_eq!(scope.resolve("let"), Some(Value::Int(9)));

        let ast = Parser::new("let let = 8")
            .parse_stmnts()
            .expect("valid name pattern");
        let mut scope = Scope::child(stdlib::new());

        assert_eq!(evaluate_ast(&ast, &mut scope), Ok(Value::Map(map! {})));
        assert_eq!(scope.resolve("let"), Some(Value::Int(8)));
    }

    #[test]
    fn rejects_duplicate_destructured_names_without_partial_bindings() {
        let ast = Parser::new("let {a, nested.{a}} = {a: 1, nested: {a: 2}}")
            .parse_stmnts()
            .expect("valid destructuring pattern");
        let mut scope = Scope::child(stdlib::new());

        assert_eq!(
            evaluate_ast(&ast, &mut scope),
            Err(EvalError::SymbolConflict("a".to_owned()))
        );
        assert_eq!(scope.resolve("a"), None);
    }

    #[test]
    fn evaluates_none_aliases_as_equal() {
        assert_eq!(evaluate("none == null"), Ok(value!(true)));
        assert_eq!(evaluate("null == nil"), Ok(value!(true)));
    }

    #[test]
    fn rejects_duplicate_map_keys() {
        assert_eq!(
            evaluate("{value: 1, value: 2}"),
            Err(EvalError::DuplicateKey("value".to_owned()))
        );
    }

    #[test]
    fn evaluates_expression_documents_to_values() {
        assert_eq!(evaluate("1 + 2 * 3"), Ok(value!(7)));
        assert_eq!(evaluate("count: 3"), Ok(Value::Map(map! { count: 3 })));
        assert_eq!(evaluate("[1, 2 * 3, [4, 5]]"), Ok(value!([1, 6, [4, 5]])));
    }

    #[test]
    fn evaluates_if_else_expressions_lazily() {
        assert_eq!(evaluate("if true { 1 } else { missing }"), Ok(value!(1)));
        assert_eq!(evaluate("if false { missing } else { 2 }"), Ok(value!(2)));
        assert_eq!(evaluate("if 1 == 1 { 1 } else { 2 }"), Ok(value!(1)));
        assert_eq!(evaluate("if (true) { 1 } else { missing }"), Ok(value!(1)));
        assert_eq!(evaluate("if (false) { missing } else { 2 }"), Ok(value!(2)));
        assert_eq!(
            evaluate("if (1) { 1 } else { 2 }"),
            Err(EvalError::TypeMismatch)
        );
    }

    #[test]
    fn evaluates_let_bindings_without_document_fields() {
        assert_eq!(
            evaluate("let value = { nested: 7 }\nresult: value.nested"),
            Ok(Value::Map(map! { result: 7 }))
        );
    }

    #[test]
    fn imports_standard_library_as_a_map() {
        assert_eq!(
            evaluate("let std = import(\"std\")\nstd.math.sin(std.math.PI / 2)"),
            Ok(value!(1.0))
        );
        assert_eq!(
            evaluate("let std = import(\"std\")\nstd.types.int.sqrt(9)"),
            Ok(value!(3.0))
        );
    }

    #[test]
    fn rejects_unknown_and_invalid_imports() {
        assert_eq!(
            evaluate("import(\"missing\")"),
            Err(EvalError::UnknownModule("missing".to_owned()))
        );
        assert_eq!(evaluate("import(1)"), Err(EvalError::TypeMismatch));
    }

    #[test]
    fn imports_nested_modules_by_dotted_path() {
        assert_eq!(
            evaluate("let assert = import(\"std.assert\")\nassert.assert(true)"),
            Ok(value!(true))
        );
        assert_eq!(
            evaluate("let int = import(\"types.int\")\nint.sqrt(9)"),
            Ok(value!(3.0))
        );
        assert_eq!(
            evaluate("import(\"std.missing\")"),
            Err(EvalError::UnknownModule("std.missing".to_owned()))
        );
    }

    #[test]
    fn rejects_duplicate_let_bindings() {
        assert_eq!(
            evaluate("let value = 1\nlet value = 2"),
            Err(EvalError::SymbolConflict("value".to_owned()))
        );
    }

    #[test]
    fn evaluates_strict_scalar_equality() {
        assert_eq!(evaluate("1 == 1"), Ok(value!(true)));
        assert_eq!(evaluate("1 == 2"), Ok(value!(false)));
        assert_eq!(evaluate("1 == 1.0"), Ok(value!(false)));
        assert_eq!(evaluate("true == true"), Ok(value!(true)));
        assert_eq!(evaluate("\"value\" == \"value\""), Ok(value!(true)));
        assert_eq!(evaluate("1 + 2 == 3"), Ok(value!(true)));
    }

    #[test]
    fn decodes_string_escapes() {
        assert_eq!(
            evaluate(r#""line\n\t\"quote\"\\path""#),
            Ok(value!("line\n\t\"quote\"\\path"))
        );
        assert_eq!(evaluate(r#""unknown\q""#), Ok(value!(r"unknown\q")));
    }

    #[test]
    fn rejects_equality_for_unsupported_values() {
        assert_eq!(evaluate("[1] == [1]"), Err(EvalError::TypeMismatch));
    }

    #[test]
    fn resolves_qualified_symbols_and_imports() {
        assert_eq!(
            evaluate("let std = import(\"std\")\nstd.math.PI"),
            Ok(value!(std::f64::consts::PI))
        );
        assert_eq!(
            evaluate("let std = import(\"std\")\nstd.math.sin(std.math.PI / 2)"),
            Ok(value!(1.0))
        );
    }

    #[test]
    fn rejects_division_by_zero() {
        assert_eq!(evaluate("result: 1 / 0"), Err(EvalError::DivisionByZero));
    }

    #[test]
    fn resolves_chained_access_and_calls() {
        assert_eq!(
            evaluate("let types = import(\"types\")\n(types.int.sqrt)(9)"),
            Ok(value!(3.0))
        );
        assert_eq!(
            evaluate("result: 9.sqrt()"),
            Ok(Value::Map(map! { result: 3.0 }))
        );
    }

    #[test]
    fn expect_errors() {
        let cases = vec![
            (
                "division by zero",
                "valid: 1\nresult: 1 / 0",
                EvalError::DivisionByZero,
            ),
            (
                "integer overflow",
                "valid: 1\nresult: 9223372036854775807 + 1",
                EvalError::Overflow,
            ),
            (
                "type mismatch",
                "valid: 1\nresult: true + 1",
                EvalError::TypeMismatch,
            ),
            (
                "unsupported expression",
                "valid: 1\nresult: unknown",
                EvalError::UnknownIdentifier("unknown".to_owned()),
            ),
            (
                "unknown function",
                "result: missing(1)",
                EvalError::UnknownFunction("missing".to_owned()),
            ),
            (
                "duplicate key",
                "result: 1\nresult: 2",
                EvalError::DuplicateKey("result".to_owned()),
            ),
            (
                "mixed document forms",
                "1\nresult: 2",
                EvalError::MixedDocumentForms,
            ),
            (
                "multiple expressions",
                "1\n2",
                EvalError::MultipleExpressions,
            ),
        ];

        for (label, input, expected) in cases {
            assert_eq!(
                evaluate(input),
                Err(expected),
                "{label}: input was {input:?}"
            );
        }
    }

    #[test]
    fn asserts_nested_entries_with_dotted_paths() {
        let document = map! {
            server: value!({ database: { host: "localhost" } }),
        };

        super::test_utils::assert_entries(
            &document,
            &[("server.database.host", value!("localhost"))],
        );
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    #[allow(clippy::missing_panics_doc)]
    pub fn assert_entries(document: &Document, expected: &[(&str, Value)]) {
        for (key, value) in expected {
            assert_eq!(document.get_path(key), Some(value));
        }
    }
}
