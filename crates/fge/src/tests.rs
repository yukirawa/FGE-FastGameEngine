use std::path::{Path, PathBuf};

use glam::Vec2;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::KeyCode,
};

use crate::{
    cli::{parse_cli_from, BenchArgs, Command, RunArgs},
    config::{
        load_manifest_from_path, parse_manifest_str, BuildSection, GameManifest, GameSection,
    },
    error::AppError,
    input::InputState,
};

fn demo_game_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo_game")
}

// ---------------------------------------------------------------------------
// CLI parsing (`parse_cli_from`)
// ---------------------------------------------------------------------------

#[test]
fn cli_bench_defaults() {
    let cli = parse_cli_from(["fge", "bench"]).expect("default bench args should parse");
    assert_eq!(
        cli.command,
        Command::Bench(BenchArgs {
            count: 100_000,
            width: 1280,
            height: 720,
            frames: 0,
            offscreen: false,
            vsync: false,
            cull: false,
            seed: 42,
            json: false,
        })
    );
}

#[test]
fn cli_bench_custom_flags() {
    let cli = parse_cli_from([
        "fge",
        "bench",
        "-n",
        "50000",
        "--frames",
        "300",
        "--offscreen",
        "--vsync",
        "--cull",
        "--seed",
        "7",
        "--json",
    ])
    .expect("custom bench flags should parse");

    assert_eq!(
        cli.command,
        Command::Bench(BenchArgs {
            count: 50_000,
            width: 1280,
            height: 720,
            frames: 300,
            offscreen: true,
            vsync: true,
            cull: true,
            seed: 7,
            json: true,
        })
    );
}

#[test]
fn cli_bench_custom_dimensions_and_long_count() {
    let cli = parse_cli_from([
        "fge", "bench", "--count", "25000", "--width", "1920", "--height", "1080",
    ])
    .expect("custom dimensions should parse");

    match cli.command {
        Command::Bench(args) => {
            assert_eq!(args.count, 25_000);
            assert_eq!(args.width, 1920);
            assert_eq!(args.height, 1080);
        }
        other => panic!("expected Command::Bench, got {other:?}"),
    }
}

#[test]
fn cli_run_with_path_and_flags() {
    let cli = parse_cli_from(["fge", "run", "./my_game", "--frames", "60", "--offscreen"])
        .expect("run command with flags should parse");

    assert_eq!(
        cli.command,
        Command::Run(RunArgs {
            project_dir: PathBuf::from("./my_game"),
            frames: 60,
            offscreen: true,
        })
    );
}

#[test]
fn cli_run_defaults() {
    let cli = parse_cli_from(["fge", "run"]).expect("default run command should parse");
    assert_eq!(
        cli.command,
        Command::Run(RunArgs {
            project_dir: PathBuf::from("."),
            frames: 0,
            offscreen: false,
        })
    );
}

#[test]
fn cli_rejects_invalid_flags_and_missing_subcommand() {
    assert!(parse_cli_from(["fge", "bench", "--unknown-flag"]).is_err());
    assert!(parse_cli_from(["fge", "bench", "--count", "not_a_number"]).is_err());
    assert!(parse_cli_from(["fge"]).is_err());
    assert!(parse_cli_from(["fge", "invalid-subcommand"]).is_err());
}

// ---------------------------------------------------------------------------
// Manifest parsing & validation (`parse_manifest_str`, `load_manifest_from_path`)
// ---------------------------------------------------------------------------

#[test]
fn manifest_parse_full_toml() {
    let raw = r#"
[game]
title = "Space Shooter"
width = 1920
height = 1080
target_fps = 144
vsync = true

[build]
entry = "src/game.c"
sources = ["src/game.c", "src/physics.c"]
"#;

    let manifest = parse_manifest_str(raw).expect("full manifest should parse and validate");
    assert_eq!(
        manifest,
        GameManifest {
            game: GameSection {
                title: String::from("Space Shooter"),
                width: 1920,
                height: 1080,
                target_fps: 144,
                vsync: true,
            },
            build: BuildSection {
                entry: String::from("src/game.c"),
                sources: vec![String::from("src/game.c"), String::from("src/physics.c")],
            },
        }
    );
}

