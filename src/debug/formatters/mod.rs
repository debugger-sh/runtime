use super::Debugger;
use crate::debug::Variable;

/// Provides custom expansion for a [`Variable`].
///
/// The first registered provider whose [`matches`](Self::matches) returns
/// `true` for a variable wins; its [`children`](Self::children) result replaces
/// the default structure/pointer expansion. Providers that only need to alter
/// matching can rely on the default `children` implementation, which yields the
/// raw structural view.
pub trait VariableProvider {
    fn matches(&self, value: &Variable) -> bool;

    fn children(&self, value: &Variable) -> Vec<Variable> {
        value.children()
    }

    fn display(&self, value: &Variable) -> String {
        value.display()
    }
}

/// Registers the built-in formatters on `dbg`.
pub fn register_defaults(_dbg: &mut Debugger) {}

// Reusable for various formatter that follow the same access pattern
pub trait VariableSliceExt {
    fn find(&self, name: &str) -> Option<&Variable>;
}

impl VariableSliceExt for [Variable] {
    fn find(&self, name: &str) -> Option<&Variable> {
        self.iter().find(|v| v.name() == name)
    }
}
