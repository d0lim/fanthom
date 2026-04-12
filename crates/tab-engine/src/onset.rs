//! Spectral-flux onset detection for plucked string instruments.

use rayon::prelude::*;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f64::consts::PI;

const ONSET_FRAME_SIZE: usize = 1024;
const ONSET_HOP_SIZE: usize = 128; // ~2.9 ms — 2x precision (was 256)
const ADAPTIVE_WINDOW: usize = 20; // scaled up with HOP_SIZE reduction (maintains ~58ms coverage)
const FLUX_MULTIPLIER: f64 = 1.5;
const FLUX_OFFSET: f64 = 0.007;
const MIN_ONSET_GAP_SECS: f64 = 0.05;

/// Detect note onset times in an audio signal using spectral flux.
/// Returns a sorted list of onset times in seconds.
pub fn detect_onsets(samples: &[f32], sample_rate: f64) -> Vec<f64> {
    if samples.len() < ONSET_FRAME_SIZE {
        return vec![];
    }
    let flux = spectral_flux(samples, sample_rate);
    peak_pick(&flux, sample_rate)
}

fn spectral_flux(samples: &[f32], _sample_rate: f64) -> Vec<f64> {
    let fft_size = ONSET_FRAME_SIZE.next_power_of_two();
    let num_bins = fft_size / 2 + 1;

    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(fft_size);

    let hann: Vec<f64> = (0..ONSET_FRAME_SIZE)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f64 / (ONSET_FRAME_SIZE - 1) as f64).cos()))
        .collect();

    let num_frames = if samples.len() >= ONSET_FRAME_SIZE {
        (samples.len() - ONSET_FRAME_SIZE) / ONSET_HOP_SIZE + 1
    } else {
        return vec![];
    };

    // Phase 1: Parallel FFT + magnitude per frame
    let magnitudes: Vec<Vec<f64>> = (0..num_frames)
        .into_par_iter()
        .map_init(
            || vec![Complex::new(0.0, 0.0); fft_size],
            |buf, i| {
                let pos = i * ONSET_HOP_SIZE;
                for j in 0..ONSET_FRAME_SIZE {
                    buf[j] = Complex::new(samples[pos + j] as f64 * hann[j], 0.0);
                }
                for b in buf[ONSET_FRAME_SIZE..].iter_mut() {
                    *b = Complex::new(0.0, 0.0);
                }
                fft.process(buf);
                (0..num_bins)
                    .map(|j| (buf[j].re.powi(2) + buf[j].im.powi(2)).sqrt())
                    .collect()
            },
        )
        .collect();

    // Phase 2: Sequential flux differencing (lightweight)
    let mut flux = vec![0.0]; // First frame has no previous
    for w in magnitudes.windows(2) {
        let f: f64 = w[1]
            .iter()
            .zip(w[0].iter())
            .map(|(c, p)| (c - p).max(0.0))
            .sum::<f64>()
            / num_bins as f64;
        flux.push(f);
    }

    flux
}

fn peak_pick(flux: &[f64], sample_rate: f64) -> Vec<f64> {
    if flux.len() < 3 {
        return vec![];
    }

    let min_gap_frames = (MIN_ONSET_GAP_SECS * sample_rate / ONSET_HOP_SIZE as f64).ceil() as usize;
    let mut onsets = Vec::new();
    let mut last_onset: Option<usize> = None;

    for i in 1..flux.len() - 1 {
        if flux[i] <= flux[i - 1] || flux[i] <= flux[i + 1] {
            continue;
        }

        let start = i.saturating_sub(ADAPTIVE_WINDOW);
        let end = (i + ADAPTIVE_WINDOW + 1).min(flux.len());
        let mut window: Vec<f64> = flux[start..end].to_vec();
        window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = window[window.len() / 2];
        let threshold = median * FLUX_MULTIPLIER + FLUX_OFFSET;

        if flux[i] > threshold {
            if let Some(last) = last_onset {
                if i - last < min_gap_frames {
                    continue;
                }
            }
            let time = i as f64 * ONSET_HOP_SIZE as f64 / sample_rate;
            onsets.push(time);
            last_onset = Some(i);
        }
    }

    onsets
}

#[cfg(test)]
mod tests {
    use super::*;

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
                let tone = (2.0 * PI * freq * t).sin() as f32 * env * 0.4;
                tone + attack
            })
            .collect()
    }

    #[test]
    fn empty_input_returns_no_onsets() {
        assert!(detect_onsets(&[], 44100.0).is_empty());
    }

    #[test]
    fn short_input_returns_no_onsets() {
        assert!(detect_onsets(&vec![0.0_f32; 100], 44100.0).is_empty());
    }

    #[test]
    fn single_pluck_detects_one_onset() {
        let sr = 44100.0;
        let mut samples = vec![0.0_f32; (0.1 * sr) as usize];
        samples.extend(plucked_note(110.0, 0.5, sr));
        samples.extend(vec![0.0_f32; (0.1 * sr) as usize]);

        let onsets = detect_onsets(&samples, sr);
        assert!(!onsets.is_empty(), "should detect onset for plucked note");
        assert!((onsets[0] - 0.1).abs() < 0.03, "onset at {:.3}s should be near 0.1s", onsets[0]);
    }

    #[test]
    fn two_plucks_detects_two_onsets() {
        let sr = 44100.0;
        let mut samples = Vec::new();
        samples.extend(plucked_note(110.0, 0.4, sr));
        samples.extend(vec![0.0_f32; (0.05 * sr) as usize]);
        samples.extend(plucked_note(82.0, 0.4, sr));

        let onsets = detect_onsets(&samples, sr);
        assert!(onsets.len() >= 2, "should detect 2+ onsets, got {}", onsets.len());
    }

    #[test]
    fn sustained_note_has_single_onset() {
        let sr = 44100.0;
        let samples = plucked_note(110.0, 2.0, sr);
        let onsets = detect_onsets(&samples, sr);
        assert_eq!(onsets.len(), 1, "sustained note should have 1 onset, got {}", onsets.len());
    }

    #[test]
    fn silence_has_no_onsets() {
        assert!(detect_onsets(&vec![0.0_f32; 44100], 44100.0).is_empty());
    }

    #[test]
    fn onsets_are_sorted() {
        let sr = 44100.0;
        let mut samples = Vec::new();
        for _ in 0..3 {
            samples.extend(plucked_note(110.0, 0.3, sr));
            samples.extend(vec![0.0_f32; (0.05 * sr) as usize]);
        }
        let onsets = detect_onsets(&samples, sr);
        for w in onsets.windows(2) {
            assert!(w[0] < w[1], "onsets must be sorted");
        }
    }

    #[test]
    fn min_gap_enforced() {
        let sr = 44100.0;
        let onsets = detect_onsets(&plucked_note(110.0, 1.0, sr), sr);
        if onsets.len() >= 2 {
            for w in onsets.windows(2) {
                assert!(w[1] - w[0] >= MIN_ONSET_GAP_SECS - 0.001, "gap {:.3}s below minimum", w[1] - w[0]);
            }
        }
    }
}
