use std::net::{SocketAddr, TcpListener, TcpStream};

use fge_abi::context::EngineContext;

use crate::{error::BridgeError, protocol::BridgeState};

#[allow(dead_code)]
struct ClientConn {
    stream: TcpStream,
    read_buf: Vec<u8>,
}

/// Single-threaded, non-blocking TCP NDJSON-RPC 2.0 server for live AI observation and intervention.
pub struct BridgeServer {
    #[allow(dead_code)]
    listener: TcpListener,
    #[allow(dead_code)]
    clients: Vec<ClientConn>,
    state: BridgeState,
}

impl BridgeServer {
    /// Binds a non-blocking TCP listener at `addr` (e.g. `"127.0.0.1:7777"` or `"127.0.0.1:0"`).
    #[allow(unused_variables)]
    pub fn bind(addr: &str) -> Result<Self, BridgeError> {
        todo!()
    }

    /// Returns the local socket address bound by this server.
    pub fn local_addr(&self) -> Result<SocketAddr, BridgeError> {
        todo!()
    }

    /// Returns the number of currently connected TCP bridge clients.
    pub fn client_count(&self) -> usize {
        todo!()
    }

    /// Immutable reference to the per-bridge state (NPC dialogues, etc.).
    pub fn state(&self) -> &BridgeState {
        &self.state
    }

    /// Mutable reference to the per-bridge state (NPC dialogues, etc.).
    pub fn state_mut(&mut self) -> &mut BridgeState {
        &mut self.state
    }

    /// Accepts pending connections, processes all complete newline-delimited JSON-RPC requests
    /// without blocking, prunes dead entity metadata, and returns total processed RPC count.
    #[allow(unused_variables)]
    pub fn poll_frame(&mut self, ctx: &mut EngineContext) -> usize {
        todo!()
    }
}
