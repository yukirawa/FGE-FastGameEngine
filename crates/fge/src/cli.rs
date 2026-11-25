use std::{ffi::OsString, path::PathBuf};

use clap::{Args, Parser, Subcommand};

/// Fast Game Engine (`fge`) command-line interface.
#[derive(Debug, Clone, PartialEq, Parser)]
#[command(name = "fge", version, about = "Fast Game Engine (FGE) - Ultra-fast DOD 2D game engine")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Subcommand)]
pub enum Command {
    /// Run the single-draw-call 2D sprite/quad rendering & SoA physics benchmark.
    Bench(BenchArgs),
    /// Load a `game.toml` project directory and launch the engine window.
    Run(RunArgs),
    /// Run a `game.toml` project headlessly without GPU/windowing and emit diagnostics.
    Test(TestArgs),
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct BenchArgs {
    /// Number of sprites/rectangles to simulate and render in 1 draw call.
    #[arg(short = 'n', long, default_value_t = 100_000)]
    pub count: usize,

    /// Render target / window width in pixels.
    #[arg(long, default_value_t = 1280)]
    pub width: u32,

    /// Render target / window height in pixels.
    #[arg(long, default_value_t = 720)]
    pub height: u32,

    /// Number of frames to run (`0` = run interactively until window closed).
    #[arg(long, default_value_t = 0)]
    pub frames: u64,

    /// Run against an offscreen GPU render texture without creating an OS window.
    #[arg(long, default_value_t = false)]
    pub offscreen: bool,

    /// Lock presentation to display VSync (`Fifo`) instead of `AutoNoVsync`.
    #[arg(long, default_value_t = false)]
    pub vsync: bool,

    /// Enable `Camera2D` frustum AABB culling when packing instances.
    #[arg(long, default_value_t = false)]
    pub cull: bool,

    /// Deterministic PRNG seed for spawning initial positions, velocities, sizes, and colors.
    #[arg(long, default_value_t = 42)]
    pub seed: u64,

    /// Emit the final `BenchSummary` as structured JSON to stdout.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct RunArgs {
    /// Path to the game directory containing `game.toml` (or direct path to `game.toml`).
    #[arg(default_value = ".")]
    pub project_dir: PathBuf,

    /// Optional frame limit (`0` = unlimited).
    #[arg(long, default_value_t = 0)]
    pub frames: u64,

    /// Run offscreen without opening an OS window.
    #[arg(long, default_value_t = false)]
    pub offscreen: bool,

    /// Disable C source polling and live hot-reload while running.
    #[arg(long, default_value_t = false)]
    pub no_hot_reload: bool,

    /// Optional TCP address to listen on for live AI Bridge JSON-RPC connections (e.g. `127.0.0.1:7777`).
    #[arg(long)]
    pub bridge: Option<String>,

    /// Optional API key required to authenticate AI Bridge JSON-RPC clients (falls back to `FGE_BRIDGE_API_KEY`).
    #[arg(long)]
    pub bridge_api_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct TestArgs {
    /// Path to the game directory containing `game.toml` (or direct path to `game.toml`).
    #[arg(default_value = ".")]
    pub project_dir: PathBuf,

    /// Number of simulation frames to execute.
    #[arg(long, default_value_t = 300)]
    pub frames: u64,

    /// Deterministic PRNG seed for the simulation.
    #[arg(long, default_value_t = 42)]
    pub seed: u64,

    /// Fixed timestep in seconds per frame.
    #[arg(long, default_value_t = 1.0 / 60.0)]
    pub dt: f32,

    /// Print the full `HeadlessTestReport` as pretty JSON to stdout.
    #[arg(long, default_value_t = false)]
    pub dump_json: bool,
}

/// Parses CLI arguments from an arbitrary iterator (test-friendly wrapper).
pub fn parse_cli_from<I, T>(itr: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Cli::try_parse_from(itr)
}