#[test]
fn manifest_parse_applies_optional_defaults() {
    let raw = r#"
[game]
title = "Minimal Game"
"#;

    let manifest = parse_manifest_str(raw).expect("minimal manifest should use defaults");
    assert_eq!(manifest.game.title, "Minimal Game");
    assert_eq!(manifest.game.width, 1280);
    assert_eq!(manifest.game.height, 720);
    assert_eq!(manifest.game.target_fps, 60);
    assert!(!manifest.game.vsync);
    assert_eq!(manifest.build.entry, "src/main.c");
    assert!(manifest.build.sources.is_empty());
}

#[test]
fn manifest_load_from_demo_game_dir_and_file() {
    let dir = demo_game_dir();
    let file = dir.join("game.toml");

    let from_dir = load_manifest_from_path(&dir).expect("loading directory should read game.toml");
    let from_file = load_manifest_from_path(&file).expect("loading direct game.toml should work");

    assert_eq!(from_dir, from_file);
    assert_eq!(from_dir.game.title, "FGE Phase 1 Demo");
    assert_eq!(from_dir.game.width, 1280);
    assert_eq!(from_dir.game.height, 720);
    assert_eq!(from_dir.game.target_fps, 60);
    assert!(!from_dir.game.vsync);
    assert_eq!(from_dir.build.entry, "src/main.c");
    assert_eq!(from_dir.build.sources, vec![String::from("src/main.c")]);
}

#[test]
fn manifest_load_nonexistent_path_returns_io_error() {
    let missing = demo_game_dir().join("does_not_exist_game.toml");
    let err = load_manifest_from_path(&missing).expect_err("missing path should fail");
    assert!(matches!(err, AppError::Io(_)), "expected AppError::Io, got {err:?}");
}

#[test]
fn manifest_parse_invalid_toml_returns_parse_error() {
    let err = parse_manifest_str("not valid = [toml").expect_err("invalid toml should fail");
    assert!(
        matches!(err, AppError::ManifestParse(_)),
        "expected AppError::ManifestParse, got {err:?}"
    );
}

#[test]
fn manifest_validate_rejects_invalid_fields() {
    let base = GameManifest {
        game: GameSection {
            title: String::from("Valid Title"),
            width: 1280,
            height: 720,
            target_fps: 60,
            vsync: false,
        },
        build: BuildSection {
            entry: String::from("src/main.c"),
            sources: vec![],
        },
    };
    assert!(base.validate().is_ok());

    let empty_title = GameManifest {
        game: GameSection {
            title: String::from("   "),
            ..base.game.clone()
        },
        ..base.clone()
    };
    assert!(matches!(
        empty_title.validate(),
        Err(AppError::InvalidManifest { .. })
    ));

    let zero_width = GameManifest {
        game: GameSection {
            width: 0,
            ..base.game.clone()
        },
        ..base.clone()
    };
    assert!(matches!(
        zero_width.validate(),
        Err(AppError::InvalidManifest { .. })
    ));

    let zero_height = GameManifest {
        game: GameSection {
            height: 0,
            ..base.game.clone()
        },
        ..base.clone()
    };
    assert!(matches!(
        zero_height.validate(),
        Err(AppError::InvalidManifest { .. })
    ));

    let zero_fps = GameManifest {
        game: GameSection {
            target_fps: 0,
            ..base.game.clone()
        },
        ..base.clone()
    };
    assert!(matches!(
        zero_fps.validate(),
        Err(AppError::InvalidManifest { .. })
    ));

    let empty_entry = GameManifest {
        build: BuildSection {
            entry: String::from(""),
            sources: vec![],
        },
        ..base
    };
    assert!(matches!(
        empty_entry.validate(),
        Err(AppError::InvalidManifest { .. })
    ));

    let parsed_bad = parse_manifest_str(
        r#"
[game]
title = ""
"#,
    );
    assert!(matches!(
        parsed_bad,
        Err(AppError::InvalidManifest { .. })
    ));
}

// ---------------------------------------------------------------------------
// Input abstraction (`InputState`)
// ---------------------------------------------------------------------------

