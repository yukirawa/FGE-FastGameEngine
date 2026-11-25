use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};

use fge_abi::api::{ensure_abi_symbols_linked, fge_get_ai_event, fge_poll_ai_events};
use fge_abi::context::{with_active_context, EngineContext};
use fge_core::handle::EntityId;
use fge_core::input_core::FgeKey;

use crate::headless::{run_headless_test, HeadlessConfig};
use crate::protocol::{handle_rpc_line, BridgeState, RpcResponse, RPC_ERR_UNAUTHORIZED};
use crate::server::BridgeServer;
use crate::snapshot::{EntitySnapshot, WorldSnapshot};

static NEXT_TEST_DIR_ID: AtomicU64 = AtomicU64::new(1);

/// RAII temporary directory under `std::env::temp_dir()` cleaned up on drop.
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
        let dir_name = format!(
            "fge_bridge_test_{}_{}_{}_{}",
            prefix,
            std::process::id(),
            id,
            nanos
        );
        let path = std::env::temp_dir().join(dir_name);
        fs::create_dir_all(&path).expect("create temp test dir");
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
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, content).expect("write test file");
}

fn parse_rpc_response(line: &str) -> RpcResponse {
    assert!(
        !line.contains('\n') && !line.contains('\r'),
        "RPC response must be single-line NDJSON without trailing newline: {line:?}"
    );
    serde_json::from_str::<RpcResponse>(line).expect("valid RpcResponse JSON")
}

