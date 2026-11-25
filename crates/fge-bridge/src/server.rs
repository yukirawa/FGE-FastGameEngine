use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

use fge_abi::context::EngineContext;

use crate::{
    error::BridgeError,
    protocol::{handle_rpc_line_with_auth, BridgeState},
};

const READ_CHUNK_SIZE: usize = 4096;
const MAX_CLIENT_READ_BUF: usize = 1024 * 1024;
const MAX_WRITE_WOULDBLOCK_RETRIES: usize = 64;

struct ClientConn {
    stream: TcpStream,
    read_buf: Vec<u8>,
    authenticated: bool,
}

/// Single-threaded, non-blocking TCP NDJSON-RPC 2.0 server for live AI observation and intervention.
///
/// Runs entirely on the main simulation thread with zero background threads and zero
/// `Arc<Mutex<T>>`, draining pending requests once per frame via [`BridgeServer::poll_frame`].
pub struct BridgeServer {
    listener: TcpListener,
    clients: Vec<ClientConn>,
    state: BridgeState,
}

impl BridgeServer {
    /// Binds a non-blocking TCP listener at `addr` (e.g. `"127.0.0.1:7777"` or `"127.0.0.1:0"`)
    /// without API-key authentication.
    pub fn bind(addr: &str) -> Result<Self, BridgeError> {
        Self::bind_with_api_key(addr, None)
    }

    /// Binds a non-blocking TCP listener at `addr` with optional API-key authentication.
    pub fn bind_with_api_key(addr: &str, api_key: Option<String>) -> Result<Self, BridgeError> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            clients: Vec::new(),
            state: BridgeState::with_api_key(api_key),
        })
    }

    /// Returns the local socket address bound by this server.
    pub fn local_addr(&self) -> Result<SocketAddr, BridgeError> {
        Ok(self.listener.local_addr()?)
    }

    /// Returns the number of currently connected TCP bridge clients.
    pub fn client_count(&self) -> usize {
        self.clients.len()
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
    pub fn poll_frame(&mut self, ctx: &mut EngineContext) -> usize {
        self.accept_pending();

        let mut processed = 0usize;
        let mut idx = 0usize;
        while idx < self.clients.len() {
            let (alive, client_processed) =
                poll_client(&mut self.clients[idx], ctx, &mut self.state);
            processed += client_processed;
            if alive {
                idx += 1;
            } else {
                self.clients.swap_remove(idx);
            }
        }

        self.state.prune_dead(&ctx.world);
        processed
    }

    fn accept_pending(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((stream, _peer)) => {
                    if let Err(err) = stream.set_nonblocking(true) {
                        log::warn!("bridge client set_nonblocking failed: {err}");
                        continue;
                    }
                    let _ = stream.set_nodelay(true);
                    self.clients.push(ClientConn {
                        stream,
                        read_buf: Vec::new(),
                        authenticated: false,
                    });
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => {
                    log::warn!("bridge accept error: {err}");
                    break;
                }
            }
        }
    }
}

/// Reads available bytes from `client`, dispatches every complete `\n`-terminated JSON-RPC line,
/// writes `<resp>\n` back, and returns `(still_alive, processed_count)`.
fn poll_client(
    client: &mut ClientConn,
    ctx: &mut EngineContext,
    state: &mut BridgeState,
) -> (bool, usize) {
    let mut alive = true;
    let mut tmp = [0u8; READ_CHUNK_SIZE];

    loop {
        match client.stream.read(&mut tmp) {
            Ok(0) => {
                alive = false;
                break;
            }
            Ok(n) => {
                if client.read_buf.len().saturating_add(n) > MAX_CLIENT_READ_BUF {
                    log::warn!("bridge client exceeded max read buffer ({MAX_CLIENT_READ_BUF} B)");
                    return (false, 0);
                }
                client.read_buf.extend_from_slice(&tmp[..n]);
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => {
                log::debug!("bridge client read error: {err}");
                alive = false;
                break;
            }
        }
    }

    let mut processed = 0usize;
    while let Some(newline_pos) = client.read_buf.iter().position(|&b| b == b'\n') {
        let raw_line = &client.read_buf[..newline_pos];
        let trimmed_bytes = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);

        let response = if trimmed_bytes.iter().all(|b| b.is_ascii_whitespace()) {
            None
        } else {
            let resp = match std::str::from_utf8(trimmed_bytes) {
                Ok(line_str) => {
                    handle_rpc_line_with_auth(line_str, ctx, state, &mut client.authenticated)
                }
                Err(_) => {
                    r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse error: invalid UTF-8"}}"#
                        .to_owned()
                }
            };
            Some(resp)
        };

        client.read_buf.drain(..=newline_pos);

        if let Some(mut resp_line) = response {
            resp_line.push('\n');
            if !write_nonblocking_all(&mut client.stream, resp_line.as_bytes()) {
                alive = false;
                break;
            }
            processed += 1;
        }
    }

    (alive, processed)
}

fn write_nonblocking_all(stream: &mut TcpStream, mut buf: &[u8]) -> bool {
    let mut would_block_retries = 0usize;
    while !buf.is_empty() {
        match stream.write(buf) {
            Ok(0) => return false,
            Ok(n) => {
                buf = &buf[n..];
                would_block_retries = 0;
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                would_block_retries += 1;
                if would_block_retries >= MAX_WRITE_WOULDBLOCK_RETRIES {
                    log::warn!("bridge client write stalled (WouldBlock); dropping client");
                    return false;
                }
                std::hint::spin_loop();
            }
            Err(err) => {
                log::debug!("bridge client write error: {err}");
                return false;
            }
        }
    }
    true
}
