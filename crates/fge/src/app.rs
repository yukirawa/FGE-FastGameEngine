use fge_core::stats::BenchSummary;

use crate::{
    cli::{BenchArgs, Cli, RunArgs},
    error::AppError,
};

/// Executes the parsed CLI command and returns an optional `BenchSummary`.
pub fn execute_cli(_cli: Cli) -> Result<Option<BenchSummary>, AppError> {
    todo!("execute_cli")
}

/// Runs the 1-draw-call `SpriteSoA` + `wgpu` rendering benchmark (either windowed via `winit` or offscreen).
pub fn run_bench(_args: &BenchArgs) -> Result<BenchSummary, AppError> {
    todo!("run_bench")
}

/// Loads `game.toml` and boots the engine shell for the target game directory.
pub fn run_project(_args: &RunArgs) -> Result<Option<BenchSummary>, AppError> {
    todo!("run_project")
}