#[test]
fn protocol_rpc_entity_lifecycle_and_errors() {
    let mut ctx = EngineContext::new(800.0, 600.0, 42).expect("create EngineContext");
    let mut state = BridgeState::new();

    // 1. fge.ping
    let ping = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":1,"method":"fge.ping","params":{}}"#,
        &mut ctx,
        &mut state,
    ));
    assert_eq!(ping.jsonrpc, "2.0");
    assert_eq!(ping.id, serde_json::json!(1));
    assert!(ping.error.is_none(), "ping failed: {:?}", ping.error);
    let ping_res = ping.result.expect("ping result");
    assert_eq!(ping_res["ok"], serde_json::json!(true));
    assert_eq!(ping_res["frame"], serde_json::json!(0));
    assert_eq!(ping_res["entities"], serde_json::json!(0));

    // 2. fge.entity.spawn (with optional velocity, hp, tag, collider, layer, dialogue)
    let spawn_req = r#"{"jsonrpc":"2.0","id":2,"method":"fge.entity.spawn","params":{"x":100.0,"y":200.0,"w":32.0,"h":48.0,"color":[10,20,30,255],"vel":[5.0,-3.0],"hp":80.0,"tag":7,"collider_radius":14.0,"layer_mask":2,"dialogue":"Hello traveler!"}}"#;
    let spawn = parse_rpc_response(&handle_rpc_line(spawn_req, &mut ctx, &mut state));
    assert!(spawn.error.is_none(), "spawn failed: {:?}", spawn.error);
    let spawn_res = spawn.result.expect("spawn result");
    let npc_id: EntityId =
        serde_json::from_value(spawn_res["entity"].clone()).expect("EntityId in spawn result");
    assert!(ctx.world.is_alive(npc_id));
    assert_eq!(ctx.world.len(), 1);
    assert_eq!(state.get_dialogue(npc_id), Some("Hello traveler!"));

    // Spawn a distant second entity on another layer to verify query_nearby filtering.
    let far_req = r#"{"jsonrpc":"2.0","id":3,"method":"fge.entity.spawn","params":{"x":700.0,"y":500.0,"layer_mask":4,"collider_radius":10.0}}"#;
    let far_resp = parse_rpc_response(&handle_rpc_line(far_req, &mut ctx, &mut state));
    assert!(far_resp.error.is_none());
    let far_id: EntityId =
        serde_json::from_value(far_resp.result.expect("far result")["entity"].clone())
            .expect("far EntityId");
    assert_eq!(ctx.world.len(), 2);

    // 3. fge.entity.get
    let get_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "fge.entity.get",
        "params": { "index": npc_id.index, "generation": npc_id.generation }
    })
    .to_string();
    let get_resp = parse_rpc_response(&handle_rpc_line(&get_req, &mut ctx, &mut state));
    assert!(get_resp.error.is_none(), "get failed: {:?}", get_resp.error);
    let snap: EntitySnapshot =
        serde_json::from_value(get_resp.result.expect("get result")).expect("EntitySnapshot");
    assert_eq!(snap.id, npc_id);
    assert_eq!(snap.pos, [100.0, 200.0]);
    assert_eq!(snap.vel, [5.0, -3.0]);
    assert_eq!(snap.size, [32.0, 48.0]);
    assert!((snap.hp - 80.0).abs() < 1e-4);
    assert_eq!(snap.tag, 7);
    assert!((snap.collider_radius - 14.0).abs() < 1e-4);
    assert_eq!(snap.layer_mask, 2);
    assert_eq!(snap.color_rgba, [10, 20, 30, 255]);
    assert_eq!(snap.dialogue.as_deref(), Some("Hello traveler!"));

    // 4. fge.entity.patch (velocity, HP, NPC dialogue)
    let patch_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "fge.entity.patch",
        "params": {
            "index": npc_id.index,
            "generation": npc_id.generation,
            "vel": [12.0, 18.0],
            "hp": 42.5,
            "dialogue": "Stay safe!"
        }
    })
    .to_string();
    let patch_resp = parse_rpc_response(&handle_rpc_line(&patch_req, &mut ctx, &mut state));
    assert!(
        patch_resp.error.is_none(),
        "patch failed: {:?}",
        patch_resp.error
    );
    assert_eq!(
        patch_resp.result.expect("patch result")["updated"],
        serde_json::json!(true)
    );

    let get_after_patch = parse_rpc_response(&handle_rpc_line(&get_req, &mut ctx, &mut state));
    let patched_snap: EntitySnapshot =
        serde_json::from_value(get_after_patch.result.expect("get after patch"))
            .expect("patched EntitySnapshot");
    assert_eq!(patched_snap.vel, [12.0, 18.0]);
    assert!((patched_snap.hp - 42.5).abs() < 1e-4);
    assert_eq!(patched_snap.dialogue.as_deref(), Some("Stay safe!"));

    // 5. fge.world.query_nearby
    let nearby_all = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":6,"method":"fge.world.query_nearby","params":{"x":110.0,"y":205.0,"radius":50.0,"layer_mask":0}}"#,
        &mut ctx,
        &mut state,
    ));
    assert!(
        nearby_all.error.is_none(),
        "query_nearby failed: {:?}",
        nearby_all.error
    );
    let nearby_list: Vec<EntitySnapshot> =
        serde_json::from_value(nearby_all.result.expect("nearby result"))
            .expect("Vec<EntitySnapshot>");
    assert_eq!(nearby_list.len(), 1);
    assert_eq!(nearby_list[0].id, npc_id);

    let nearby_other_layer = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":7,"method":"fge.world.query_nearby","params":{"x":110.0,"y":205.0,"radius":50.0,"layer_mask":8}}"#,
        &mut ctx,
        &mut state,
    ));
    let empty_list: Vec<EntitySnapshot> =
        serde_json::from_value(nearby_other_layer.result.expect("nearby filtered result"))
            .expect("Vec<EntitySnapshot>");
    assert!(empty_list.is_empty());

    // 6. fge.entity.despawn -> stale handle returns JSON-RPC error -32001
    let despawn_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "fge.entity.despawn",
        "params": { "index": npc_id.index, "generation": npc_id.generation }
    })
    .to_string();
    let despawn_resp = parse_rpc_response(&handle_rpc_line(&despawn_req, &mut ctx, &mut state));
    assert!(despawn_resp.error.is_none());
    assert_eq!(
        despawn_resp.result.expect("despawn result")["despawned"],
        serde_json::json!(true)
    );
    assert!(!ctx.world.is_alive(npc_id));
    assert!(ctx.world.is_alive(far_id));

    state.prune_dead(&ctx.world);
    assert!(state.get_dialogue(npc_id).is_none());

    let stale_get = parse_rpc_response(&handle_rpc_line(&get_req, &mut ctx, &mut state));
    assert_eq!(
        stale_get.error.as_ref().map(|e| e.code),
        Some(-32001),
        "stale fge.entity.get must return -32001, got: {stale_get:?}"
    );

    let stale_patch = parse_rpc_response(&handle_rpc_line(&patch_req, &mut ctx, &mut state));
    assert_eq!(
        stale_patch.error.as_ref().map(|e| e.code),
        Some(-32001),
        "stale fge.entity.patch must return -32001, got: {stale_patch:?}"
    );

    // 7. Malformed JSON (-32700) & unknown method (-32601)
    let parse_err = parse_rpc_response(&handle_rpc_line("{not valid json", &mut ctx, &mut state));
    assert_eq!(
        parse_err.error.as_ref().map(|e| e.code),
        Some(-32700),
        "malformed JSON must return -32700, got: {parse_err:?}"
    );

    let method_err = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":9,"method":"fge.does.not.exist","params":{}}"#,
        &mut ctx,
        &mut state,
    ));
    assert_eq!(
        method_err.error.as_ref().map(|e| e.code),
        Some(-32601),
        "unknown method must return -32601, got: {method_err:?}"
    );
}

