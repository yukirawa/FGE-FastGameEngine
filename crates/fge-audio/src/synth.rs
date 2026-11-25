/// Procedural chiptune waveform kinds (ABI-matched with `enum FgeWaveform` in `fge.h`).
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Waveform {
    Square = 0,
    Triangle = 1,
    Saw = 2,
    Noise = 3,
}

impl Waveform {
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Square),
            1 => Some(Self::Triangle),
            2 => Some(Self::Saw),
            3 => Some(Self::Noise),
            _ => None,
        }
    }
}

/// Built-in procedural SFX presets (ABI-matched with `enum FgeSynthPreset` in `fge.h`).
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SfxPreset {
    Slash = 0,
    Hit = 1,
    Explosion = 2,
    Pickup = 3,
    Blip = 4,
    Confirm = 5,
    Heal = 6,
    LevelUp = 7,
}

impl SfxPreset {
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Slash),
            1 => Some(Self::Hit),
            2 => Some(Self::Explosion),
            3 => Some(Self::Pickup),
            4 => Some(Self::Blip),
            5 => Some(Self::Confirm),
            6 => Some(Self::Heal),
            7 => Some(Self::LevelUp),
            _ => None,
        }
    }
}

/// Built-in procedural RPG BGM presets (ABI-matched with `enum FgeBgmPreset` in `fge.h`).
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BgmPreset {
    None = 0,
    Village = 1,
    Field = 2,
    Battle = 3,
    Boss = 4,
}

impl BgmPreset {
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Village),
            2 => Some(Self::Field),
            3 => Some(Self::Battle),
            4 => Some(Self::Boss),
            _ => None,
        }
    }
}

/// Parameters describing a single procedural chiptune tone/sweep.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SynthParams {
    pub waveform: Waveform,
    pub freq_start_hz: f32,
    pub freq_end_hz: f32,
    pub duration_sec: f32,
    pub volume: f32,
    pub delay_sec: f32,
}

