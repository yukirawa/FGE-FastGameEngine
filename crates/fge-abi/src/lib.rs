//! `fge-abi`: Zero-raw-pointer C-ABI engine context and exported `fge_*` host functions.

pub mod api;
pub mod context;
pub mod types;

#[cfg(test)]
mod tests;
