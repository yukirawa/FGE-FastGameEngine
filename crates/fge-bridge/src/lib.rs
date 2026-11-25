//! `fge-bridge`: Unified JSON-RPC 2.0 AI bridge, world snapshot serialization, and headless test runner.

pub mod error;
pub mod headless;
pub mod protocol;
pub mod server;
pub mod snapshot;

#[cfg(test)]
mod tests;
