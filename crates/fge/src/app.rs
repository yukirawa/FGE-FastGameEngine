use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use fge_abi::context::EngineContext;
use fge_core::{
    camera::Camera2D,
    input_core::FgeKey,
    sim::{BulkSimulation, SimConfig, SpriteSoA},
    stats::{BenchSummary, FrameStatsTracker},
};
use fge_loader::{compiler::CBuildPlan, hot_reload::HotReloader};
use fge_render::{
    context::{FrameRenderStats, GpuRenderer, PresentPolicy},
    error::RenderError,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    keyboard::KeyCode,
    window::{Window, WindowId},
};

use crate::{
    cli::{BenchArgs, Cli, Command, RunArgs},
    config::{load_manifest_from_path, GameManifest, GameSection},
    error::AppError,
    input::InputState,
    keymap::apply_window_event_to_core_input,
};

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.05,
    g: 0.06,
    b: 0.09,
    a: 1.0,
};
const FIXED_DT: f32 = 1.0 / 60.0;
const MAX_FRAME_DT: f32 = 0.1;
const DEFAULT_OFFSCREEN_FRAMES: u64 = 300;
const DEFAULT_GAME_OFFSCREEN_FRAMES: u64 = 60;
const HOT_RELOAD_POLL_INTERVAL_FRAMES: u64 = 15;
const INITIAL_WORLD_RENDER_CAPACITY: usize = 1024;

/// Executes the parsed CLI command and returns an optional `BenchSummary`.
pub fn execute_cli(cli: Cli) -> Result<Option<BenchSummary>, AppError> {
    match cli.command {
        Command::Bench(args) => {
            let summary = run_bench(&args)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                log::info!(
                    "Bench complete: {} frames, {} sprites ({} rendered, {} draw call/frame), avg {:.3} ms ({:.1} FPS, p99 {:.3} ms) on {} [{}]",
                    summary.total_frames,
                    summary.sprite_count,
                    summary.rendered_instances,
                    summary.draw_calls_per_frame,
                    summary.avg_frame_ms,
                    summary.avg_fps,
                    summary.p99_frame_ms,
                    summary.adapter_name,
                    summary.backend_name,
                );
                println!(
                    "FGE Bench Summary: frames={} sprites={} rendered={} draw_calls={} avg_fps={:.1} avg_ms={:.3} min_ms={:.3} max_ms={:.3} p99_ms={:.3} target_60fps={} backend={} adapter={} offscreen={}",
                    summary.total_frames,
                    summary.sprite_count,
                    summary.rendered_instances,
                    summary.draw_calls_per_frame,
                    summary.avg_fps,
                    summary.avg_frame_ms,
                    summary.min_frame_ms,
                    summary.max_frame_ms,
                    summary.p99_frame_ms,
                    summary.target_60fps_met,
                    summary.backend_name,
                    summary.adapter_name,
                    summary.offscreen,
                );
            }
            Ok(Some(summary))
        }
        Command::Run(args) => run_project(&args),
        Command::Test(_args) => todo!(),
    }
}

/// Runs the 1-draw-call `SpriteSoA` + `wgpu` rendering benchmark (either windowed via `winit` or offscreen).
pub fn run_bench(args: &BenchArgs) -> Result<BenchSummary, AppError> {
    let title = format!("FGE Benchmark ({} sprites)", args.count);
    run_bench_with_title(args, &title)
}

fn resolve_project_dir(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }
}

