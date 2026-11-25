use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use fge_abi::context::EngineContext;

use crate::compiler::{
    compile_shared_library, stage_embedded_header, CBuildPlan, CompilerKind, EMBEDDED_FGE_H,
};
use crate::error::LoaderError;
use crate::hot_reload::HotReloader;
use crate::module::LoadedGameModule;

static NEXT_TEST_DIR_ID: AtomicU64 = AtomicU64::new(1);

/// RAII temporary directory under `std::env::temp_dir()` removed when dropped.
struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let id = NEXT_TEST_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir_name = format!("fge_loader_test_{}_{}_{}_{}", prefix, std::process::id(), id, nanos);
        let path = std::env::temp_dir().join(dir_name);
        fs::create_dir_all(&path).expect("create unique test temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_text(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, content).expect("write test source file");
}

#[test]
fn detect_compiler_and_stage_embedded_header() {
    let (kind, compiler_path) =
        CompilerKind::detect().expect("a C11 compiler (tcc/clang/gcc) should be available");
    assert!(
        !kind.executable_name().is_empty(),
        "compiler executable name should be non-empty"
    );
    assert!(
        !compiler_path.as_os_str().is_empty(),
        "detected compiler path should be non-empty"
    );

    let tmp = TempTestDir::new("stage_header");
    let out_dir = tmp.path().join("out");

    let include_dir = stage_embedded_header(&out_dir).expect("stage embedded fge.h");
    assert_eq!(include_dir, out_dir.join("include"));

    let header_path = include_dir.join("fge.h");
    assert!(header_path.is_file(), "staged fge.h should exist on disk");
    let staged_contents = fs::read_to_string(&header_path).expect("read staged fge.h");
    assert_eq!(staged_contents, EMBEDDED_FGE_H);

    // Calling again should remain idempotent and succeed.
    let include_dir_again = stage_embedded_header(&out_dir).expect("re-stage embedded fge.h");
    assert_eq!(include_dir_again, include_dir);
}

#[test]
fn multifile_compile_load_step_collision_and_shutdown() {
    let tmp = TempTestDir::new("multifile");
    let project_dir = tmp.path().join("project");
    let out_dir = tmp.path().join("out");
    fs::create_dir_all(project_dir.join("src")).expect("create src dir");
    fs::create_dir_all(&out_dir).expect("create out dir");

    write_text(
        &project_dir.join("src/helper.h"),
        r#"#ifndef HELPER_H
#define HELPER_H
#include "fge.h"
FgeEntity spawn_actor(float x, float y, float vx, float vy, float hp, uint32_t layer);
uint32_t check_pair_collisions(uint32_t mask_a, uint32_t mask_b);
#endif
"#,
    );

    write_text(
        &project_dir.join("src/helper.c"),
        r#"#include "helper.h"

FgeEntity spawn_actor(float x, float y, float vx, float vy, float hp, uint32_t layer) {
    FgeEntity e = fge_spawn_sprite(x, y, 20.0f, 20.0f, fge_rgba(255, 120, 40, 255));
    fge_set_velocity(e, vx, vy);
    fge_set_hp(e, hp);
    fge_set_collider(e, 12.0f, layer);
    return e;
}

uint32_t check_pair_collisions(uint32_t mask_a, uint32_t mask_b) {
    return fge_detect_collisions(mask_a, mask_b);
}
"#,
    );

    write_text(
        &project_dir.join("src/main.c"),
        r#"#include "fge.h"
#include "helper.h"

static FgeEntity g_left;
static FgeEntity g_right;
static uint32_t g_detected = 0;

void fge_init(void) {
    g_left = spawn_actor(100.0f, 100.0f, 120.0f, 0.0f, 50.0f, 1u);
    g_right = spawn_actor(126.0f, 100.0f, -120.0f, 0.0f, 75.0f, 2u);
}

void fge_update(float dt) {
    (void)dt;
    g_detected = check_pair_collisions(1u, 2u);
}

void fge_shutdown(void) {
    fge_set_tag(g_left, 99u);
    fge_set_tag(g_right, 100u + g_detected);
}
"#,
    );

    let plan = CBuildPlan {
        project_dir: project_dir.clone(),
        entry: "src/main.c".to_string(),
        sources: vec!["src/helper.c".to_string()],
        out_dir,
        optimize: false,
    };

    let mut ctx = EngineContext::new(800.0, 600.0, 42).expect("create EngineContext");
    let reloader =
        HotReloader::initial_build_and_load(plan, &mut ctx).expect("initial build and load");
    assert_eq!(reloader.generation(), 1);
    assert_eq!(ctx.world.len(), 2);
    assert!((ctx.world.hp[0] - 50.0).abs() < 1e-4);
    assert!((ctx.world.hp[1] - 75.0).abs() < 1e-4);

    let dt = 1.0 / 60.0;
    for _ in 0..5 {
        reloader.step_frame(&mut ctx, dt).expect("step frame");
    }

    assert_eq!(ctx.frame_count, 5);
    assert_eq!(ctx.world.len(), 2);
    assert!(
        ctx.world.pos_x[0] > 100.0,
        "left entity should have moved right, got {}",
        ctx.world.pos_x[0]
    );
    assert!(
        ctx.world.pos_x[1] < 126.0,
        "right entity should have moved left, got {}",
        ctx.world.pos_x[1]
    );
    assert!(
        !ctx.last_collisions.is_empty(),
        "expected collision pair between layer 1 and layer 2 entities"
    );

    reloader.shutdown(&mut ctx);
    assert_eq!(ctx.world.tag[0], 99);
    assert_eq!(ctx.world.tag[1], 101);
}

