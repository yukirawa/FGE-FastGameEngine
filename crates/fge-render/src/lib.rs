//! `fge-render`: Single-draw-call `wgpu` 2D instanced renderer (windowed & offscreen).

pub mod atlas;
pub mod context;
pub mod error;
pub mod instance;
pub mod pipeline;
pub mod world_batch;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod phase2_tests;

#[cfg(test)]
mod phase5_tests;
