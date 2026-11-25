use fge_abi::context::EngineContext;
use fge_core::{handle::EntityId, spatial::CollisionPair};
use serde::{Deserialize, Serialize};

/// Serializable snapshot of a single live entity in `EntityWorld`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: EntityId,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub size: [f32; 2],
    pub rotation: f32,
    pub hp: f32,
    pub tag: u32,
    pub layer_mask: u32,
    pub collider_radius: f32,
    pub flags: u32,
    pub color_rgba: [u8; 4],
    pub dialogue: Option<String>,
}

/// Serializable snapshot of the active 2D camera.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraSnapshot {
    pub center: [f32; 2],
    pub viewport_size: [f32; 2],
    pub zoom: f32,
}

/// Complete serializable snapshot of the gameplay world and bridge telemetry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub frame_count: u64,
    pub dt: f32,
    pub entity_count: usize,
    pub camera: CameraSnapshot,
    pub entities: Vec<EntitySnapshot>,
    pub last_collisions: Vec<CollisionPair>,
    pub params: Vec<(u32, f32)>,
    pub metrics: Vec<(u32, f32)>,
}

/// Timing statistics collected during a headless simulation run.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimTimingReport {
    pub total_ms: f64,
    pub avg_step_us: f64,
    pub max_step_us: f64,
}

/// Structured C11 compiler diagnostic surfaced when headless build fails.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileDiagnostic {
    pub compiler: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// Structured JSON report emitted by `run_headless_test` / `fge test`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessTestReport {
    pub status: String,
    pub project_title: String,
    pub seed: u64,
    pub frames_requested: u64,
    pub frames_executed: u64,
    pub dt: f32,
    pub sim_timing: SimTimingReport,
    pub collision_events_total: u64,
    pub collision_samples: Vec<CollisionPair>,
    pub compile_error: Option<CompileDiagnostic>,
    pub world_snapshot: Option<WorldSnapshot>,
    pub logs: Vec<String>,
}

/// Captures a serializable `WorldSnapshot` from `ctx`, optionally truncating `entities` to `max_entities`.
#[allow(unused_variables)]
pub fn capture_world_snapshot(
    ctx: &EngineContext,
    dialogues: &[(EntityId, String)],
    max_entities: Option<usize>,
) -> WorldSnapshot {
    todo!()
}

/// Queries entities within `radius` around `(x, y)` (filtered by `layer_mask` when non-zero)
/// and serializes them into `EntitySnapshot` records.
#[allow(unused_variables)]
pub fn query_nearby_snapshots(
    ctx: &EngineContext,
    dialogues: &[(EntityId, String)],
    x: f32,
    y: f32,
    radius: f32,
    layer_mask: u32,
) -> Vec<EntitySnapshot> {
    todo!()
}
