//! YIN pitch detection for monophonic bass audio (FFT-accelerated).
//!
//! Takes a WAV file (typically the bass track separated by Demucs) and
//! returns a sequence of [`MidiNote`]s suitable for the tab engine.

use crate::onset;
use crate::MidiNote;
use rayon::prelude::*;
use rustfft::{num_complex::Complex, FftPlanner};
use std::path::Path;
use std::sync::Arc;

// ── Detection parameters ──────────────────────────────────────────

const FRAME_SIZE: usize = 8192; // ~186 ms at 44 100 Hz — ≥3 periods of E1 (41 Hz)
const HOP_SIZE: usize = 1024; // ~23 ms — 2x precision (was 2048)
const YIN_THRESHOLD: f64 = 0.15;
const MIN_FREQ: f64 = 30.0;
const MAX_FREQ: f64 = 500.0;
const SILENCE_RMS: f64 = 0.005;
const MIN_NOTE_SECS: f64 = 0.05;

#[derive(Clone, Copy, Debug)]
struct PitchFrame {
    time: f64,
    midi: Option<u8>,
    rms: f64,
    confidence: f64,
}

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
        self.detect_with_onsets(samples)
    }

    fn pitch_frames(&self, samples: &[f32]) -> Vec<PitchFrame> {
        let fft_size = (FRAME_SIZE * 2).next_power_of_two();
        let mut planner = FftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let ifft = planner.plan_fft_inverse(fft_size);

        let num_frames = if samples.len() >= FRAME_SIZE {
            (samples.len() - FRAME_SIZE) / HOP_SIZE + 1
        } else {
            return vec![];
        };

        (0..num_frames)
            .into_par_iter()
            .map_init(
                || {
                    (
                        vec![Complex::new(0.0, 0.0); fft_size],
                        vec![Complex::new(0.0, 0.0); fft_size],
                    )
                },
                |(buf_half, buf_full), i| {
                    let pos = i * HOP_SIZE;
                    let frame = &samples[pos..pos + FRAME_SIZE];
                    let time = pos as f64 / self.sample_rate;
                    let rms = rms_energy(frame);

                    let (midi, confidence) = if rms < SILENCE_RMS {
                        (None, 0.0)
                    } else {
                        let (freq, conf) =
                            self.yin_pitch_fft(frame, &fft, &ifft, buf_half, buf_full);
                        let midi = freq.and_then(|f| {
                            let m = freq_to_midi(f);
                            if (28..=67).contains(&m) { Some(m) } else { None }
                        });
                        (midi, conf)
                    };

                    PitchFrame { time, midi, rms, confidence }
                },
            )
            .collect()
    }

    fn detect_with_onsets(&self, samples: &[f32]) -> Vec<MidiNote> {
        let frames = self.pitch_frames(samples);
        let onsets = onset::detect_onsets(samples, self.sample_rate);

        if onsets.is_empty() {
            return self.segment_notes(&frames);
        }

        let mut notes = self.segment_at_onsets(&frames, &onsets);
        detect_slides(&mut notes, &frames, self.sample_rate);
        notes
    }

    fn segment_at_onsets(
        &self,
        frames: &[PitchFrame],
        onsets: &[f64],
    ) -> Vec<MidiNote> {
        let frame_dur = HOP_SIZE as f64 / self.sample_rate;
        let end_time = frames.last().map_or(0.0, |f| f.time + frame_dur);
        let mut notes = Vec::new();

        for (idx, &onset_time) in onsets.iter().enumerate() {
            let next_boundary = if idx + 1 < onsets.len() {
                onsets[idx + 1]
            } else {
                end_time
            };

            let pitched: Vec<(u8, f64, f64)> = frames
                .iter()
                .filter(|f| f.time >= onset_time && f.time < next_boundary)
                .filter_map(|f| f.midi.map(|m| (m, f.rms, f.confidence)))
                .collect();

            if pitched.is_empty() {
                continue;
            }

            let pitch = predominant_pitch(&pitched);
            let peak_rms = pitched.iter().map(|(_, r, _)| *r).fold(0.0_f64, f64::max);
            let offset = next_boundary.min(end_time);

            if offset - onset_time >= MIN_NOTE_SECS {
                notes.push(MidiNote {
                    pitch,
                    onset: onset_time,
                    offset,
                    velocity: rms_to_velocity(peak_rms),
                    technique: None,
                });
            }
        }

        notes
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
    ) -> (Option<f64>, f64) {
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
                    return (None, 0.0);
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

        (Some(self.sample_rate / refined), (1.0 - cmnd[tau]).clamp(0.0, 1.0))
    }

    fn segment_notes(&self, frames: &[PitchFrame]) -> Vec<MidiNote> {
        let mut notes = Vec::new();
        let mut current: Option<(u8, f64, f64)> = None;
        let frame_dur = HOP_SIZE as f64 / self.sample_rate;

        for frame in frames {
            match (current, frame.midi) {
                (Some((cp, onset, peak)), Some(m)) if cp == m => {
                    current = Some((cp, onset, peak.max(frame.rms)));
                }
                (Some((cp, onset, peak)), next) => {
                    let duration = frame.time - onset;
                    if duration >= MIN_NOTE_SECS {
                        notes.push(MidiNote {
                            pitch: cp,
                            onset,
                            offset: frame.time,
                            velocity: rms_to_velocity(peak),
                            technique: None,
                        });
                    }
                    current = next.map(|m| (m, frame.time, frame.rms));
                }
                (None, Some(m)) => {
                    current = Some((m, frame.time, frame.rms));
                }
                (None, None) => {}
            }
        }

        if let Some((cp, onset, peak)) = current {
            let offset = frames.last().map_or(onset, |f| f.time + frame_dur);
            if offset - onset >= MIN_NOTE_SECS {
                notes.push(MidiNote {
                    pitch: cp,
                    onset,
                    offset,
                    velocity: rms_to_velocity(peak),
                    technique: None,
                });
            }
        }

        notes
    }
}

