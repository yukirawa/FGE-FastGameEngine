// Phase 6 unit & integration tests for `fge new` project scaffolding.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command as StdCommand,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    app::{run_new, run_test_cmd},
    cli::{parse_cli_from, Command, NewArgs, TestArgs},
    config::load_manifest_from_path,
    error::AppError,
    scaffold::{scaffold_project, validate_project_name},
};

static NEXT_TMP_ID: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let id = NEXT_TMP_ID.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "fge_phase6_{}_{}_{}_{}",
            prefix,
            std::process::id(),
            id,
            nanos
        ));
        fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn workspace_include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../include")
}

#[test]
fn cli_parses_new_command_with_default_and_explicit_dir() {
    let cli_default =
        parse_cli_from(["fge", "new", "my_game"]).expect("fge new <name> should parse");
    assert_eq!(
        cli_default.command,
        Command::New(NewArgs {
            name: String::from("my_game"),
            dir: PathBuf::from("."),
        })
    );

    let cli_custom = parse_cli_from(["fge", "new", "space-game_2", "--dir", "/tmp/projects"])
        .expect("fge new <name> --dir <parent> should parse");
    assert_eq!(
        cli_custom.command,
        Command::New(NewArgs {
            name: String::from("space-game_2"),
            dir: PathBuf::from("/tmp/projects"),
        })
    );
}

#[test]
fn project_name_validation_accepts_valid_and_rejects_invalid() {
    for valid in ["game", "my_game", "my-game-01", "A9_z-0"] {
        validate_project_name(valid)
            .unwrap_or_else(|e| panic!("expected valid name `{valid}` to succeed: {e}"));
    }

    for invalid in [
        "",
        " ",
        "my game",
        "bad/name",
        "bad\\name",
        "..",
        ".",
        "game.toml",
        "hello!",
        "日本語",
    ] {
        let err = validate_project_name(invalid)
            .expect_err(&format!("expected invalid name `{invalid}` to be rejected"));
        assert!(
            matches!(err, AppError::Io(ref io_err) if io_err.kind() == ErrorKind::InvalidInput),
            "expected InvalidInput for `{invalid}`, got {err:?}"
        );
    }
}

#[test]
fn scaffold_refuses_existing_non_empty_dir_and_allows_empty_dir() {
    let tmp = TempDir::new("non_empty");
    let non_empty_target = tmp.path().join("existing_game");
    fs::create_dir_all(&non_empty_target).expect("create existing target dir");
    fs::write(non_empty_target.join("keep.txt"), "do not overwrite").expect("write marker file");

    let err = scaffold_project(tmp.path(), "existing_game")
        .expect_err("scaffolding into non-empty directory must fail");
    assert!(
        matches!(err, AppError::Io(ref io_err) if io_err.kind() == ErrorKind::AlreadyExists),
        "expected AlreadyExists error, got {err:?}"
    );
    assert!(non_empty_target.join("keep.txt").is_file());
    assert!(!non_empty_target.join("game.toml").exists());

    // An existing regular file at the target path must also be refused.
    let file_target = tmp.path().join("file_blocker");
    fs::write(&file_target, "not a directory").expect("write file blocker");
    let file_err = scaffold_project(tmp.path(), "file_blocker")
        .expect_err("scaffolding onto an existing file must fail");
    assert!(
        matches!(file_err, AppError::Io(ref io_err) if io_err.kind() == ErrorKind::AlreadyExists),
        "expected AlreadyExists error for file blocker, got {file_err:?}"
    );

    // An existing *empty* directory is permitted.
    let empty_target = tmp.path().join("empty_game");
    fs::create_dir_all(&empty_target).expect("create empty target dir");
    let created =
        scaffold_project(tmp.path(), "empty_game").expect("empty existing dir should succeed");
    assert_eq!(created, empty_target);
    assert!(created.join("game.toml").is_file());
}

#[test]
fn scaffold_generates_valid_manifest_strict_c11_and_passes_headless_test() {
    let tmp = TempDir::new("e2e");
    let new_args = NewArgs {
        name: String::from("starter_quest"),
        dir: tmp.path().to_path_buf(),
    };
    run_new(&new_args).expect("run_new should scaffold project");

    let project_dir = tmp.path().join("starter_quest");
    let manifest_path = project_dir.join("game.toml");
    let main_c_path = project_dir.join("src/main.c");
    let gitignore_path = project_dir.join(".gitignore");

    assert!(manifest_path.is_file(), "game.toml should exist");
    assert!(main_c_path.is_file(), "src/main.c should exist");
    assert!(gitignore_path.is_file(), ".gitignore should exist");

    // 1. Parse generated game.toml via existing config loader.
    let manifest =
        load_manifest_from_path(&project_dir).expect("generated game.toml should load and validate");
    assert_eq!(manifest.game.title, "starter_quest");
    assert_eq!(manifest.game.width, 1280);
    assert_eq!(manifest.game.height, 720);
    assert_eq!(manifest.game.target_fps, 60);
    assert_eq!(manifest.build.entry, "src/main.c");
    assert!(manifest.build.sources.is_empty());

    // 2. Verify generated src/main.c contains required symbols and passes strict gcc C11 syntax check.
    let main_c_src = fs::read_to_string(&main_c_path).expect("read generated src/main.c");
    assert!(main_c_src.contains("fge_on_reload"));
    assert!(main_c_src.contains("fge_ui_text"));
    assert!(main_c_src.contains("fge_str(\"Hello FGE\")"));

    let include_dir = workspace_include_dir();
    let gcc_out = StdCommand::new("gcc")
        .arg("-fsyntax-only")
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-pedantic")
        .arg("-I")
        .arg(&include_dir)
        .arg(&main_c_path)
        .output()
        .expect("invoke gcc -fsyntax-only on generated src/main.c");
    assert!(
        gcc_out.status.success(),
        "gcc strict C11 check failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&gcc_out.stdout),
        String::from_utf8_lossy(&gcc_out.stderr)
    );

    // 3. Run headless `fge test` equivalent for 30 frames and assert status == "ok".
    let test_args = TestArgs {
        project_dir: project_dir.clone(),
        frames: 30,
        seed: 42,
        dt: 1.0 / 60.0,
        dump_json: false,
    };
    let report = run_test_cmd(&test_args).expect("run_test_cmd on scaffolded project should work");
    assert_eq!(
        report.status, "ok",
        "expected headless test status ok, got {:?}",
        report
    );
    assert_eq!(report.frames_executed, 30);
    let snapshot = report
        .world_snapshot
        .as_ref()
        .expect("world_snapshot should be present");
    assert!(
        snapshot.entity_count >= 1,
        "expected spawned player sprite entity in world_snapshot, got {}",
        snapshot.entity_count
    );
}
