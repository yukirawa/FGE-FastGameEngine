use std::path::Path;

use crate::error::AudioError;

/// Decoded interleaved stereo PCM clip resampled to the mixer's target sample rate.
#[derive(Clone, Debug)]
pub struct AudioClip {
    pub frames: Vec<[f32; 2]>,
    pub sample_rate: u32,
}

/// Parses an in-memory RIFF/WAVE buffer (8-bit PCM, 16-bit PCM, or 32-bit IEEE float, mono or stereo)
/// and linearly resamples it to `target_sample_rate`.
pub fn parse_wav_bytes(_bytes: &[u8], _target_sample_rate: u32) -> Result<AudioClip, AudioError> {
    todo!("implemented by impl-audio")
}

/// Reads and decodes a `.wav` file from disk, resampling to `target_sample_rate`.
pub fn load_wav_file(_path: &Path, _target_sample_rate: u32) -> Result<AudioClip, AudioError> {
    todo!("implemented by impl-audio")
}
