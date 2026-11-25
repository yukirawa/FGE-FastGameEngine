use std::sync::mpsc;

use crate::{
    engine::{AudioEngine, DEFAULT_MIXER_SAMPLE_RATE, SoundId},
    error::AudioError,
    mixer::{AudioCommand, MixerState},
    synth::{BgmPreset, BgmSequencer, SfxPreset, SynthParams, SynthVoice, Waveform},
    wav::parse_wav_bytes,
};

/// Builds a valid uncompressed PCM RIFF/WAVE byte buffer from interleaved `i16` samples.
fn build_pcm16_wav(channels: u16, sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_bytes_len = (samples.len() * 2) as u32;
    let riff_chunk_size = 36 + data_bytes_len;
    let byte_rate = sample_rate * u32::from(channels) * 2;
    let block_align = channels * 2;

    let mut buf = Vec::with_capacity(44 + data_bytes_len as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_chunk_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // format_tag = 1 (PCM)
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits_per_sample

    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_bytes_len.to_le_bytes());
    for &sample in samples {
        buf.extend_from_slice(&sample.to_le_bytes());
    }
    buf
}

fn peak_abs(samples: &[f32]) -> f32 {
    samples.iter().copied().fold(0.0f32, |acc, s| acc.max(s.abs()))
}

fn peak_abs_frames(frames: &[[f32; 2]]) -> f32 {
    frames
        .iter()
        .fold(0.0f32, |acc, &[l, r]| acc.max(l.abs()).max(r.abs()))
}

fn assert_all_finite_in_unit_range(samples: &[f32], context: &str) {
    for (i, &s) in samples.iter().enumerate() {
        assert!(
            s.is_finite(),
            "{context}: sample[{i}] is not finite ({s})"
        );
        assert!(
            (-1.0..=1.0).contains(&s),
            "{context}: sample[{i}] out of [-1.0, 1.0] ({s})"
        );
    }
}

// ============================================================================
// WAV Parsing & Resampling Tests
// ============================================================================

#[test]
fn wav_parses_valid_mono_16bit_pcm_preserving_count_and_amplitude() {
    let raw_samples = [0i16, 16_384, 32_767, -16_384, -32_768, 8_192, -8_192, 0];
    let wav = build_pcm16_wav(1, 44_100, &raw_samples);

    let clip = parse_wav_bytes(&wav, 44_100).expect("valid mono 16-bit WAV should parse");
    assert_eq!(clip.sample_rate, 44_100);
    assert_eq!(clip.frames.len(), raw_samples.len());

    // Mono duplicated identically to stereo L/R channels.
    for (idx, (&raw, &[l, r])) in raw_samples.iter().zip(clip.frames.iter()).enumerate() {
        assert!(l.is_finite() && r.is_finite());
        assert!((l - r).abs() < 1e-6, "frame {idx} L/R mismatch: {l} vs {r}");
        let expected = f32::from(raw) / 32_768.0;
        assert!(
            (l - expected).abs() < 1e-3,
            "frame {idx} amplitude mismatch: got {l}, expected ~{expected}"
        );
    }
}

#[test]
fn wav_parses_stereo_16bit_pcm_and_resamples_to_target_rate() {
    // 100 stereo frames at 22_050 Hz -> ~200 stereo frames at 44_100 Hz.
    let src_frames = 100usize;
    let mut raw_samples = Vec::with_capacity(src_frames * 2);
    for i in 0..src_frames {
        let phase = (i as f32 / src_frames as f32) * std::f32::consts::TAU * 4.0;
        let left = (phase.sin() * 20_000.0) as i16;
        let right = (phase.cos() * -12_000.0) as i16;
        raw_samples.push(left);
        raw_samples.push(right);
    }
    let wav = build_pcm16_wav(2, 22_050, &raw_samples);

    let clip = parse_wav_bytes(&wav, 44_100).expect("valid stereo 16-bit WAV should parse");
    assert_eq!(clip.sample_rate, 44_100);
    assert!(
        (198..=202).contains(&clip.frames.len()),
        "expected ~200 resampled frames, got {}",
        clip.frames.len()
    );

    for &[l, r] in &clip.frames {
        assert!(l.is_finite() && r.is_finite());
        assert!((-1.0..=1.0).contains(&l) && (-1.0..=1.0).contains(&r));
    }
    assert!(peak_abs_frames(&clip.frames) > 0.3);
}

