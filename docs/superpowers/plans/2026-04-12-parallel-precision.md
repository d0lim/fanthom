# Parallel Precision Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Parallelize pitch/onset detection at frame level with rayon, reduce hop sizes for 2x precision, and add confidence-weighted pitch estimation.

**Architecture:** Use `rayon::par_iter::map_init()` for thread-local FFT buffer reuse across parallel frame processing. Split onset spectral flux into parallel magnitude computation + sequential differencing. Run pitch and onset detection concurrently via `rayon::join()`. Reduce HOP_SIZE (2048→1024) and ONSET_HOP_SIZE (256→128) for doubled timing precision.

**Tech Stack:** Rust, rayon 1.x, rustfft 6.x

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/tab-engine/Cargo.toml` | Modify | Add `rayon = "1"` |
| `Cargo.toml` (workspace root) | Modify | Add rayon dev profile opt-level |
| `crates/tab-engine/src/pitch.rs` | Modify | PitchFrame struct, confidence, parallel pitch_frames, HOP reduction, weighted pitch |
| `crates/tab-engine/src/onset.rs` | Modify | Parallel magnitudes, HOP reduction |

---

### Task 1: Add rayon, PitchFrame struct, confidence from YIN

**Files:**
- Modify: `crates/tab-engine/Cargo.toml`
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/tab-engine/src/pitch.rs`

This task adds the rayon dependency, introduces the `PitchFrame` struct to replace the `(f64, Option<u8>, f64)` tuple, adds a confidence return value from YIN, and updates all consumers.

- [ ] **Step 1: Add rayon dependency**

In `crates/tab-engine/Cargo.toml`, add:
```toml
rayon = "1"
```

In root `Cargo.toml`, add to dev profile optimizations:
```toml
[profile.dev.package.rayon]
opt-level = 2
```

- [ ] **Step 2: Write test for confidence value**

Add to pitch.rs tests:
```rust
#[test]
fn pitch_frame_has_confidence() {
    let samples = sine_wave(110.0, 1.0, 44100.0);
    let det = YinDetector::new(44100.0);
    let frames = det.pitch_frames(&samples);
    let pitched: Vec<_> = frames.iter().filter(|f| f.midi.is_some()).collect();
    assert!(!pitched.is_empty());
    // Confidence should be high for a clean sine wave
    for f in &pitched {
        assert!(f.confidence > 0.5, "confidence {:.2} should be > 0.5 for clean sine", f.confidence);
    }
}
```

- [ ] **Step 3: Define PitchFrame struct and update pitch_frames()**

Add PitchFrame struct in pitch.rs (after the constants, before the public API):
```rust
#[derive(Clone, Copy, Debug)]
struct PitchFrame {
    time: f64,
    midi: Option<u8>,
    rms: f64,
    confidence: f64,
}
```

Create `yin_pitch_fft_with_confidence` — a copy of `yin_pitch_fft` that also returns the confidence value (1.0 - cmnd[best_tau]). The only change is the return type and the final return statement:

```rust
fn yin_pitch_fft_with_confidence(
    &self,
    frame: &[f32],
    fft: &Arc<dyn rustfft::Fft<f64>>,
    ifft: &Arc<dyn rustfft::Fft<f64>>,
    buf_half: &mut [Complex<f64>],
    buf_full: &mut [Complex<f64>],
) -> (Option<f64>, f64) {
    // ... identical YIN logic as yin_pitch_fft ...
    // At the end, instead of:
    //   Some(self.sample_rate / refined)
    // Return:
    //   (Some(self.sample_rate / refined), 1.0 - cmnd[tau])
    // And for early None returns, return (None, 0.0)
}
```

Update `pitch_frames()` to return `Vec<PitchFrame>` and call the new method:

```rust
fn pitch_frames(&self, samples: &[f32]) -> Vec<PitchFrame> {
    let fft_size = (FRAME_SIZE * 2).next_power_of_two();
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(fft_size);
    let ifft = planner.plan_fft_inverse(fft_size);

    let mut buf_half = vec![Complex::new(0.0, 0.0); fft_size];
    let mut buf_full = vec![Complex::new(0.0, 0.0); fft_size];

    let mut frames = Vec::new();
    let mut pos = 0;

    while pos + FRAME_SIZE <= samples.len() {
        let frame = &samples[pos..pos + FRAME_SIZE];
        let time = pos as f64 / self.sample_rate;
        let rms = rms_energy(frame);

        let (midi, confidence) = if rms < SILENCE_RMS {
            (None, 0.0)
        } else {
            let (freq, conf) = self.yin_pitch_fft_with_confidence(
                frame, &fft, &ifft, &mut buf_half, &mut buf_full,
            );
            let midi = freq.and_then(|f| {
                let m = freq_to_midi(f);
                if (28..=67).contains(&m) { Some(m) } else { None }
            });
            (midi, conf)
        };

        frames.push(PitchFrame { time, midi, rms, confidence });
        pos += HOP_SIZE;
    }

    frames
}
```

