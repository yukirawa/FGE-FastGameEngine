use thiserror::Error;

/// Errors produced by the `fge-bridge` live server or headless simulation runner.
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("core error: {0}")]
    Core(#[from] fge_core::error::CoreError),

    #[error("loader error: {0}")]
    Loader(#[from] fge_loader::error::LoaderError),
}