/// Builds the procedural [`SynthParams`] layer(s) for a built-in [`SfxPreset`].
pub fn sfx_preset_params(preset: SfxPreset, volume: f32, pitch_scale: f32) -> Vec<SynthParams> {
    let vol = if volume.is_finite() {
        volume.clamp(0.0, 2.0)
    } else {
        0.0
    };
    let pitch = if pitch_scale.is_finite() && pitch_scale > 0.0 {
        pitch_scale.clamp(0.25, 4.0)
    } else {
        1.0
    };

    if vol <= 0.0 {
        return Vec::new();
    }

    match preset {
        SfxPreset::Slash => vec![
            SynthParams {
                waveform: Waveform::Saw,
                freq_start_hz: 920.0 * pitch,
                freq_end_hz: 210.0 * pitch,
                duration_sec: 0.11,
                volume: vol * 0.55,
                delay_sec: 0.0,
            },
            SynthParams {
                waveform: Waveform::Noise,
                freq_start_hz: 3200.0 * pitch,
                freq_end_hz: 700.0 * pitch,
                duration_sec: 0.09,
                volume: vol * 0.45,
                delay_sec: 0.0,
            },
        ],
        SfxPreset::Hit => vec![
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 220.0 * pitch,
                freq_end_hz: 55.0 * pitch,
                duration_sec: 0.10,
                volume: vol * 0.70,
                delay_sec: 0.0,
            },
            SynthParams {
                waveform: Waveform::Noise,
                freq_start_hz: 1400.0 * pitch,
                freq_end_hz: 260.0 * pitch,
                duration_sec: 0.08,
                volume: vol * 0.50,
                delay_sec: 0.0,
            },
        ],
        SfxPreset::Explosion => vec![
            SynthParams {
                waveform: Waveform::Noise,
                freq_start_hz: 900.0 * pitch,
                freq_end_hz: 95.0 * pitch,
                duration_sec: 0.42,
                volume: vol * 0.85,
                delay_sec: 0.0,
            },
            SynthParams {
                waveform: Waveform::Triangle,
                freq_start_hz: 140.0 * pitch,
                freq_end_hz: 36.0 * pitch,
                duration_sec: 0.38,
                volume: vol * 0.70,
                delay_sec: 0.0,
            },
        ],
        SfxPreset::Pickup => vec![
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 987.77 * pitch,
                freq_end_hz: 987.77 * pitch,
                duration_sec: 0.07,
                volume: vol * 0.55,
                delay_sec: 0.0,
            },
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 1318.51 * pitch,
                freq_end_hz: 1318.51 * pitch,
                duration_sec: 0.14,
                volume: vol * 0.55,
                delay_sec: 0.065,
            },
        ],
        SfxPreset::Blip => vec![SynthParams {
            waveform: Waveform::Square,
            freq_start_hz: 660.0 * pitch,
            freq_end_hz: 720.0 * pitch,
            duration_sec: 0.032,
            volume: vol * 0.45,
            delay_sec: 0.0,
        }],
        SfxPreset::Confirm => vec![
            SynthParams {
                waveform: Waveform::Triangle,
                freq_start_hz: 523.25 * pitch,
                freq_end_hz: 523.25 * pitch,
                duration_sec: 0.06,
                volume: vol * 0.60,
                delay_sec: 0.0,
            },
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 783.99 * pitch,
                freq_end_hz: 783.99 * pitch,
                duration_sec: 0.12,
                volume: vol * 0.45,
                delay_sec: 0.055,
            },
        ],
        SfxPreset::Heal => vec![
            SynthParams {
                waveform: Waveform::Triangle,
                freq_start_hz: 523.25 * pitch,
                freq_end_hz: 523.25 * pitch,
                duration_sec: 0.14,
                volume: vol * 0.55,
                delay_sec: 0.0,
            },
            SynthParams {
                waveform: Waveform::Triangle,
                freq_start_hz: 659.25 * pitch,
                freq_end_hz: 659.25 * pitch,
                duration_sec: 0.14,
                volume: vol * 0.55,
                delay_sec: 0.08,
            },
            SynthParams {
                waveform: Waveform::Triangle,
                freq_start_hz: 783.99 * pitch,
                freq_end_hz: 783.99 * pitch,
                duration_sec: 0.16,
                volume: vol * 0.55,
                delay_sec: 0.16,
            },
            SynthParams {
                waveform: Waveform::Triangle,
                freq_start_hz: 1046.50 * pitch,
                freq_end_hz: 1046.50 * pitch,
                duration_sec: 0.24,
                volume: vol * 0.55,
                delay_sec: 0.24,
            },
        ],
        SfxPreset::LevelUp => vec![
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 523.25 * pitch,
                freq_end_hz: 523.25 * pitch,
                duration_sec: 0.09,
                volume: vol * 0.55,
                delay_sec: 0.0,
            },
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 659.25 * pitch,
                freq_end_hz: 659.25 * pitch,
                duration_sec: 0.09,
                volume: vol * 0.55,
                delay_sec: 0.085,
            },
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 783.99 * pitch,
                freq_end_hz: 783.99 * pitch,
                duration_sec: 0.09,
                volume: vol * 0.55,
                delay_sec: 0.17,
            },
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 1046.50 * pitch,
                freq_end_hz: 1046.50 * pitch,
                duration_sec: 0.13,
                volume: vol * 0.60,
                delay_sec: 0.255,
            },
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 783.99 * pitch,
                freq_end_hz: 783.99 * pitch,
                duration_sec: 0.09,
                volume: vol * 0.50,
                delay_sec: 0.39,
            },
            SynthParams {
                waveform: Waveform::Square,
                freq_start_hz: 1046.50 * pitch,
                freq_end_hz: 1046.50 * pitch,
                duration_sec: 0.34,
                volume: vol * 0.65,
                delay_sec: 0.48,
            },
            SynthParams {
                waveform: Waveform::Triangle,
                freq_start_hz: 261.63 * pitch,
                freq_end_hz: 261.63 * pitch,
                duration_sec: 0.36,
                volume: vol * 0.55,
                delay_sec: 0.48,
            },
        ],
    }
}

