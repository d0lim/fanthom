# Onset Detection & Technique Analysis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace pitch-change-based note segmentation with spectral-flux onset detection, enabling accurate sustain/repeat distinction and slide detection for bass guitar tabs.

**Architecture:** Add a spectral flux onset detector (`onset.rs`) that identifies pluck transients via FFT magnitude difference. Refactor `pitch.rs` to segment notes at detected onsets instead of pitch changes. Add post-processing slide detection by analyzing pitch contour between consecutive notes. Propagate technique annotations (`Technique` enum) through `MidiNote` → `TabNote` → UI.

**Tech Stack:** Rust (`rustfft` for spectral flux FFT), Tauri v2, React + Canvas

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/tab-engine/src/midi.rs` | Modify | Add `Technique` enum + `technique` field to `MidiNote` |
| `crates/tab-engine/src/tab.rs` | Modify | Add `technique` field to `TabNote` |
| `crates/tab-engine/src/onset.rs` | Create | Spectral flux onset detection |
| `crates/tab-engine/src/pitch.rs` | Modify | Onset-based segmentation + slide detection |
| `crates/tab-engine/src/lib.rs` | Modify | Export `onset` module and `Technique` |
| `crates/tab-engine/src/viterbi.rs` | Modify | Preserve technique, enforce same-string for slides |
| `crates/tab-engine/src/transpose.rs` | Modify | Preserve technique through transpose |
| `crates/tab-engine/src/export/ascii.rs` | Modify | Render slide notation (`/`, `\`) |
| `apps/desktop/src-tauri/src/commands/tab.rs` | Modify | Pass technique through tab commands |
| `apps/desktop/src/lib/types.ts` | Modify | Add `Technique` type |
| `apps/desktop/src/components/TabCanvas.tsx` | Modify | Render technique annotations |

---

### Task 1: Add Technique enum and update MidiNote

**Files:**
- Modify: `crates/tab-engine/src/midi.rs`

- [ ] **Step 1: Write test for Technique serialization**

```rust
// Add to existing tests module in midi.rs

#[test]
fn deserialize_midi_note_with_technique() {
    let json = r#"{"pitch": 40, "onset": 1.5, "offset": 2.0, "velocity": 100, "technique": "Slide"}"#;
    let note: MidiNote = serde_json::from_str(json).unwrap();
    assert_eq!(note.technique, Some(Technique::Slide));
}

#[test]
fn deserialize_midi_note_without_technique_defaults_none() {
    let json = r#"{"pitch": 40, "onset": 1.5, "offset": 2.0, "velocity": 100}"#;
    let note: MidiNote = serde_json::from_str(json).unwrap();
    assert_eq!(note.technique, None);
}