#[test]
fn protocol_rpc_params_events_input_and_camera() {
    let mut ctx = EngineContext::new(1280.0, 720.0, 99).expect("create EngineContext");
    let mut state = BridgeState::new();

    // fge.param.set / fge.param.get
    let set_param = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":10,"method":"fge.param.set","params":{"id":10,"value":3.5}}"#,
        &mut ctx,
        &mut state,
    ));
    assert!(
        set_param.error.is_none(),
        "param.set failed: {:?}",
        set_param.error
    );
    assert!((ctx.get_param(10, 0.0) - 3.5).abs() < 1e-5);

    let get_param = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":11,"method":"fge.param.get","params":{"id":10,"default":1.0}}"#,
        &mut ctx,
        &mut state,
    ));
    assert!(get_param.error.is_none());
    let param_val = get_param.result.expect("param.get result")["value"]
        .as_f64()
        .expect("numeric param value");
    assert!((param_val - 3.5).abs() < 1e-5);

    let get_default = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":12,"method":"fge.param.get","params":{"id":999,"default":7.25}}"#,
        &mut ctx,
        &mut state,
    ));
    assert!(get_default.error.is_none());
    let default_val = get_default.result.expect("param.get default result")["value"]
        .as_f64()
        .expect("numeric default param value");
    assert!((default_val - 7.25).abs() < 1e-5);

    // fge.event.emit verified via ctx and C-ABI fge_poll_ai_events / fge_get_ai_event
    let emit_req = r#"{"jsonrpc":"2.0","id":13,"method":"fge.event.emit","params":{"kind":3,"target":{"index":4,"generation":1},"arg0":12.5,"arg1":-4.0}}"#;
    let emit_resp = parse_rpc_response(&handle_rpc_line(emit_req, &mut ctx, &mut state));
    assert!(
        emit_resp.error.is_none(),
        "event.emit failed: {:?}",
        emit_resp.error
    );
    assert_eq!(ctx.pending_ai_events.len(), 1);

    let (polled_count, polled_ev) = with_active_context(&mut ctx, || {
        let n = fge_poll_ai_events();
        let ev = fge_get_ai_event(0);
        (n, ev)
    });
    assert_eq!(polled_count, 1);
    assert_eq!(polled_ev.kind, 3);
    assert_eq!(
        polled_ev.target,
        EntityId {
            index: 4,
            generation: 1
        }
    );
    assert!((polled_ev.arg0 - 12.5).abs() < 1e-5);
    assert!((polled_ev.arg1 + 4.0).abs() < 1e-5);

    // fge.input.inject
    let key_down_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "fge.input.inject",
        "params": { "key": FgeKey::Space as u32, "pressed": true }
    })
    .to_string();
    let key_down = parse_rpc_response(&handle_rpc_line(&key_down_req, &mut ctx, &mut state));
    assert!(
        key_down.error.is_none(),
        "input.inject down failed: {:?}",
        key_down.error
    );
    assert!(ctx.input.is_key_down(FgeKey::Space));
    assert!(ctx.input.is_key_pressed(FgeKey::Space));

    let key_up_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 15,
        "method": "fge.input.inject",
        "params": { "key": FgeKey::Space as u32, "pressed": false }
    })
    .to_string();
    let key_up = parse_rpc_response(&handle_rpc_line(&key_up_req, &mut ctx, &mut state));
    assert!(key_up.error.is_none());
    assert!(!ctx.input.is_key_down(FgeKey::Space));
    assert!(ctx.input.is_key_released(FgeKey::Space));

    let bad_key = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":16,"method":"fge.input.inject","params":{"key":9999,"pressed":true}}"#,
        &mut ctx,
        &mut state,
    ));
    assert_eq!(bad_key.error.as_ref().map(|e| e.code), Some(-32602));

    // fge.camera.set
    let cam_resp = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":17,"method":"fge.camera.set","params":{"center_x":320.0,"center_y":240.0,"zoom":2.0}}"#,
        &mut ctx,
        &mut state,
    ));
    assert!(
        cam_resp.error.is_none(),
        "camera.set failed: {:?}",
        cam_resp.error
    );
    assert!((ctx.camera.center.x - 320.0).abs() < 1e-4);
    assert!((ctx.camera.center.y - 240.0).abs() < 1e-4);
    assert!((ctx.camera.zoom - 2.0).abs() < 1e-4);

    // fge.world.snapshot reflects camera & params
    let snap_resp = parse_rpc_response(&handle_rpc_line(
        r#"{"jsonrpc":"2.0","id":18,"method":"fge.world.snapshot","params":{}}"#,
        &mut ctx,
        &mut state,
    ));
    assert!(snap_resp.error.is_none());
    let world_snap: WorldSnapshot =
        serde_json::from_value(snap_resp.result.expect("snapshot result"))
            .expect("WorldSnapshot");
    assert_eq!(world_snap.camera.center, [320.0, 240.0]);
    assert!((world_snap.camera.zoom - 2.0).abs() < 1e-4);
    assert!(world_snap
        .params
        .iter()
        .any(|&(id, val)| id == 10 && (val - 3.5).abs() < 1e-5));
}

