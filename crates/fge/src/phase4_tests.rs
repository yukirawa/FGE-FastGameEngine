// Phase 4 CLI, `fge test`, and live `--bridge` integration tests (Step 2 TDD).

use std::path::{Path, PathBuf};

use fge_bridge::snapshot::HeadlessTestReport;
use fge_render::error::RenderError;

use crate::{
    app::{run_project, run_test_cmd},
    cli::{parse_cli_from, Command, RunArgs, TestArgs},
    error::AppError,
};

fn demo_game_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo_game")
}

#[test]
fn cli_test_parses_explicit_flags_and_defaults() {
    let cli = parse_cli_from([
        "fge",
        "test",
        "./my_game",
        "--frames",
        "600",
        "--seed",
        "42",
        "--dump-json",
    ])
    .expect("fge test with explicit flags should parse");

    match cli.command {
        Command::Test(args) => {
            assert_eq!(args.project_dir, PathBuf::from("./my_game"));
            assert_eq!(args.frames, 600);
            assert_eq!(args.seed, 42);
            assert!((args.dt - (1.0 / 60.0)).abs() < 1e-6);
            assert!(args.dump_json);
        }
        other => panic!("expected Command::Test, got {other:?}"),
    }

    let defaults_cli = parse_cli_from(["fge", "test", "./my_game"])
        .expect("fge test with default flags should parse");

    assert_eq!(
        defaults_cli.command,
        Command::Test(TestArgs {
            project_dir: PathBuf::from("./my_game"),
            frames: 300,
            seed: 42,
            dt: 1.0 / 60.0,
            dump_json: false,
        })
    );
}

#[test]
fn cli_run_parses_bridge_flag() {
    let cli = parse_cli_from([
        "fge",
        "run",
        "./my_game",
        "--bridge",
        "127.0.0.1:7777",
    ])
    .expect("fge run with --bridge should parse");

    assert_eq!(
        cli.command,
        Command::Run(RunArgs {
            project_dir: PathBuf::from("./my_game"),
            frames: 0,
            offscreen: false,
            no_hot_reload: false,
            bridge: Some(String::from("127.0.0.1:7777")),
        })
    );
}

#[test]
fn run_test_cmd_executes_demo_game_deterministically_with_json_roundtrip() {
    let args = TestArgs {
        project_dir: demo_game_dir(),
        frames: 120,
        seed: 42,
        dt: 1.0 / 60.0,
        dump_json: true,
    };

    let report1 = run_test_cmd(&args).expect("first run_test_cmd on demo_game should succeed");
    let report2 = run_test_cmd(&args).expect("second run_test_cmd on demo_game should succeed");

    for report in [&report1, &report2] {
        assert_eq!(report.status, "ok");
        assert_eq!(report.frames_executed, 120);
        assert_eq!(report.frames_requested, 120);
        assert_eq!(report.seed, 42);
        assert!(report.compile_error.is_none());
    }

    let json = serde_json::to_string(&report1)
        .expect("HeadlessTestReport should serialize cleanly via serde_json::to_string");
    let roundtrip: HeadlessTestReport = serde_json::from_str(&json)
        .expect("HeadlessTestReport should deserialize cleanly from JSON");
    assert_eq!(roundtrip.status, report1.status);
    assert_eq!(roundtrip.project_title, report1.project_title);
    assert_eq!(roundtrip.seed, report1.seed);
    assert_eq!(roundtrip.frames_requested, report1.frames_requested);
    assert_eq!(roundtrip.frames_executed, report1.frames_executed);
    assert_eq!(roundtrip.dt, report1.dt);
    assert!((roundtrip.sim_timing.total_ms - report1.sim_timing.total_ms).abs() < 1e-9);
    assert!((roundtrip.sim_timing.avg_step_us - report1.sim_timing.avg_step_us).abs() < 1e-9);
    assert!((roundtrip.sim_timing.max_step_us - report1.sim_timing.max_step_us).abs() < 1e-9);
    assert_eq!(roundtrip.collision_events_total, report1.collision_events_total);
    assert_eq!(roundtrip.collision_samples, report1.collision_samples);
    assert_eq!(roundtrip.compile_error, report1.compile_error);
    assert_eq!(roundtrip.world_snapshot, report1.world_snapshot);
    assert_eq!(roundtrip.logs, report1.logs);

    let snap1 = report1
        .world_snapshot
        .as_ref()
        .expect("report1 should include a world_snapshot");
    let snap2 = report2
        .world_snapshot
        .as_ref()
        .expect("report2 should include a world_snapshot");

    assert_eq!(snap1.frame_count, 120);
    assert_eq!(snap2.frame_count, 120);
    assert!(
        snap1.entity_count > 0,
        "demo_game should have live entities after 120 frames"
    );
    assert_eq!(snap1.entity_count, snap2.entity_count);
    assert_eq!(snap1.entities.len(), snap2.entities.len());

    for (e1, e2) in snap1.entities.iter().zip(snap2.entities.iter()) {
        assert_eq!(e1.id, e2.id);
        assert_eq!(
            e1.pos, e2.pos,
            "entity {:?} position differed across deterministic seed=42 runs",
            e1.id
        );
    }
}

#[test]
fn run_project_with_bridge_offscreen_runs_demo_game() {
    let args = RunArgs {
        project_dir: demo_game_dir(),
        frames: 5,
        offscreen: true,
        no_hot_reload: false,
        bridge: Some(String::from("127.0.0.1:0")),
    };

    let summary = match run_project(&args) {
        Ok(Some(summary)) => summary,
        Ok(None) => panic!("expected run_project to return Some(BenchSummary) for offscreen run"),
        Err(AppError::Render(RenderError::RequestAdapter(err))) => {
            log::warn!(
                "Skipping Phase 4 offscreen bridge integration test (no GPU adapter): {err}"
            );
            return;
        }
        Err(err) => panic!("unexpected error running examples/demo_game with --bridge: {err}"),
    };

    assert_eq!(summary.total_frames, 5);
    assert_eq!(summary.draw_calls_per_frame, 1);
    assert!(summary.offscreen);
}
