#![allow(
    clippy::cast_precision_loss,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation
)]

use std::collections::BTreeMap;

use crate::{
    ast::{AST, BinaryOp, Expr, Identifier, Statement, UnaryOp},
    builtins,
};

pub fn evaluate_ast<'input>(
    ast: &AST<'input>,
    root: &Scope<'static>,
) -> Result<Value<'input>, EvalError> {
    let mut scope = Scope::child();
    let mut document = Document::new();
    let mut expression = None;

    for statement in &ast.statements {
        let value = match statement {
            Statement::Use(import) => {
                scope.import(import, root)?;
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
        Expr::Str(value) => Ok(Value::Str(value)),
        Expr::Id(Identifier::Simple(name)) => match scope.symbols.get(name) {
            Some(Symbol::Value(value)) => Ok(value.clone()),
            Some(Symbol::Function(_) | Symbol::Module(_)) => {
                Err(EvalError::UnknownIdentifier((*name).to_owned()))
            }
            None => Err(EvalError::UnknownIdentifier((*name).to_owned())),
        },
        Expr::Id(Identifier::Qualified(path)) => scope
            .resolve_path(path)
            .and_then(symbol_value)
            .ok_or_else(|| EvalError::UnknownIdentifier(path.join("."))),
        Expr::Unary { op, value } => evaluate_unary(op, evaluate_expr(value, scope)?),
        Expr::Binary { left, op, right } => evaluate_binary(
            op,
            evaluate_expr(left, scope)?,
            evaluate_expr(right, scope)?,
        ),
        Expr::Call { path, arguments } => evaluate_call(path, arguments, scope),
    }
}

fn evaluate_call<'input>(
    path: &[&str],
    arguments: &[Expr<'input>],
    scope: &Scope<'input>,
) -> Result<Value<'input>, EvalError> {
    let name = path.join(".");
    let function = match scope.resolve_path(path) {
        Some(Symbol::Function(function)) => function,
        Some(Symbol::Value(_) | Symbol::Module(_)) | None => {
            return Err(EvalError::UnknownFunction(name));
        }
    };
    let arguments = arguments
        .iter()
        .map(|argument| evaluate_expr(argument, scope))
        .collect::<Result<Vec<_>, _>>()?;

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

#[derive(Debug, PartialEq, Clone)]
pub enum Value<'input> {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'input str),
    Map(Map<'input>),
}

//region Scope
#[derive(Clone)]
pub struct Scope<'input> {
    symbols: BTreeMap<&'input str, Symbol<'input>>,
}

type BuiltinFunction = for<'input> fn(&[Value<'input>]) -> Result<Value<'input>, EvalError>;

#[derive(Clone)]
pub enum Symbol<'input> {
    Value(Value<'input>),
    Function(BuiltinFunction),
    Module(Module),
}

#[derive(Clone, Copy)]
pub enum Module {
    Std,
}

impl<'input> Scope<'input> {
    #[must_use]
    pub fn root() -> Scope<'static> {
        Scope {
            symbols: BTreeMap::from([("std", Symbol::Module(Module::Std))]),
        }
    }

    const fn child() -> Self {
        Self {
            symbols: BTreeMap::new(),
        }
    }

    fn import(
        &mut self,
        import: &crate::ast::Use<'input>,
        root: &Scope<'static>,
    ) -> Result<(), EvalError> {
        if import.path.len() == 1 && !import.wildcard {
            let name = import.path[0];
            let module = root.resolve_module(&import.path)?;
            self.insert_symbol(name, Symbol::Module(module))?;
            return Ok(());
        }

        let module_path = if import.wildcard {
            &import.path[..]
        } else {
            &import.path[..import.path.len() - 1]
        };
        let module = root.resolve_module(module_path)?;
        let name = *import.path.last().expect("validated non-empty import");
        if import.wildcard {
            for (name, symbol) in module_symbols(module) {
                self.insert_symbol(name, symbol)?;
            }
        } else {
            let symbol = module_symbols(module)
                .into_iter()
                .find(|(symbol_name, _)| *symbol_name == name)
                .map(|(_, symbol)| symbol)
                .ok_or_else(|| EvalError::UnknownIdentifier(import.path.join(".")))?;
            self.insert_symbol(name, symbol)?;
        }
        Ok(())
    }

    fn insert_symbol(
        &mut self,
        name: &'input str,
        symbol: Symbol<'input>,
    ) -> Result<(), EvalError> {
        if let Some(existing) = self.symbols.get(name) {
            if same_symbol(existing, &symbol) {
                return Ok(());
            }
            return Err(EvalError::SymbolConflict(name.to_owned()));
        }
        self.symbols.insert(name, symbol);
        Ok(())
    }

    fn resolve_module(&self, path: &[&str]) -> Result<Module, EvalError> {
        if path.len() != 1 {
            return Err(EvalError::UnknownModule(path.join(".")));
        }
        match self.symbols.get(path[0]) {
            Some(Symbol::Module(module)) => Ok(*module),
            _ => Err(EvalError::UnknownModule(path.join("."))),
        }
    }

    fn resolve_path(&self, path: &[&str]) -> Option<Symbol<'input>> {
        if path.len() == 1 {
            return self.symbols.get(path[0]).cloned();
        }
        let module = match self.symbols.get(path[0])? {
            Symbol::Module(module) => *module,
            _ => return None,
        };
        module_symbols(module)
            .into_iter()
            .find(|(name, _)| *name == path[1])
            .map(|(_, symbol)| symbol)
    }
}

