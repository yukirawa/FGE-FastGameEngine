use std::sync::mpsc::Receiver;

use crate::{
    synth::{BgmPreset, BgmSequencer, SfxPreset, SynthParams, SynthVoice, sfx_preset_params},
    wav::AudioClip,
};

const MAX_ACTIVE_CLIPS: usize = 64;
const MAX_ACTIVE_VOICES: usize = 64;

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

#[derive(Copy, Clone, Debug)]
struct ActiveClip {
    clip_index: usize,
    frame_pos: usize,
    volume: f32,
}

/// Exclusive audio-thread mixer state (ZERO `Arc<Mutex<T>>`).
pub struct MixerState {
    pub sample_rate: u32,
    pub master_volume: f32,
    pub receiver: Option<Receiver<AudioCommand>>,
    clips: Vec<Option<AudioClip>>,
    active_clips: Vec<ActiveClip>,
    active_voices: Vec<SynthVoice>,
    bgm: BgmSequencer,
}

impl MixerState {
    pub fn new(sample_rate: u32, receiver: Option<Receiver<AudioCommand>>) -> Self {
        let rate = sample_rate.max(1);
        Self {
            sample_rate: rate,
            master_volume: 1.0,
            receiver,
            clips: Vec::new(),
            active_clips: Vec::with_capacity(16),
            active_voices: Vec::with_capacity(16),
            bgm: BgmSequencer::new(rate),
        }
    }

    pub fn apply_command(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::SetMasterVolume(volume) => {
                if volume.is_finite() {
                    self.master_volume = volume.clamp(0.0, 2.0);
                }
            }
            AudioCommand::RegisterClip { index, clip } => {
                if index >= self.clips.len() {
                    self.clips.resize(index + 1, None);
                }
                self.clips[index] = Some(clip);
            }
            AudioCommand::PlayClip { index, volume } => {
                if !volume.is_finite() || volume <= 0.0 {
                    return;
                }
                let has_frames = self
                    .clips
                    .get(index)
                    .and_then(|slot| slot.as_ref())
                    .is_some_and(|clip| !clip.frames.is_empty());
                if !has_frames {
                    return;
                }
                if self.active_clips.len() >= MAX_ACTIVE_CLIPS {
                    self.active_clips.remove(0);
                }
                self.active_clips.push(ActiveClip {
                    clip_index: index,
                    frame_pos: 0,
                    volume: volume.clamp(0.0, 2.0),
                });
            }
            AudioCommand::PlayTone { params } => {
                self.push_voice(SynthVoice::new(params));
            }
            AudioCommand::PlaySfx {
                preset,
                volume,
                pitch_scale,
            } => {
                for params in sfx_preset_params(preset, volume, pitch_scale) {
                    self.push_voice(SynthVoice::new(params));
                }
            }
            AudioCommand::PlayBgm { preset, volume } => {
                self.bgm.set_preset(preset, volume);
            }
            AudioCommand::StopBgm => {
                self.bgm.stop();
            }
        }
    }

    fn push_voice(&mut self, voice: SynthVoice) {
        if self.active_voices.len() >= MAX_ACTIVE_VOICES {
            self.active_voices.remove(0);
        }
        self.active_voices.push(voice);
    }

    /// Drains pending `AudioCommand`s and mixes active clips, synth voices, and BGM into `out`.
    pub fn mix_interleaved_f32(&mut self, out: &mut [f32], channels: usize) {
        if let Some(rx) = self.receiver.take() {
            while let Ok(cmd) = rx.try_recv() {
                self.apply_command(cmd);
            }
            self.receiver = Some(rx);
        }

        if channels == 0 || out.is_empty() {
            return;
        }

        let frame_count = out.len() / channels;
        let sample_rate = self.sample_rate;
        let master = self.master_volume;

        for frame_idx in 0..frame_count {
            let mut left = 0.0_f32;
            let mut right = 0.0_f32;

            // Mix active WAV clips.
            let mut i = 0;
            while i < self.active_clips.len() {
                let active = &mut self.active_clips[i];
                let mut finished = true;
                if let Some(Some(clip)) = self.clips.get(active.clip_index) {
                    if let Some(&[cl, cr]) = clip.frames.get(active.frame_pos) {
                        left += cl * active.volume;
                        right += cr * active.volume;
                        active.frame_pos += 1;
                        finished = active.frame_pos >= clip.frames.len();
                    }
                }
                if finished {
                    self.active_clips.swap_remove(i);
                } else {
                    i += 1;
                }
            }

            // Mix active procedural synth voices.
            let mut v = 0;
            while v < self.active_voices.len() {
                if let Some(sample) = self.active_voices[v].next_sample(sample_rate) {
                    left += sample;
                    right += sample;
                    v += 1;
                } else {
                    self.active_voices.swap_remove(v);
                }
            }

            // Mix looping BGM sequencer.
            let [bl, br] = self.bgm.next_stereo_sample();
            left = ((left + bl) * master).clamp(-1.0, 1.0);
            right = ((right + br) * master).clamp(-1.0, 1.0);

            let base = frame_idx * channels;
            if channels == 1 {
                out[base] = 0.5 * (left + right);
            } else {
                out[base] = left;
                out[base + 1] = right;
                for extra in 2..channels {
                    out[base + extra] = 0.0;
                }
            }
        }
    }
}