#[test]
fn wav_rejects_truncated_and_corrupt_headers() {
    let valid = build_pcm16_wav(1, 44_100, &[1000, -1000, 500, -500]);

    // Empty and short truncated headers
    assert!(matches!(
        parse_wav_bytes(&[], 44_100),
        Err(AudioError::InvalidWav { .. })
    ));
    assert!(matches!(
        parse_wav_bytes(&valid[..12], 44_100),
        Err(AudioError::InvalidWav { .. })
    ));
    assert!(matches!(
        parse_wav_bytes(&valid[..32], 44_100),
        Err(AudioError::InvalidWav { .. })
    ));

    // Truncated data payload (declared data size larger than remaining bytes)
    let truncated_payload = &valid[..valid.len() - 4];
    assert!(matches!(
        parse_wav_bytes(truncated_payload, 44_100),
        Err(AudioError::InvalidWav { .. })
    ));

    // Corrupt RIFF magic
    let mut bad_riff = valid.clone();
    bad_riff[0..4].copy_from_slice(b"NOPE");
    assert!(matches!(
        parse_wav_bytes(&bad_riff, 44_100),
        Err(AudioError::InvalidWav { .. })
    ));

    // Corrupt WAVE magic
    let mut bad_wave = valid.clone();
    bad_wave[8..12].copy_from_slice(b"WXYZ");
    assert!(matches!(
        parse_wav_bytes(&bad_wave, 44_100),
        Err(AudioError::InvalidWav { .. })
    ));

    // Unsupported format tag (e.g., 99)
    let mut bad_fmt = valid.clone();
    bad_fmt[20..22].copy_from_slice(&99u16.to_le_bytes());
    assert!(matches!(
        parse_wav_bytes(&bad_fmt, 44_100),
        Err(AudioError::InvalidWav { .. })
    ));

    // Zero channels
    let mut zero_ch = valid.clone();
    zero_ch[22..24].copy_from_slice(&0u16.to_le_bytes());
    assert!(matches!(
        parse_wav_bytes(&zero_ch, 44_100),
        Err(AudioError::InvalidWav { .. })
    ));

    // Zero target sample rate
    assert!(matches!(
        parse_wav_bytes(&valid, 0),
        Err(AudioError::InvalidWav { .. })
    ));
}

// ============================================================================
// SynthVoice, SfxPreset, BgmSequencer & MixerState Tests
// ============================================================================

#[test]
fn synth_voice_produces_bounded_energy_then_terminates() {
    let mut voice = SynthVoice::new(SynthParams {
        waveform: Waveform::Square,
        freq_start_hz: 660.0,
        freq_end_hz: 220.0,
        duration_sec: 0.05,
        volume: 0.75,
        delay_sec: 0.0,
    });

    let mut samples = Vec::new();
    while let Some(s) = voice.next_sample(DEFAULT_MIXER_SAMPLE_RATE) {
        assert!(s.is_finite());
        assert!((-1.0..=1.0).contains(&s));
        samples.push(s);
        assert!(samples.len() < DEFAULT_MIXER_SAMPLE_RATE as usize, "voice failed to terminate");
    }

    assert!(
        samples.len() >= (DEFAULT_MIXER_SAMPLE_RATE as f32 * 0.04) as usize,
        "voice ended too early: {} samples",
        samples.len()
    );
    assert!(peak_abs(&samples) > 0.05, "voice produced near-zero energy");
}

#[test]
fn mixer_sfx_presets_produce_finite_bounded_energy_and_decay_to_silence() {
    for preset in [SfxPreset::Slash, SfxPreset::Blip, SfxPreset::LevelUp] {
        let mut mixer = MixerState::new(DEFAULT_MIXER_SAMPLE_RATE, None);
        mixer.apply_command(AudioCommand::PlaySfx {
            preset,
            volume: 0.85,
            pitch_scale: 1.0,
        });

        // Render 1.5 seconds of stereo interleaved samples (enough to cover arpeggios like LevelUp).
        let mut active_buf = vec![0.0f32; DEFAULT_MIXER_SAMPLE_RATE as usize * 2 * 3 / 2];
        mixer.mix_interleaved_f32(&mut active_buf, 2);

        assert_all_finite_in_unit_range(&active_buf, &format!("SfxPreset::{preset:?}"));
        let energy = peak_abs(&active_buf);
        assert!(
            energy > 0.01,
            "SfxPreset::{preset:?} produced insufficient energy: {energy}"
        );

        // Drain another 1.5 seconds to guarantee all voices/envelopes are done, then verify silence.
        let mut drain_buf = vec![0.0f32; DEFAULT_MIXER_SAMPLE_RATE as usize * 2 * 3 / 2];
        mixer.mix_interleaved_f32(&mut drain_buf, 2);
        assert_all_finite_in_unit_range(&drain_buf, "drain");

        let mut silent_buf = vec![0.0f32; 2048 * 2];
        mixer.mix_interleaved_f32(&mut silent_buf, 2);
        assert_eq!(
            peak_abs(&silent_buf),
            0.0,
            "SfxPreset::{preset:?} did not decay to silence after completion"
        );
    }
}

