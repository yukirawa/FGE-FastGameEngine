use thiserror::Error;

/// Errors produced by the `fge-render` GPU backend.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("core error: {0}")]
    Core(#[from] fge_core::error::CoreError),

    #[error("invalid render resolution: {width}x{height} (must be > 0)")]
    InvalidResolution { width: u32, height: u32 },

    #[error("failed to create surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),

    #[error("failed to request GPU adapter: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),

    #[error("failed to request GPU device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),

    #[error("surface configuration unsupported by adapter")]
    UnsupportedSurface,

    #[error("fatal surface acquisition error: {0}")]
    SurfaceFatal(&'static str),

    #[error("gpu poll error: {0}")]
    Poll(#[from] wgpu::PollError),
}