/// Loads `game.toml`, compiles and dynamically loads the C11 game module via `fge-loader`,
/// and runs the engine loop (either offscreen or windowed via `winit`) with optional hot-reload.
pub fn run_project(args: &RunArgs) -> Result<Option<BenchSummary>, AppError> {
    let GameManifest { game, build } = load_manifest_from_path(&args.project_dir)?;
    let project_dir = resolve_project_dir(&args.project_dir);

    log::info!(
        "Booting '{}' ({}x{}, target {} FPS, vsync={}, entry={}, hot_reload={})",
        game.title,
        game.width,
        game.height,
        game.target_fps,
        game.vsync,
        build.entry,
        !args.no_hot_reload,
    );

    let mut ctx = EngineContext::new(game.width as f32, game.height as f32, 42)?;
    let plan = CBuildPlan {
        project_dir,
        entry: build.entry,
        sources: build.sources,
        out_dir: std::env::temp_dir().join("fge_build_cache"),
        optimize: true,
    };
    let mut reloader = HotReloader::initial_build_and_load(plan, &mut ctx)?;

    if args.offscreen {
        let mut renderer = GpuRenderer::new_offscreen(
            game.width,
            game.height,
            ctx.world.len().max(INITIAL_WORLD_RENDER_CAPACITY),
        )?;
        let target_frames = if args.frames == 0 {
            DEFAULT_GAME_OFFSCREEN_FRAMES
        } else {
            args.frames
        };
        let dt = 1.0 / (game.target_fps.max(1) as f32);
        let mut tracker = FrameStatsTracker::with_capacity(target_frames.min(100_000) as usize);
        let mut last_stats = FrameRenderStats::default();
        let mut last_entity_count = ctx.world.len();

        let loop_result: Result<(), AppError> = (|| {
            for _ in 0..target_frames {
                let frame_start = Instant::now();
                if !args.no_hot_reload {
                    match reloader.poll_and_reload(&mut ctx) {
                        Ok(true) => {
                            log::info!(
                                "Hot-reloaded C game module to generation {}",
                                reloader.generation()
                            );
                        }
                        Ok(false) => {}
                        Err(err) => {
                            log::warn!(
                                "Hot-reload failed (keeping generation {}): {}",
                                reloader.generation(),
                                err
                            );
                        }
                    }
                }

                reloader.step_frame(&mut ctx, dt)?;
                last_entity_count = ctx.world.len();
                last_stats = renderer.render_world(&ctx.camera, &ctx.world, true, CLEAR_COLOR)?;

                let frame_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
                tracker.record_frame_ms(frame_ms);

                if ctx.quit_requested {
                    break;
                }
            }
            Ok(())
        })();

        reloader.shutdown(&mut ctx);
        loop_result?;

        let info = renderer.adapter_info();
        let backend_name = format!("{:?}", info.backend);
        let summary = tracker.summarize(
            last_entity_count,
            last_stats.submitted_instances,
            last_stats.draw_calls,
            &backend_name,
            &info.name,
            true,
        );
        return Ok(Some(summary));
    }

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let display_handle = event_loop.owned_display_handle();
    let initial_cap = if args.frames > 0 {
        args.frames.min(100_000) as usize
    } else {
        1024
    };
    let initial_entity_count = ctx.world.len();

    let mut app = WindowedGameApp {
        args,
        game: &game,
        display_handle: Some(display_handle),
        window: None,
        renderer: None,
        ctx,
        reloader,
        tracker: FrameStatsTracker::with_capacity(initial_cap),
        last_stats: FrameRenderStats::default(),
        last_entity_count: initial_entity_count,
        last_frame_instant: None,
        frame_counter: 0,
        fatal_error: None,
    };

    let run_result = event_loop.run_app(&mut app);
    app.reloader.shutdown(&mut app.ctx);
    run_result?;

    if let Some(err) = app.fatal_error {
        return Err(err);
    }

    let (backend_name, adapter_name) = match app.renderer.as_ref() {
        Some(renderer) => {
            let info = renderer.adapter_info();
            (format!("{:?}", info.backend), info.name.as_str())
        }
        None => (String::from("Unknown"), "Unknown"),
    };

    Ok(Some(app.tracker.summarize(
        app.last_entity_count,
        app.last_stats.submitted_instances,
        app.last_stats.draw_calls,
        &backend_name,
        adapter_name,
        false,
    )))
}

fn run_bench_with_title(args: &BenchArgs, window_title: &str) -> Result<BenchSummary, AppError> {
    if args.width == 0 || args.height == 0 {
        return Err(AppError::Render(RenderError::InvalidResolution {
            width: args.width,
            height: args.height,
        }));
    }

    let sim_cfg = SimConfig {
        world_width: args.width as f32,
        world_height: args.height as f32,
        ..SimConfig::default()
    };
    let mut soa = SpriteSoA::spawn_random_batch(args.count, args.seed, &sim_cfg)?;
    let camera = Camera2D::from_top_left_viewport(args.width as f32, args.height as f32)?;

    if args.offscreen {
        let mut renderer = GpuRenderer::new_offscreen(args.width, args.height, args.count.max(1))?;
        let target_frames = if args.frames == 0 {
            DEFAULT_OFFSCREEN_FRAMES
        } else {
            args.frames
        };
        let mut tracker = FrameStatsTracker::with_capacity(target_frames.min(100_000) as usize);
        let mut last_stats = FrameRenderStats::default();
        let bounds_w = args.width as f32;
        let bounds_h = args.height as f32;

        for _ in 0..target_frames {
            let frame_start = Instant::now();
            soa.step(FIXED_DT, bounds_w, bounds_h)?;
            last_stats = renderer.render_soa(&camera, &soa, args.cull, CLEAR_COLOR)?;
            let frame_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
            tracker.record_frame_ms(frame_ms);
        }

        let info = renderer.adapter_info();
        let backend_name = format!("{:?}", info.backend);
        return Ok(tracker.summarize(
            args.count,
            last_stats.submitted_instances,
            last_stats.draw_calls,
            &backend_name,
            &info.name,
            true,
        ));
    }

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let display_handle = event_loop.owned_display_handle();
    let initial_cap = if args.frames > 0 {
        args.frames.min(100_000) as usize
    } else {
        1024
    };

    let mut app = WindowedBenchApp {
        args,
        title: window_title,
        display_handle: Some(display_handle),
        window: None,
        renderer: None,
        camera,
        soa,
        input: InputState::new(),
        tracker: FrameStatsTracker::with_capacity(initial_cap),
        last_stats: FrameRenderStats::default(),
        last_frame_instant: None,
        bounds_w: args.width as f32,
        bounds_h: args.height as f32,
        fatal_error: None,
    };

    event_loop.run_app(&mut app)?;

    if let Some(err) = app.fatal_error {
        return Err(err);
    }

    let (backend_name, adapter_name) = match app.renderer.as_ref() {
        Some(renderer) => {
            let info = renderer.adapter_info();
            (format!("{:?}", info.backend), info.name.as_str())
        }
        None => (String::from("Unknown"), "Unknown"),
    };

    Ok(app.tracker.summarize(
        args.count,
        app.last_stats.submitted_instances,
        app.last_stats.draw_calls,
        &backend_name,
        adapter_name,
        false,
    ))
}