// ── Onset segmentation helpers ───────────────────────────────────

fn predominant_pitch(pitched: &[(u8, f64, f64)]) -> u8 {
    let mut counts = std::collections::HashMap::new();
    for &(p, _, _) in pitched {
        *counts.entry(p).or_insert(0u32) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(p, _)| p)
        .unwrap_or(pitched[0].0)
}

fn detect_slides(
    notes: &mut [MidiNote],
    frames: &[PitchFrame],
    _sample_rate: f64,
) {
    if notes.len() < 2 {
        return;
    }

    for i in 1..notes.len() {
        let prev_pitch = notes[i - 1].pitch;
        let curr_pitch = notes[i].pitch;
        if prev_pitch == curr_pitch {
            continue;
        }

        // Look at transition zone: last 30% of prev note to first 30% of current note
        let prev_end = notes[i - 1].offset;
        let curr_start = notes[i].onset;
        let transition_start = prev_end - (prev_end - notes[i - 1].onset) * 0.3;
        let transition_end = curr_start + (notes[i].offset - curr_start) * 0.3;

        let transition_frames: Vec<u8> = frames
            .iter()
            .filter(|f| f.time >= transition_start && f.time <= transition_end)
            .filter_map(|f| f.midi)
            .collect();

        if transition_frames.len() < 3 {
            continue;
        }

        // Check monotonic pitch change
        let ascending = prev_pitch < curr_pitch;
        let mut is_monotonic = true;
        for w in transition_frames.windows(2) {
            if ascending && w[1] < w[0] {
                is_monotonic = false;
                break;
            }
            if !ascending && w[1] > w[0] {
                is_monotonic = false;
                break;
            }
        }

        // Check for intermediate pitches (not just a jump)
        let has_intermediate = transition_frames
            .iter()
            .any(|&p| p != prev_pitch && p != curr_pitch);

        if is_monotonic && has_intermediate {
            notes[i].technique = Some(crate::midi::Technique::Slide);
        }
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
        // Both pitches should be present in detected notes
        let has_a2 = notes.iter().any(|n| n.pitch == 45);
        let has_e2 = notes.iter().any(|n| n.pitch == 40);
        assert!(has_a2, "should detect A2 (MIDI 45)");
        assert!(has_e2, "should detect E2 (MIDI 40)");
        // A2 notes should come before E2 notes
        let first_a2 = notes.iter().position(|n| n.pitch == 45).unwrap();
        let first_e2 = notes.iter().position(|n| n.pitch == 40).unwrap();
        assert!(
            first_a2 < first_e2,
            "A2 should appear before E2 in the sequence"
        );
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

    fn plucked_note(freq: f64, duration: f64, sr: f64) -> Vec<f32> {
        let n = (duration * sr) as usize;
        (0..n)
            .map(|i| {
                let t = i as f64 / sr;
                let env = (-t * 3.0).exp() as f32;
                let attack = if t < 0.005 {
                    let click = (2.0 * PI * 200.0 * t).sin()
                        + (2.0 * PI * 800.0 * t).sin()
                        + (2.0 * PI * 2000.0 * t).sin();
                    click as f32 * (-t * 500.0).exp() as f32 * 0.3
                } else {
                    0.0
                };
                (2.0 * PI * freq * t).sin() as f32 * env * 0.4 + attack
            })
            .collect()
    }

    #[test]
    fn repeated_same_pitch_with_gap_gives_two_notes() {
        let sr = 44100.0;
        let mut samples = Vec::new();
        samples.extend(plucked_note(110.0, 0.3, sr));
        samples.extend(vec![0.0_f32; (0.1 * sr) as usize]);
        samples.extend(plucked_note(110.0, 0.3, sr));

        let det = YinDetector::new(sr);
        let notes = det.detect(&samples);
        assert!(
            notes.len() >= 2,
            "should detect 2 separate plucks, got {}",
            notes.len()
        );
        // Both should be A2 (MIDI 45)
        for note in &notes {
            assert_eq!(note.pitch, 45);
        }
    }

    #[test]
    fn slide_detection_marks_smooth_transition() {
        // This test verifies the detect_slides function directly
        use crate::midi::Technique;
        let frames: Vec<PitchFrame> = vec![
            // Note 1 region: pitch 45 (A2)
            PitchFrame { time: 0.0, midi: Some(45), rms: 0.1, confidence: 0.9 },
            PitchFrame { time: 0.046, midi: Some(45), rms: 0.09, confidence: 0.9 },
            PitchFrame { time: 0.093, midi: Some(45), rms: 0.08, confidence: 0.9 },
            PitchFrame { time: 0.139, midi: Some(45), rms: 0.07, confidence: 0.9 },
            // Transition: pitch glides 45 -> 46 -> 47 -> 48
            PitchFrame { time: 0.186, midi: Some(46), rms: 0.06, confidence: 0.8 },
            PitchFrame { time: 0.232, midi: Some(47), rms: 0.05, confidence: 0.8 },
            // Note 2 region: pitch 48 (C3)
            PitchFrame { time: 0.279, midi: Some(48), rms: 0.05, confidence: 0.9 },
            PitchFrame { time: 0.325, midi: Some(48), rms: 0.05, confidence: 0.9 },
            PitchFrame { time: 0.372, midi: Some(48), rms: 0.04, confidence: 0.9 },
        ];

        let mut notes = vec![
            MidiNote {
                pitch: 45,
                onset: 0.0,
                offset: 0.186,
                velocity: 80,
                technique: None,
            },
            MidiNote {
                pitch: 48,
                onset: 0.186,
                offset: 0.4,
                velocity: 60,
                technique: None,
            },
        ];

        detect_slides(&mut notes, &frames, 44100.0);
        assert_eq!(notes[0].technique, None, "first note should not be a slide");
        assert_eq!(
            notes[1].technique,
            Some(Technique::Slide),
            "second note should be detected as slide"
        );
    }

    #[test]
    fn pitch_frame_has_confidence() {
        let samples = sine_wave(110.0, 1.0, 44100.0);
        let det = YinDetector::new(44100.0);
        let frames = det.pitch_frames(&samples);
        let pitched: Vec<_> = frames.iter().filter(|f| f.midi.is_some()).collect();
        assert!(!pitched.is_empty());
        for f in &pitched {
            assert!(f.confidence > 0.5, "confidence {:.2} should be > 0.5 for clean sine", f.confidence);
        }
    }
}
