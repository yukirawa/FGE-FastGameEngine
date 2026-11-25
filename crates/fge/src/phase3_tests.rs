// Phase 3 CLI and `fge run` integration tests.

use std::path::{Path, PathBuf};

use fge_render::error::RenderError;

use crate::{
    app::run_project,
    cli::{parse_cli_from, Command, RunArgs},
    error::AppError,
};

fn demo_game_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo_game")
}

#[test]
fn cli_run_parses_demo_game_with_frames_offscreen_and_no_hot_reload() {
    let cli = parse_cli_from([
        "fge",
        "run",
        "examples/demo_game",
        "--frames",
        "20",
        "--offscreen",
        "--no-hot-reload",
    ])
    .expect("fge run with --frames, --offscreen, and --no-hot-reload should parse");

    assert_eq!(
        cli.command,
        Command::Run(RunArgs {
            project_dir: PathBuf::from("examples/demo_game"),
            frames: 20,
            offscreen: true,
            no_hot_reload: true,
        })
    );
}

#[test]
fn run_project_executes_demo_game_offscreen_for_20_frames() {
    let args = RunArgs {
        project_dir: demo_game_dir(),
        frames: 20,
        offscreen: true,
        no_hot_reload: false,
    };

    let summary = match run_project(&args) {
        Ok(Some(summary)) => summary,
        Ok(None) => panic!("expected run_project to return Some(BenchSummary) for offscreen run"),
        Err(AppError::Render(RenderError::RequestAdapter(err))) => {
            log::warn!("Skipping Phase 3 demo_game integration test (no GPU adapter): {err}");
            return;
        }
        Err(err) => panic!("unexpected error running examples/demo_game offscreen: {err}"),
    };

    assert_eq!(summary.total_frames, 20);
    assert!(
        summary.rendered_instances > 0,
        "expected demo_game to render at least 1 sprite instance, got {}",
        summary.rendered_instances
    );
    assert_eq!(summary.draw_calls_per_frame, 1);
    assert!(summary.offscreen);
}
