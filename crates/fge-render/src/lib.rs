//! `fge-render`: Single-draw-call `wgpu` 2D instanced renderer (windowed & offscreen).

pub mod context;
pub mod error;
pub mod instance;
pub mod pipeline;

#[cfg(test)]
mod tests;
