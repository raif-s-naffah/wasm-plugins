// SPDX-License-Identifier: GPL-3.0-or-later

use std::num::ParseIntError;
use thiserror::Error;

/// Enumeration of error variants raised by this.
#[derive(Debug, Error)]
pub enum WasmPluginError {
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("WASM error: {0}")]
    Wasm(#[from] wasmtime::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] ParseIntError),
}
