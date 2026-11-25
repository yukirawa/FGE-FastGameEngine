//! `fge`: CLI interface, `game.toml` loader, `winit` input abstraction, and benchmark runner.

pub mod app;
pub mod cli;
pub mod config;
pub mod error;
pub mod input;

#[cfg(test)]
mod tests;
