//! WASM instrumentation for debugging.

use crate::types::DebugInfo;
use wasm_encoder::reencode;

pub mod encoder;
pub mod function;

pub type Error = anyhow::Error;
pub type InstrError = reencode::Error<Error>;
pub type InstrResult<T = ()> = Result<T, InstrError>;

macro_rules! error {
    ($($arg:tt)*) => {
        Err($crate::debug::instrument::InstrError::UserError(
            $crate::debug::instrument::Error::msg(format!($($arg)*)),
        ))
    };
}

/// Instrument a WASM binary to support debugging
pub fn instrument_wasm(wasm_bytes: &[u8], debug_info: &mut DebugInfo) -> Result<Vec<u8>, String> {
    let mut instrumenter = encoder::Instrumenter::new(debug_info);
    let mut module = wasm_encoder::Module::new();
    reencode::utils::parse_core_module(
        &mut instrumenter,
        &mut module,
        wasmparser::Parser::new(0),
        wasm_bytes,
    )
    .map_err(|e| format!("Failed to reencode WASM: {:?}", e))?;
    Ok(module.finish())
}
