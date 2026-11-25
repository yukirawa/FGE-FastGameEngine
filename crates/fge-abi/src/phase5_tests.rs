// Step 5 unit tests for `fge-abi` and `include/fge.h` (zero-pointer C-ABI,
// `fge_str` UTF-8 handling in C11, audio, emitters, TTL, and UI/text exports).

use std::path::{Path, PathBuf};
use std::process::Command;

use fge_core::handle::EntityId;
use fge_core::ui::{FgeTextBuf, UiDrawCmd};

use crate::api::*;
use crate::context::{with_active_context, EngineContext};
use crate::types::FgeColor;

struct TempFileGuard(Vec<PathBuf>);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn new_test_ctx() -> EngineContext {
    EngineContext::new(1280.0, 720.0, 0x5150_5150).expect("EngineContext::new should succeed")
}

#[test]
fn fge_h_and_fge_str_japanese_compile_and_run_with_strict_gcc_c11() {
    let include_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../include");
    let header_path = include_dir.join("fge.h");
    assert!(
        header_path.exists(),
        "expected header at {}",
        header_path.display()
    );

    let syntax_out = Command::new("gcc")
        .args([
            "-fsyntax-only",
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            header_path.to_str().expect("valid UTF-8 header path"),
        ])
        .output()
        .expect("failed to invoke gcc for C11 header syntax check");
    assert!(
        syntax_out.status.success(),
        "include/fge.h strict C11 syntax check failed:\n{}",
        String::from_utf8_lossy(&syntax_out.stderr)
    );

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_dir = std::env::temp_dir();
    let c_path = tmp_dir.join(format!("fge_p5_fge_str_{}_{stamp}.c", std::process::id()));
    let bin_path = tmp_dir.join(format!("fge_p5_fge_str_{}_{stamp}.out", std::process::id()));
    let _cleanup = TempFileGuard(vec![c_path.clone(), bin_path.clone()]);

    let c_source = r#"
#include "fge.h"
#include <string.h>

int main(void) {
    const char *jp = "\xE5\x8B\x87\xE8\x80\x85\xEF\xBC\x9A\xE3\x81\x84\xE3\x81\x8F\xE3\x81\x9E\xEF\xBC\x81"; /* 勇者：いくぞ！ */
    FgeTextBuf buf = fge_str(jp);
    size_t expected_len = strlen(jp);
    if ((size_t)buf.len != expected_len) {
        return 1;
    }
    if (memcmp(buf.bytes, jp, expected_len) != 0) {
        return 2;
    }
    FgeTextBuf empty = fge_str(NULL);
    if (empty.len != 0u) {
        return 3;
    }
    /* Over-120 byte literal must clamp safely at 120 bytes */
    char long_str[160];
    memset(long_str, 'A', sizeof(long_str) - 1u);
    long_str[sizeof(long_str) - 1u] = '\0';
    FgeTextBuf clamped = fge_str(long_str);
    if (clamped.len != 120u) {
        return 4;
    }
    return 0;
}
"#;
    std::fs::write(&c_path, c_source).expect("failed to write temporary C11 snippet");

    let compile_out = Command::new("gcc")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-I",
            include_dir.to_str().expect("valid UTF-8 include dir"),
            c_path.to_str().expect("valid UTF-8 C source path"),
            "-o",
            bin_path.to_str().expect("valid UTF-8 binary path"),
        ])
        .output()
        .expect("failed to invoke gcc to compile C11 snippet");
    assert!(
        compile_out.status.success(),
        "C11 fge_str snippet failed to compile:\n{}",
        String::from_utf8_lossy(&compile_out.stderr)
    );

    let run_out = Command::new(&bin_path)
        .output()
        .expect("failed to execute compiled C11 fge_str binary");
    assert!(
        run_out.status.success(),
        "C11 fge_str binary exited with {:?}",
        run_out.status.code()
    );
}

