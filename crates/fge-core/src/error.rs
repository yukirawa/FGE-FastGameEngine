use thiserror::Error;

/// Errors produced by `fge-core` math, spatial grid, and simulation primitives.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CoreError {
    #[error("invalid dimensions: width={width}, height={height} (must be finite and > 0)")]
    InvalidDimensions { width: f32, height: f32 },

    #[error("invalid camera zoom: {zoom} (must be finite and > 0)")]
    InvalidZoom { zoom: f32 },

    #[error("invalid delta time: {dt} (must be finite and >= 0)")]
    InvalidDeltaTime { dt: f32 },

    #[error("invalid size range: min={min}, max={max} (must satisfy 0 < min <= max and finite)")]
    InvalidSizeRange { min: f32, max: f32 },

    #[error("invalid speed parameter: {value} (must be finite and >= 0)")]
    InvalidSpeed { value: f32 },

    #[error("invalid spatial grid cell size: {cell_size} (must be finite and > 0)")]
    InvalidCellSize { cell_size: f32 },

    #[error("invalid spatial grid bucket count: {buckets} (must be a non-zero power of two)")]
    InvalidBucketCount { buckets: usize },
}
