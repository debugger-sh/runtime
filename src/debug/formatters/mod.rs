//! Built-in [`VariableProvider`](super::VariableProvider) implementations and
//! their default registration on a [`Debugger`](super::Debugger).

use super::Debugger;

/// Registers the built-in formatters on `dbg`.
pub fn register_defaults(_dbg: &mut Debugger) {}