/// Active phase-accumulated procedural voice.
#[derive(Clone, Debug)]
pub struct SynthVoice {
    pub params: SynthParams,
    pub elapsed_sec: f32,
    pub phase: f32,
    pub lfsr: u32,
}

impl SynthVoice {
    pub fn new(params: SynthParams) -> Self {
        Self {
            params,
            elapsed_sec: 0.0,
            phase: 0.0,
            lfsr: 0x7FFF,
        }
    }

    pub fn next_sample(&mut self, sample_rate: u32) -> Option<f32> {
        if sample_rate == 0
            || !self.params.duration_sec.is_finite()
            || self.params.duration_sec <= 0.0
        {
            return None;
        }

        let dt = 1.0 / sample_rate as f32;
        let delay = if self.params.delay_sec.is_finite() && self.params.delay_sec > 0.0 {
            self.params.delay_sec
        } else {
            0.0
        };

        if self.elapsed_sec < delay {
            self.elapsed_sec += dt;
            return Some(0.0);
        }

        let active_t = self.elapsed_sec - delay;
        if active_t >= self.params.duration_sec {
            return None;
        }

        let progress = (active_t / self.params.duration_sec).clamp(0.0, 1.0);
        let f0 = if self.params.freq_start_hz.is_finite() {
            self.params.freq_start_hz
        } else {
            440.0
        };
        let f1 = if self.params.freq_end_hz.is_finite() {
            self.params.freq_end_hz
        } else {
            f0
        };

        let raw_freq = if f0 > 1.0 && f1 > 1.0 {
            f0 * (f1 / f0).powf(progress)
        } else {
            f0 + (f1 - f0) * progress
        };
        let max_freq = (sample_rate as f32 * 0.45).max(20.0);
        let freq = raw_freq.clamp(20.0, max_freq);

        let raw = match self.params.waveform {
            Waveform::Square => {
                self.phase = (self.phase + freq * dt).fract();
                if self.phase < 0.5 { 1.0 } else { -1.0 }
            }
            Waveform::Triangle => {
                self.phase = (self.phase + freq * dt).fract();
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
            Waveform::Saw => {
                self.phase = (self.phase + freq * dt).fract();
                2.0 * self.phase - 1.0
            }
            Waveform::Noise => {
                self.phase += freq * dt;
                if self.phase >= 1.0 {
                    self.phase -= self.phase.floor();
                    step_lfsr(&mut self.lfsr);
                }
                if (self.lfsr & 1) != 0 { 1.0 } else { -1.0 }
            }
        };

        let attack_sec = (self.params.duration_sec * 0.08).clamp(0.0005, 0.005);
        let attack = if active_t < attack_sec {
            active_t / attack_sec
        } else {
            1.0
        };
        let decay = (-3.5 * progress).exp();
        let fade_out_sec = (self.params.duration_sec * 0.15).clamp(0.001, 0.012);
        let remaining = self.params.duration_sec - active_t;
        let fade_out = if remaining < fade_out_sec {
            (remaining / fade_out_sec).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let vol = if self.params.volume.is_finite() {
            self.params.volume.clamp(0.0, 2.0)
        } else {
            0.0
        };

        self.elapsed_sec += dt;
        Some(raw * attack * decay * fade_out * vol)
    }
}

#[inline]
fn step_lfsr(state: &mut u32) {
    let bit = (*state ^ (*state >> 1)) & 1;
    *state = (*state >> 1) | (bit << 14);
    if *state == 0 {
        *state = 0x7FFF;
    }
}

#[inline]
fn midi_to_hz(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}

struct BgmPattern {
    step_duration_sec: f32,
    melody: &'static [u8],
    bass: &'static [u8],
    perc: &'static [u8],
    melody_duty: f32,
    aggressive: bool,
}

// Warm pentatonic/major RPG village theme (C major / G major feel, 16 steps).
const VILLAGE_PATTERN: BgmPattern = BgmPattern {
    step_duration_sec: 0.22,
    melody: &[
        72, 74, 76, 79, 76, 74, 72, 0,
        74, 76, 79, 81, 79, 76, 74, 0,
    ],
    bass: &[
        48, 0, 55, 0, 52, 0, 55, 0,
        50, 0, 55, 0, 53, 0, 55, 0,
    ],
    perc: &[
        0, 0, 1, 0, 0, 0, 1, 0,
        0, 0, 1, 0, 0, 0, 1, 0,
    ],
    melody_duty: 0.35,
    aggressive: false,
};

// Adventurous upbeat field theme (G major, 16 steps).
const FIELD_PATTERN: BgmPattern = BgmPattern {
    step_duration_sec: 0.155,
    melody: &[
        67, 71, 74, 79, 76, 74, 71, 74,
        72, 74, 76, 79, 81, 79, 74, 0,
    ],
    bass: &[
        43, 43, 50, 43, 47, 47, 50, 47,
        48, 48, 52, 48, 50, 50, 57, 50,
    ],
    perc: &[
        2, 0, 1, 0, 2, 0, 1, 1,
        2, 0, 1, 0, 2, 1, 1, 0,
    ],
    melody_duty: 0.5,
    aggressive: false,
};

// Driving minor RPG battle theme (A minor, 16 steps).
const BATTLE_PATTERN: BgmPattern = BgmPattern {
    step_duration_sec: 0.125,
    melody: &[
        69, 72, 76, 72, 77, 76, 72, 71,
        69, 72, 76, 79, 77, 76, 74, 71,
    ],
    bass: &[
        45, 45, 57, 45, 41, 41, 53, 41,
        43, 43, 55, 43, 40, 40, 52, 40,
    ],
    perc: &[
        2, 1, 2, 1, 2, 1, 2, 1,
        2, 1, 2, 1, 2, 1, 2, 2,
    ],
    melody_duty: 0.25,
    aggressive: true,
};

// Fast intense boss theme (D minor / Phrygian tension, 16 steps).
const BOSS_PATTERN: BgmPattern = BgmPattern {
    step_duration_sec: 0.10,
    melody: &[
        74, 75, 77, 81, 79, 77, 75, 74,
        72, 74, 77, 82, 81, 79, 77, 75,
    ],
    bass: &[
        38, 50, 38, 50, 39, 51, 39, 51,
        36, 48, 36, 48, 37, 49, 45, 49,
    ],
    perc: &[
        2, 1, 1, 2, 2, 1, 2, 1,
        2, 1, 1, 2, 2, 2, 1, 2,
    ],
    melody_duty: 0.25,
    aggressive: true,
};

/// Multi-channel looping chiptune BGM step sequencer.
#[derive(Clone, Debug)]
pub struct BgmSequencer {
    pub preset: BgmPreset,
    pub volume: f32,
    pub sample_rate: u32,
    pub step_index: usize,
    pub step_elapsed_sec: f32,
    melody_phase: f32,
    bass_phase: f32,
    perc_phase: f32,
    perc_lfsr: u32,
}

impl BgmSequencer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            preset: BgmPreset::None,
            volume: 0.0,
            sample_rate,
            step_index: 0,
            step_elapsed_sec: 0.0,
            melody_phase: 0.0,
            bass_phase: 0.0,
            perc_phase: 0.0,
            perc_lfsr: 0x5A3D,
        }
    }

    pub fn set_preset(&mut self, preset: BgmPreset, volume: f32) {
        let clamped_vol = if volume.is_finite() {
            volume.clamp(0.0, 2.0)
        } else {
            0.0
        };
        if self.preset != preset {
            self.step_index = 0;
            self.step_elapsed_sec = 0.0;
            self.melody_phase = 0.0;
            self.bass_phase = 0.0;
            self.perc_phase = 0.0;
        }
        self.preset = preset;
        self.volume = if preset == BgmPreset::None {
            0.0
        } else {
            clamped_vol
        };
    }

    pub fn stop(&mut self) {
        self.preset = BgmPreset::None;
        self.volume = 0.0;
        self.step_index = 0;
        self.step_elapsed_sec = 0.0;
    }

    pub fn next_stereo_sample(&mut self) -> [f32; 2] {
        if self.preset == BgmPreset::None || self.volume <= 0.0 || self.sample_rate == 0 {
            return [0.0, 0.0];
        }

        let pattern = match self.preset {
            BgmPreset::None => return [0.0, 0.0],
            BgmPreset::Village => &VILLAGE_PATTERN,
            BgmPreset::Field => &FIELD_PATTERN,
            BgmPreset::Battle => &BATTLE_PATTERN,
            BgmPreset::Boss => &BOSS_PATTERN,
        };

        let dt = 1.0 / self.sample_rate as f32;
        self.step_elapsed_sec += dt;
        if self.step_elapsed_sec >= pattern.step_duration_sec {
            self.step_elapsed_sec -= pattern.step_duration_sec;
            self.step_index = (self.step_index + 1) % pattern.melody.len();
        }

        let step = self.step_index % pattern.melody.len();
        let progress = (self.step_elapsed_sec / pattern.step_duration_sec).clamp(0.0, 1.0);
        let attack = (self.step_elapsed_sec / 0.004).clamp(0.0, 1.0);
        let tail = ((1.0 - progress) / 0.08).clamp(0.0, 1.0);

        // 1. Melody channel (pulse wave)
        let mel_note = pattern.melody[step];
        let melody_sample = if mel_note > 0 {
            let freq = midi_to_hz(mel_note);
            self.melody_phase = (self.melody_phase + freq * dt).fract();
            let raw = if self.melody_phase < pattern.melody_duty {
                1.0
            } else {
                -1.0
            };
            let env = attack * (-2.2 * progress).exp() * tail;
            raw * env * 0.42
        } else {
            0.0
        };

        // 2. Bass channel (triangle for calm, saw/triangle blend for aggressive)
        let bass_note = pattern.bass[step];
        let bass_sample = if bass_note > 0 {
            let freq = midi_to_hz(bass_note);
            self.bass_phase = (self.bass_phase + freq * dt).fract();
            let tri = if self.bass_phase < 0.5 {
                4.0 * self.bass_phase - 1.0
            } else {
                3.0 - 4.0 * self.bass_phase
            };
            let raw = if pattern.aggressive {
                let saw = 2.0 * self.bass_phase - 1.0;
                0.65 * tri + 0.35 * saw
            } else {
                tri
            };
            let env = attack * (-1.6 * progress).exp() * tail;
            raw * env * 0.46
        } else {
            0.0
        };

        // 3. Light percussion channel (hi-hat tick or snare/kick accent)
        let perc_kind = pattern.perc[step];
        let perc_sample = match perc_kind {
            1 => {
                self.perc_phase += 5200.0 * dt;
                if self.perc_phase >= 1.0 {
                    self.perc_phase -= self.perc_phase.floor();
                    step_lfsr(&mut self.perc_lfsr);
                }
                let raw = if (self.perc_lfsr & 1) != 0 { 1.0 } else { -1.0 };
                raw * (-22.0 * progress).exp() * 0.14
            }
            2 => {
                self.perc_phase += 1900.0 * dt;
                if self.perc_phase >= 1.0 {
                    self.perc_phase -= self.perc_phase.floor();
                    step_lfsr(&mut self.perc_lfsr);
                }
                let raw = if (self.perc_lfsr & 1) != 0 { 1.0 } else { -1.0 };
                raw * (-14.0 * progress).exp() * 0.24
            }
            _ => 0.0,
        };

        let gain = self.volume * 0.40;
        let left = (melody_sample * 0.55 + bass_sample * 0.50 + perc_sample * 0.45) * gain;
        let right = (melody_sample * 0.45 + bass_sample * 0.50 + perc_sample * 0.55) * gain;
        [left, right]
    }
}
