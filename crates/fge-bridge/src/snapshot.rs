use fge_abi::context::EngineContext;
use fge_core::{handle::EntityId, spatial::CollisionPair, world::EntityWorld};
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

#[inline]
fn lookup_dialogue(id: EntityId, dialogues: &[(EntityId, String)]) -> Option<String> {
    dialogues
        .iter()
        .find(|(eid, _)| *eid == id)
        .map(|(_, text)| text.clone())
}

#[inline]
fn entity_snapshot_at(
    world: &EntityWorld,
    idx: usize,
    dialogues: &[(EntityId, String)],
) -> EntitySnapshot {
    let id = world.entities[idx];
    EntitySnapshot {
        id,
        pos: [world.pos_x[idx], world.pos_y[idx]],
        vel: [world.vel_x[idx], world.vel_y[idx]],
        size: [world.size_w[idx], world.size_h[idx]],
        rotation: world.rotation[idx],
        hp: world.hp[idx],
        tag: world.tag[idx],
        layer_mask: world.layer_mask[idx],
        collider_radius: world.collider_radius[idx],
        flags: world.flags[idx],
        color_rgba: world.color_rgba[idx],
        dialogue: lookup_dialogue(id, dialogues),
    }
}

/// Captures a serializable `WorldSnapshot` from `ctx`, optionally truncating `entities` to `max_entities`.
pub fn capture_world_snapshot(
    ctx: &EngineContext,
    dialogues: &[(EntityId, String)],
    max_entities: Option<usize>,
) -> WorldSnapshot {
    let world = &ctx.world;
    let total = world.len();
    let count = max_entities.map_or(total, |limit| limit.min(total));
    let mut entities = Vec::with_capacity(count);
    for idx in 0..count {
        entities.push(entity_snapshot_at(world, idx, dialogues));
    }

    WorldSnapshot {
        frame_count: ctx.frame_count,
        dt: ctx.dt,
        entity_count: total,
        camera: CameraSnapshot {
            center: [ctx.camera.center.x, ctx.camera.center.y],
            viewport_size: [ctx.camera.viewport_size.x, ctx.camera.viewport_size.y],
            zoom: ctx.camera.zoom,
        },
        entities,
        last_collisions: ctx.last_collisions.clone(),
        params: ctx.params.clone(),
        metrics: ctx.metrics.clone(),
    }
}

/// Queries entities within `radius` around `(x, y)` (filtered by `layer_mask` when non-zero)
/// and serializes them into `EntitySnapshot` records.
pub fn query_nearby_snapshots(
    ctx: &EngineContext,
    dialogues: &[(EntityId, String)],
    x: f32,
    y: f32,
    radius: f32,
    layer_mask: u32,
) -> Vec<EntitySnapshot> {
    if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius < 0.0 {
        return Vec::new();
    }

    let world = &ctx.world;
    let pos_x = &world.pos_x;
    let pos_y = &world.pos_y;
    let radii = &world.collider_radius;
    let masks = &world.layer_mask;

    let mut out = Vec::new();
    for idx in 0..world.len() {
        if layer_mask != 0 && (masks[idx] & layer_mask) == 0 {
            continue;
        }
        let dx = pos_x[idx] - x;
        let dy = pos_y[idx] - y;
        let r_i = radii[idx].max(0.0);
        let max_dist = radius + r_i;
        if dx * dx + dy * dy <= max_dist * max_dist {
            out.push(entity_snapshot_at(world, idx, dialogues));
        }
    }
    out
}
