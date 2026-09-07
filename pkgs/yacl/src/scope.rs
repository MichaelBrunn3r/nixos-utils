use std::collections::BTreeMap;

use crate::{
    builtins,
    eval::{EvalError, Value},
};

#[derive(Clone)]
pub struct Scope<'input> {
    pub symbols: BTreeMap<&'input str, Symbol<'input>>,
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

    #[must_use]
    pub const fn child() -> Self {
        Self {
            symbols: BTreeMap::new(),
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
        let name = import.path[import.path.len() - 1];
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

    #[must_use]
    pub fn resolve_path(&self, path: &[&str]) -> Option<Symbol<'input>> {
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

#[must_use]
pub fn symbol_value(symbol: Symbol<'_>) -> Option<Value<'_>> {
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
