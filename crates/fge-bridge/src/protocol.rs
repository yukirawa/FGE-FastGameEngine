use fge_abi::{context::EngineContext, types::FgeAiEvent};
use fge_core::{handle::EntityId, input_core::FgeKey, world::EntityWorld};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::snapshot::{capture_world_snapshot, query_nearby_snapshots, EntitySnapshot};

pub const RPC_ERR_PARSE: i32 = -32700;
pub const RPC_ERR_METHOD_NOT_FOUND: i32 = -32601;
pub const RPC_ERR_INVALID_PARAMS: i32 = -32602;
pub const RPC_ERR_ENTITY_NOT_FOUND: i32 = -32001;

/// Mutable per-session state maintained by the AI Bridge alongside `EngineContext`.
#[derive(Clone, Debug, Default)]
pub struct BridgeState {
    pub dialogues: Vec<(EntityId, String)>,
}

impl BridgeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or updates the dialogue/note associated with `entity`.
    pub fn set_dialogue(&mut self, entity: EntityId, text: String) {
        if let Some((_, existing)) = self.dialogues.iter_mut().find(|(id, _)| *id == entity) {
            *existing = text;
        } else {
            self.dialogues.push((entity, text));
        }
    }

    /// Returns the dialogue/note associated with `entity`, if any.
    pub fn get_dialogue(&self, entity: EntityId) -> Option<&str> {
        self.dialogues
            .iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, text)| text.as_str())
    }

    /// Removes dialogue entries whose entities are no longer alive in `world`.
    pub fn prune_dead(&mut self, world: &EntityWorld) {
        self.dialogues.retain(|(id, _)| world.is_alive(*id));
    }
}

/// Inbound JSON-RPC 2.0 request envelope.
#[derive(Clone, Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: Option<String>,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// Standard JSON-RPC 2.0 error object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RpcErrorObj {
    pub code: i32,
    pub message: String,
}

/// Outbound JSON-RPC 2.0 response envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErrorObj>,
}

impl RpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result: None,
            error: Some(RpcErrorObj {
                code,
                message: message.into(),
            }),
        }
    }
}

