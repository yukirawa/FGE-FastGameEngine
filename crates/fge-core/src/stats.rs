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
    pub fn with_capacity(expected_frames: usize) -> Self {
        Self {
            frame_times_ms: Vec::with_capacity(expected_frames),
        }
    }

    /// Records a single frame duration in milliseconds (non-finite or negative values are ignored).
    pub fn record_frame_ms(&mut self, frame_ms: f64) {
        if frame_ms.is_finite() && frame_ms >= 0.0 {
            self.frame_times_ms.push(frame_ms);
        }
    }

    /// Number of recorded frames.
    pub fn frame_count(&self) -> u64 {
        self.frame_times_ms.len() as u64
    }

    /// Returns a slice of recorded frame durations in milliseconds.
    pub fn samples_ms(&self) -> &[f64] {
        &self.frame_times_ms
    }

    /// Computes rolling average FPS over the most recent `window` frames (`0.0` if empty or total time `<= 0`).
    pub fn rolling_fps(&self, window: usize) -> f64 {
        if window == 0 || self.frame_times_ms.is_empty() {
            return 0.0;
        }
        let count = window.min(self.frame_times_ms.len());
        let start = self.frame_times_ms.len() - count;
        let sum_ms: f64 = self.frame_times_ms[start..].iter().sum();
        if sum_ms > 0.0 {
            (count as f64) * 1000.0 / sum_ms
        } else {
            0.0
        }
    }

    /// Resets recorded samples while retaining capacity.
    pub fn reset(&mut self) {
        self.frame_times_ms.clear();
    }

    /// Produces a complete `BenchSummary` from all recorded frames.
    pub fn summarize(
        &self,
        sprite_count: usize,
        rendered_instances: u32,
        draw_calls_per_frame: u32,
        backend_name: &str,
        adapter_name: &str,
        offscreen: bool,
    ) -> BenchSummary {
        let total_frames = self.frame_times_ms.len() as u64;
        if total_frames == 0 {
            return BenchSummary {
                total_frames: 0,
                sprite_count,
                rendered_instances,
                draw_calls_per_frame,
                total_elapsed_ms: 0.0,
                avg_frame_ms: 0.0,
                min_frame_ms: 0.0,
                max_frame_ms: 0.0,
                p99_frame_ms: 0.0,
                avg_fps: 0.0,
                target_60fps_met: false,
                backend_name: backend_name.to_owned(),
                adapter_name: adapter_name.to_owned(),
                offscreen,
            };
        }

        let mut total_elapsed_ms = 0.0f64;
        let mut min_frame_ms = f64::INFINITY;
        let mut max_frame_ms = 0.0f64;
        for &ms in &self.frame_times_ms {
            total_elapsed_ms += ms;
            min_frame_ms = min_frame_ms.min(ms);
            max_frame_ms = max_frame_ms.max(ms);
        }
        let avg_frame_ms = total_elapsed_ms / (total_frames as f64);

        let mut scratch = self.frame_times_ms.to_vec();
        let p99_idx = ((scratch.len() as f64 * 0.99).ceil() as usize)
            .saturating_sub(1)
            .min(scratch.len() - 1);
        let (_, &mut p99_frame_ms, _) = scratch.select_nth_unstable_by(p99_idx, f64::total_cmp);

        let avg_fps = if total_elapsed_ms > 0.0 {
            (total_frames as f64) * 1000.0 / total_elapsed_ms
        } else {
            0.0
        };

        BenchSummary {
            total_frames,
            sprite_count,
            rendered_instances,
            draw_calls_per_frame,
            total_elapsed_ms,
            avg_frame_ms,
            min_frame_ms,
            max_frame_ms,
            p99_frame_ms,
            avg_fps,
            target_60fps_met: avg_fps >= 60.0,
            backend_name: backend_name.to_owned(),
            adapter_name: adapter_name.to_owned(),
            offscreen,
        }
    }
}
