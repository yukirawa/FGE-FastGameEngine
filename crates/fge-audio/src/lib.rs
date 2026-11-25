//! `fge-audio`: Lock-free `cpal` audio mixer, WAV loader, and chiptune SFX/BGM synthesizer.

pub mod engine;
pub mod error;
pub mod mixer;
pub mod synth;
pub mod wav;

#[cfg(test)]
mod tests;