- [ ] **Step 4: Update all consumers of PitchFrame**

Update function signatures and field accesses:

**`segment_notes()`**: Change parameter from `&[(f64, Option<u8>, f64)]` to `&[PitchFrame]`. Update field access: `f.0` → `f.time`, `f.1` → `f.midi` (or destructure via pattern), `f.2` → `f.rms`. The `for &(time, midi, rms)` pattern becomes `for frame in frames` with `frame.time`, `frame.midi`, `frame.rms`.

**`segment_at_onsets()`**: Same parameter change. Update `frames.iter().filter(|(t, _, _)| ...)` to `frames.iter().filter(|f| f.time >= onset_time && f.time < next_boundary)`. The pitched collection becomes `Vec<(u8, f64, f64)>` (midi, rms, confidence): `.filter_map(|f| f.midi.map(|m| (m, f.rms, f.confidence)))`.

**`detect_slides()`**: Same parameter change. Update filter patterns.

**`slide_detection_marks_smooth_transition` test**: Update the frame construction from tuples to PitchFrame structs:
```rust
let frames: Vec<PitchFrame> = vec![
    PitchFrame { time: 0.0, midi: Some(45), rms: 0.1, confidence: 0.9 },
    PitchFrame { time: 0.046, midi: Some(45), rms: 0.09, confidence: 0.9 },
    // ... etc
];
```

- [ ] **Step 5: Remove old yin_pitch_fft**

Delete the old `yin_pitch_fft` method. Rename `yin_pitch_fft_with_confidence` to `yin_pitch_fft`. Update the signature and all call sites.

- [ ] **Step 6: Run tests**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine`
Expected: All tests pass (49 existing + 1 new = 50).

- [ ] **Step 7: Commit**

```bash
git add crates/tab-engine/Cargo.toml Cargo.toml crates/tab-engine/src/pitch.rs
git commit -m "feat(tab-engine): add rayon, PitchFrame struct, confidence from YIN"
```

---

### Task 2: Parallelize pitch_frames + reduce HOP_SIZE

**Files:**
- Modify: `crates/tab-engine/src/pitch.rs`

- [ ] **Step 1: Reduce HOP_SIZE**

Change constant:
```rust
const HOP_SIZE: usize = 1024; // ~23 ms — 2x precision (was 2048)
```

- [ ] **Step 2: Parallelize pitch_frames with rayon**

Add import at top of pitch.rs:
```rust
use rayon::prelude::*;
```

Rewrite `pitch_frames()`:

```rust
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
```

Key points:
- `map_init` allocates FFT buffers once per rayon worker thread, not per frame
- FFT plans (`fft`, `ifft`) are `Arc<dyn Fft<f64>>` which is Send+Sync — shared across threads
- `self` is `&YinDetector` (only primitive fields) — Send+Sync
- `samples` is `&[f32]` — Send+Sync
- Results are collected in order by rayon's parallel iterator

- [ ] **Step 3: Run tests**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine`
Expected: All 50 tests pass. HOP_SIZE change may affect timing precision in some tests — existing assertions use tolerant ranges so should be fine.

- [ ] **Step 4: Commit**

```bash
git add crates/tab-engine/src/pitch.rs
git commit -m "feat(tab-engine): parallelize pitch_frames with rayon, reduce HOP to 1024"
```

---

### Task 3: Parallelize onset + reduce ONSET_HOP_SIZE + rayon::join

**Files:**
- Modify: `crates/tab-engine/src/onset.rs`
- Modify: `crates/tab-engine/src/pitch.rs`

- [ ] **Step 1: Reduce ONSET_HOP_SIZE**

In onset.rs, change:
```rust
const ONSET_HOP_SIZE: usize = 128; // ~2.9 ms — 2x precision (was 256)
```

- [ ] **Step 2: Parallelize spectral flux magnitude computation**

