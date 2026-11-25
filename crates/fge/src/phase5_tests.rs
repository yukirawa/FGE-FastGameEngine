// Step 5 config tests for `[audio]` and `[ui]` sections in `game.toml`, plus
// end-to-end headless & Vulkan offscreen verification of `examples/crystal_chronicle`.

use std::path::{Path, PathBuf};

use fge_render::error::RenderError;

use crate::{
    app::{run_project, run_test_cmd},
    cli::{RunArgs, TestArgs},
    config::{parse_manifest_str, AudioSection, UiSection},
    error::AppError,
};

fn crystal_chronicle_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/crystal_chronicle")
}

#[test]
fn manifest_audio_and_ui_sections_apply_defaults_when_omitted() {
    let raw = r#"
[game]
title = "Crystal Chronicle"
"#;
    let manifest = parse_manifest_str(raw).expect("minimal manifest should parse");
    assert_eq!(manifest.audio, AudioSection::default());
    assert!((manifest.audio.master_volume - 0.8).abs() < 1e-6);
    assert!(!manifest.audio.muted);
    assert_eq!(manifest.ui, UiSection::default());
    assert_eq!(manifest.ui.font, None);
}

#[test]
fn manifest_parses_explicit_audio_and_ui_sections() {
    let raw = r#"
[game]
title = "Crystal Chronicle"
width = 1280
height = 720

[audio]
master_volume = 0.45
muted = true

[ui]
font = "assets/NotoSansCJK-Regular.ttc"
"#;
    let manifest = parse_manifest_str(raw).expect("manifest with [audio] and [ui] should parse");
    assert!((manifest.audio.master_volume - 0.45).abs() < 1e-6);
    assert!(manifest.audio.muted);
    assert_eq!(
        manifest.ui.font.as_deref(),
        Some("assets/NotoSansCJK-Regular.ttc")
    );
}

#[test]
fn manifest_partial_audio_inherits_default_volume_and_allows_zero_volume() {
    let muted_only = r#"
[game]
title = "Muted Game"

[audio]
muted = true
"#;
    let manifest_muted = parse_manifest_str(muted_only).expect("partial [audio] should parse");
    assert!(manifest_muted.audio.muted);
    assert!((manifest_muted.audio.master_volume - 0.8).abs() < 1e-6);

    let zero_vol = r#"
[game]
title = "Silent Volume Game"

[audio]
master_volume = 0.0
"#;
    let manifest_zero = parse_manifest_str(zero_vol).expect("zero master_volume should be valid");
    assert!((manifest_zero.audio.master_volume - 0.0).abs() < 1e-6);
}

#[test]
fn manifest_rejects_invalid_audio_volume_and_empty_ui_font() {
    let neg_vol = r#"
[game]
title = "Invalid Volume"

[audio]
master_volume = -0.25
"#;
    assert!(matches!(
        parse_manifest_str(neg_vol),
        Err(AppError::InvalidManifest { .. })
    ));

    let empty_font = r#"
[game]
title = "Invalid Font"

[ui]
font = "   "
"#;
    assert!(matches!(
        parse_manifest_str(empty_font),
        Err(AppError::InvalidManifest { .. })
    ));
}

