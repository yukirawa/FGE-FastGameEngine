use std::path::PathBuf;
use std::time::Instant;

use fge_abi::context::EngineContext;
use fge_core::spatial::CollisionPair;
use fge_loader::{
    compiler::{CBuildPlan, CompilerKind},
    error::LoaderError,
    hot_reload::HotReloader,
};

use crate::snapshot::{
    capture_world_snapshot, CompileDiagnostic, HeadlessTestReport, SimTimingReport,
};

const MAX_COLLISION_SAMPLES: usize = 64;

/// Configuration for a GPU-free, deterministic headless simulation test run.
#[derive(Clone, Debug)]
pub struct HeadlessConfig {
    pub project_dir: PathBuf,
    pub project_title: String,
    pub width: u32,
    pub height: u32,
    pub entry: String,
    pub sources: Vec<String>,
    pub frames: u64,
    pub seed: u64,
    pub dt: f32,
    pub snapshot_entity_limit: Option<usize>,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            project_dir: PathBuf::from("."),
            project_title: String::from("FGE Headless Test"),
            width: 1280,
            height: 720,
            entry: String::from("src/main.c"),
            sources: Vec::new(),
            frames: 300,
            seed: 42,
            dt: 1.0 / 60.0,
            snapshot_entity_limit: None,
        }
    }
}

fn empty_report(cfg: &HeadlessConfig, status: &str) -> HeadlessTestReport {
    HeadlessTestReport {
        status: String::from(status),
        project_title: cfg.project_title.clone(),
        seed: cfg.seed,
        frames_requested: cfg.frames,
        frames_executed: 0,
        dt: cfg.dt,
        sim_timing: SimTimingReport::default(),
        collision_events_total: 0,
        collision_samples: Vec::new(),
        compile_error: None,
        world_snapshot: None,
        logs: Vec::new(),
    }
}

/// Compiles, loads, and executes a C11 game project without opening a window or initializing `wgpu`,
/// using default spatial hash grid settings (`cell_size = 64.0`, `buckets = 4096`) and default compiler priority.
pub fn run_headless_test(cfg: &HeadlessConfig) -> HeadlessTestReport {
    run_headless_test_with_options(cfg, 64.0, 4096, None)
}

/// Compiles, loads, and executes a C11 game project headlessly with explicit spatial grid
/// parameters and optional preferred compiler toolchain.
pub fn run_headless_test_with_options(
    cfg: &HeadlessConfig,
    cell_size: f32,
    buckets: usize,
    preferred_compiler: Option<CompilerKind>,
) -> HeadlessTestReport {
    if !cfg.dt.is_finite() || cfg.dt < 0.0 {
        let mut report = empty_report(cfg, "config_error");
        report
            .logs
            .push(format!("invalid simulation dt: {}", cfg.dt));
        return report;
    }

    let mut ctx = match EngineContext::with_spatial_config(
        cfg.width as f32,
        cfg.height as f32,
        cfg.seed,
        cell_size,
        buckets,
    ) {
        Ok(ctx) => ctx,
        Err(err) => {
            let mut report = empty_report(cfg, "config_error");
            report.logs.push(err.to_string());
            return report;
        }
    };

    let plan = CBuildPlan {
        project_dir: cfg.project_dir.clone(),
        entry: cfg.entry.clone(),
        sources: cfg.sources.clone(),
        out_dir: std::env::temp_dir().join("fge_headless_cache"),
        optimize: true,
        preferred_compiler,
    };

    let reloader = match HotReloader::initial_build_and_load(plan, &mut ctx) {
        Ok(reloader) => reloader,
        Err(LoaderError::CompileFailed {
            compiler,
            stderr,
            status,
        }) => {
            let mut report = empty_report(cfg, "compile_error");
            report.logs.push(format!(
                "C compilation failed with {} (exit status {:?})",
                compiler, status
            ));
            report.compile_error = Some(CompileDiagnostic {
                compiler,
                stderr,
                exit_code: status,
            });
            return report;
        }
        Err(err) => {
            let mut report = empty_report(cfg, "load_error");
            report.logs.push(err.to_string());
            return report;
        }
    };

    let mut frames_executed: u64 = 0;
    let mut total_step_us: f64 = 0.0;
    let mut max_step_us: f64 = 0.0;
    let mut collision_events_total: u64 = 0;
    let mut collision_samples: Vec<CollisionPair> = Vec::with_capacity(MAX_COLLISION_SAMPLES);
    let mut logs: Vec<String> = Vec::new();
    let mut status = "ok";

    let sim_start = Instant::now();
    for _ in 0..cfg.frames {
        let step_start = Instant::now();
        if let Err(err) = reloader.step_frame(&mut ctx, cfg.dt) {
            status = "runtime_error";
            logs.push(err.to_string());
            break;
        }
        let step_us = step_start.elapsed().as_secs_f64() * 1_000_000.0;

        frames_executed = frames_executed.saturating_add(1);
        total_step_us += step_us;
        if step_us > max_step_us {
            max_step_us = step_us;
        }

        let frame_collisions = &ctx.last_collisions;
        if !frame_collisions.is_empty() {
            collision_events_total =
                collision_events_total.saturating_add(frame_collisions.len() as u64);
            if frame_collisions.len() >= MAX_COLLISION_SAMPLES {
                collision_samples.clear();
                let start = frame_collisions.len() - MAX_COLLISION_SAMPLES;
                collision_samples.extend(frame_collisions[start..].iter().copied());
            } else {
                let overflow = (collision_samples.len() + frame_collisions.len())
                    .saturating_sub(MAX_COLLISION_SAMPLES);
                if overflow > 0 {
                    collision_samples.drain(..overflow);
                }
                collision_samples.extend(frame_collisions.iter().copied());
            }
        }

        if ctx.quit_requested {
            break;
        }
    }
    let total_ms = sim_start.elapsed().as_secs_f64() * 1000.0;

    let world_snapshot = Some(capture_world_snapshot(
        &ctx,
        &[],
        cfg.snapshot_entity_limit,
    ));

    reloader.shutdown(&mut ctx);
    let avg_step_us = if frames_executed > 0 {
        total_step_us / (frames_executed as f64)
    } else {
        0.0
    };

    HeadlessTestReport {
        status: String::from(status),
        project_title: cfg.project_title.clone(),
        seed: cfg.seed,
        frames_requested: cfg.frames,
        frames_executed,
        dt: cfg.dt,
        sim_timing: SimTimingReport {
            total_ms,
            avg_step_us,
            max_step_us,
        },
        collision_events_total,
        collision_samples,
        compile_error: None,
        world_snapshot,
        logs,
    }
}
