//! `fge-abi`: Zero-raw-pointer C-ABI engine context and exported `fge_*` host functions.

pub mod api;
pub mod context;
pub mod types;

#[cfg(test)]
mod tests;

/// Prevents the linker from dead-stripping `extern "C"` C-ABI symbols when embedding `fge-abi`.
pub fn ensure_abi_symbols_linked() -> usize {
    crate::api::ensure_abi_symbols_linked()
}