Add import:
```rust
use rayon::prelude::*;
```

Rewrite `spectral_flux()` to split into parallel magnitude + sequential differencing:

```rust
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

    // Phase 2: Sequential flux differencing (O(n), lightweight)
    let mut flux = vec![0.0]; // First frame has no previous → 0 flux
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
```

- [ ] **Step 3: Update peak_pick for new HOP size**

In `peak_pick()`, the `min_gap_frames` calculation already uses `ONSET_HOP_SIZE` dynamically — no change needed. But verify:
```rust
let min_gap_frames = (MIN_ONSET_GAP_SECS * sample_rate / ONSET_HOP_SIZE as f64).ceil() as usize;
```
With ONSET_HOP_SIZE=128, min_gap_frames = ceil(0.05 * 44100 / 128) = ceil(17.2) = 18. Fine.

- [ ] **Step 4: Add rayon::join for concurrent pitch + onset**

In pitch.rs, update `detect_with_onsets()`:

```rust
fn detect_with_onsets(&self, samples: &[f32]) -> Vec<MidiNote> {
    let (frames, onsets) = rayon::join(
        || self.pitch_frames(samples),
        || onset::detect_onsets(samples, self.sample_rate),
    );

    if onsets.is_empty() {
        return self.segment_notes(&frames);
    }

    let mut notes = self.segment_at_onsets(&frames, &onsets);
    detect_slides(&mut notes, &frames, self.sample_rate);
    notes
}
```

- [ ] **Step 5: Run tests**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/tab-engine/src/onset.rs crates/tab-engine/src/pitch.rs
git commit -m "feat(tab-engine): parallelize onset detection, reduce HOP to 128, rayon::join"
```

---

### Task 4: Confidence-weighted predominant pitch

**Files:**
- Modify: `crates/tab-engine/src/pitch.rs`

- [ ] **Step 1: Write test for confidence weighting**

```rust
#[test]
fn confidence_weighted_pitch_prefers_confident_frames() {
    // 3 frames say pitch 45 with low confidence, 2 frames say pitch 46 with high confidence
    // Confidence-weighted should prefer 46
    let pitched = vec![
        (45_u8, 0.1_f64, 0.2_f64), // low confidence
        (45, 0.1, 0.2),
        (45, 0.1, 0.2),
        (46, 0.1, 0.9), // high confidence
        (46, 0.1, 0.9),
    ];
    let result = predominant_pitch(&pitched);
    assert_eq!(result, 46, "should prefer the higher-confidence pitch");
}
```

- [ ] **Step 2: Update predominant_pitch signature and implementation**

Change from simple mode counting to confidence-weighted sum:

```rust
fn predominant_pitch(pitched: &[(u8, f64, f64)]) -> u8 {
    let mut weights = std::collections::HashMap::new();
    for &(p, _, confidence) in pitched {
        *weights.entry(p).or_insert(0.0_f64) += confidence;
    }
    weights
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(p, _)| p)
        .unwrap_or(pitched[0].0)
}
```

- [ ] **Step 3: Update segment_at_onsets to pass confidence**

The pitched collection in `segment_at_onsets` should already be `Vec<(u8, f64, f64)>` from Task 1 (midi, rms, confidence). Verify the `predominant_pitch` call passes this correctly.

- [ ] **Step 4: Run tests**

Run: `cd /Users/limdongyoung0/Develop/d0lim/fanthom && cargo test -p tab-engine`
Expected: All tests pass (51 total).

- [ ] **Step 5: Commit**

```bash
git add crates/tab-engine/src/pitch.rs
git commit -m "feat(tab-engine): confidence-weighted predominant pitch estimation"
```

---

## Self-Review

1. **Spec coverage:** Rayon parallelization (Tasks 2, 3), HOP reduction (Tasks 2, 3), confidence from YIN (Task 1), PitchFrame struct (Task 1), rayon::join (Task 3), confidence-weighted pitch (Task 4). All spec requirements covered.

2. **Placeholder scan:** No TBDs. All code blocks complete. The `yin_pitch_fft_with_confidence` implementation references "identical YIN logic" — this is explained in context (copy the function, change return type and final return).

3. **Type consistency:** `PitchFrame` defined in Task 1, used consistently in Tasks 2-4. `predominant_pitch` signature `&[(u8, f64, f64)]` used in Tasks 1 and 4. `yin_pitch_fft` returns `(Option<f64>, f64)` in Task 1, consumed in Tasks 1-2.
