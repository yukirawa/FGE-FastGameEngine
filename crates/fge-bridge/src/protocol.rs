use fge_abi::context::EngineContext;
use fge_core::{handle::EntityId, world::EntityWorld};
use serde::{Deserialize, Serialize};

/// Mutable per-session state maintained by the AI Bridge alongside `EngineContext`.
#[derive(Clone, Debug, Default)]
pub struct BridgeState {
    pub dialogues: Vec<(EntityId, String)>,
}

impl BridgeState {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(unused_variables)]
    pub fn set_dialogue(&mut self, entity: EntityId, text: String) {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn get_dialogue(&self, entity: EntityId) -> Option<&str> {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn prune_dead(&mut self, world: &EntityWorld) {
        todo!()
    }
}

/// Inbound JSON-RPC 2.0 request envelope.
#[derive(Clone, Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: Option<String>,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
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
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErrorObj>,
}

/// Parses and dispatches a single JSON-RPC 2.0 request line against `ctx` and `state`,
/// returning a single-line JSON response string without a trailing newline.
#[allow(unused_variables)]
pub fn handle_rpc_line(line: &str, ctx: &mut EngineContext, state: &mut BridgeState) -> String {
    todo!()
}