#[test]
fn missing_fge_update_returns_missing_symbol() {
    let tmp = TempTestDir::new("missing_symbol");
    let project_dir = tmp.path().join("project");
    let out_dir = tmp.path().join("out");
    fs::create_dir_all(project_dir.join("src")).expect("create src dir");
    fs::create_dir_all(&out_dir).expect("create out dir");

    write_text(
        &project_dir.join("src/main.c"),
        r#"#include "fge.h"
void fge_init(void) {}
"#,
    );

    let plan = CBuildPlan {
        project_dir,
        entry: "src/main.c".to_string(),
        sources: Vec::new(),
        out_dir,
        optimize: false,
    };

    let so_path = compile_shared_library(&plan, 1).expect("compile module without fge_update");
    let res = LoadedGameModule::load(so_path, 1, true);
    match res {
        Err(LoaderError::MissingSymbol { symbol }) => {
            assert_eq!(symbol, "fge_update");
        }
        Err(other) => panic!("expected LoaderError::MissingSymbol, got {other:?}"),
        Ok(_) => panic!("expected MissingSymbol error when fge_update is absent"),
    }
    let remaining_so = fs::read_dir(&plan.out_dir)
        .expect("read out_dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("so"))
        .count();
    assert_eq!(
        remaining_so, 0,
        "failed load with cleanup_on_drop=true must unlink temp .so"
    );
}

#[test]
fn hot_reload_resilience_and_state_preservation() {
    let tmp = TempTestDir::new("hot_reload");
    let project_dir = tmp.path().join("project");
    let out_dir = tmp.path().join("out");
    fs::create_dir_all(project_dir.join("src")).expect("create src dir");
    fs::create_dir_all(&out_dir).expect("create out dir");

    let main_c = project_dir.join("src/main.c");
    let v1_code = r#"#include "fge.h"

static FgeEntity g_player;

void fge_init(void) {
    g_player = fge_spawn_sprite(10.0f, 20.0f, 16.0f, 16.0f, fge_rgba(255, 255, 255, 255));
    fge_set_velocity(g_player, 10.0f, 0.0f);
    fge_set_collider(g_player, 8.0f, 1u);
    fge_set_tag(g_player, 1u);
}

void fge_update(float dt) {
    (void)dt;
}
"#;
    write_text(&main_c, v1_code);

    let plan = CBuildPlan {
        project_dir: project_dir.clone(),
        entry: "src/main.c".to_string(),
        sources: Vec::new(),
        out_dir,
        optimize: false,
    };

    let mut ctx = EngineContext::new(800.0, 600.0, 7).expect("create EngineContext");
    let mut reloader =
        HotReloader::initial_build_and_load(plan, &mut ctx).expect("initial build v1");
    assert_eq!(reloader.generation(), 1);
    assert!(reloader.last_compile_error().is_none());
    assert_eq!(ctx.world.len(), 1);

    // Step 1 frame under v1 (vx = +10.0, dt = 0.5 => x = 15.0).
    reloader.step_frame(&mut ctx, 0.5).expect("step v1 frame 1");
    assert!((ctx.world.pos_x[0] - 15.0).abs() < 1e-4);
    assert_eq!(ctx.world.tag[0], 1);

    // Ensure distinct mtime & byte length for broken source overwrite.
    std::thread::sleep(Duration::from_millis(20));
    let broken_code = "#include \"fge.h\"\nSYNTAX_ERROR_BROKEN_C @@@;\n";
    assert_ne!(broken_code.len(), v1_code.len());
    write_text(&main_c, broken_code);

    let broken_res = reloader.poll_and_reload(&mut ctx);
    assert!(
        matches!(broken_res, Err(LoaderError::CompileFailed { .. })),
        "expected CompileFailed on broken C source, got {broken_res:?}"
    );
    assert!(
        reloader.last_compile_error().is_some(),
        "last_compile_error should be populated after compile failure"
    );
    assert_eq!(reloader.generation(), 1);

    // v1 module must remain active and advance the existing entity.
    reloader
        .step_frame(&mut ctx, 0.5)
        .expect("v1 module should still step after failed reload");
    assert!((ctx.world.pos_x[0] - 20.0).abs() < 1e-4);

    // Overwrite with valid v2 changing velocity and tag via fge_on_reload.
    std::thread::sleep(Duration::from_millis(20));
    let v2_code = r#"#include "fge.h"

void fge_init(void) {}

void fge_on_reload(void) {
    uint32_t count = fge_query_circle(20.0f, 20.0f, 64.0f, 1u);
    for (uint32_t i = 0; i < count; ++i) {
        FgeEntity e = fge_get_query_entity(i);
        fge_set_velocity(e, -20.0f, 5.0f);
        fge_set_tag(e, 42u);
    }
}

void fge_update(float dt) {
    (void)dt;
}
"#;
    assert_ne!(v2_code.len(), broken_code.len());
    assert_ne!(v2_code.len(), v1_code.len());
    write_text(&main_c, v2_code);

    let reloaded = reloader
        .poll_and_reload(&mut ctx)
        .expect("v2 reload should succeed");
    assert!(reloaded, "poll_and_reload should report Ok(true) on v2");
    assert_eq!(reloader.generation(), 2);
    assert!(reloader.last_compile_error().is_none());

    // Existing entity survived across reload and received updated velocity & tag.
    assert_eq!(ctx.world.len(), 1);
    assert_eq!(ctx.world.tag[0], 42);
    assert!((ctx.world.vel_x[0] + 20.0).abs() < 1e-4);
    assert!((ctx.world.vel_y[0] - 5.0).abs() < 1e-4);

    // Step 1 frame under v2 (dt = 0.5 => x moves 20 -> 10, y moves 20 -> 22.5).
    reloader.step_frame(&mut ctx, 0.5).expect("step v2 frame");
    assert!((ctx.world.pos_x[0] - 10.0).abs() < 1e-4);
    assert!((ctx.world.pos_y[0] - 22.5).abs() < 1e-4);

    reloader.shutdown(&mut ctx);
}
