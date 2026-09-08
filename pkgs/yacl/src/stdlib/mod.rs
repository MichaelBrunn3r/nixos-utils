pub mod boolean;
pub mod float;
pub mod int;
pub mod map;
pub mod math;
pub mod string;

use std::rc::Rc;

use crate::scope::{Scope, Symbol};

#[must_use]
pub fn new() -> Rc<Scope<'static>> {
    let types = Scope::from_symbols([
        ("bool", Symbol::Scope(boolean::create_scope())),
        ("float", Symbol::Scope(float::create_scope())),
        ("int", Symbol::Scope(int::create_scope())),
        ("map", Symbol::Scope(map::create_scope())),
        ("str", Symbol::Scope(string::create_scope())),
    ]);
    Rc::new(Scope::from_symbols([
        ("types", Symbol::Scope(types)),
        (
            "std",
            Symbol::Scope(Scope::from_symbols([(
                "math",
                Symbol::Scope(math::create_scope()),
            )])),
        ),
    ]))
}
