use std::path::Path;

use crate::error::AudioError;

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

/// Decoded interleaved stereo PCM clip resampled to the mixer's target sample rate.
#[derive(Clone, Debug)]
pub struct AudioClip {
    pub frames: Vec<[f32; 2]>,
    pub sample_rate: u32,
}

#[derive(Copy, Clone, Debug)]
struct WavFormat {
    format_tag: u16,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
}

/// Parses an in-memory RIFF/WAVE buffer (8-bit unsigned PCM, 16-bit signed PCM, or 32-bit IEEE
/// float, mono or stereo) and linearly resamples it to `target_sample_rate`.
pub fn parse_wav_bytes(bytes: &[u8], target_sample_rate: u32) -> Result<AudioClip, AudioError> {
    if target_sample_rate == 0 {
        return Err(AudioError::InvalidWav {
            reason: "target sample rate must be greater than zero".to_string(),
        });
    }
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::InvalidWav {
            reason: "missing RIFF/WAVE header".to_string(),
        });
    }

    let mut fmt: Option<WavFormat> = None;
    let mut data_slice: Option<&[u8]> = None;

    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        let data_start = offset + 8;
        let Some(data_end) = data_start.checked_add(chunk_size) else {
            return Err(AudioError::InvalidWav {
                reason: "chunk size overflow".to_string(),
            });
        };
        if data_end > bytes.len() {
            return Err(AudioError::InvalidWav {
                reason: "truncated WAV chunk".to_string(),
            });
        }

        let payload = &bytes[data_start..data_end];
        if chunk_id == b"fmt " {
            if payload.len() < 16 {
                return Err(AudioError::InvalidWav {
                    reason: "fmt chunk shorter than 16 bytes".to_string(),
                });
            }
            let mut format_tag = u16::from_le_bytes([payload[0], payload[1]]);
            let channels = u16::from_le_bytes([payload[2], payload[3]]);
            let sample_rate =
                u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
            let bits_per_sample = u16::from_le_bytes([payload[14], payload[15]]);

            if format_tag == WAVE_FORMAT_EXTENSIBLE && payload.len() >= 40 {
                format_tag = u16::from_le_bytes([payload[24], payload[25]]);
            }

            fmt = Some(WavFormat {
                format_tag,
                channels,
                sample_rate,
                bits_per_sample,
            });
        } else if chunk_id == b"data" {
            data_slice = Some(payload);
        }

        let pad = chunk_size & 1;
        let Some(next_offset) = data_end.checked_add(pad) else {
            break;
        };
        offset = next_offset;
    }

    let fmt = fmt.ok_or_else(|| AudioError::InvalidWav {
        reason: "missing fmt chunk".to_string(),
    })?;
    let data = data_slice.ok_or_else(|| AudioError::InvalidWav {
        reason: "missing data chunk".to_string(),
    })?;

    if fmt.sample_rate == 0 {
        return Err(AudioError::InvalidWav {
            reason: "WAV sample rate is zero".to_string(),
        });
    }
    if fmt.channels != 1 && fmt.channels != 2 {
        return Err(AudioError::InvalidWav {
            reason: format!("unsupported channel count: {} (expected 1 or 2)", fmt.channels),
        });
    }

    let src_frames = decode_stereo_frames(fmt, data)?;
    let frames = resample_linear(&src_frames, fmt.sample_rate, target_sample_rate);
    Ok(AudioClip {
        frames,
        sample_rate: target_sample_rate,
    })
}

/// Reads and decodes a `.wav` file from disk, resampling to `target_sample_rate`.
pub fn load_wav_file(path: &Path, target_sample_rate: u32) -> Result<AudioClip, AudioError> {
    let bytes = std::fs::read(path)?;
    parse_wav_bytes(&bytes, target_sample_rate)
}

fn decode_stereo_frames(fmt: WavFormat, data: &[u8]) -> Result<Vec<[f32; 2]>, AudioError> {
    let channels = fmt.channels as usize;
    match (fmt.format_tag, fmt.bits_per_sample) {
        (WAVE_FORMAT_PCM, 8) => {
            let frame_bytes = channels;
            let mut frames = Vec::with_capacity(data.len() / frame_bytes);
            for chunk in data.chunks_exact(frame_bytes) {
                let l = decode_u8_pcm(chunk[0]);
                let r = if channels == 2 {
                    decode_u8_pcm(chunk[1])
                } else {
                    l
                };
                frames.push([l, r]);
            }
            Ok(frames)
        }
        (WAVE_FORMAT_PCM, 16) => {
            let frame_bytes = channels * 2;
            let mut frames = Vec::with_capacity(data.len() / frame_bytes);
            for chunk in data.chunks_exact(frame_bytes) {
                let l = decode_i16_pcm(i16::from_le_bytes([chunk[0], chunk[1]]));
                let r = if channels == 2 {
                    decode_i16_pcm(i16::from_le_bytes([chunk[2], chunk[3]]))
                } else {
                    l
                };
                frames.push([l, r]);
            }
            Ok(frames)
        }
        (WAVE_FORMAT_IEEE_FLOAT, 32) => {
            let frame_bytes = channels * 4;
            let mut frames = Vec::with_capacity(data.len() / frame_bytes);
            for chunk in data.chunks_exact(frame_bytes) {
                let l = decode_f32_pcm(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                let r = if channels == 2 {
                    decode_f32_pcm(f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]))
                } else {
                    l
                };
                frames.push([l, r]);
            }
            Ok(frames)
        }
        (tag, bits) => Err(AudioError::InvalidWav {
            reason: format!("unsupported WAV format_tag={tag}, bits_per_sample={bits}"),
        }),
    }
}

#[inline]
fn decode_u8_pcm(sample: u8) -> f32 {
    ((sample as f32 - 128.0) / 128.0).clamp(-1.0, 1.0)
}

#[inline]
fn decode_i16_pcm(sample: i16) -> f32 {
    (sample as f32 / 32768.0).clamp(-1.0, 1.0)
}

#[inline]
fn decode_f32_pcm(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn resample_linear(src: &[[f32; 2]], src_rate: u32, dst_rate: u32) -> Vec<[f32; 2]> {
    if src.is_empty() || src_rate == dst_rate {
        return src.to_vec();
    }

    let out_len = ((src.len() as f64 * dst_rate as f64) / src_rate as f64)
        .round()
        .max(1.0) as usize;
    let ratio = src_rate as f64 / dst_rate as f64;
    let last_idx = src.len() - 1;

    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx0 = (src_pos.floor() as usize).min(last_idx);
        let idx1 = (idx0 + 1).min(last_idx);
        let frac = (src_pos - idx0 as f64).clamp(0.0, 1.0) as f32;
        let a = src[idx0];
        let b = src[idx1];
        out.push([a[0] + (b[0] - a[0]) * frac, a[1] + (b[1] - a[1]) * frac]);
    }
    out
}
