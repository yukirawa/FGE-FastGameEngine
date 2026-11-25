use std::{
    path::Path,
    sync::mpsc::{self, SyncSender},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use fge_core::handle::{EntityId, GenerationalArena};

use crate::{
    error::AudioError,
    mixer::{AudioCommand, MixerState},
    synth::{BgmPreset, SfxPreset, SynthParams, Waveform},
    wav,
};

/// Generational sound handle (`FgeSound` in `fge.h`).
pub type SoundId = EntityId;

pub const DEFAULT_MIXER_SAMPLE_RATE: u32 = 44_100;
const AUDIO_COMMAND_QUEUE_CAPACITY: usize = 512;

/// Main-thread audio controller communicating with [`MixerState`] over a bounded SPSC `sync_channel`.
///
/// Automatically falls back to headless silent mixing when no `cpal` output device is available.
pub struct AudioEngine {
    pub(crate) sample_rate: u32,
    pub(crate) sound_arena: GenerationalArena,
    pub(crate) sender: Option<SyncSender<AudioCommand>>,
    pub(crate) headless_mixer: Option<MixerState>,
    pub(crate) stream: Option<cpal::Stream>,
}

impl AudioEngine {
    /// Attempts to open the default `cpal` output stream; seamlessly falls back to `new_headless()`
    /// when running without audio hardware or in headless test environments.
    pub fn new_live() -> Self {
        match Self::try_open_live_stream() {
            Ok(engine) => engine,
            Err(err) => {
                log::info!(
                    "fge-audio: live output unavailable ({err}); falling back to headless silent mixer"
                );
                Self::new_headless()
            }
        }
    }

    fn try_open_live_stream() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::DeviceUnavailable)?;
        let supported_config =
            device
                .default_output_config()
                .map_err(|err| AudioError::StreamBuild {
                    reason: err.to_string(),
                })?;

        let sample_rate = supported_config.sample_rate().0.max(8_000);
        let channels = supported_config.channels() as usize;
        let sample_format = supported_config.sample_format();
        let config: cpal::StreamConfig = supported_config.into();

        let (tx, rx) = mpsc::sync_channel::<AudioCommand>(AUDIO_COMMAND_QUEUE_CAPACITY);
        let mut mixer = MixerState::new(sample_rate, Some(rx));

        let err_fn = |err| {
            log::warn!("fge-audio: output stream error: {err}");
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    mixer.mix_interleaved_f32(data, channels);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => {
                let mut scratch = Vec::<f32>::new();
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        scratch.resize(data.len(), 0.0);
                        mixer.mix_interleaved_f32(&mut scratch, channels);
                        for (dst, &src) in data.iter_mut().zip(scratch.iter()) {
                            *dst = (src.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut scratch = Vec::<f32>::new();
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        scratch.resize(data.len(), 0.0);
                        mixer.mix_interleaved_f32(&mut scratch, channels);
                        for (dst, &src) in data.iter_mut().zip(scratch.iter()) {
                            let norm = src.clamp(-1.0, 1.0) * 0.5 + 0.5;
                            *dst = (norm * u16::MAX as f32) as u16;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(AudioError::StreamBuild {
                    reason: format!("unsupported cpal sample format: {other:?}"),
                });
            }
        }
        .map_err(|err| AudioError::StreamBuild {
            reason: err.to_string(),
        })?;

        stream.play().map_err(|err| AudioError::StreamBuild {
            reason: err.to_string(),
        })?;

        log::debug!(
            "fge-audio: opened live cpal output stream ({sample_rate} Hz, {channels} ch, {sample_format:?})"
        );

        Ok(Self {
            sample_rate,
            sound_arena: GenerationalArena::default(),
            sender: Some(tx),
            headless_mixer: None,
            stream: Some(stream),
        })
    }

    /// Creates a silent, deterministic headless `AudioEngine` retaining `SoundId` validation and
    /// an inspectable [`MixerState`] for unit tests.
    pub fn new_headless() -> Self {
        Self {
            sample_rate: DEFAULT_MIXER_SAMPLE_RATE,
            sound_arena: GenerationalArena::default(),
            sender: None,
            headless_mixer: Some(MixerState::new(DEFAULT_MIXER_SAMPLE_RATE, None)),
            stream: None,
        }
    }

    #[inline]
    pub fn is_headless(&self) -> bool {
        self.stream.is_none()
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn dispatch(&mut self, cmd: AudioCommand) {
        if let Some(mixer) = self.headless_mixer.as_mut() {
            mixer.apply_command(cmd);
        } else if let Some(tx) = self.sender.as_ref() {
            if let Err(err) = tx.try_send(cmd) {
                log::debug!("fge-audio: dropped audio command ({err})");
            }
        }
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.dispatch(AudioCommand::SetMasterVolume(volume));
    }

    pub fn load_wav_bytes(&mut self, bytes: &[u8]) -> Result<SoundId, AudioError> {
        let clip = wav::parse_wav_bytes(bytes, self.sample_rate)?;
        let id = self.sound_arena.allocate();
        self.dispatch(AudioCommand::RegisterClip {
            index: id.index as usize,
            clip,
        });
        Ok(id)
    }

    pub fn load_wav_file(&mut self, path: &Path) -> Result<SoundId, AudioError> {
        let clip = wav::load_wav_file(path, self.sample_rate)?;
        let id = self.sound_arena.allocate();
        self.dispatch(AudioCommand::RegisterClip {
            index: id.index as usize,
            clip,
        });
        Ok(id)
    }

    pub fn play_sound(&mut self, id: SoundId, volume: f32) -> bool {
        if self.sound_arena.resolve(id).is_none() {
            return false;
        }
        self.dispatch(AudioCommand::PlayClip {
            index: id.index as usize,
            volume,
        });
        true
    }

    pub fn play_sfx(&mut self, preset: SfxPreset, volume: f32, pitch_scale: f32) {
        self.dispatch(AudioCommand::PlaySfx {
            preset,
            volume,
            pitch_scale,
        });
    }

    pub fn play_tone(
        &mut self,
        waveform: Waveform,
        freq_start_hz: f32,
        freq_end_hz: f32,
        duration_sec: f32,
        volume: f32,
    ) {
        self.dispatch(AudioCommand::PlayTone {
            params: SynthParams {
                waveform,
                freq_start_hz,
                freq_end_hz,
                duration_sec,
                volume,
                delay_sec: 0.0,
            },
        });
    }

    pub fn play_bgm_preset(&mut self, preset: BgmPreset, volume: f32) {
        self.dispatch(AudioCommand::PlayBgm { preset, volume });
    }

    pub fn stop_bgm(&mut self) {
        self.dispatch(AudioCommand::StopBgm);
    }

    /// Renders `frames` stereo `[f32; 2]` samples in headless mode for deterministic verification.
    pub fn render_test_frames(&mut self, frames: usize) -> Vec<[f32; 2]> {
        if frames == 0 {
            return Vec::new();
        }
        let Some(mixer) = self.headless_mixer.as_mut() else {
            return vec![[0.0, 0.0]; frames];
        };
        let mut interleaved = vec![0.0_f32; frames * 2];
        mixer.mix_interleaved_f32(&mut interleaved, 2);
        interleaved
            .chunks_exact(2)
            .map(|chunk| [chunk[0], chunk[1]])
            .collect()
    }
}
