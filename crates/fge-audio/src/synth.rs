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

/// Active phase-accumulated procedural voice.
#[derive(Clone, Debug)]
pub struct SynthVoice {
    pub params: SynthParams,
    pub elapsed_sec: f32,
    pub phase: f32,
    pub lfsr: u32,
}

impl SynthVoice {
    pub fn new(_params: SynthParams) -> Self {
        todo!("implemented by impl-audio")
    }

    pub fn next_sample(&mut self, _sample_rate: u32) -> Option<f32> {
        todo!("implemented by impl-audio")
    }
}

/// Multi-channel looping chiptune BGM step sequencer.
#[derive(Clone, Debug)]
pub struct BgmSequencer {
    pub preset: BgmPreset,
    pub volume: f32,
    pub sample_rate: u32,
    pub step_index: usize,
    pub step_elapsed_sec: f32,
}

impl BgmSequencer {
    pub fn new(_sample_rate: u32) -> Self {
        todo!("implemented by impl-audio")
    }

    pub fn set_preset(&mut self, _preset: BgmPreset, _volume: f32) {
        todo!("implemented by impl-audio")
    }

    pub fn stop(&mut self) {
        todo!("implemented by impl-audio")
    }

    pub fn next_stereo_sample(&mut self) -> [f32; 2] {
        todo!("implemented by impl-audio")
    }
}
