use std::{collections::BTreeMap, rc::Rc};

use crate::eval::{EvalError, Value};

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
    pub fn from_symbols(symbols: impl IntoIterator<Item = (&'input str, Symbol<'input>)>) -> Self {
        Self {
            symbols: symbols
                .into_iter()
                .map(|(name, symbol)| (name, Rc::new(symbol)))
                .collect(),
            parent: None,
        }
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