#[test]
fn input_state_initial_clean() {
    let mut input = InputState::new();
    assert_eq!(input.cursor_position(), Vec2::ZERO);
    assert_eq!(input.scroll_delta(), Vec2::ZERO);
    assert!(!input.close_requested());
    assert_eq!(input.take_resized(), None);
    assert!(!input.is_mouse_down(MouseButton::Left));
    assert!(!input.is_mouse_down(MouseButton::Right));
    assert!(!input.is_key_down(KeyCode::Space));
    assert!(!input.is_key_just_pressed(KeyCode::Space));
    assert!(!input.is_key_just_released(KeyCode::Space));
}

#[test]
fn input_state_close_requested() {
    let mut input = InputState::new();
    assert!(!input.close_requested());

    input.process_window_event(&WindowEvent::CloseRequested);
    assert!(input.close_requested());

    // `begin_frame` should not clear `close_requested` once the user requested closing.
    input.begin_frame();
    assert!(input.close_requested());
}

#[test]
fn input_state_resized_and_take_resized() {
    let mut input = InputState::new();
    input.process_window_event(&WindowEvent::Resized(PhysicalSize::new(1920, 1080)));

    assert_eq!(input.take_resized(), Some((1920, 1080)));
    assert_eq!(input.take_resized(), None);

    // Unconsumed resize is cleared on `begin_frame`.
    input.process_window_event(&WindowEvent::Resized(PhysicalSize::new(800, 600)));
    input.begin_frame();
    assert_eq!(input.take_resized(), None);
}

#[test]
fn input_state_cursor_moved() {
    let mut input = InputState::new();
    let device_id = DeviceId::dummy();

    input.process_window_event(&WindowEvent::CursorMoved {
        device_id,
        position: PhysicalPosition::new(320.5, 240.25),
    });
    assert_eq!(input.cursor_position(), Vec2::new(320.5, 240.25));

    // Cursor position persists across frames until updated.
    input.begin_frame();
    assert_eq!(input.cursor_position(), Vec2::new(320.5, 240.25));
}

#[test]
fn input_state_mouse_wheel_line_and_pixel_delta_reset_on_begin_frame() {
    let mut input = InputState::new();
    let device_id = DeviceId::dummy();

    input.process_window_event(&WindowEvent::MouseWheel {
        device_id,
        delta: MouseScrollDelta::LineDelta(1.5, -2.0),
        phase: TouchPhase::Moved,
    });
    assert_eq!(input.scroll_delta(), Vec2::new(1.5, -2.0));

    input.begin_frame();
    assert_eq!(input.scroll_delta(), Vec2::ZERO);

    input.process_window_event(&WindowEvent::MouseWheel {
        device_id,
        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(12.0, -24.0)),
        phase: TouchPhase::Moved,
    });
    assert!(input.scroll_delta().x > 0.0);
    assert!(input.scroll_delta().y < 0.0);

    input.begin_frame();
    assert_eq!(input.scroll_delta(), Vec2::ZERO);
}

#[test]
fn input_state_mouse_input_press_and_release() {
    let mut input = InputState::new();
    let device_id = DeviceId::dummy();

    input.process_window_event(&WindowEvent::MouseInput {
        device_id,
        state: ElementState::Pressed,
        button: MouseButton::Left,
    });
    assert!(input.is_mouse_down(MouseButton::Left));
    assert!(!input.is_mouse_down(MouseButton::Right));

    input.process_window_event(&WindowEvent::MouseInput {
        device_id,
        state: ElementState::Pressed,
        button: MouseButton::Right,
    });
    assert!(input.is_mouse_down(MouseButton::Left));
    assert!(input.is_mouse_down(MouseButton::Right));

    // Held mouse buttons persist across `begin_frame`.
    input.begin_frame();
    assert!(input.is_mouse_down(MouseButton::Left));
    assert!(input.is_mouse_down(MouseButton::Right));

    input.process_window_event(&WindowEvent::MouseInput {
        device_id,
        state: ElementState::Released,
        button: MouseButton::Left,
    });
    assert!(!input.is_mouse_down(MouseButton::Left));
    assert!(input.is_mouse_down(MouseButton::Right));
}