#[derive(Deserialize)]
struct SnapshotParams {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct QueryNearbyParams {
    x: f32,
    y: f32,
    radius: f32,
    #[serde(default)]
    layer_mask: Option<u32>,
}

#[derive(Deserialize)]
struct EntityHandleParams {
    index: u32,
    generation: u32,
}

#[derive(Deserialize)]
struct SpawnParams {
    x: f32,
    y: f32,
    #[serde(default)]
    w: Option<f32>,
    #[serde(default)]
    h: Option<f32>,
    #[serde(default)]
    color: Option<[u8; 4]>,
    #[serde(default)]
    vel: Option<[f32; 2]>,
    #[serde(default)]
    hp: Option<f32>,
    #[serde(default)]
    tag: Option<u32>,
    #[serde(default)]
    collider_radius: Option<f32>,
    #[serde(default)]
    layer_mask: Option<u32>,
    #[serde(default)]
    dialogue: Option<String>,
}

#[derive(Deserialize)]
struct PatchParams {
    index: u32,
    generation: u32,
    #[serde(default)]
    pos: Option<[f32; 2]>,
    #[serde(default)]
    vel: Option<[f32; 2]>,
    #[serde(default)]
    size: Option<[f32; 2]>,
    #[serde(default)]
    hp: Option<f32>,
    #[serde(default)]
    tag: Option<u32>,
    #[serde(default)]
    color: Option<[u8; 4]>,
    #[serde(default)]
    flags: Option<u32>,
    #[serde(default)]
    dialogue: Option<String>,
}

#[derive(Deserialize)]
struct InputInjectParams {
    key: u32,
    pressed: bool,
}

#[derive(Deserialize)]
struct EventEmitParams {
    kind: u32,
    #[serde(default)]
    target: Option<EntityId>,
    #[serde(default)]
    arg0: Option<f32>,
    #[serde(default)]
    arg1: Option<f32>,
}

#[derive(Deserialize)]
struct ParamSetParams {
    id: u32,
    value: f32,
}

#[derive(Deserialize)]
struct ParamGetParams {
    id: u32,
    #[serde(default)]
    default: Option<f32>,
}

#[derive(Deserialize)]
struct CameraSetParams {
    center_x: f32,
    center_y: f32,
    zoom: f32,
}

fn parse_params<T: for<'de> Deserialize<'de>>(params: Value) -> Result<T, RpcErrorObj> {
    let normalized = if params.is_null() {
        Value::Object(serde_json::Map::new())
    } else {
        params
    };
    serde_json::from_value(normalized).map_err(|err| RpcErrorObj {
        code: RPC_ERR_INVALID_PARAMS,
        message: format!("invalid params: {err}"),
    })
}

fn dispatch_method(
    method: &str,
    params: Value,
    ctx: &mut EngineContext,
    state: &mut BridgeState,
) -> Result<Value, RpcErrorObj> {
    match method {
        "fge.ping" => Ok(json!({
            "ok": true,
            "frame": ctx.frame_count,
            "entities": ctx.world.len(),
        })),

        "fge.world.snapshot" => {
            let p: SnapshotParams = parse_params(params)?;
            let snapshot = capture_world_snapshot(ctx, &state.dialogues, p.limit);
            serde_json::to_value(snapshot).map_err(|err| RpcErrorObj {
                code: RPC_ERR_INVALID_PARAMS,
                message: format!("failed to serialize world snapshot: {err}"),
            })
        }

        "fge.world.query_nearby" => {
            let p: QueryNearbyParams = parse_params(params)?;
            if !p.x.is_finite() || !p.y.is_finite() || !p.radius.is_finite() || p.radius < 0.0 {
                return Err(RpcErrorObj {
                    code: RPC_ERR_INVALID_PARAMS,
                    message: "x, y must be finite and radius must be finite and >= 0".to_owned(),
                });
            }
            let hits = query_nearby_snapshots(
                ctx,
                &state.dialogues,
                p.x,
                p.y,
                p.radius,
                p.layer_mask.unwrap_or(0),
            );
            serde_json::to_value(hits).map_err(|err| RpcErrorObj {
                code: RPC_ERR_INVALID_PARAMS,
                message: format!("failed to serialize nearby query: {err}"),
            })
        }

        "fge.entity.get" => {
            let p: EntityHandleParams = parse_params(params)?;
            let id = EntityId {
                index: p.index,
                generation: p.generation,
            };
            let Some(idx) = ctx.world.dense_index_of(id) else {
                return Err(RpcErrorObj {
                    code: RPC_ERR_ENTITY_NOT_FOUND,
                    message: format!("entity not alive: ({}, {})", id.index, id.generation),
                });
            };
            let snap = EntitySnapshot {
                id,
                pos: [ctx.world.pos_x[idx], ctx.world.pos_y[idx]],
                vel: [ctx.world.vel_x[idx], ctx.world.vel_y[idx]],
                size: [ctx.world.size_w[idx], ctx.world.size_h[idx]],
                rotation: ctx.world.rotation[idx],
                hp: ctx.world.hp[idx],
                tag: ctx.world.tag[idx],
                layer_mask: ctx.world.layer_mask[idx],
                collider_radius: ctx.world.collider_radius[idx],
                flags: ctx.world.flags[idx],
                color_rgba: ctx.world.color_rgba[idx],
                dialogue: state.get_dialogue(id).map(ToOwned::to_owned),
            };
            serde_json::to_value(snap).map_err(|err| RpcErrorObj {
                code: RPC_ERR_INVALID_PARAMS,
                message: format!("failed to serialize entity snapshot: {err}"),
            })
        }

        "fge.entity.spawn" => {
            let p: SpawnParams = parse_params(params)?;
            let w = p.w.unwrap_or(16.0);
            let h = p.h.unwrap_or(16.0);
            if !p.x.is_finite() || !p.y.is_finite() || !w.is_finite() || !h.is_finite() {
                return Err(RpcErrorObj {
                    code: RPC_ERR_INVALID_PARAMS,
                    message: "position and size must be finite".to_owned(),
                });
            }
            if let Some(vel) = p.vel {
                if !vel[0].is_finite() || !vel[1].is_finite() {
                    return Err(RpcErrorObj {
                        code: RPC_ERR_INVALID_PARAMS,
                        message: "velocity must be finite".to_owned(),
                    });
                }
            }
            if let Some(hp) = p.hp {
                if !hp.is_finite() {
                    return Err(RpcErrorObj {
                        code: RPC_ERR_INVALID_PARAMS,
                        message: "hp must be finite".to_owned(),
                    });
                }
            }
            if let Some(r) = p.collider_radius {
                if !r.is_finite() || r < 0.0 {
                    return Err(RpcErrorObj {
                        code: RPC_ERR_INVALID_PARAMS,
                        message: "collider_radius must be finite and >= 0".to_owned(),
                    });
                }
            }

            let color = p.color.unwrap_or([255, 255, 255, 255]);
            let id = ctx.world.spawn_sprite([p.x, p.y], [w, h], color);
            if let Some(vel) = p.vel {
                ctx.world.set_velocity(id, vel);
            }
            if let Some(hp) = p.hp {
                ctx.world.set_hp(id, hp);
            }
            if let Some(tag) = p.tag {
                ctx.world.set_tag(id, tag);
            }
            if p.collider_radius.is_some() || p.layer_mask.is_some() {
                let radius = p.collider_radius.unwrap_or(0.0);
                let mask = p.layer_mask.unwrap_or(0);
                ctx.world.set_collider(id, radius, mask);
            }
            if let Some(dialogue) = p.dialogue {
                state.set_dialogue(id, dialogue);
            }

            Ok(json!({ "entity": id }))
        }

        "fge.entity.despawn" => {
            let p: EntityHandleParams = parse_params(params)?;
            let id = EntityId {
                index: p.index,
                generation: p.generation,
            };
            let despawned = ctx.world.despawn(id);
            if despawned {
                state.dialogues.retain(|(existing, _)| *existing != id);
            }
            Ok(json!({ "despawned": despawned }))
        }

        "fge.entity.patch" => {
            let p: PatchParams = parse_params(params)?;
            let id = EntityId {
                index: p.index,
                generation: p.generation,
            };
            if !ctx.world.is_alive(id) {
                return Err(RpcErrorObj {
                    code: RPC_ERR_ENTITY_NOT_FOUND,
                    message: format!("entity not alive: ({}, {})", id.index, id.generation),
                });
            }
            if let Some(pos) = p.pos {
                if !pos[0].is_finite() || !pos[1].is_finite() {
                    return Err(RpcErrorObj {
                        code: RPC_ERR_INVALID_PARAMS,
                        message: "pos must be finite".to_owned(),
                    });
                }
            }
            if let Some(vel) = p.vel {
                if !vel[0].is_finite() || !vel[1].is_finite() {
                    return Err(RpcErrorObj {
                        code: RPC_ERR_INVALID_PARAMS,
                        message: "vel must be finite".to_owned(),
                    });
                }
            }
            if let Some(size) = p.size {
                if !size[0].is_finite() || !size[1].is_finite() {
                    return Err(RpcErrorObj {
                        code: RPC_ERR_INVALID_PARAMS,
                        message: "size must be finite".to_owned(),
                    });
                }
            }
            if let Some(hp) = p.hp {
                if !hp.is_finite() {
                    return Err(RpcErrorObj {
                        code: RPC_ERR_INVALID_PARAMS,
                        message: "hp must be finite".to_owned(),
                    });
                }
            }

            if let Some(pos) = p.pos {
                ctx.world.set_position(id, pos);
            }
            if let Some(vel) = p.vel {
                ctx.world.set_velocity(id, vel);
            }
            if let Some(size) = p.size {
                ctx.world.set_size(id, size);
            }
            if let Some(hp) = p.hp {
                ctx.world.set_hp(id, hp);
            }
            if let Some(tag) = p.tag {
                ctx.world.set_tag(id, tag);
            }
            if let Some(color) = p.color {
                ctx.world.set_color(id, color);
            }
            if let Some(flags) = p.flags {
                ctx.world.set_flags(id, flags);
            }
            if let Some(dialogue) = p.dialogue {
                state.set_dialogue(id, dialogue);
            }

            Ok(json!({ "updated": true }))
        }

        "fge.input.inject" => {
            let p: InputInjectParams = parse_params(params)?;
            let Some(key) = FgeKey::from_u32(p.key) else {
                return Err(RpcErrorObj {
                    code: RPC_ERR_INVALID_PARAMS,
                    message: format!("invalid FgeKey code: {}", p.key),
                });
            };
            ctx.input.set_key_state(key, p.pressed);
            Ok(json!({ "ok": true, "key": p.key, "pressed": p.pressed }))
        }

        "fge.event.emit" => {
            let p: EventEmitParams = parse_params(params)?;
            let arg0 = p.arg0.unwrap_or(0.0);
            let arg1 = p.arg1.unwrap_or(0.0);
            if !arg0.is_finite() || !arg1.is_finite() {
                return Err(RpcErrorObj {
                    code: RPC_ERR_INVALID_PARAMS,
                    message: "event args must be finite".to_owned(),
                });
            }
            let target = p.target.unwrap_or(EntityId::INVALID);
            ctx.push_ai_event(FgeAiEvent {
                kind: p.kind,
                target,
                arg0,
                arg1,
            });
            Ok(json!({ "queued": true }))
        }

        "fge.param.set" => {
            let p: ParamSetParams = parse_params(params)?;
            if !p.value.is_finite() {
                return Err(RpcErrorObj {
                    code: RPC_ERR_INVALID_PARAMS,
                    message: "param value must be finite".to_owned(),
                });
            }
            ctx.set_param(p.id, p.value);
            Ok(json!({ "id": p.id, "value": p.value }))
        }

        "fge.param.get" => {
            let p: ParamGetParams = parse_params(params)?;
            let default_val = p.default.unwrap_or(0.0);
            if !default_val.is_finite() {
                return Err(RpcErrorObj {
                    code: RPC_ERR_INVALID_PARAMS,
                    message: "default param value must be finite".to_owned(),
                });
            }
            let value = ctx.get_param(p.id, default_val);
            Ok(json!({ "id": p.id, "value": value }))
        }

        "fge.camera.set" => {
            let p: CameraSetParams = parse_params(params)?;
            if !p.center_x.is_finite()
                || !p.center_y.is_finite()
                || !p.zoom.is_finite()
                || p.zoom <= 0.0
            {
                return Err(RpcErrorObj {
                    code: RPC_ERR_INVALID_PARAMS,
                    message: "camera center must be finite and zoom must be finite and > 0"
                        .to_owned(),
                });
            }
            ctx.camera.center = Vec2::new(p.center_x, p.center_y);
            ctx.camera.zoom = p.zoom;
            Ok(json!({
                "center": [p.center_x, p.center_y],
                "zoom": p.zoom,
            }))
        }

        _ => Err(RpcErrorObj {
            code: RPC_ERR_METHOD_NOT_FOUND,
            message: format!("method not found: {method}"),
        }),
    }
}

/// Parses and dispatches a single JSON-RPC 2.0 request line against `ctx` and `state`,
/// returning a single-line JSON response string without a trailing newline.
pub fn handle_rpc_line(line: &str, ctx: &mut EngineContext, state: &mut BridgeState) -> String {
    let req: RpcRequest = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(err) => {
            let resp = RpcResponse::err(Value::Null, RPC_ERR_PARSE, format!("parse error: {err}"));
            return serialize_response(&resp);
        }
    };

    let id = req.id.unwrap_or(Value::Null);
    let resp = match dispatch_method(req.method.as_str(), req.params, ctx, state) {
        Ok(result) => RpcResponse::ok(id, result),
        Err(error) => RpcResponse::err(id, error.code, error.message),
    };
    serialize_response(&resp)
}

fn serialize_response(resp: &RpcResponse) -> String {
    serde_json::to_string(resp).unwrap_or_else(|_| {
        r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"serialization error"}}"#
            .to_owned()
    })
}
