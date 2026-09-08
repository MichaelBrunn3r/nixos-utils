use crate::scope::Scope;

#[must_use]
pub fn create_scope() -> Scope<'static> {
    Scope::from_symbols([])
}