struct WindowedBenchApp<'a> {
    args: &'a BenchArgs,
    title: &'a str,
    display_handle: Option<OwnedDisplayHandle>,
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    camera: Camera2D,
    soa: SpriteSoA,
    input: InputState,
    tracker: FrameStatsTracker,
    last_stats: FrameRenderStats,
    last_frame_instant: Option<Instant>,
    bounds_w: f32,
    bounds_h: f32,
    fatal_error: Option<AppError>,
}

impl ApplicationHandler for WindowedBenchApp<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title(self.title)
            .with_inner_size(LogicalSize::new(self.args.width, self.args.height));

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                self.fatal_error = Some(AppError::WindowOs(err));
                event_loop.exit();
                return;
            }
        };

        let physical = window.inner_size();
        let init_w = physical.width.max(1);
        let init_h = physical.height.max(1);
        let policy = if self.args.vsync {
            PresentPolicy::Fifo
        } else {
            PresentPolicy::AutoNoVsync
        };

        let instance_desc = match self.display_handle.take() {
            Some(handle) => {
                wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(handle))
            }
            None => wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
        };

        match GpuRenderer::new_windowed(
            instance_desc,
            Arc::clone(&window),
            init_w,
            init_h,
            policy,
            self.args.count.max(1),
        ) {
            Ok(renderer) => {
                self.bounds_w = init_w as f32;
                self.bounds_h = init_h as f32;
                if let Ok(cam) = Camera2D::from_top_left_viewport(self.bounds_w, self.bounds_h) {
                    self.camera = cam;
                }
                self.renderer = Some(renderer);
                self.window = Some(window);
                self.last_frame_instant = Some(Instant::now());
            }
            Err(err) => {
                self.fatal_error = Some(AppError::Render(err));
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        self.input.process_window_event(&event);

        if self.input.close_requested() || self.input.is_key_just_pressed(KeyCode::Escape) {
            event_loop.exit();
            return;
        }

        if let Some((w, h)) = self.input.take_resized() {
            if w > 0 && h > 0 {
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(err) = renderer.resize(w, h) {
                        self.fatal_error = Some(AppError::Render(err));
                        event_loop.exit();
                        return;
                    }
                }
                self.bounds_w = w as f32;
                self.bounds_h = h as f32;
                if let Ok(cam) = Camera2D::from_top_left_viewport(self.bounds_w, self.bounds_h) {
                    self.camera = cam;
                }
            }
        }

        if matches!(event, WindowEvent::RedrawRequested) {
            let Some(renderer) = self.renderer.as_mut() else {
                return;
            };

            let frame_start = Instant::now();
            let dt = match self.last_frame_instant.replace(frame_start) {
                Some(prev) => prev.elapsed().as_secs_f32().clamp(0.0, MAX_FRAME_DT),
                None => FIXED_DT,
            };

            if let Err(err) = self.soa.step(dt, self.bounds_w, self.bounds_h) {
                self.fatal_error = Some(AppError::Core(err));
                event_loop.exit();
                return;
            }

            match renderer.render_soa(&self.camera, &self.soa, self.args.cull, CLEAR_COLOR) {
                Ok(stats) => {
                    if !stats.skipped {
                        let frame_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
                        self.tracker.record_frame_ms(frame_ms);
                        self.last_stats = stats;
                    }
                }
                Err(err) => {
                    self.fatal_error = Some(AppError::Render(err));
                    event_loop.exit();
                    return;
                }
            }

            self.input.begin_frame();

            if self.args.frames > 0 && self.tracker.frame_count() >= self.args.frames {
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

struct WindowedGameApp<'a> {
    args: &'a RunArgs,
    game: &'a GameSection,
    display_handle: Option<OwnedDisplayHandle>,
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    ctx: EngineContext,
    reloader: HotReloader,
    tracker: FrameStatsTracker,
    last_stats: FrameRenderStats,
    last_entity_count: usize,
    last_frame_instant: Option<Instant>,
    frame_counter: u64,
    fatal_error: Option<AppError>,
}

impl ApplicationHandler for WindowedGameApp<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title(&self.game.title)
            .with_inner_size(LogicalSize::new(self.game.width, self.game.height));

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                self.fatal_error = Some(AppError::WindowOs(err));
                event_loop.exit();
                return;
            }
        };

        let physical = window.inner_size();
        let init_w = physical.width.max(1);
        let init_h = physical.height.max(1);
        let policy = if self.game.vsync {
            PresentPolicy::Fifo
        } else {
            PresentPolicy::AutoNoVsync
        };

        let instance_desc = match self.display_handle.take() {
            Some(handle) => {
                wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(handle))
            }
            None => wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
        };

        match GpuRenderer::new_windowed(
            instance_desc,
            Arc::clone(&window),
            init_w,
            init_h,
            policy,
            self.ctx.world.len().max(INITIAL_WORLD_RENDER_CAPACITY),
        ) {
            Ok(renderer) => {
                let _ = self
                    .ctx
                    .camera
                    .set_viewport_size(init_w as f32, init_h as f32);
                self.renderer = Some(renderer);
                self.window = Some(window);
                self.last_frame_instant = Some(Instant::now());
            }
            Err(err) => {
                self.fatal_error = Some(AppError::Render(err));
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        apply_window_event_to_core_input(&mut self.ctx.input, &event);

        if matches!(event, WindowEvent::CloseRequested)
            || self.ctx.input.is_key_pressed(FgeKey::Escape)
        {
            event_loop.exit();
            return;
        }

        if let WindowEvent::Resized(size) = event {
            if size.width > 0 && size.height > 0 {
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(err) = renderer.resize(size.width, size.height) {
                        self.fatal_error = Some(AppError::Render(err));
                        event_loop.exit();
                        return;
                    }
                }
                let _ = self
                    .ctx
                    .camera
                    .set_viewport_size(size.width as f32, size.height as f32);
            }
            return;
        }

        if matches!(event, WindowEvent::RedrawRequested) {
            let Some(renderer) = self.renderer.as_mut() else {
                return;
            };

            let frame_start = Instant::now();
            let dt = match self.last_frame_instant.replace(frame_start) {
                Some(prev) => prev.elapsed().as_secs_f32().clamp(0.0, MAX_FRAME_DT),
                None => 1.0 / (self.game.target_fps.max(1) as f32),
            };

            if !self.args.no_hot_reload
                && self.frame_counter > 0
                && self.frame_counter % HOT_RELOAD_POLL_INTERVAL_FRAMES == 0
            {
                match self.reloader.poll_and_reload(&mut self.ctx) {
                    Ok(true) => {
                        log::info!(
                            "Hot-reloaded C game module to generation {}",
                            self.reloader.generation()
                        );
                    }
                    Ok(false) => {}
                    Err(err) => {
                        log::warn!(
                            "Hot-reload failed (keeping generation {}): {}",
                            self.reloader.generation(),
                            err
                        );
                    }
                }
            }

            if let Err(err) = self.reloader.step_frame(&mut self.ctx, dt) {
                self.fatal_error = Some(AppError::Loader(err));
                event_loop.exit();
                return;
            }

            self.frame_counter = self.frame_counter.saturating_add(1);
            self.last_entity_count = self.ctx.world.len();

            match renderer.render_world(&self.ctx.camera, &self.ctx.world, true, CLEAR_COLOR) {
                Ok(stats) => {
                    if !stats.skipped {
                        let frame_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
                        self.tracker.record_frame_ms(frame_ms);
                        self.last_stats = stats;
                    }
                }
                Err(err) => {
                    self.fatal_error = Some(AppError::Render(err));
                    event_loop.exit();
                    return;
                }
            }

            if self.ctx.quit_requested
                || (self.args.frames > 0 && self.tracker.frame_count() >= self.args.frames)
            {
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

/// Executes a `game.toml` project headlessly via `fge-bridge` and returns a structured report.
pub fn run_test_cmd(
    _args: &crate::cli::TestArgs,
) -> Result<fge_bridge::snapshot::HeadlessTestReport, crate::error::AppError> {
    todo!()
}
