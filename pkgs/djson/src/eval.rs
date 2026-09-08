#![allow(
    clippy::cast_precision_loss,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation
)]

use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    ast::{AST, BinaryOp, Expr, Identifier, Statement, UnaryOp},
    scope::{BuiltinFunction, Scope},
    stdlib,
};

pub fn evaluate_ast<'input>(
    ast: &AST<'input>,
    root: &std::rc::Rc<Scope<'static>>,
) -> Result<Value<'input>, EvalError> {
    let mut scope = Scope::child(std::rc::Rc::clone(root));
    let mut document = Document::new();
    let mut expression = None;

    for statement in &ast.statements {
        let value = match statement {
            Statement::Let(binding) => {
                let value = evaluate_expr(&binding.expr, &scope)?;
                scope.bind_value(binding.name, value)?;
                continue;
            }
            Statement::Expr(node) => {
                if !document.is_empty() {
                    return Err(EvalError::MixedDocumentForms);
                }
                if expression.is_some() {
                    return Err(EvalError::MultipleExpressions);
                }
                expression = Some(evaluate_expr(node, &scope)?);
                continue;
            }
            Statement::KV(pair) => {
                if expression.is_some() {
                    return Err(EvalError::MixedDocumentForms);
                }
                evaluate_expr(&pair.expr, &scope)?
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

fn evaluate_expr<'input>(
    node: &Expr<'input>,
    scope: &Scope<'input>,
) -> Result<Value<'input>, EvalError> {
    match node {
        Expr::Bool(value) => Ok(Value::Bool(*value)),
        Expr::Int(value) => Ok(Value::Int(*value)),
        Expr::Float(value) => Ok(Value::Float(*value)),
        Expr::Str(value) => Ok(Value::Str(decode_string(value).into())),
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

fn evaluate_access<'input>(
    object: &Expr<'input>,
    name: &str,
    scope: &Scope<'input>,
) -> Result<Value<'input>, EvalError> {
    let object = evaluate_expr(object, scope)?;
    resolve_member(&object, name).map(|(value, _)| value)
}

fn resolve_member<'input>(
    object: &Value<'input>,
    name: &str,
) -> Result<(Value<'input>, bool), EvalError> {
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

fn evaluate_call<'input>(
    callee: &Expr<'input>,
    arguments: &[Expr<'input>],
    scope: &Scope<'input>,
) -> Result<Value<'input>, EvalError> {
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

fn evaluate_unary<'input>(
    operator: &UnaryOp,
    value: Value<'input>,
) -> Result<Value<'input>, EvalError> {
    match (operator, value) {
        (UnaryOp::Positive, value @ (Value::Int(_) | Value::Float(_))) => Ok(value),
        (UnaryOp::Negative, Value::Int(value)) => value
            .checked_neg()
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (UnaryOp::Negative, Value::Float(value)) => Ok(Value::Float(-value)),
        (UnaryOp::Positive | UnaryOp::Negative, _) => Err(EvalError::TypeMismatch),
    }
}