fn module_symbols(module: Module) -> Vec<(&'static str, Symbol<'static>)> {
    match module {
        Module::Std => vec![
            ("pi", Symbol::Value(builtins::PI)),
            ("abs", Symbol::Function(builtins::abs)),
            ("acos", Symbol::Function(builtins::acos)),
            ("asin", Symbol::Function(builtins::asin)),
            ("atan", Symbol::Function(builtins::atan)),
            ("atan2", Symbol::Function(builtins::atan2)),
            ("clamp", Symbol::Function(builtins::clamp)),
            ("sin", Symbol::Function(builtins::sin)),
            ("cos", Symbol::Function(builtins::cos)),
            ("floor", Symbol::Function(builtins::floor)),
            ("ceil", Symbol::Function(builtins::ceil)),
            ("round", Symbol::Function(builtins::round)),
            ("is_finite", Symbol::Function(builtins::is_finite)),
            ("is_infinite", Symbol::Function(builtins::is_infinite)),
            ("is_nan", Symbol::Function(builtins::is_nan)),
            ("ln", Symbol::Function(builtins::ln)),
            ("log", Symbol::Function(builtins::log)),
            ("log2", Symbol::Function(builtins::log2)),
            ("log10", Symbol::Function(builtins::log10)),
            ("max", Symbol::Function(builtins::max)),
            ("min", Symbol::Function(builtins::min)),
            ("tan", Symbol::Function(builtins::tan)),
            ("sqrt", Symbol::Function(builtins::sqrt)),
        ],
    }
}

fn symbol_value(symbol: Symbol<'_>) -> Option<Value<'_>> {
    match symbol {
        Symbol::Value(value) => Some(value),
        Symbol::Function(_) | Symbol::Module(_) => None,
    }
}

fn same_symbol(left: &Symbol<'_>, right: &Symbol<'_>) -> bool {
    match (left, right) {
        (Symbol::Value(left), Symbol::Value(right)) => left == right,
        (Symbol::Function(left), Symbol::Function(right)) => std::ptr::fn_addr_eq(*left, *right),
        (Symbol::Module(left), Symbol::Module(right)) => {
            std::mem::discriminant(left) == std::mem::discriminant(right)
        }
        _ => false,
    }
}

//endregion Scope

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::test_utils::*;
    use super::*;
    use crate::parser::Parser;

    fn evaluate(input: &str) -> Result<Value<'_>, EvalError> {
        let ast = Parser::new(input).parse().expect("valid input");
        let scope = Scope::root();
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
                "count = 3",
                vec![("count", Value::Int(3))],
            ),
            (
                "boolean literal",
                "enabled = true",
                vec![("enabled", Value::Bool(true))],
            ),
            (
                "nested numeric expression",
                "result = 1 + 2 * 3",
                vec![("result", Value::Int(7))],
            ),
            (
                "unary and mixed numeric expressions",
                "negative = -2\nmixed = 1 + 2.5",
                vec![("negative", Value::Int(-2)), ("mixed", Value::Float(3.5))],
            ),
        ];

        for (label, input, expected) in cases {
            expect_document(label, input, &expected);
        }
    }

    #[test]
    fn evaluates_expression_documents_to_values() {
        assert_eq!(evaluate("1 + 2 * 3"), Ok(Value::Int(7)));
        assert_eq!(
            evaluate("count = 3"),
            Ok(Value::Map(BTreeMap::from([("count", Value::Int(3))])))
        );
    }

    #[test]
    fn rejects_division_by_zero() {
        assert_eq!(evaluate("result = 1 / 0"), Err(EvalError::DivisionByZero));
    }

    #[test]
    fn expect_err_msg() {
        let cases = vec![
            (
                "division by zero",
                "valid = 1\nresult = 1 / 0",
                EvalError::DivisionByZero,
            ),
            (
                "integer overflow",
                "valid = 1\nresult = 9223372036854775807 + 1",
                EvalError::Overflow,
            ),
            (
                "type mismatch",
                "valid = 1\nresult = true + 1",
                EvalError::TypeMismatch,
            ),
            (
                "unsupported expression",
                "valid = 1\nresult = unknown",
                EvalError::UnknownIdentifier("unknown".to_owned()),
            ),
            (
                "unknown function",
                "result = missing(1)",
                EvalError::UnknownFunction("missing".to_owned()),
            ),
            (
                "duplicate key",
                "result = 1\nresult = 2",
                EvalError::DuplicateKey("result".to_owned()),
            ),
            (
                "mixed document forms",
                "1\nresult = 2",
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
        assert_duplicate_key(&evaluate("count = 1\ncount = 2"), "count");
    }

    #[test]
    fn asserts_nested_entries_with_dotted_paths() {
        let mut database = BTreeMap::new();
        database.insert("host", Value::Str("localhost"));

        let mut server = BTreeMap::new();
        server.insert("database", Value::Map(database));

        let mut document = BTreeMap::new();
        document.insert("server", Value::Map(server));

        super::test_utils::assert_entries(
            &document,
            &[("server.database.host", Value::Str("localhost"))],
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
