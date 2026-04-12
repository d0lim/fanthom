//! YIN pitch detection for monophonic bass audio (FFT-accelerated).
//!
//! Takes a WAV file (typically the bass track separated by Demucs) and
//! returns a sequence of [`MidiNote`]s suitable for the tab engine.

use crate::MidiNote;
use rustfft::{num_complex::Complex, FftPlanner};
use std::path::Path;
use std::sync::Arc;

// ── Detection parameters ──────────────────────────────────────────

const FRAME_SIZE: usize = 8192; // ~186 ms at 44 100 Hz — ≥3 periods of E1 (41 Hz)
const HOP_SIZE: usize = 2048; // ~46 ms
const YIN_THRESHOLD: f64 = 0.15;
const MIN_FREQ: f64 = 30.0;
const MAX_FREQ: f64 = 500.0;
const SILENCE_RMS: f64 = 0.005;
const MIN_NOTE_SECS: f64 = 0.05;

// ── Public API ────────────────────────────────────────────────────

/// Read a WAV file and detect pitched notes using the YIN algorithm.
pub fn transcribe_wav(wav_path: &str) -> Result<Vec<MidiNote>, String> {
    let path = Path::new(wav_path);
    let reader =
        hound::WavReader::open(path).map_err(|e| format!("Failed to open WAV {wav_path}: {e}"))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f64;
    let samples = read_samples_mono(reader, &spec)?;

    let detector = YinDetector::new(sample_rate);
    Ok(detector.detect(&samples))
}

// ── WAV reading ───────────────────────────────────────────────────

fn read_samples_mono(
    reader: hound::WavReader<std::io::BufReader<std::fs::File>>,
    spec: &hound::WavSpec,
) -> Result<Vec<f32>, String> {
    let channels = spec.channels as usize;

    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .map(|s| s.unwrap_or(0.0))
            .collect(),
        hound::SampleFormat::Int => {
            let max_val = (1_i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.unwrap_or(0) as f32 / max_val)
                .collect()
        }
    };

    if channels == 1 {
        return Ok(raw);
    }

    Ok(raw
        .chunks(channels)
        .map(|ch| ch.iter().sum::<f32>() / channels as f32)
        .collect())
}

// ── YIN pitch detector (FFT-accelerated) ──────────────────────────

struct YinDetector {
    sample_rate: f64,
    min_lag: usize,
    max_lag: usize,
}

