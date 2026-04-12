use crate::{
    debug::dwarf::{Die, Visit},
    util::weak_error,
};
use gimli::read::Expression;

use super::R;

/// Gets all live variables at the given PC.
///
/// `die` should be a reference to a subprogram,
/// and `pc` should be an offset within the wasm code segment.
pub fn get_variables<'a>(die: &Die<'a>, pc: usize) -> Vec<Die<'a>> {
    assert!(
        die.tag() == gimli::DW_TAG_subprogram,
        "get_variables requires subprogram die"
    );

    let mut result: Vec<Die<'a>> = vec![];

    die.traverse(|child| {
        let tag = child.tag();

        match tag {
            gimli::DW_TAG_formal_parameter | gimli::DW_TAG_local_variable => {
                result.push(child);
            }

            _ => {
                if let Some(low) = child.low_pc()
                    && pc < low
                {
                    return Visit::SkipChildren;
                }

                if let Some(high) = child.high_pc()
                    && pc >= high
                {
                    return Visit::SkipChildren;
                }
            }
        }

        Visit::Continue
    });

    result
}

/// Gets the location expression for a variable at the given PC
pub fn get_location(die: &Die<'_>, pc: usize) -> Option<Expression<R>> {
    let Some(attr) = die.attr_value(gimli::DW_AT_location) else {
        return None;
    };

    let unit = die.ctx().unit_ref();
    let addr = pc as u64;

    match attr {
        gimli::AttributeValue::Exprloc(expr) => Some(expr),
        other => {
            let Some(it) = weak_error!(unit.attr_locations(other))? else {
                return None;
            };
            for res in it {
                let entry = weak_error!(res)?;
                if addr >= entry.range.begin && addr < entry.range.end {
                    return Some(entry.data);
                }
            }
            None
        }
    }
}
