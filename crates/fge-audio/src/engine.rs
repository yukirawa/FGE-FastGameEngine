use std::{path::Path, sync::mpsc::SyncSender};

use fge_core::handle::{EntityId, GenerationalArena};

use crate::{
    error::AudioError,
    mixer::{AudioCommand, MixerState},
    synth::{BgmPreset, SfxPreset, Waveform},
};

/// Generational sound handle (`FgeSound` in `fge.h`).
pub type SoundId = EntityId;

pub const DEFAULT_MIXER_SAMPLE_RATE: u32 = 44_100;

/// Main-thread audio controller communicating with [`MixerState`] over a bounded SPSC `sync_channel`.
///
/// Automatically falls back to headless silent mixing when no `cpal` output device is available.
pub struct AudioEngine {
    #[allow(dead_code)]
    pub(crate) sample_rate: u32,
    #[allow(dead_code)]
    pub(crate) sound_arena: GenerationalArena,
    #[allow(dead_code)]
    pub(crate) sender: Option<SyncSender<AudioCommand>>,
    #[allow(dead_code)]
    pub(crate) headless_mixer: Option<MixerState>,
    #[allow(dead_code)]
    pub(crate) stream: Option<cpal::Stream>,
}

impl AudioEngine {
    /// Attempts to open the default `cpal` output stream; seamlessly falls back to `new_headless()`
    /// when running without audio hardware or in headless test environments.
    pub fn new_live() -> Self {
        todo!("implemented by impl-audio")
    }

    /// Creates a silent, deterministic headless `AudioEngine` retaining `SoundId` validation and
    /// an inspectable [`MixerState`] for unit tests.
    pub fn new_headless() -> Self {
        todo!("implemented by impl-audio")
    }

    pub fn is_headless(&self) -> bool {
        todo!("implemented by impl-audio")
    }

    pub fn sample_rate(&self) -> u32 {
        todo!("implemented by impl-audio")
    }

    pub fn set_master_volume(&mut self, _volume: f32) {
        todo!("implemented by impl-audio")
    }

    pub fn load_wav_bytes(&mut self, _bytes: &[u8]) -> Result<SoundId, AudioError> {
        todo!("implemented by impl-audio")
    }

    pub fn load_wav_file(&mut self, _path: &Path) -> Result<SoundId, AudioError> {
        todo!("implemented by impl-audio")
    }

    pub fn play_sound(&mut self, _id: SoundId, _volume: f32) -> bool {
        todo!("implemented by impl-audio")
    }

    pub fn play_sfx(&mut self, _preset: SfxPreset, _volume: f32, _pitch_scale: f32) {
        todo!("implemented by impl-audio")
    }

    pub fn play_tone(
        &mut self,
        _waveform: Waveform,
        _freq_start_hz: f32,
        _freq_end_hz: f32,
        _duration_sec: f32,
        _volume: f32,
    ) {
        todo!("implemented by impl-audio")
    }

    pub fn play_bgm_preset(&mut self, _preset: BgmPreset, _volume: f32) {
        todo!("implemented by impl-audio")
    }

    pub fn stop_bgm(&mut self) {
        todo!("implemented by impl-audio")
    }

    /// Renders `frames` stereo `[f32; 2]` samples in headless mode for deterministic verification.
    pub fn render_test_frames(&mut self, _frames: usize) -> Vec<[f32; 2]> {
        todo!("implemented by impl-audio")
    }
}