#[test]
fn run_test_cmd_verifies_crystal_chronicle_900_frames_headless() {
    let args = TestArgs {
        project_dir: crystal_chronicle_dir(),
        frames: 900,
        seed: 42,
        dt: 1.0 / 60.0,
        dump_json: true,
    };

    let report =
        run_test_cmd(&args).expect("run_test_cmd on examples/crystal_chronicle should succeed");

    assert_eq!(
        report.status, "ok",
        "crystal_chronicle headless run failed: compile_error={:?}, logs={:?}",
        report.compile_error, report.logs
    );
    assert_eq!(report.frames_requested, 900);
    assert_eq!(report.frames_executed, 900);
    assert!(report.compile_error.is_none());
    assert!(
        report.collision_events_total > 0,
        "expected RPG combat collisions over 900 frames (15.0s), got 0"
    );

    let snapshot = report
        .world_snapshot
        .as_ref()
        .expect("crystal_chronicle report should include a world_snapshot");
    assert_eq!(snapshot.frame_count, 900);
    assert!(
        snapshot.entity_count > 10,
        "expected > 10 live world entities after 900 frames, got {}",
        snapshot.entity_count
    );

    let metric = |id: u32| -> Option<f32> {
        snapshot
            .metrics
            .iter()
            .find(|(m_id, _)| *m_id == id)
            .map(|(_, v)| *v)
    };
    let hero_level = metric(1).unwrap_or(0.0);
    let monsters_slain = metric(2).unwrap_or(0.0);
    let quest_stage = metric(3).unwrap_or(0.0);
    let gold = metric(4).unwrap_or(0.0);
    let sim_fps = if report.sim_timing.total_ms > 0.0 {
        (report.frames_executed as f64) / (report.sim_timing.total_ms / 1000.0)
    } else {
        0.0
    };

    assert!(
        hero_level >= 1.0,
        "expected hero_level >= 1 after 900 frames, got {hero_level}"
    );
    assert!(
        quest_stage >= 1.0,
        "expected quest_stage >= 1 after talking to Elder in 900 frames, got {quest_stage}"
    );
    assert!(
        monsters_slain > 0.0,
        "expected monsters_slain > 0 after 900 frames, got {monsters_slain}"
    );

    eprintln!(
        "crystal_chronicle 900f headless telemetry: status={}, sim_fps={:.1}, total_ms={:.2}, avg_step_us={:.2}, max_step_us={:.2}, entity_count={}, collisions={}, hero_level={:.0}, monsters_slain={:.0}, quest_stage={:.0}, gold={:.0}",
        report.status,
        sim_fps,
        report.sim_timing.total_ms,
        report.sim_timing.avg_step_us,
        report.sim_timing.max_step_us,
        snapshot.entity_count,
        report.collision_events_total,
        hero_level,
        monsters_slain,
        quest_stage,
        gold,
    );
}

#[test]
fn run_project_offscreen_verifies_crystal_chronicle_20_frames_gpu() {
    let args = RunArgs {
        project_dir: crystal_chronicle_dir(),
        frames: 20,
        offscreen: true,
        no_hot_reload: true,
        bridge: None,
        bridge_api_key: None,
    };

    let summary = match run_project(&args) {
        Ok(Some(summary)) => summary,
        Ok(None) => panic!("expected run_project to return Some(BenchSummary) for offscreen run"),
        Err(AppError::Render(RenderError::RequestAdapter(err))) => {
            log::warn!(
                "Skipping crystal_chronicle offscreen GPU smoke test (no GPU adapter): {err}"
            );
            return;
        }
        Err(err) => {
            panic!("unexpected error running examples/crystal_chronicle offscreen on GPU: {err}")
        }
    };

    assert_eq!(summary.total_frames, 20);
    assert!(summary.offscreen);
    assert!(
        summary.rendered_instances > 0,
        "expected rendered world + Japanese RPG UI instances > 0, got 0"
    );
    assert!(
        (1..=2).contains(&summary.draw_calls_per_frame),
        "expected 1-2 draw calls per frame (world + screen UI), got {}",
        summary.draw_calls_per_frame
    );

    eprintln!(
        "crystal_chronicle 20f GPU offscreen telemetry: backend={}, adapter={}, frames={}, entities={}, rendered_instances={}, draw_calls={}, avg_ms={:.3}, avg_fps={:.1}",
        summary.backend_name,
        summary.adapter_name,
        summary.total_frames,
        summary.sprite_count,
        summary.rendered_instances,
        summary.draw_calls_per_frame,
        summary.avg_frame_ms,
        summary.avg_fps,
    );
}
