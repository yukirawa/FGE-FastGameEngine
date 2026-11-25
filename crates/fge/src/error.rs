use thiserror::Error;

/// Top-level application and CLI errors for the `fge` runner.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("core error: {0}")]
    Core(#[from] fge_core::error::CoreError),

    #[error("render error: {0}")]
    Render(#[from] fge_render::error::RenderError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse game.toml: {0}")]
    ManifestParse(#[from] toml::de::Error),

    #[error("invalid game.toml configuration: {reason}")]
    InvalidManifest { reason: String },

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("windowing event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    #[error("OS window creation error: {0}")]
    WindowOs(#[from] winit::error::OsError),
}
