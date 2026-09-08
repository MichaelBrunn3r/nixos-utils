use std::{collections::BTreeMap, rc::Rc};

use crate::{
    builtins,
    eval::{EvalError, Value},
};

#[derive(Clone)]
pub struct Scope<'input> {
    pub symbols: BTreeMap<&'input str, SymbolRef<'input>>,
    parent: Option<Rc<Self>>,
}

type BuiltinFunction = for<'input> fn(&[Value<'input>]) -> Result<Value<'input>, EvalError>;
pub type SymbolRef<'input> = Rc<Symbol<'input>>;

pub enum Symbol<'input> {
    Value(Value<'input>),
    Function(BuiltinFunction),
    Scope(Scope<'input>),
}

impl<'input> Scope<'input> {
    #[must_use]
    pub fn root() -> Rc<Scope<'static>> {
        let std = Scope {
            symbols: std_symbols(),
            parent: None,
        };
        Rc::new(Scope {
            symbols: BTreeMap::from([("std", Rc::new(Symbol::Scope(std)))]),
            parent: None,
        })
    }

    #[must_use]
    pub const fn child(parent: Rc<Self>) -> Self {
        Self {
            symbols: BTreeMap::new(),
            parent: Some(parent),
        }
    }

    /// Imports a module or symbol into this scope.
    ///
    /// # Errors
    ///
    /// Returns an error when the import refers to an unknown module or symbol,
    /// or when it conflicts with an existing symbol.
    pub fn import(
        &mut self,
        import: &crate::ast::Use<'input>,
        root: &Scope<'static>,
    ) -> Result<(), EvalError> {
        if import.path.is_empty() {
            return Err(EvalError::UnknownModule(String::new()));
        }

        if import.path.len() == 1 && !import.wildcard {
            let name = import.path[0];
            let symbol = root
                .resolve_path(&import.path)
                .ok_or_else(|| EvalError::UnknownModule(name.to_owned()))?;
            if !matches!(symbol.as_ref(), Symbol::Scope(_)) {
                return Err(EvalError::UnknownModule(name.to_owned()));
            }
            self.insert_symbol(name, symbol)?;
            return Ok(());
        }

        if import.wildcard {
            let module_symbol = root
                .resolve_path(&import.path)
                .ok_or_else(|| EvalError::UnknownModule(import.path.join(".")))?;
            let module = match module_symbol.as_ref() {
                Symbol::Scope(scope) => scope,
                Symbol::Value(_) | Symbol::Function(_) => {
                    return Err(EvalError::UnknownModule(import.path.join(".")));
                }
            };
            for (name, symbol) in &module.symbols {
                self.insert_symbol(name, Rc::clone(symbol))?;
            }
        } else {
            let name = import.path[import.path.len() - 1];
            let symbol = root
                .resolve_path(&import.path)
                .ok_or_else(|| EvalError::UnknownIdentifier(import.path.join(".")))?;
            self.insert_symbol(name, symbol)?;
        }
        Ok(())
    }

    fn insert_symbol(
        &mut self,
        name: &'input str,
        symbol: SymbolRef<'input>,
    ) -> Result<(), EvalError> {
        if let Some(existing) = self.symbols.get(name) {
            if Rc::ptr_eq(existing, &symbol) {
                return Ok(());
            }
            return Err(EvalError::SymbolConflict(name.to_owned()));
        }
        self.symbols.insert(name, symbol);
        Ok(())
    }

    #[must_use]
    pub fn resolve_path(&self, path: &[&str]) -> Option<SymbolRef<'input>> {
        let mut symbol = self.resolve_name(path.first()?)?;
        for name in &path[1..] {
            symbol = match symbol.as_ref() {
                Symbol::Scope(scope) => scope.symbols.get(name).cloned()?,
                Symbol::Value(_) | Symbol::Function(_) => return None,
            };
        }
        Some(symbol)
    }

    fn resolve_name(&self, name: &str) -> Option<SymbolRef<'input>> {
        self.symbols
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_deref()?.resolve_name(name))
    }
}

fn std_symbols() -> BTreeMap<&'static str, SymbolRef<'static>> {
    BTreeMap::from([
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
    ])
    .into_iter()
    .map(|(name, symbol)| (name, Rc::new(symbol)))
    .collect()
}
