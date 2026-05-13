use std::ops::Range;

use anyhow::{Result, anyhow};

use super::Debugger;
use crate::debug::Variable;

pub struct ChildCounts {
    /// The number of indexed children the variable has.
    ///
    /// Indexed children usually correspond to elements in container data types and
    /// usually have names like `[0]`, `[1]`, `[2]`, and so on.
    pub indexed: usize,

    /// The number of named children the variable has.
    ///
    /// Named children usually correspond to members in structured data types and have
    /// names corresponding to the names of those members.
    pub named: usize,
}

impl ChildCounts {
    /// Children counts for a variable with `indexed` indexed children and `named` named children.
    pub fn mixed(indexed: usize, named: usize) -> ChildCounts {
        ChildCounts { indexed, named }
    }

    /// Children counts for a variable with `count` indexed children and no named children.
    pub fn indexed(count: usize) -> ChildCounts {
        ChildCounts {
            indexed: count,
            named: 0,
        }
    }

    /// Children counts for a variable with `count` named children and no indexed children.
    pub fn named(count: usize) -> ChildCounts {
        ChildCounts {
            indexed: 0,
            named: count,
        }
    }
}

/// Provides custom expansion for a [Variable].
///
/// Implement this trait to provide custom formatting for variable children
/// and/or variable values.
pub trait VariableFormatter {
    /// Performs a match on a [Variable] to see if this formatter can
    /// format it. If this returns `true`, this formatter confirms that it can format the variable.
    fn matches(&self, value: &Variable) -> bool;

    /// Computes the number of children that a variable has.
    ///
    /// Returns [None] if this formatter cannot provide children for this node.
    /// If the return value is [Some], the debugger may proceed with calling
    /// [indexed_children](Self::indexed_children) and [named_children](Self::named_children)
    /// for this variable.
    #[allow(unused)]
    fn num_children(&self, value: &Variable) -> Result<ChildCounts>;

    /// Provides the indexed children for a [Variable] within a range of indices.
    #[allow(unused)]
    fn indexed_children(&self, value: &Variable, range: Range<usize>) -> Result<Vec<Variable>> {
        Err(anyhow!("indexed_children not implemented"))
    }

    /// Provides the named children for a [Variable] within a range of indices.
    #[allow(unused)]
    fn named_children(&self, value: &Variable, range: Range<usize>) -> Result<Vec<Variable>> {
        Err(anyhow!("named_children not implemented"))
    }

    /// Renders the value for a [Variable].
    ///
    /// The first matching formatter who returns a non-[None] value from
    /// [display](Self::display) wins and replaces the default expansion logic.
    ///
    /// In order to handle errors, if this method returns [None], matching will
    /// proceed with the next registered provider, or the default one if none exist.
    #[allow(unused)]
    fn display(&self, value: &Variable) -> Result<String> {
        Ok(value.display())
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