#[test]
fn bridge_server_live_tcp_loopback_partial_writes() {
    let mut server = BridgeServer::bind("127.0.0.1:0").expect("bind loopback BridgeServer");
    let addr = server.local_addr().expect("local_addr");
    let mut ctx = EngineContext::new(1280.0, 720.0, 7).expect("create EngineContext");

    let mut client = TcpStream::connect(addr).expect("connect TcpStream to BridgeServer");
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set read timeout");

    let line1 = r#"{"jsonrpc":"2.0","id":1,"method":"fge.entity.spawn","params":{"x":42.0,"y":84.0,"hp":99.0,"dialogue":"Loopback NPC"}}"#;
    let line2 = r#"{"jsonrpc":"2.0","id":2,"method":"fge.world.snapshot","params":{}}"#;
    let full_payload = format!("{line1}\n{line2}\n");
    let bytes = full_payload.as_bytes();

    // Split writes across arbitrary mid-JSON and newline boundaries.
    let split_a = 24;
    let split_b = line1.len() + 15;
    client.write_all(&bytes[..split_a]).expect("write chunk 1");
    client.flush().expect("flush chunk 1");

    for _ in 0..50 {
        let count = server.poll_frame(&mut ctx);
        assert_eq!(count, 0, "incomplete NDJSON line must not dispatch yet");
        if server.client_count() == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(server.client_count(), 1);
    assert!(
        ctx.world.is_empty(),
        "entity must not spawn before newline arrives"
    );

    client
        .write_all(&bytes[split_a..split_b])
        .expect("write chunk 2");
    client.flush().expect("flush chunk 2");
    thread::sleep(Duration::from_millis(2));
    client.write_all(&bytes[split_b..]).expect("write chunk 3");
    client.flush().expect("flush chunk 3");

    let mut total_processed = 0usize;
    for _ in 0..100 {
        total_processed += server.poll_frame(&mut ctx);
        if total_processed >= 2 {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(
        total_processed, 2,
        "expected both NDJSON-RPC lines to be processed"
    );

    let mut reader = BufReader::new(client);
    let mut resp1_line = String::new();
    let mut resp2_line = String::new();
    reader.read_line(&mut resp1_line).expect("read response 1");
    reader.read_line(&mut resp2_line).expect("read response 2");

    let resp1: RpcResponse =
        serde_json::from_str(resp1_line.trim_end()).expect("parse response 1 JSON");
    assert_eq!(resp1.id, serde_json::json!(1));
    assert!(resp1.error.is_none(), "spawn error: {:?}", resp1.error);
    let spawned_id: EntityId =
        serde_json::from_value(resp1.result.expect("spawn result")["entity"].clone())
            .expect("spawned EntityId");

    assert!(ctx.world.is_alive(spawned_id));
    assert_eq!(ctx.world.len(), 1);
    assert!((ctx.world.pos_x[0] - 42.0).abs() < 1e-4);
    assert!((ctx.world.pos_y[0] - 84.0).abs() < 1e-4);
    assert_eq!(server.state().get_dialogue(spawned_id), Some("Loopback NPC"));

    let resp2: RpcResponse =
        serde_json::from_str(resp2_line.trim_end()).expect("parse response 2 JSON");
    assert_eq!(resp2.id, serde_json::json!(2));
    assert!(resp2.error.is_none(), "snapshot error: {:?}", resp2.error);
    let snapshot: WorldSnapshot =
        serde_json::from_value(resp2.result.expect("snapshot result")).expect("WorldSnapshot");
    assert_eq!(snapshot.entity_count, 1);
    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(snapshot.entities[0].id, spawned_id);
    assert_eq!(
        snapshot.entities[0].dialogue.as_deref(),
        Some("Loopback NPC")
    );
}

#[test]
fn bridge_server_live_tcp_api_key_authentication() {
    let mut server = BridgeServer::bind_with_api_key("127.0.0.1:0", Some(String::from("secret-token")))
        .expect("bind authenticated loopback BridgeServer");
    let addr = server.local_addr().expect("local_addr");
    let mut ctx = EngineContext::new(1280.0, 720.0, 13).expect("create EngineContext");

    let mut client = TcpStream::connect(addr).expect("connect TcpStream to BridgeServer");
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set read timeout");
    let mut reader = BufReader::new(client.try_clone().expect("clone TcpStream"));

    let mut send_and_recv = |req_json: &str| -> RpcResponse {
        client
            .write_all(format!("{req_json}\n").as_bytes())
            .expect("write request");
        client.flush().expect("flush request");

        let mut processed = 0usize;
        for _ in 0..100 {
            processed += server.poll_frame(&mut ctx);
            if processed >= 1 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(processed, 1, "expected 1 RPC line processed for {req_json}");

        let mut line = String::new();
        reader.read_line(&mut line).expect("read RPC response");
        parse_rpc_response(line.trim_end())
    };

    // 1. Unauthenticated request must fail with -32003.
    let unauth = send_and_recv(r#"{"jsonrpc":"2.0","id":1,"method":"fge.ping","params":{}}"#);
    assert_eq!(
        unauth.error.as_ref().map(|e| e.code),
        Some(RPC_ERR_UNAUTHORIZED),
        "expected -32003 for unauthenticated ping, got {unauth:?}"
    );
    assert_eq!(RPC_ERR_UNAUTHORIZED, -32003);

    // 2. Wrong api_key on fge.auth must also fail with -32003.
    let wrong_auth = send_and_recv(
        r#"{"jsonrpc":"2.0","id":2,"method":"fge.auth","params":{"api_key":"wrong-token"}}"#,
    );
    assert_eq!(
        wrong_auth.error.as_ref().map(|e| e.code),
        Some(RPC_ERR_UNAUTHORIZED)
    );

    // 3. Per-request params.api_key succeeds without permanently authenticating the session.
    let per_req = send_and_recv(
        r#"{"jsonrpc":"2.0","id":3,"method":"fge.ping","params":{"api_key":"secret-token"}}"#,
    );
    assert!(per_req.error.is_none(), "per-request api_key failed: {per_req:?}");
    assert_eq!(per_req.result.expect("ping result")["ok"], serde_json::json!(true));

    let still_unauth = send_and_recv(r#"{"jsonrpc":"2.0","id":4,"method":"fge.ping","params":{}}"#);
    assert_eq!(
        still_unauth.error.as_ref().map(|e| e.code),
        Some(RPC_ERR_UNAUTHORIZED)
    );

    // 4. fge.auth with the valid token authenticates the TCP connection.
    let auth_ok = send_and_recv(
        r#"{"jsonrpc":"2.0","id":5,"method":"fge.auth","params":{"api_key":"secret-token"}}"#,
    );
    assert!(auth_ok.error.is_none(), "fge.auth failed: {auth_ok:?}");
    assert_eq!(
        auth_ok.result.expect("auth result")["authenticated"],
        serde_json::json!(true)
    );

    // 5. Subsequent calls on the same authenticated TCP connection succeed without api_key.
    let post_auth_ping =
        send_and_recv(r#"{"jsonrpc":"2.0","id":6,"method":"fge.ping","params":{}}"#);
    assert!(post_auth_ping.error.is_none(), "post-auth ping failed: {post_auth_ping:?}");
    assert_eq!(
        post_auth_ping.result.expect("ping result")["ok"],
        serde_json::json!(true)
    );

    let spawn_ok = send_and_recv(
        r#"{"jsonrpc":"2.0","id":7,"method":"fge.entity.spawn","params":{"x":12.0,"y":34.0}}"#,
    );
    assert!(spawn_ok.error.is_none(), "post-auth spawn failed: {spawn_ok:?}");
    assert_eq!(ctx.world.len(), 1);
}

#[test]
fn headless_valid_multifile_project_is_fast_and_deterministic() {
    assert!(ensure_abi_symbols_linked() >= 42);

    let tmp = TempTestDir::new("headless_valid");
    let project_dir = tmp.path().join("game");

    write_text(
        &project_dir.join("src/helper.h"),
        r#"#ifndef HELPER_H
#define HELPER_H
#include "fge.h"
FgeEntity spawn_actor(float x, float y, float vx, uint32_t layer);
#endif
"#,
    );

    write_text(
        &project_dir.join("src/helper.c"),
        r#"#include "helper.h"

FgeEntity spawn_actor(float x, float y, float vx, uint32_t layer) {
    FgeEntity e = fge_spawn_sprite(x, y, 24.0f, 24.0f, fge_rgba(120, 200, 255, 255));
    fge_set_velocity(e, vx, 0.0f);
    fge_set_collider(e, 14.0f, layer);
    return e;
}
"#,
    );

    write_text(
        &project_dir.join("src/main.c"),
        r#"#include "fge.h"
#include "helper.h"

static FgeEntity g_a;
static FgeEntity g_b;
static uint32_t g_total_hits = 0;

void fge_init(void) {
    fge_param_set(1u, 1.5f);
    float speed = fge_param_get(1u, 1.0f) * fge_rand_range(40.0f, 80.0f);
    g_a = spawn_actor(100.0f, 200.0f, speed, 1u);
    g_b = spawn_actor(140.0f, 200.0f, -speed, 2u);
}

void fge_update(float dt) {
    (void)dt;
    uint32_t hits = fge_detect_collisions(1u, 2u);
    g_total_hits += hits;
    fge_log_metric(1u, (float)fge_entity_count());
    fge_log_metric(2u, (float)g_total_hits);
}

void fge_shutdown(void) {
    fge_set_tag(g_a, 10u);
    fge_set_tag(g_b, 20u);
}
"#,
    );

    let cfg = HeadlessConfig {
        project_dir,
        project_title: String::from("Multi-file Headless Test"),
        width: 800,
        height: 600,
        entry: String::from("src/main.c"),
        sources: vec![String::from("src/helper.c")],
        frames: 100,
        seed: 42,
        dt: 1.0 / 60.0,
        snapshot_entity_limit: None,
    };

    let report_a = run_headless_test(&cfg);
    let report_b = run_headless_test(&cfg);

    for report in [&report_a, &report_b] {
        assert_eq!(report.status, "ok", "logs: {:?}", report.logs);
        assert_eq!(report.frames_requested, 100);
        assert_eq!(report.frames_executed, 100);
        assert!(report.compile_error.is_none());
        assert!(
            report.collision_events_total > 0,
            "expected collisions during 100 frames"
        );
        assert!(
            report.sim_timing.total_ms >= 0.0 && report.sim_timing.total_ms < 5000.0,
            "unexpected simulation duration: {:?}",
            report.sim_timing
        );
        let snap = report
            .world_snapshot
            .as_ref()
            .expect("world_snapshot should be Some");
        assert_eq!(snap.frame_count, 100);
        assert_eq!(snap.entity_count, 2);
        assert!(snap
            .params
            .iter()
            .any(|&(id, val)| id == 1 && (val - 1.5).abs() < 1e-5));
        assert!(snap
            .metrics
            .iter()
            .any(|&(id, val)| id == 1 && (val - 2.0).abs() < 1e-5));
    }

    assert_eq!(
        report_a.world_snapshot, report_b.world_snapshot,
        "same seed must produce identical world snapshots"
    );
    assert_eq!(
        report_a.collision_events_total,
        report_b.collision_events_total
    );
    assert_eq!(report_a.collision_samples, report_b.collision_samples);
}

#[test]
fn headless_broken_c_syntax_reports_compile_error() {
    let tmp = TempTestDir::new("headless_compile_err");
    let project_dir = tmp.path().join("broken_game");

    write_text(
        &project_dir.join("src/main.c"),
        r#"#include "fge.h"
void fge_init(void) {
    int syntax_error_here = ;
}
void fge_update(float dt) { (void)dt; }
"#,
    );

    let cfg = HeadlessConfig {
        project_dir,
        project_title: String::from("Broken C Project"),
        width: 800,
        height: 600,
        entry: String::from("src/main.c"),
        sources: Vec::new(),
        frames: 100,
        seed: 42,
        dt: 1.0 / 60.0,
        snapshot_entity_limit: None,
    };

    let report = run_headless_test(&cfg);
    assert_eq!(report.status, "compile_error");
    assert_eq!(report.frames_executed, 0);
    assert!(report.world_snapshot.is_none());
    let diag = report
        .compile_error
        .as_ref()
        .expect("compile_error diagnostic must be populated");
    assert!(!diag.compiler.is_empty());
    assert!(
        !diag.stderr.trim().is_empty(),
        "compiler stderr must be non-empty"
    );
}
