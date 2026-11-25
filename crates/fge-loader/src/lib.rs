//! `fge-loader`: Multi-file C11 compiler driver, `libloading` `.so` loader, and live hot-reloader.

pub mod compiler;
pub mod error;
pub mod hot_reload;
pub mod module;

#[cfg(test)]
mod tests;
