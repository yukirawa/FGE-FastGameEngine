//! `fge`: CLI interface, `game.toml` loader, `winit` input abstraction, and benchmark runner.

pub use fge_abi;

pub mod app;
pub mod cli;
pub mod config;
pub mod error;
pub mod input;
pub mod keymap;
pub mod scaffold;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod phase2_tests;

#[cfg(test)]
mod phase3_tests;

#[cfg(test)]
mod phase4_tests;

#[cfg(test)]
mod phase5_tests;

#[cfg(test)]
mod phase6_tests;
