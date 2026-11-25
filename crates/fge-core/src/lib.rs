//! `fge-core`: Data-Oriented Design (SoA) simulation, generational ECS world,
//! spatial hash grid, 2D camera math, input state, and frame telemetry.

pub mod camera;
pub mod emitter;
pub mod error;
pub mod handle;
pub mod input_core;
pub mod sim;
pub mod spatial;
pub mod stats;
pub mod ui;
pub mod world;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod phase2_tests;

#[cfg(test)]
mod phase5_tests;
