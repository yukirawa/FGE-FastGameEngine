use serde::{Deserialize, Serialize};

/// Structured benchmark report suitable for terminal logging and AI JSON consumption.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchSummary {
    pub total_frames: u64,
    pub sprite_count: usize,
    pub rendered_instances: u32,
    pub draw_calls_per_frame: u32,
    pub total_elapsed_ms: f64,
    pub avg_frame_ms: f64,
    pub min_frame_ms: f64,
    pub max_frame_ms: f64,
    pub p99_frame_ms: f64,
    pub avg_fps: f64,
    pub target_60fps_met: bool,
    pub backend_name: String,
    pub adapter_name: String,
    pub offscreen: bool,
}

/// Accumulates per-frame durations (in milliseconds) to compute FPS and percentile metrics.
#[derive(Clone, Debug, Default)]
pub struct FrameStatsTracker {
    frame_times_ms: Vec<f64>,
}

impl FrameStatsTracker {
    /// Creates a new tracker with pre-allocated capacity for `expected_frames`.
    pub fn with_capacity(_expected_frames: usize) -> Self {
        todo!("FrameStatsTracker::with_capacity")
    }

    /// Records a single frame duration in milliseconds (non-finite or negative values are ignored).
    pub fn record_frame_ms(&mut self, _frame_ms: f64) {
        todo!("FrameStatsTracker::record_frame_ms")
    }

    /// Number of recorded frames.
    pub fn frame_count(&self) -> u64 {
        todo!("FrameStatsTracker::frame_count")
    }

    /// Returns a slice of recorded frame durations in milliseconds.
    pub fn samples_ms(&self) -> &[f64] {
        &self.frame_times_ms
    }

    /// Computes rolling average FPS over the most recent `window` frames (`0.0` if empty or total time `<= 0`).
    pub fn rolling_fps(&self, _window: usize) -> f64 {
        todo!("FrameStatsTracker::rolling_fps")
    }

    /// Resets recorded samples while retaining capacity.
    pub fn reset(&mut self) {
        todo!("FrameStatsTracker::reset")
    }

    /// Produces a complete `BenchSummary` from all recorded frames.
    pub fn summarize(
        &self,
        _sprite_count: usize,
        _rendered_instances: u32,
        _draw_calls_per_frame: u32,
        _backend_name: &str,
        _adapter_name: &str,
        _offscreen: bool,
    ) -> BenchSummary {
        todo!("FrameStatsTracker::summarize")
    }
}