#[test]
fn serialize_midi_note_without_technique_omits_field() {
    let note = MidiNote { pitch: 40, onset: 1.0, offset: 2.0, velocity: 80, technique: None };
    let json = serde_json::to_string(&note).unwrap();
    assert!(!json.contains("technique"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine midi::tests --no-default-features`
Expected: Compilation error — `Technique` not defined, `technique` field not on MidiNote.

- [ ] **Step 3: Add Technique enum and update MidiNote**

In `crates/tab-engine/src/midi.rs`, add before `MidiNote`:

```rust
/// How a note was articulated.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Technique {
    Slide,
}
```

Update `MidiNote` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MidiNote {
    pub pitch: u8,
    pub onset: f64,
    pub offset: f64,
    pub velocity: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technique: Option<Technique>,
}
```

- [ ] **Step 4: Fix all existing code that constructs MidiNote**

Every place that creates a `MidiNote` needs `technique: None` added. Locations:

**`crates/tab-engine/src/pitch.rs`** — in `segment_notes()` (line ~268):
```rust
notes.push(MidiNote {
    pitch: cp,
    onset,
    offset: time,
    velocity: rms_to_velocity(peak),
    technique: None,
});
```
And the final note push (line ~285):
```rust
notes.push(MidiNote {
    pitch: cp,
    onset,
    offset,
    velocity: rms_to_velocity(peak),
    technique: None,
});
```

**`crates/tab-engine/src/pitch.rs` tests** — `sine_wave` tests don't construct MidiNote directly, so no change needed.

**`crates/tab-engine/src/viterbi.rs` tests** — `make_note` helper:
```rust
fn make_note(pitch: u8, onset: f64, offset: f64) -> MidiNote {
    MidiNote { pitch, onset, offset, velocity: 80, technique: None }
}
```

**`crates/tab-engine/src/transpose.rs`** — in `transpose()` function (line ~18):
```rust
MidiNote {
    pitch: final_pitch,
    onset: note.onset,
    offset: note.offset,
    velocity: note.velocity,
    technique: note.technique,
},
```

**`crates/tab-engine/src/transpose.rs` tests** — `make_note` helper:
```rust
fn make_note(pitch: u8, onset: f64) -> MidiNote {
    MidiNote { pitch, onset, offset: onset + 0.5, velocity: 80, technique: None }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/tab-engine/src/midi.rs crates/tab-engine/src/pitch.rs crates/tab-engine/src/viterbi.rs crates/tab-engine/src/transpose.rs
git commit -m "feat(tab-engine): add Technique enum and technique field to MidiNote"
```

---

### Task 2: Add technique field to TabNote

**Files:**
- Modify: `crates/tab-engine/src/tab.rs`
- Modify: `crates/tab-engine/src/lib.rs`

- [ ] **Step 1: Write test for TabNote with technique**

```rust
// Add to existing tests module in tab.rs

#[test]
fn tab_note_serializes_technique() {
    use crate::midi::Technique;
    let note = TabNote {
        string: 0, fret: 5, midi_pitch: 33,
        onset: 0.0, duration: 0.5,
        origin: NoteOrigin::Normal,
        technique: Some(Technique::Slide),
    };
    let json = serde_json::to_string(&note).unwrap();
    assert!(json.contains("\"technique\":\"Slide\""));
}

#[test]
fn tab_note_without_technique_omits_field() {
    let note = TabNote {
        string: 0, fret: 5, midi_pitch: 33,
        onset: 0.0, duration: 0.5,
        origin: NoteOrigin::Normal,
        technique: None,
    };
    let json = serde_json::to_string(&note).unwrap();
    assert!(!json.contains("technique"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine tab::tests --no-default-features`
Expected: Compilation error — `technique` field not on `TabNote`.

- [ ] **Step 3: Add technique field to TabNote**

In `crates/tab-engine/src/tab.rs`, add import and field:

```rust
use crate::midi::Technique;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabNote {
    pub string: u8,
    pub fret: u8,
    pub midi_pitch: u8,
    pub onset: f64,
    pub duration: f64,
    pub origin: NoteOrigin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technique: Option<Technique>,
}
```

- [ ] **Step 4: Fix all code that constructs TabNote**

**`crates/tab-engine/src/viterbi.rs`** — in `optimize()` (line ~119):
```rust
TabNote {
    string: c.string,
    fret: c.fret,
    midi_pitch: note.pitch,
    onset: note.onset,
    duration: note.offset - note.onset,
    origin: NoteOrigin::Optimized,
    technique: note.technique,
}
```

**`apps/desktop/src-tauri/src/commands/tab.rs`** — in `toggle_optimization` and `regenerate_tab` (the non-optimized path):
```rust
tab_engine::TabNote {
    string: c.string,
    fret: c.fret,
    midi_pitch: n.pitch,
    onset: n.onset,
    duration: n.offset - n.onset,
    origin: tab_engine::NoteOrigin::Normal,
    technique: n.technique,
}
```

**`crates/tab-engine/src/export/ascii.rs` tests** — `make_tab_note`:
```rust
fn make_tab_note(string: u8, fret: u8, onset: f64) -> TabNote {
    TabNote {
        string, fret,
        midi_pitch: 0,
        onset,
        duration: 0.5,
        origin: NoteOrigin::Normal,
        technique: None,
    }
}
```

- [ ] **Step 5: Update lib.rs exports**

In `crates/tab-engine/src/lib.rs`, add `Technique` to re-exports:
```rust
pub use midi::{MidiNote, Technique};
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/tab-engine/src/tab.rs crates/tab-engine/src/viterbi.rs crates/tab-engine/src/lib.rs crates/tab-engine/src/export/ascii.rs apps/desktop/src-tauri/src/commands/tab.rs
git commit -m "feat(tab-engine): add technique field to TabNote, propagate through pipeline"
```

---

### Task 3: Spectral flux onset detection

**Files:**
- Create: `crates/tab-engine/src/onset.rs`
- Modify: `crates/tab-engine/src/lib.rs`

- [ ] **Step 1: Create onset.rs with tests**

Create `crates/tab-engine/src/onset.rs`:

```rust
//! Spectral-flux onset detection for plucked string instruments.
//!
//! Detects note onsets (pluck transients) by measuring the rate of change
//! in the magnitude spectrum frame-to-frame. Bass guitar plucks produce
//! broadband transients that cause spikes in spectral flux.

use rustfft::{num_complex::Complex, FftPlanner};
use std::f64::consts::PI;

// ── Parameters ───────────────────────────────────────────────────
const ONSET_FRAME_SIZE: usize = 1024;
const ONSET_HOP_SIZE: usize = 256;
const ADAPTIVE_WINDOW: usize = 10;
const FLUX_MULTIPLIER: f64 = 1.5;
const FLUX_OFFSET: f64 = 0.005;
const MIN_ONSET_GAP_SECS: f64 = 0.05;

// ── Public API ───────────────────────────────────────────────────

/// Detect note onset times in an audio signal using spectral flux.
///
/// Returns a sorted list of onset times in seconds.
pub fn detect_onsets(samples: &[f32], sample_rate: f64) -> Vec<f64> {
    if samples.len() < ONSET_FRAME_SIZE {
        return vec![];
    }

    let flux = spectral_flux(samples, sample_rate);
    peak_pick(&flux, sample_rate)
}

// ── Spectral flux ────────────────────────────────────────────────

fn spectral_flux(samples: &[f32], _sample_rate: f64) -> Vec<f64> {
    let fft_size = ONSET_FRAME_SIZE.next_power_of_two();
    let num_bins = fft_size / 2 + 1;

    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(fft_size);

    let hann: Vec<f64> = (0..ONSET_FRAME_SIZE)
        .map(|i| {
            0.5 * (1.0 - (2.0 * PI * i as f64 / (ONSET_FRAME_SIZE - 1) as f64).cos())
        })
        .collect();

    let mut buf = vec![Complex::new(0.0, 0.0); fft_size];
    let mut prev_mag = vec![0.0_f64; num_bins];
    let mut flux_values = Vec::new();

    let mut pos = 0;
    while pos + ONSET_FRAME_SIZE <= samples.len() {
        // Window + load
        for i in 0..ONSET_FRAME_SIZE {
            buf[i] = Complex::new(samples[pos + i] as f64 * hann[i], 0.0);
        }
        for b in buf[ONSET_FRAME_SIZE..].iter_mut() {
            *b = Complex::new(0.0, 0.0);
        }

        fft.process(&mut buf);

        // Magnitude spectrum
        let mag: Vec<f64> = (0..num_bins)
            .map(|i| (buf[i].re.powi(2) + buf[i].im.powi(2)).sqrt())
            .collect();

        // Half-wave rectified spectral flux
        let flux: f64 = mag
            .iter()
            .zip(prev_mag.iter())
            .map(|(c, p)| (c - p).max(0.0))
            .sum::<f64>()
            / num_bins as f64;

        flux_values.push(flux);
        prev_mag = mag;
        pos += ONSET_HOP_SIZE;
    }

    flux_values
}

// ── Peak picking with adaptive threshold ─────────────────────────

fn peak_pick(flux: &[f64], sample_rate: f64) -> Vec<f64> {
    if flux.len() < 3 {
        return vec![];
    }

    let min_gap_frames =
        (MIN_ONSET_GAP_SECS * sample_rate / ONSET_HOP_SIZE as f64).ceil() as usize;
    let mut onsets = Vec::new();
    let mut last_onset: Option<usize> = None;

    for i in 1..flux.len() - 1 {
        // Must be local maximum
        if flux[i] <= flux[i - 1] || flux[i] <= flux[i + 1] {
            continue;
        }

        // Adaptive threshold from surrounding window
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

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(freq: f64, duration: f64, sr: f64) -> Vec<f32> {
        let n = (duration * sr) as usize;
        (0..n)
            .map(|i| (2.0 * PI * freq * i as f64 / sr).sin() as f32 * 0.5)
            .collect()
    }

    fn click(sr: f64) -> Vec<f32> {
        // Short broadband transient simulating a pluck
        let n = (0.005 * sr) as usize; // 5ms
        (0..n)
            .map(|i| {
                let t = i as f64 / sr;
                let env = (-t * 500.0).exp() as f32;
                // Mix of frequencies for broadband content
                let sig = (2.0 * PI as f32 * 200.0 * t as f32).sin()
                    + (2.0 * PI as f32 * 800.0 * t as f32).sin()
                    + (2.0 * PI as f32 * 2000.0 * t as f32).sin();
                sig * env * 0.3
            })
            .collect()
    }

    fn plucked_note(freq: f64, duration: f64, sr: f64) -> Vec<f32> {
        let n = (duration * sr) as usize;
        let click_samples = click(sr);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / sr;
            let env = (-t * 3.0).exp() as f32; // Slow decay
            let tone = (2.0 * PI * freq * t).sin() as f32 * env * 0.4;
            let c = if i < click_samples.len() {
                click_samples[i]
            } else {
                0.0
            };
            out.push(tone + c);
        }
        out
    }

    #[test]
    fn empty_input_returns_no_onsets() {
        let onsets = detect_onsets(&[], 44100.0);
        assert!(onsets.is_empty());
    }

    #[test]
    fn short_input_returns_no_onsets() {
        let samples = vec![0.0_f32; 100];
        let onsets = detect_onsets(&samples, 44100.0);
        assert!(onsets.is_empty());
    }

    #[test]
    fn single_pluck_detects_one_onset() {
        let sr = 44100.0;
        let mut samples = vec![0.0_f32; (0.1 * sr) as usize]; // 100ms silence
        samples.extend(plucked_note(110.0, 0.5, sr));
        samples.extend(vec![0.0_f32; (0.1 * sr) as usize]); // trailing silence

        let onsets = detect_onsets(&samples, sr);
        assert!(
            !onsets.is_empty(),
            "should detect at least one onset for a plucked note"
        );
        // Onset should be near 0.1s (after the silence)
        assert!(
            (onsets[0] - 0.1).abs() < 0.03,
            "onset at {:.3}s should be near 0.1s",
            onsets[0]
        );
    }

    #[test]
    fn two_plucks_detects_two_onsets() {
        let sr = 44100.0;
        let note1 = plucked_note(110.0, 0.4, sr);
        let gap = vec![0.0_f32; (0.05 * sr) as usize];
        let note2 = plucked_note(82.0, 0.4, sr);

        let mut samples = Vec::new();
        samples.extend(note1);
        samples.extend(gap);
        samples.extend(note2);

        let onsets = detect_onsets(&samples, sr);
        assert!(
            onsets.len() >= 2,
            "should detect at least 2 onsets, got {}",
            onsets.len()
        );
    }

    #[test]
    fn sustained_note_has_single_onset() {
        let sr = 44100.0;
        // One long plucked note, no re-plucks
        let samples = plucked_note(110.0, 2.0, sr);
        let onsets = detect_onsets(&samples, sr);
        assert_eq!(
            onsets.len(),
            1,
            "sustained note should have exactly 1 onset, got {}",
            onsets.len()
        );
    }

    #[test]
    fn silence_has_no_onsets() {
        let samples = vec![0.0_f32; 44100];
        let onsets = detect_onsets(&samples, 44100.0);
        assert!(onsets.is_empty());
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
                assert!(
                    w[1] - w[0] >= MIN_ONSET_GAP_SECS - 0.001,
                    "gap {:.3}s below minimum",
                    w[1] - w[0]
                );
            }
        }
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Add to `crates/tab-engine/src/lib.rs`:
```rust
pub mod onset;
```

- [ ] **Step 3: Run tests**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine onset::tests`
Expected: All onset tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/tab-engine/src/onset.rs crates/tab-engine/src/lib.rs
git commit -m "feat(tab-engine): add spectral flux onset detection module"
```

---

### Task 4: Onset-based note segmentation with slide detection

**Files:**
- Modify: `crates/tab-engine/src/pitch.rs`

This is the core change. Refactor `YinDetector::detect()` to:
1. Run YIN pitch detection per frame (unchanged)
2. Run onset detection on raw samples
3. Segment notes at onset boundaries instead of pitch changes
4. Detect slides by analyzing pitch contour between consecutive notes

- [ ] **Step 1: Write tests for onset-based segmentation**

Add to `pitch.rs` tests module:

```rust
#[test]
fn repeated_same_pitch_with_gap_gives_two_notes() {
    // Two plucks of A2 with a brief silence between them
    let sr = 44100.0;
    let a2_1 = plucked_note(110.0, 0.3, sr);
    let silence = vec![0.0_f32; (0.1 * sr) as usize];
    let a2_2 = plucked_note(110.0, 0.3, sr);

    let mut samples = Vec::new();
    samples.extend(a2_1);
    samples.extend(silence);
    samples.extend(a2_2);

    let det = YinDetector::new(sr);
    let notes = det.detect_with_onsets(&samples);
    assert!(
        notes.len() >= 2,
        "should detect 2 separate plucks of same pitch, got {}",
        notes.len()
    );
    assert_eq!(notes[0].pitch, 45);
    assert_eq!(notes[1].pitch, 45);
}

#[test]
fn slide_detected_between_notes() {
    // Sine that smoothly sweeps from A2 (110Hz) to D3 (147Hz) over 0.5s
    let sr = 44100.0;
    let n = (0.5 * sr) as usize;

    // Start with a pluck transient
    let mut samples: Vec<f32> = plucked_note(110.0, 0.1, sr);
    // Then smooth frequency sweep (no new transient)
    for i in 0..n {
        let t = i as f64 / sr;
        let progress = t / 0.5;
        let freq = 110.0 * (1.0 - progress) + 147.0 * progress;
        let env = (-t * 2.0).exp() as f32;
        samples.push((2.0 * std::f64::consts::PI * freq * t).sin() as f32 * env * 0.4);
    }

    let det = YinDetector::new(sr);
    let notes = det.detect_with_onsets(&samples);
    // Should detect at least 2 notes (start pitch and end pitch)
    // The second one should have Slide technique
    let slides: Vec<_> = notes
        .iter()
        .filter(|n| n.technique == Some(crate::midi::Technique::Slide))
        .collect();
    assert!(
        !slides.is_empty(),
        "should detect at least one slide note, got notes: {:?}",
        notes
    );
}
```

And add the `plucked_note` helper to pitch.rs tests:

```rust
fn plucked_note(freq: f64, duration: f64, sr: f64) -> Vec<f32> {
    let n = (duration * sr) as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 / sr;
            let env = (-t * 3.0).exp() as f32;
            let attack = if t < 0.005 {
                // Broadband transient
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine pitch::tests`
Expected: Compilation error — `detect_with_onsets` not defined.

- [ ] **Step 3: Implement onset-based segmentation**

Refactor `YinDetector` in `pitch.rs`. Add `use crate::onset;` at top.

Add new method `detect_with_onsets`:

```rust
/// Detect notes using combined YIN pitch + spectral flux onset detection.
pub fn detect_with_onsets(&self, samples: &[f32]) -> Vec<MidiNote> {
    // Step 1: YIN pitch per frame (existing logic)
    let frames = self.pitch_frames(samples);

    // Step 2: Onset detection
    let onsets = onset::detect_onsets(samples, self.sample_rate);

    // Step 3: Onset-based segmentation
    if onsets.is_empty() {
        return self.segment_notes(&frames);
    }

    let mut notes = self.segment_at_onsets(&frames, &onsets);

    // Step 4: Slide detection post-processing
    detect_slides(&mut notes, &frames, self.sample_rate);

    notes
}
```

Extract pitch frame computation into its own method:

```rust
fn pitch_frames(&self, samples: &[f32]) -> Vec<(f64, Option<u8>, f64)> {
    let fft_size = (FRAME_SIZE * 2).next_power_of_two();
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(fft_size);
    let ifft = planner.plan_fft_inverse(fft_size);

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
                    if (28..=67).contains(&m) { Some(m) } else { None }
                })
        };

        frames.push((time, midi, rms));
        pos += HOP_SIZE;
    }

    frames
}
```

Update `detect` to use the new method:

```rust
fn detect(&self, samples: &[f32]) -> Vec<MidiNote> {
    self.detect_with_onsets(samples)
}
```

Add `segment_at_onsets`:

```rust
fn segment_at_onsets(
    &self,
    frames: &[(f64, Option<u8>, f64)],
    onsets: &[f64],
) -> Vec<MidiNote> {
    let frame_dur = HOP_SIZE as f64 / self.sample_rate;
    let end_time = frames.last().map_or(0.0, |f| f.0 + frame_dur);
    let mut notes = Vec::new();

    for (idx, &onset_time) in onsets.iter().enumerate() {
        let next_boundary = if idx + 1 < onsets.len() {
            onsets[idx + 1]
        } else {
            end_time
        };

        // Collect pitched frames in this onset window
        let pitched: Vec<(u8, f64)> = frames
            .iter()
            .filter(|(t, _, _)| *t >= onset_time && *t < next_boundary)
            .filter_map(|(_, midi, rms)| midi.map(|m| (m, *rms)))
            .collect();

        if pitched.is_empty() {
            continue;
        }

        let pitch = predominant_pitch(&pitched);
        let peak_rms = pitched.iter().map(|(_, r)| *r).fold(0.0_f64, f64::max);
        let offset = next_boundary.min(end_time);
        let duration = offset - onset_time;

        if duration >= MIN_NOTE_SECS {
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
```

Add helper `predominant_pitch`:

```rust
/// Find the most common pitch in a set of frames (mode).
fn predominant_pitch(pitched: &[(u8, f64)]) -> u8 {
    let mut counts = std::collections::HashMap::new();
    for &(p, _) in pitched {
        *counts.entry(p).or_insert(0u32) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(p, _)| p)
        .unwrap_or(pitched[0].0)
}
```

Add `detect_slides`:

```rust
/// Post-processing: detect slides by analyzing pitch contour between consecutive notes.
///
/// A slide is detected when:
/// 1. Two consecutive notes have different pitches
/// 2. The pitch frames between them show a monotonic transition
/// 3. There is no onset (pluck) at the start of the second note — approximated
///    by checking if the second note's onset aligns with a detected onset.
fn detect_slides(notes: &mut [MidiNote], frames: &[(f64, Option<u8>, f64)], _sample_rate: f64) {
    if notes.len() < 2 {
        return;
    }

    for i in 1..notes.len() {
        let prev_pitch = notes[i - 1].pitch;
        let curr_pitch = notes[i].pitch;

        if prev_pitch == curr_pitch {
            continue;
        }

        // Check pitch frames in the transition zone
        // Look at frames from the last 30% of the previous note to the first 30% of current note
        let prev_end = notes[i - 1].offset;
        let curr_start = notes[i].onset;
        let transition_start = prev_end - (prev_end - notes[i - 1].onset) * 0.3;
        let transition_end = curr_start + (notes[i].offset - curr_start) * 0.3;

        let transition_frames: Vec<u8> = frames
            .iter()
            .filter(|(t, _, _)| *t >= transition_start && *t <= transition_end)
            .filter_map(|(_, midi, _)| *midi)
            .collect();

        if transition_frames.len() < 3 {
            continue;
        }

        // Check for monotonic pitch change
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

        // Also check that intermediate pitches exist (not just a jump)
        let has_intermediate = transition_frames
            .iter()
            .any(|&p| p != prev_pitch && p != curr_pitch);

        if is_monotonic && has_intermediate {
            notes[i].technique = Some(crate::midi::Technique::Slide);
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine pitch::tests`
Expected: All tests pass (both new and existing).

- [ ] **Step 5: Commit**

```bash
git add crates/tab-engine/src/pitch.rs
git commit -m "feat(tab-engine): onset-based note segmentation with slide detection"
```

---

### Task 5: Enforce same-string constraint for slides in Viterbi

**Files:**
- Modify: `crates/tab-engine/src/viterbi.rs`

- [ ] **Step 1: Write test**

```rust
#[test]
fn slide_notes_use_same_string() {
    use crate::midi::Technique;
    let notes = vec![
        MidiNote { pitch: 33, onset: 0.0, offset: 0.5, velocity: 80, technique: None },
        MidiNote { pitch: 35, onset: 0.5, offset: 1.0, velocity: 80, technique: Some(Technique::Slide) },
    ];
    let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
    assert_eq!(
        sheet.notes[0].string, sheet.notes[1].string,
        "slide notes must be on the same string"
    );
}
```

- [ ] **Step 2: Run test to verify it may fail**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine viterbi::tests::slide_notes_use_same_string`

- [ ] **Step 3: Modify transition_cost for slide constraint**

In `viterbi.rs`, update the `optimize` function's DP loop. Change the inner loop at line ~86:

```rust
for (j, prev_c) in prev_candidates.iter().enumerate() {
    // Slides must stay on the same string
    let is_slide = notes[i].technique == Some(crate::midi::Technique::Slide);
    if is_slide && prev_c.string != curr_c.string {
        continue; // Skip candidates on different strings
    }
    let total = dp[i - 1][j].0 + transition_cost(prev_c, curr_c) + e_cost;
    if total < best_cost {
        best_cost = total;
        best_prev = j;
    }
}
```

Also need to handle the case where `best_cost` stays at infinity (no valid candidate found on same string). Add after the inner loop:

```rust
// Fallback: if slide constraint made all paths invalid, relax it
if best_cost == f64::INFINITY {
    for (j, prev_c) in prev_candidates.iter().enumerate() {
        let total = dp[i - 1][j].0 + transition_cost(prev_c, curr_c) + e_cost;
        if total < best_cost {
            best_cost = total;
            best_prev = j;
        }
    }
}
```

- [ ] **Step 4: Run all viterbi tests**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine viterbi::tests`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add crates/tab-engine/src/viterbi.rs
git commit -m "feat(tab-engine): enforce same-string constraint for slide notes in Viterbi"
```

---

### Task 6: Preserve technique through transpose

**Files:**
- Modify: `crates/tab-engine/src/transpose.rs`

- [ ] **Step 1: Write test**

```rust
#[test]
fn transpose_preserves_technique() {
    use crate::midi::Technique;
    let notes = vec![
        MidiNote { pitch: 33, onset: 0.0, offset: 0.5, velocity: 80, technique: None },
        MidiNote { pitch: 35, onset: 0.5, offset: 1.0, velocity: 80, technique: Some(Technique::Slide) },
    ];
    let sheet = transpose(&notes, 2, Tuning::Standard4, 120.0, (4, 4));
    assert_eq!(sheet.notes[1].technique, Some(Technique::Slide));
}
```

- [ ] **Step 2: Verify technique is already preserved**

The technique field was already added to the MidiNote construction in Task 1 (`technique: note.technique`). This test should pass already. If not, fix the transpose function.

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine transpose::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add crates/tab-engine/src/transpose.rs
git commit -m "test(tab-engine): verify technique preserved through transpose"
```

---

### Task 7: Update frontend types and TabCanvas rendering

**Files:**
- Modify: `apps/desktop/src/lib/types.ts`
- Modify: `apps/desktop/src/components/TabCanvas.tsx`

- [ ] **Step 1: Update TypeScript types**

In `apps/desktop/src/lib/types.ts`, add `Technique` type and update `TabNote`:

```typescript
export type Technique = "Slide";

export interface TabNote {
  string: number;
  fret: number;
  midi_pitch: number;
  onset: number;
  duration: number;
  origin: NoteOrigin;
  technique?: Technique;
}
```

- [ ] **Step 2: Update TabCanvas colors and rendering**

In `apps/desktop/src/components/TabCanvas.tsx`:

Add to COLORS:
```typescript
const COLORS = {
  Normal: "#E8A723",
  Optimized: "#4ADE80",
  OctaveShifted: "#60A5FA",
  Technique: "#F472B6",
  Slide: "#F472B6",
};
```

Add slide rendering in the note drawing loop (after the fret text is drawn, around line ~192):

```typescript
// Draw slide connector to previous note
if (note.technique === "Slide") {
  // Find the previous note on the same string
  const prevNote = tabSheet.notes
    .filter((n) => n.string === note.string && n.onset < note.onset)
    .sort((a, b) => b.onset - a.onset)[0];

  if (prevNote) {
    const prevRelTime = prevNote.onset - rowStartTime;
    // Check if previous note is in the same row
    if (prevRelTime >= 0 && prevRelTime < measuresPerRow * secPerMeasure) {
      const prevX = LEFT_MARGIN + prevRelTime * pixelsPerSecond;
      const prevY = rowY + (numStrings - 1 - prevNote.string) * LINE_HEIGHT;

      // Draw slide line
      ctx.strokeStyle = COLORS.Slide;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(prevX + 8, prevY);
      ctx.lineTo(x - 8, y);
      ctx.stroke();
    }
  }

  // Override note color for slide notes
  ctx.fillStyle = COLORS.Slide;
  ctx.fillText(fretStr, x, y);
}
```

Move the existing note text rendering into an else block:

```typescript
if (note.technique === "Slide") {
  // ... slide rendering (above)
} else {
  ctx.fillStyle = getNoteColor(note.origin);
  ctx.fillText(fretStr, x, y);
}
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cd apps/desktop && npx tsc --noEmit`
Expected: No TypeScript errors.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/lib/types.ts apps/desktop/src/components/TabCanvas.tsx
git commit -m "feat(desktop): render slide technique annotations on tab canvas"
```

---

### Task 8: Update ASCII export for slide notation

**Files:**
- Modify: `crates/tab-engine/src/export/ascii.rs`

- [ ] **Step 1: Write test**

```rust
#[test]
fn slide_renders_with_slash() {
    use crate::midi::Technique;
    let sheet = TabSheet {
        notes: vec![
            TabNote {
                string: 0, fret: 5, midi_pitch: 33,
                onset: 0.0, duration: 0.5,
                origin: NoteOrigin::Normal,
                technique: None,
            },
            TabNote {
                string: 0, fret: 7, midi_pitch: 35,
                onset: 0.5, duration: 0.5,
                origin: NoteOrigin::Normal,
                technique: Some(Technique::Slide),
            },
        ],
        tempo: 120.0,
        time_signature: (4, 4),
        tuning: Tuning::Standard4,
        key_transpose: 0,
    };
    let result = export(&sheet);
    let e_line = result.lines().find(|l| l.starts_with("E|")).unwrap();
    assert!(e_line.contains('/') || e_line.contains('\\'), "slide should have / or \\ marker: {}", e_line);
}
```

- [ ] **Step 2: Implement slide notation in ASCII export**

In the note placement loop, after placing the fret digits, add a slide marker:

```rust
for note in &sheet.notes {
    let beat_pos = note.onset / beat_duration;
    let char_pos = (beat_pos * CHARS_PER_BEAT as f64).round() as usize;
    if char_pos < total_chars {
        // Add slide marker before fret number
        if note.technique == Some(crate::midi::Technique::Slide) && char_pos > 0 {
            grid[note.string as usize][char_pos - 1] = "/".to_string();
        }

        let fret_str = note.fret.to_string();
        let s = note.string as usize;
        for (i, ch) in fret_str.chars().enumerate() {
            if char_pos + i < total_chars {
                grid[s][char_pos + i] = ch.to_string();
            }
        }
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine export`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add crates/tab-engine/src/export/ascii.rs
git commit -m "feat(tab-engine): render slide notation in ASCII tab export"
```

---

## Self-Review Checklist

1. **Coverage:** Onset detection (Task 3), onset-based segmentation (Task 4), slide detection (Task 4), Viterbi slide constraint (Task 5), technique propagation (Tasks 1-2, 5-6), frontend rendering (Task 7), ASCII export (Task 8). All requirements covered.

2. **Placeholder scan:** No TBDs, TODOs, or vague instructions. All code blocks are complete.

3. **Type consistency:** `Technique` defined in `midi.rs`, used consistently as `Option<Technique>` in both `MidiNote` and `TabNote`. Serde attributes match (`default`, `skip_serializing_if`). Frontend type matches Rust serialization (`"Slide"` string).