#[test]
fn step5_abi_calls_outside_active_context_are_safe() {
    let invalid = EntityId::INVALID;
    let zero = EntityId {
        index: 0,
        generation: 0,
    };
    let white = FgeColor {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    let text = FgeTextBuf::from_str_lossy("勇者：いくぞ！");

    fge_audio_set_master_volume(0.75);
    fge_audio_play_sfx(0, 1.0, 1.0);
    fge_audio_play_tone(0, 440.0, 880.0, 0.1, 0.5);
    assert!(fge_sound_load_wav(FgeTextBuf::from_str_lossy("assets/missing.wav")).is_null());
    assert!(!fge_sound_play(zero, 1.0));
    assert!(!fge_sound_play(invalid, 1.0));
    fge_bgm_play_preset(1, 0.6);
    fge_bgm_stop();

    assert!(!fge_set_ttl(zero, 1.0));
    assert!(!fge_set_ttl(invalid, 1.0));

    let emitter = fge_emitter_create_radial(100.0, 200.0, 10.0, 4, 150.0, 1.0, 6.0, 0x01, white);
    assert!(emitter.is_null());
    assert!(!fge_emitter_set_position(zero, 10.0, 20.0));
    assert!(!fge_emitter_set_spin(zero, 0.0, 1.5));
    assert!(!fge_emitter_set_active(zero, true));
    assert!(!fge_emitter_destroy(zero));

    assert!(fge_texture_load_png(FgeTextBuf::from_str_lossy("assets/missing.png")).is_null());
    assert!(!fge_set_texture(zero, zero));

    fge_ui_panel(10.0, 10.0, 100.0, 50.0, white, white, 2.0);
    fge_ui_bar(10.0, 10.0, 100.0, 12.0, 0.5, white, white);
    fge_ui_text(10.0, 10.0, 18.0, white, text);
    fge_ui_number(10.0, 30.0, 18.0, 12345, white);
    fge_ui_dialog_box(
        10.0,
        500.0,
        600.0,
        140.0,
        FgeTextBuf::from_str_lossy("村長"),
        text,
        4,
    );
    fge_world_text(300.0, 400.0, 16.0, white, text);
}

#[test]
fn step5_abi_audio_ttl_emitter_and_ui_inside_step_with_callback() {
    let mut ctx = new_test_ctx();
    let white = FgeColor {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    let panel_bg = FgeColor {
        r: 20,
        g: 24,
        b: 40,
        a: 220,
    };

    let mut ttl_entity = EntityId::INVALID;
    let mut emitter_id = EntityId::INVALID;

    ctx.step_with_callback(1.0 / 60.0, || {
        fge_audio_set_master_volume(0.8);
        fge_audio_play_sfx(0, 0.9, 1.0);
        fge_audio_play_tone(1, 330.0, 660.0, 0.08, 0.4);
        fge_bgm_play_preset(1, 0.5);

        ttl_entity = fge_spawn_sprite(100.0, 100.0, 16.0, 16.0, white);
        assert!(!ttl_entity.is_null());
        assert!(fge_set_ttl(ttl_entity, 0.04));
        assert!(!fge_set_ttl(EntityId::INVALID, 1.0));

        emitter_id = fge_emitter_create_radial(
            400.0,
            300.0,
            120.0,
            4,
            120.0,
            1.5,
            5.0,
            0x04,
            white,
        );
        assert!(!emitter_id.is_null());
        assert!(fge_emitter_set_position(emitter_id, 420.0, 320.0));
        assert!(fge_emitter_set_spin(emitter_id, 0.25, 1.5));
        assert!(fge_emitter_set_active(emitter_id, true));

        fge_ui_panel(16.0, 16.0, 300.0, 80.0, panel_bg, white, 2.0);
        fge_ui_bar(24.0, 48.0, 200.0, 14.0, 0.75, white, panel_bg);
        fge_ui_text(
            24.0,
            24.0,
            20.0,
            white,
            FgeTextBuf::from_str_lossy("勇者：いくぞ！"),
        );
        fge_ui_number(24.0, 68.0, 18.0, 999, white);
        fge_ui_dialog_box(
            100.0,
            500.0,
            900.0,
            150.0,
            FgeTextBuf::from_str_lossy("村長"),
            FgeTextBuf::from_str_lossy("勇者よ！北の遺跡のゴーレムを討伐してくれ！"),
            10,
        );
        fge_world_text(
            420.0,
            280.0,
            16.0,
            white,
            FgeTextBuf::from_str_lossy("-34"),
        );
    })
    .expect("step_with_callback should succeed");

    // UI commands emitted inside callback remain available on ctx.ui for rendering.
    assert_eq!(ctx.ui.len(), 6);
    assert!(matches!(ctx.ui.commands()[0], UiDrawCmd::Panel { .. }));
    assert!(matches!(ctx.ui.commands()[1], UiDrawCmd::Bar { .. }));
    assert!(matches!(ctx.ui.commands()[2], UiDrawCmd::ScreenText { .. }));
    assert!(matches!(ctx.ui.commands()[3], UiDrawCmd::ScreenText { .. }));
    assert!(matches!(ctx.ui.commands()[4], UiDrawCmd::DialogBox { .. }));
    assert!(matches!(ctx.ui.commands()[5], UiDrawCmd::WorldText { .. }));

    // Step additional frames so the 0.04s TTL expires and the radial emitter spawns bullets.
    for _ in 0..4 {
        ctx.step_with_callback(1.0 / 60.0, || {})
            .expect("subsequent step_with_callback should succeed");
    }

    // Empty callback frame clears previous UI commands.
    assert!(ctx.ui.is_empty());

    with_active_context(&mut ctx, || {
        assert!(
            !fge_entity_is_alive(ttl_entity),
            "entity with 0.04s TTL should auto-despawn after 5 frames at 60 FPS"
        );
        assert!(
            fge_entity_count() >= 4,
            "active radial emitter should have spawned bulk bullets into EntityWorld"
        );
        assert!(fge_emitter_set_active(emitter_id, false));
        assert!(fge_emitter_destroy(emitter_id));
        assert!(!fge_emitter_set_active(emitter_id, true));
        assert!(!fge_emitter_destroy(emitter_id));
        fge_bgm_stop();
    });
}