impl YinDetector {
    fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            min_lag: (sample_rate / MAX_FREQ).ceil() as usize,
            max_lag: (sample_rate / MIN_FREQ).floor() as usize,
        }
    }

    fn detect(&self, samples: &[f32]) -> Vec<MidiNote> {
        let fft_size = (FRAME_SIZE * 2).next_power_of_two(); // 8192
        let mut planner = FftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let ifft = planner.plan_fft_inverse(fft_size);

        // Reusable buffers (avoid per-frame allocation)
        let mut buf_half = vec![Complex::new(0.0, 0.0); fft_size];
        let mut buf_full = vec![Complex::new(0.0, 0.0); fft_size];

        let mut frames: Vec<(f64, Option<u8>, f64)> = Vec::new();
        let mut pos = 0;

        while pos + FRAME_SIZE <= samples.len() {
            let frame = &samples[pos..pos + FRAME_SIZE];
            let time = pos as f64 / self.sample_rate;
            let rms = rms_energy(frame);

            let midi = if rms < SILENCE_RMS {
                None
            } else {
                self.yin_pitch_fft(frame, &fft, &ifft, &mut buf_half, &mut buf_full)
                    .and_then(|freq| {
                        let m = freq_to_midi(freq);
                        if (28..=67).contains(&m) {
                            Some(m)
                        } else {
                            None
                        }
                    })
            };

            frames.push((time, midi, rms));
            pos += HOP_SIZE;
        }

        self.segment_notes(&frames)
    }

    /// YIN pitch via FFT cross-correlation between first-half and full frame.
    ///
    /// The YIN difference function d(τ) = Σ_{j=0}^{W-1} (x[j] - x[j+τ])²
    /// needs the *partial* correlation r(τ) = Σ_{j=0}^{W-1} x[j]·x[j+τ],
    /// where W = half the frame. We compute this as a cross-correlation
    /// between x₁ = x[0..W] and x₂ = x[0..N] via FFT.
    fn yin_pitch_fft(
        &self,
        frame: &[f32],
        fft: &Arc<dyn rustfft::Fft<f64>>,
        ifft: &Arc<dyn rustfft::Fft<f64>>,
        buf_half: &mut [Complex<f64>],
        buf_full: &mut [Complex<f64>],
    ) -> Option<f64> {
        let n = frame.len();
        let half = n / 2;
        let fft_size = buf_half.len();
        let max_lag = self.max_lag.min(half - 1);

        // ── Prepare x₁ (first half, zero-padded) ──
        for i in 0..half {
            buf_half[i] = Complex::new(frame[i] as f64, 0.0);
        }
        for b in buf_half[half..].iter_mut() {
            *b = Complex::new(0.0, 0.0);
        }

        // ── Prepare x₂ (full frame, zero-padded) ──
        for (i, &s) in frame.iter().enumerate() {
            buf_full[i] = Complex::new(s as f64, 0.0);
        }
        for b in buf_full[n..].iter_mut() {
            *b = Complex::new(0.0, 0.0);
        }

        // ── Forward FFT both ──
        fft.process(buf_half);
        fft.process(buf_full);

        // ── Cross power spectrum: X₁(f) · conj(X₂(f)) ──
        for i in 0..fft_size {
            let a = buf_half[i];
            let b = buf_full[i];
            // a * conj(b) = (a.re*b.re + a.im*b.im) + i*(a.im*b.re - a.re*b.im)
            buf_half[i] = Complex::new(
                a.re * b.re + a.im * b.im,
                a.im * b.re - a.re * b.im,
            );
        }

        // ── Inverse FFT → cross-correlation r(τ) ──
        ifft.process(buf_half);
        let norm = 1.0 / fft_size as f64;

        // ── Energy terms via cumulative sum of squares ──
        let mut sq_cum = vec![0.0_f64; n + 1];
        for i in 0..n {
            sq_cum[i + 1] = sq_cum[i] + (frame[i] as f64).powi(2);
        }
        let s1 = sq_cum[half]; // Σ x[j]² for j = 0..half

        // ── Difference function: d(τ) = S₁ + S₂(τ) − 2·r(τ) ──
        let mut diff = vec![0.0_f64; max_lag + 1];
        for tau in 1..=max_lag {
            let s2 = sq_cum[tau + half] - sq_cum[tau];
            let r_tau = buf_half[tau].re * norm;
            diff[tau] = (s1 + s2 - 2.0 * r_tau).max(0.0);
        }

        // ── Cumulative mean normalized difference ──
        let mut cmnd = vec![1.0_f64; max_lag + 1];
        let mut running = 0.0;
        for tau in 1..=max_lag {
            running += diff[tau];
            cmnd[tau] = if running > 0.0 {
                diff[tau] * tau as f64 / running
            } else {
                1.0
            };
        }

        // ── Threshold search ──
        let mut best = None;
        let mut tau = self.min_lag;
        while tau <= max_lag {
            if cmnd[tau] < YIN_THRESHOLD {
                while tau + 1 <= max_lag && cmnd[tau + 1] < cmnd[tau] {
                    tau += 1;
                }
                best = Some(tau);
                break;
            }
            tau += 1;
        }

        let tau = match best {
            Some(t) => t,
            None => {
                let t = (self.min_lag..=max_lag)
                    .min_by(|&a, &b| {
                        cmnd[a]
                            .partial_cmp(&cmnd[b])
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or(self.min_lag);
                if cmnd[t] > 0.5 {
                    return None;
                }
                t
            }
        };

        // ── Parabolic interpolation ──
        let refined = if tau > self.min_lag && tau < max_lag {
            let a = cmnd[tau - 1];
            let b = cmnd[tau];
            let c = cmnd[tau + 1];
            let denom = 2.0 * (a - 2.0 * b + c);
            if denom.abs() > f64::EPSILON {
                tau as f64 + (a - c) / denom
            } else {
                tau as f64
            }
        } else {
            tau as f64
        };

        Some(self.sample_rate / refined)
    }

    fn segment_notes(&self, frames: &[(f64, Option<u8>, f64)]) -> Vec<MidiNote> {
        let mut notes = Vec::new();
        let mut current: Option<(u8, f64, f64)> = None;
        let frame_dur = HOP_SIZE as f64 / self.sample_rate;

        for &(time, midi, rms) in frames {
            match (current, midi) {
                (Some((cp, onset, peak)), Some(m)) if cp == m => {
                    current = Some((cp, onset, peak.max(rms)));
                }
                (Some((cp, onset, peak)), next) => {
                    let duration = time - onset;
                    if duration >= MIN_NOTE_SECS {
                        notes.push(MidiNote {
                            pitch: cp,
                            onset,
                            offset: time,
                            velocity: rms_to_velocity(peak),
                        });
                    }
                    current = next.map(|m| (m, time, rms));
                }
                (None, Some(m)) => {
                    current = Some((m, time, rms));
                }
                (None, None) => {}
            }
        }

        if let Some((cp, onset, peak)) = current {
            let offset = frames.last().map_or(onset, |f| f.0 + frame_dur);
            if offset - onset >= MIN_NOTE_SECS {
                notes.push(MidiNote {
                    pitch: cp,
                    onset,
                    offset,
                    velocity: rms_to_velocity(peak),
                });
            }
        }

        notes
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn rms_energy(frame: &[f32]) -> f64 {
    let sum: f64 = frame.iter().map(|&s| (s as f64).powi(2)).sum();
    (sum / frame.len() as f64).sqrt()
}

fn freq_to_midi(freq: f64) -> u8 {
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    midi.round().clamp(0.0, 127.0) as u8
}

fn rms_to_velocity(rms: f64) -> u8 {
    if rms < 1e-6 {
        return 0;
    }
    let db = 20.0 * rms.log10();
    ((db + 46.0) / 46.0 * 126.0 + 1.0).clamp(1.0, 127.0) as u8
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn sine_wave(freq: f64, duration: f64, sr: f64) -> Vec<f32> {
        let n = (duration * sr) as usize;
        (0..n)
            .map(|i| (2.0 * PI * freq * i as f64 / sr).sin() as f32 * 0.5)
            .collect()
    }

    #[test]
    fn detect_a2_110hz() {
        let samples = sine_wave(110.0, 1.0, 44100.0);
        let det = YinDetector::new(44100.0);
        let notes = det.detect(&samples);
        assert!(!notes.is_empty(), "should detect at least one note");
        assert_eq!(notes[0].pitch, 45); // A2
    }

    #[test]
    fn detect_e1_lowest_bass() {
        // E1 = MIDI 28 = 440 * 2^((28-69)/12) ≈ 41.203 Hz
        let e1_freq = 440.0 * 2.0_f64.powf((28.0 - 69.0) / 12.0);
        let samples = sine_wave(e1_freq, 1.5, 44100.0);
        let det = YinDetector::new(44100.0);
        let notes = det.detect(&samples);
        assert!(!notes.is_empty());
        assert_eq!(notes[0].pitch, 28); // E1
    }

    #[test]
    fn detect_g3_high_bass() {
        let samples = sine_wave(196.0, 1.0, 44100.0);
        let det = YinDetector::new(44100.0);
        let notes = det.detect(&samples);
        assert!(!notes.is_empty());
        assert_eq!(notes[0].pitch, 55); // G3
    }

    #[test]
    fn silence_produces_no_notes() {
        let samples = vec![0.0_f32; 44100];
        let det = YinDetector::new(44100.0);
        let notes = det.detect(&samples);
        assert!(notes.is_empty());
    }

    #[test]
    fn short_blip_filtered_out() {
        let mut samples = vec![0.0_f32; 44100];
        let blip = sine_wave(110.0, 0.03, 44100.0);
        samples[..blip.len()].copy_from_slice(&blip);
        let det = YinDetector::new(44100.0);
        let notes = det.detect(&samples);
        assert!(notes.is_empty(), "notes shorter than 50ms should be filtered");
    }

    #[test]
    fn two_notes_segmented() {
        let sr = 44100.0;
        let a2 = sine_wave(110.0, 0.5, sr);
        let silence = vec![0.0_f32; (0.1 * sr) as usize];
        let e2 = sine_wave(82.41, 0.5, sr);

        let mut samples = Vec::new();
        samples.extend_from_slice(&a2);
        samples.extend_from_slice(&silence);
        samples.extend_from_slice(&e2);

        let det = YinDetector::new(sr);
        let notes = det.detect(&samples);
        assert!(
            notes.len() >= 2,
            "should detect at least 2 notes, got {}",
            notes.len()
        );
        assert_eq!(notes[0].pitch, 45); // A2
        assert_eq!(notes[1].pitch, 40); // E2
    }

    #[test]
    fn velocity_reflects_loudness() {
        let loud = sine_wave(110.0, 0.5, 44100.0);
        let quiet: Vec<f32> = sine_wave(110.0, 0.5, 44100.0)
            .iter()
            .map(|s| s * 0.1)
            .collect();

        let det = YinDetector::new(44100.0);
        let loud_notes = det.detect(&loud);
        let quiet_notes = det.detect(&quiet);

        assert!(!loud_notes.is_empty());
        assert!(!quiet_notes.is_empty());
        assert!(
            loud_notes[0].velocity > quiet_notes[0].velocity,
            "loud {} should be > quiet {}",
            loud_notes[0].velocity,
            quiet_notes[0].velocity
        );
    }
}
