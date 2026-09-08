pub mod math;

use std::rc::Rc;

use crate::scope::{Scope, Symbol};

#[must_use]
pub fn new() -> Rc<Scope<'static>> {
    Rc::new(Scope::from_symbols([(
        "std",
        Symbol::Scope(Scope::from_symbols([(
            "math",
            Symbol::Scope(math::create_scope()),
        )])),
    )]))
}
