# Frame-Level Parallelization + Precision Improvements

## Goal

Parallelize pitch detection and onset detection at frame level using rayon, and invest the speed gains into doubled precision (smaller hop sizes) and confidence-weighted pitch estimation.

## Architecture

### Parallelization Strategy: Frame-Level (No Chunking)

Each audio frame reads independently from a shared immutable sample buffer. No seam/stitching issues.

- `pitch_frames()`: parallelize per-frame FFT+YIN with `rayon::par_iter::map_init()` for thread-local buffer reuse
- `onset::spectral_flux()`: parallelize per-frame FFT+magnitude, then sequential flux differencing
- `pitch_frames()` and `onset::detect_onsets()` run concurrently via `rayon::join()`

### Precision Improvements

1. **HOP_SIZE**: 2048 → 1024 (~46ms → ~23ms pitch timing precision)
2. **ONSET_HOP_SIZE**: 256 → 128 (~5.8ms → ~2.9ms onset timing precision)
3. **Confidence-weighted pitch**: YIN returns CMND confidence per frame; `segment_at_onsets()` uses confidence-weighted voting instead of simple mode

### Expected Outcome

- Frame count doubles (2x) from HOP reduction
- Parallelization on 4-8 cores provides 3-6x speedup
- Net result: faster or equal speed with 2x precision + better pitch accuracy

## Detailed Changes

### 1. Cargo.toml

Add `rayon = "1"` to tab-engine dependencies.

### 2. pitch.rs — pitch_frames() parallelization

Current: sequential loop, shared mutable FFT buffers.

New:
```rust
use rayon::prelude::*;

fn pitch_frames(&self, samples: &[f32]) -> Vec<PitchFrame> {
    let fft_size = (FRAME_SIZE * 2).next_power_of_two();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let ifft = planner.plan_fft_inverse(fft_size);

    let num_frames = samples.len().saturating_sub(FRAME_SIZE) / HOP_SIZE + 1;

    (0..num_frames)
        .into_par_iter()
        .map_init(
            || (vec![Complex::new(0.0, 0.0); fft_size], vec![Complex::new(0.0, 0.0); fft_size]),
            |(buf_half, buf_full), i| {
                let pos = i * HOP_SIZE;
                let frame = &samples[pos..pos + FRAME_SIZE];
                let time = pos as f64 / self.sample_rate;
                let rms = rms_energy(frame);
                // ... YIN with confidence
            },
        )
        .collect()
}
```

Key: `map_init()` allocates FFT buffers once per rayon worker thread, not per frame.

### 3. pitch.rs — HOP_SIZE reduction

```rust
const HOP_SIZE: usize = 1024; // ~23ms (was 2048)
```

FRAME_SIZE stays at 8192 (required for low-frequency bass detection).

### 4. pitch.rs — Confidence from YIN

Modify `yin_pitch_fft()` to return `(Option<f64>, f64)` — (frequency, confidence).

Confidence = `1.0 - cmnd[best_tau]`. Range: 0.0 (no confidence) to ~1.0 (very confident).

### 5. pitch.rs — PitchFrame struct

Replace tuple `(f64, Option<u8>, f64)` with a named struct for clarity:

```rust
struct PitchFrame {
    time: f64,
    midi: Option<u8>,
    rms: f64,
    confidence: f64,
}
```

### 6. pitch.rs — Confidence-weighted predominant pitch

Replace simple mode with confidence-weighted sum:

```rust
fn predominant_pitch(pitched: &[(u8, f64, f64)]) -> u8 {
    let mut weights: HashMap<u8, f64> = HashMap::new();
    for &(pitch, _rms, confidence) in pitched {
        *weights.entry(pitch).or_insert(0.0) += confidence;
    }
    weights.into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(p, _)| p)
        .unwrap_or(pitched[0].0)
}
```

### 7. pitch.rs — detect_with_onsets() concurrent execution

```rust
fn detect_with_onsets(&self, samples: &[f32]) -> Vec<MidiNote> {
    let (frames, onsets) = rayon::join(
        || self.pitch_frames(samples),
        || onset::detect_onsets(samples, self.sample_rate),
    );
    // ... segment + slides
}
```

### 8. onset.rs — Parallel magnitude computation

Split spectral_flux into two phases:
1. `compute_magnitudes()` — parallel FFT+magnitude per frame via rayon
2. Sequential flux differencing + peak_pick (unchanged, lightweight)

### 9. onset.rs — HOP reduction

```rust
const ONSET_HOP_SIZE: usize = 128; // ~2.9ms (was 256)
```

ONSET_FRAME_SIZE stays at 1024.

## Files Changed

| File | Change |
|------|--------|
| `crates/tab-engine/Cargo.toml` | Add `rayon = "1"` |
| `crates/tab-engine/src/pitch.rs` | Parallel pitch_frames, HOP reduction, PitchFrame struct, confidence, weighted pitch, rayon::join |
| `crates/tab-engine/src/onset.rs` | Parallel magnitudes, HOP reduction |

## Testing

- All 49 existing tests must still pass
- Add benchmark comparing sequential vs parallel (optional, criterion)
- Verify numerical equivalence: parallel pitch detection produces same results as sequential