fn evaluate_binary<'input>(
    op: &BinaryOp,
    left: Value<'input>,
    right: Value<'input>,
) -> Result<Value<'input>, EvalError> {
    match (op, left, right) {
        (BinaryOp::Add, Value::Int(left), Value::Int(right)) => left
            .checked_add(right)
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (BinaryOp::Sub, Value::Int(left), Value::Int(right)) => left
            .checked_sub(right)
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (BinaryOp::Mul, Value::Int(left), Value::Int(right)) => left
            .checked_mul(right)
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (BinaryOp::Div, Value::Int(_), Value::Int(0))
        | (BinaryOp::Div, Value::Float(_), Value::Float(0.0)) => Err(EvalError::DivisionByZero),
        (BinaryOp::Div, Value::Int(left), Value::Int(right)) => left
            .checked_div(right)
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        (BinaryOp::Exp, Value::Int(left), Value::Int(right)) if right >= 0 => {
            let exponent = u32::try_from(right).map_err(|_| EvalError::Overflow)?;
            left.checked_pow(exponent)
                .map(Value::Int)
                .ok_or(EvalError::Overflow)
        }
        (BinaryOp::Add, Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
        (BinaryOp::Sub, Value::Float(left), Value::Float(right)) => Ok(Value::Float(left - right)),
        (BinaryOp::Mul, Value::Float(left), Value::Float(right)) => Ok(Value::Float(left * right)),
        (BinaryOp::Div, Value::Float(left), Value::Float(right)) => Ok(Value::Float(left / right)),
        (BinaryOp::Exp, Value::Float(left), Value::Float(right)) => {
            Ok(Value::Float(left.powf(right)))
        }
        (BinaryOp::Equal, left, right)
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

pub type Document<'input> = Map<'input>;
pub type Map<'input> = BTreeMap<&'input str, Value<'input>>;

#[derive(Debug, Clone)]
pub enum Value<'input> {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Cow<'input, str>),
    List(Vec<Self>),
    Map(Map<'input>),
    Function(BuiltinFunction),
}

impl PartialEq for Value<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Str(left), Self::Str(right)) => left == right,
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Map(left), Self::Map(right)) => left == right,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::test_utils::*;
    use super::*;
    use crate::parser::Parser;

    fn evaluate(input: &str) -> Result<Value<'_>, EvalError> {
        let ast = Parser::new(input).parse().expect("valid input");
        let scope = crate::stdlib::new();
        evaluate_ast(&ast, &scope)
    }

    fn expect_document(label: &str, input: &str, expected: &[(&str, Value<'_>)]) {
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
            (
                "literal entries",
                "count: 3",
                vec![("count", Value::Int(3))],
            ),
            (
                "boolean literal",
                "enabled: true",
                vec![("enabled", Value::Bool(true))],
            ),
            (
                "none aliases",
                "none_value: none\nnull_value: null\nnil_value: nil",
                vec![
                    ("none_value", Value::None),
                    ("null_value", Value::None),
                    ("nil_value", Value::None),
                ],
            ),
            (
                "nested numeric expression",
                "result: 1 + 2 * 3",
                vec![("result", Value::Int(7))],
            ),
            (
                "unary and mixed numeric expressions",
                "negative: -2\nmixed: 1 + 2.5",
                vec![("negative", Value::Int(-2)), ("mixed", Value::Float(3.5))],
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
            Ok(Value::Map(BTreeMap::from([(
                "quoted-key",
                Value::Bool(true),
            )])))
        );
        assert_eq!(evaluate("{a: 1,}.a"), Ok(Value::Int(1)));
    }

    #[test]
    fn evaluates_none_aliases_as_equal() {
        assert_eq!(evaluate("none == null"), Ok(Value::Bool(true)));
        assert_eq!(evaluate("null == nil"), Ok(Value::Bool(true)));
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
        assert_eq!(evaluate("1 + 2 * 3"), Ok(Value::Int(7)));
        assert_eq!(
            evaluate("count: 3"),
            Ok(Value::Map(BTreeMap::from([("count", Value::Int(3))])))
        );
        assert_eq!(
            evaluate("[1, 2 * 3, [4, 5]]"),
            Ok(Value::List(vec![
                Value::Int(1),
                Value::Int(6),
                Value::List(vec![Value::Int(4), Value::Int(5)]),
            ]))
        );
    }

    #[test]
    fn evaluates_let_bindings_without_document_fields() {
        assert_eq!(
            evaluate("let value = { nested: 7 }\nresult: value.nested"),
            Ok(Value::Map(BTreeMap::from([("result", Value::Int(7))])))
        );
    }

    #[test]
    fn imports_standard_library_as_a_map() {
        assert_eq!(
            evaluate("let std = import(\"std\")\nstd.math.sin(std.math.PI / 2)"),
            Ok(Value::Float(1.0))
        );
        assert_eq!(
            evaluate("let std = import(\"std\")\nstd.types.int.sqrt(9)"),
            Ok(Value::Float(3.0))
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
    fn rejects_duplicate_let_bindings() {
        assert_eq!(
            evaluate("let value = 1\nlet value = 2"),
            Err(EvalError::SymbolConflict("value".to_owned()))
        );
    }

    #[test]
    fn evaluates_strict_scalar_equality() {
        assert_eq!(evaluate("1 == 1"), Ok(Value::Bool(true)));
        assert_eq!(evaluate("1 == 2"), Ok(Value::Bool(false)));
        assert_eq!(evaluate("1 == 1.0"), Ok(Value::Bool(false)));
        assert_eq!(evaluate("true == true"), Ok(Value::Bool(true)));
        assert_eq!(evaluate("\"value\" == \"value\""), Ok(Value::Bool(true)));
        assert_eq!(evaluate("1 + 2 == 3"), Ok(Value::Bool(true)));
    }

    #[test]
    fn decodes_string_escapes() {
        assert_eq!(
            evaluate(r#""line\n\t\"quote\"\\path""#),
            Ok(Value::Str("line\n\t\"quote\"\\path".into()))
        );
        assert_eq!(
            evaluate(r#""unknown\q""#),
            Ok(Value::Str(r"unknown\q".into()))
        );
    }

    #[test]
    fn rejects_equality_for_unsupported_values() {
        assert_eq!(evaluate("[1] == [1]"), Err(EvalError::TypeMismatch));
    }

    #[test]
    fn resolves_qualified_symbols_and_imports() {
        assert_eq!(
            evaluate("let std = import(\"std\")\nstd.math.PI"),
            Ok(Value::Float(std::f64::consts::PI))
        );
        assert_eq!(
            evaluate("let std = import(\"std\")\nstd.math.sin(std.math.PI / 2)"),
            Ok(Value::Float(1.0))
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
            Ok(Value::Float(3.0))
        );
        assert_eq!(
            evaluate("result: 9.sqrt()"),
            Ok(Value::Map(BTreeMap::from([("result", Value::Float(3.0),)])))
        );
    }

    #[test]
    fn expect_err_msg() {
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
    fn rejects_duplicate_keys() {
        assert_duplicate_key(&evaluate("count: 1\ncount: 2"), "count");
    }

    #[test]
    fn asserts_nested_entries_with_dotted_paths() {
        let mut database = BTreeMap::new();
        database.insert("host", Value::Str(Cow::Borrowed("localhost")));

        let mut server = BTreeMap::new();
        server.insert("database", Value::Map(database));

        let mut document = BTreeMap::new();
        document.insert("server", Value::Map(server));

        super::test_utils::assert_entries(
            &document,
            &[(
                "server.database.host",
                Value::Str(Cow::Borrowed("localhost")),
            )],
        );
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    #[allow(clippy::missing_panics_doc)]
    pub fn assert_entries(document: &Document<'_>, expected: &[(&str, Value<'_>)]) {
        for (key, value) in expected {
            assert_eq!(value_at_path(document, key), Some(value));
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn assert_duplicate_key(result: &Result<Value<'_>, EvalError>, key: &str) {
        assert_eq!(*result, Err(EvalError::DuplicateKey(key.to_owned())));
    }

    fn value_at_path<'document, 'input>(
        document: &'document Document<'input>,
        path: &str,
    ) -> Option<&'document Value<'input>> {
        let mut value = None;

        for (index, segment) in path.split('.').enumerate() {
            value = if index == 0 {
                document.get(segment)
            } else {
                match value? {
                    Value::Map(map) => map.get(segment),
                    _ => return None,
                }
            };
        }

        value
    }
}
