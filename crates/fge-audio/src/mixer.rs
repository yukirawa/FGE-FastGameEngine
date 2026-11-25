use std::sync::mpsc::Receiver;

use crate::{
    synth::{BgmPreset, SfxPreset, SynthParams},
    wav::AudioClip,
};

/// Single-producer lock-free command sent from [`crate::engine::AudioEngine`] to [`MixerState`].
#[derive(Debug)]
pub enum AudioCommand {
    SetMasterVolume(f32),
    RegisterClip {
        index: usize,
        clip: AudioClip,
    },
    PlayClip {
        index: usize,
        volume: f32,
    },
    PlayTone {
        params: SynthParams,
    },
    PlaySfx {
        preset: SfxPreset,
        volume: f32,
        pitch_scale: f32,
    },
    PlayBgm {
        preset: BgmPreset,
        volume: f32,
    },
    StopBgm,
}

/// Exclusive audio-thread mixer state (ZERO `Arc<Mutex<T>>`).
pub struct MixerState {
    pub sample_rate: u32,
    pub master_volume: f32,
    pub receiver: Option<Receiver<AudioCommand>>,
}

impl MixerState {
    pub fn new(_sample_rate: u32, _receiver: Option<Receiver<AudioCommand>>) -> Self {
        todo!("implemented by impl-audio")
    }

    pub fn apply_command(&mut self, _cmd: AudioCommand) {
        todo!("implemented by impl-audio")
    }

    /// Drains pending `AudioCommand`s and mixes active clips, synth voices, and BGM into `out`.
    pub fn mix_interleaved_f32(&mut self, _out: &mut [f32], _channels: usize) {
        todo!("implemented by impl-audio")
    }
}