#[test]
fn mixer_bgm_presets_produce_energy_and_silence_after_stop_bgm() {
    let (tx, rx) = mpsc::sync_channel(64);
    let mut mixer = MixerState::new(DEFAULT_MIXER_SAMPLE_RATE, Some(rx));

    for preset in [BgmPreset::Village, BgmPreset::Boss] {
        tx.send(AudioCommand::PlayBgm {
            preset,
            volume: 0.65,
        })
        .unwrap();

        let mut bgm_buf = vec![0.0f32; 8192 * 2];
        mixer.mix_interleaved_f32(&mut bgm_buf, 2);
        assert_all_finite_in_unit_range(&bgm_buf, &format!("BgmPreset::{preset:?}"));
        assert!(
            peak_abs(&bgm_buf) > 0.01,
            "BgmPreset::{preset:?} produced zero energy"
        );

        // Stop BGM + allow any short release/voice completion, then verify full silence.
        tx.send(AudioCommand::StopBgm).unwrap();
        let mut release_buf = vec![0.0f32; (DEFAULT_MIXER_SAMPLE_RATE as usize / 2) * 2];
        mixer.mix_interleaved_f32(&mut release_buf, 2);
        assert_all_finite_in_unit_range(&release_buf, "stop_bgm release");

        let mut silent_buf = vec![0.0f32; 2048 * 2];
        mixer.mix_interleaved_f32(&mut silent_buf, 2);
        assert_eq!(
            peak_abs(&silent_buf),
            0.0,
            "BgmPreset::{preset:?} did not go silent after StopBgm"
        );
    }
}

#[test]
fn bgm_sequencer_direct_step_and_stop() {
    let mut seq = BgmSequencer::new(DEFAULT_MIXER_SAMPLE_RATE);
    seq.set_preset(BgmPreset::Village, 0.7);

    let mut frames = Vec::with_capacity(4096);
    for _ in 0..4096 {
        let [l, r] = seq.next_stereo_sample();
        assert!(l.is_finite() && r.is_finite());
        assert!((-1.0..=1.0).contains(&l) && (-1.0..=1.0).contains(&r));
        frames.push([l, r]);
    }
    assert!(peak_abs_frames(&frames) > 0.01);

    seq.stop();
    // Drain any short anti-click ramp if present, then check strict silence.
    for _ in 0..2048 {
        let _ = seq.next_stereo_sample();
    }
    for _ in 0..512 {
        assert_eq!(seq.next_stereo_sample(), [0.0, 0.0]);
    }
}

// ============================================================================
// AudioEngine Headless & Generational SoundId Validation Tests
// ============================================================================

#[test]
fn headless_engine_validates_sound_ids_and_renders_audio() {
    let mut engine = AudioEngine::new_headless();
    assert!(engine.is_headless());
    assert!(engine.sample_rate() > 0);

    // Reject {0, 0}, SoundId::INVALID, and unallocated handles.
    let zero_id = SoundId {
        index: 0,
        generation: 0,
    };
    assert!(!engine.play_sound(zero_id, 1.0));
    assert!(!engine.play_sound(SoundId::INVALID, 1.0));
    assert!(!engine.play_sound(
        SoundId {
            index: 42,
            generation: 1
        },
        1.0
    ));

    // Reject invalid WAV bytes.
    assert!(matches!(
        engine.load_wav_bytes(b"INVALID"),
        Err(AudioError::InvalidWav { .. })
    ));

    // Load a valid in-memory WAV and verify handle validity + playback energy.
    let samples: Vec<i16> = (0..1024)
        .map(|i| {
            let t = i as f32 / 44_100.0;
            ((t * 440.0 * std::f32::consts::TAU).sin() * 20_000.0) as i16
        })
        .collect();
    let wav = build_pcm16_wav(1, 44_100, &samples);

    let sound_id = engine
        .load_wav_bytes(&wav)
        .expect("headless engine should load valid WAV bytes");
    assert!(!sound_id.is_null());
    assert!(sound_id.generation >= 1);

    // {0, 0} and SoundId::INVALID must STILL be rejected after slot 0 is allocated.
    assert!(!engine.play_sound(zero_id, 1.0));
    assert!(!engine.play_sound(SoundId::INVALID, 1.0));

    // Playing valid SoundId succeeds and renders non-zero audio.
    assert!(engine.play_sound(sound_id, 0.8));
    let frames = engine.render_test_frames(1024);
    assert_eq!(frames.len(), 1024);
    for &[l, r] in &frames {
        assert!(l.is_finite() && r.is_finite());
        assert!((-1.0..=1.0).contains(&l) && (-1.0..=1.0).contains(&r));
    }
    assert!(peak_abs_frames(&frames) > 0.05);

    // SFX + BGM through headless engine + StopBgm silence check.
    engine.play_sfx(SfxPreset::Slash, 0.8, 1.0);
    engine.play_bgm_preset(BgmPreset::Boss, 0.6);
    let bgm_frames = engine.render_test_frames(4096);
    assert!(peak_abs_frames(&bgm_frames) > 0.01);

    engine.stop_bgm();
    let _ = engine.render_test_frames(DEFAULT_MIXER_SAMPLE_RATE as usize);
    let silent = engine.render_test_frames(512);
    assert_eq!(peak_abs_frames(&silent), 0.0);
}
