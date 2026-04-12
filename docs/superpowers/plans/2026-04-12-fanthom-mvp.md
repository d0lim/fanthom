# Fanthom MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri desktop app that extracts bass tracks from YouTube videos and generates optimized bass guitar tablature with Viterbi DP fingering optimization.

**Architecture:** Tauri Command-centric monorepo. Rust core orchestrates a Python sidecar (Demucs + basic-pitch) for AI workloads and an in-process tab-engine crate for MIDI-to-tab conversion, Viterbi optimization, transpose, and export. React frontend renders tabs on Canvas 2D.

**Tech Stack:** Tauri v2, React + Vite + Tailwind, Rust (tab-engine crate, rusqlite, rmp-serde), Python (yt-dlp, Demucs, basic-pitch), mise, pnpm

---

## File Map

### `crates/tab-engine/`
| File | Responsibility |
|------|---------------|
| `src/lib.rs` | Public API re-exports |
| `src/midi.rs` | `MidiNote` struct, JSON deserialization from basic-pitch output |
| `src/tab.rs` | `TabNote`, `TabSheet`, `Tuning`, `NoteOrigin` types + `pitch_to_candidates()` + greedy mapping |
| `src/viterbi.rs` | Cost functions + Viterbi DP optimizer |
| `src/transpose.rs` | Transpose + octave correction |
| `src/export/mod.rs` | Export module re-exports |
| `src/export/ascii.rs` | ASCII text tab export |
| `src/export/musicxml.rs` | MusicXML export |

### `python/ai-pipeline/`
| File | Responsibility |
|------|---------------|
| `main.py` | stdin/stdout JSON Lines dispatcher |
| `protocol.py` | Shared message helpers (progress, result, error) |
| `extract.py` | yt-dlp audio extraction |
| `separate.py` | Demucs source separation |
| `transcribe.py` | basic-pitch pitch detection |

### `apps/desktop/src-tauri/`
| File | Responsibility |
|------|---------------|
| `src/main.rs` | Tauri app entry, plugin registration |
| `src/commands/mod.rs` | Command module re-exports |
| `src/commands/pipeline.rs` | `process_url` command — full pipeline orchestration |
| `src/commands/tab.rs` | `transpose`, `toggle_optimization`, `export_tab` commands |
| `src/sidecar.rs` | Python sidecar spawn/communication |
| `src/db.rs` | SQLite schema init, song/tab CRUD |
| `src/state.rs` | Tauri managed state (DB handle, app dirs) |

### `apps/desktop/src/`
| File | Responsibility |
|------|---------------|
| `main.tsx` | React entry |
| `App.tsx` | Root layout, context provider |
| `state.ts` | AppState type, reducer, context |
| `components/UrlInput.tsx` | URL input bar + file upload |
| `components/PipelineProgress.tsx` | 4-step progress display |
| `components/TabCanvas.tsx` | Canvas 2D tab rendering |
| `components/TransposeControl.tsx` | Semitone slider |
| `components/OptimizeToggle.tsx` | Viterbi toggle |
| `components/ExportMenu.tsx` | MusicXML / ASCII export |
| `lib/tauri.ts` | Typed invoke/listen wrappers |
| `lib/types.ts` | Shared TypeScript types (mirroring Rust) |

---

## Task 1: Project Scaffolding

**Files:**
- Create: `.mise.toml`, `Cargo.toml` (workspace), `package.json` (root), `pnpm-workspace.yaml`, `LICENSE`, `.gitignore`

- [ ] **Step 1: Initialize git repository**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom
git init
```

- [ ] **Step 2: Create `.mise.toml`**

```toml
[tools]
node = "22"
python = "3.11"
rust = "1.83"

[env]
PYTHONPATH = "./python/ai-pipeline"
```

- [ ] **Step 3: Create `.gitignore`**

```gitignore
# Rust
target/
*.swp

# Node
node_modules/
dist/

# Python
__pycache__/
*.pyc
.venv/
*.egg-info/

# Tauri
apps/desktop/src-tauri/target/

# IDE
.idea/
.vscode/
*.code-workspace

# OS
.DS_Store
Thumbs.db

# App data
*.wav
*.mp3
```

- [ ] **Step 4: Create `LICENSE` (MIT)**

```
MIT License

Copyright (c) 2026 Effize

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 5: Create Cargo workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "crates/tab-engine",
    "apps/desktop/src-tauri",
]
```

- [ ] **Step 6: Create pnpm workspace files**

`package.json`:
```json
{
  "name": "fanthom",
  "private": true,
  "scripts": {
    "dev": "pnpm --filter desktop tauri dev",
    "build": "pnpm --filter desktop tauri build",
    "lint": "pnpm --filter desktop lint",
    "typecheck": "pnpm --filter desktop typecheck"
  }
}
```

`pnpm-workspace.yaml`:
```yaml
packages:
  - "apps/*"
```

- [ ] **Step 7: Install runtimes and verify**

```bash
mise install
node --version   # v22.x
python --version # 3.11.x
rustc --version  # 1.83.x
```

- [ ] **Step 8: Commit**

```bash
git add .mise.toml .gitignore LICENSE Cargo.toml package.json pnpm-workspace.yaml
git commit -m "chore: initialize monorepo with mise, Cargo workspace, pnpm workspace"
```

---

## Task 2: Tab Engine — Data Model

**Files:**
- Create: `crates/tab-engine/Cargo.toml`
- Create: `crates/tab-engine/src/lib.rs`
- Create: `crates/tab-engine/src/midi.rs`
- Create: `crates/tab-engine/src/tab.rs`

- [ ] **Step 1: Create `crates/tab-engine/Cargo.toml`**

```toml
[package]
name = "tab-engine"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Bass guitar tab generation engine with Viterbi DP fingering optimization"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Write tests for MidiNote deserialization**

Create `crates/tab-engine/src/midi.rs`:

```rust
use serde::Deserialize;

/// A single note extracted from basic-pitch output.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MidiNote {
    pub pitch: u8,
    pub onset: f64,
    pub offset: f64,
    pub velocity: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_midi_note_from_json() {
        let json = r#"{"pitch": 40, "onset": 1.5, "offset": 2.0, "velocity": 100}"#;
        let note: MidiNote = serde_json::from_str(json).unwrap();
        assert_eq!(note.pitch, 40);
        assert_eq!(note.onset, 1.5);
        assert_eq!(note.offset, 2.0);
        assert_eq!(note.velocity, 100);
    }

    #[test]
    fn deserialize_midi_note_sequence() {
        let json = r#"[
            {"pitch": 28, "onset": 0.0, "offset": 0.5, "velocity": 80},
            {"pitch": 33, "onset": 0.5, "offset": 1.0, "velocity": 90}
        ]"#;
        let notes: Vec<MidiNote> = serde_json::from_str(json).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].pitch, 28);
        assert_eq!(notes[1].pitch, 33);
    }
}
```

- [ ] **Step 3: Write tab types and `pitch_to_candidates()`**

Create `crates/tab-engine/src/tab.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Bass guitar tuning. Open string MIDI pitches.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Tuning {
    Standard4, // E=28, A=33, D=38, G=43
}

impl Tuning {
    /// Returns open-string MIDI pitches from lowest to highest.
    pub fn open_pitches(&self) -> &[u8] {
        match self {
            Tuning::Standard4 => &[28, 33, 38, 43],
        }
    }

    pub fn num_strings(&self) -> u8 {
        self.open_pitches().len() as u8
    }
}

/// How a note ended up at its position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NoteOrigin {
    Normal,
    Optimized,
    OctaveShifted(i8),
}

/// A single note placed on the fretboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabNote {
    pub string: u8,
    pub fret: u8,
    pub midi_pitch: u8,
    pub onset: f64,
    pub duration: f64,
    pub origin: NoteOrigin,
}

/// A complete tab sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSheet {
    pub notes: Vec<TabNote>,
    pub tempo: f64,
    pub time_signature: (u8, u8),
    pub tuning: Tuning,
    pub key_transpose: i8,
}

/// A candidate fretboard position for a given pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub string: u8,
    pub fret: u8,
}

const MAX_FRET: u8 = 24;

/// Returns all valid (string, fret) positions for a MIDI pitch on the given tuning.
pub fn pitch_to_candidates(pitch: u8, tuning: Tuning) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for (i, &open) in tuning.open_pitches().iter().enumerate() {
        if pitch >= open && pitch - open <= MAX_FRET {
            candidates.push(Candidate {
                string: i as u8,
                fret: pitch - open,
            });
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e_open_string_has_one_candidate() {
        // E1 (MIDI 28) can only be played on E string open
        let candidates = pitch_to_candidates(28, Tuning::Standard4);
        assert_eq!(candidates, vec![Candidate { string: 0, fret: 0 }]);
    }

    #[test]
    fn a_open_has_two_candidates() {
        // A1 (MIDI 33) = E string 5th fret OR A string open
        let candidates = pitch_to_candidates(33, Tuning::Standard4);
        assert_eq!(candidates, vec![
            Candidate { string: 0, fret: 5 },
            Candidate { string: 1, fret: 0 },
        ]);
    }

    #[test]
    fn middle_note_has_multiple_candidates() {
        // D2 (MIDI 38) = E:10, A:5, D:0
        let candidates = pitch_to_candidates(38, Tuning::Standard4);
        assert_eq!(candidates, vec![
            Candidate { string: 0, fret: 10 },
            Candidate { string: 1, fret: 5 },
            Candidate { string: 2, fret: 0 },
        ]);
    }

    #[test]
    fn high_note_on_g_string_only() {
        // MIDI 67 = G string 24th fret (highest playable note)
        let candidates = pitch_to_candidates(67, Tuning::Standard4);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], Candidate { string: 3, fret: 24 });
    }

    #[test]
    fn out_of_range_returns_empty() {
        // MIDI 27 is below E1 — no candidates
        let candidates = pitch_to_candidates(27, Tuning::Standard4);
        assert!(candidates.is_empty());

        // MIDI 68 is above G string 24th fret — no candidates
        let candidates = pitch_to_candidates(68, Tuning::Standard4);
        assert!(candidates.is_empty());
    }
}
```

- [ ] **Step 4: Create `crates/tab-engine/src/lib.rs`**

```rust
pub mod midi;
pub mod tab;

pub use midi::MidiNote;
pub use tab::{Candidate, NoteOrigin, TabNote, TabSheet, Tuning, pitch_to_candidates};
```

- [ ] **Step 5: Run tests**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom
cargo test -p tab-engine
```

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat(tab-engine): add data model, MidiNote, TabNote, pitch_to_candidates"
```

---

## Task 3: Tab Engine — Viterbi DP Optimization

**Files:**
- Create: `crates/tab-engine/src/viterbi.rs`
- Modify: `crates/tab-engine/src/lib.rs`

- [ ] **Step 1: Write failing tests for cost functions**

Create `crates/tab-engine/src/viterbi.rs`:

```rust
use crate::midi::MidiNote;
use crate::tab::{Candidate, NoteOrigin, TabNote, TabSheet, Tuning, pitch_to_candidates};

const OPEN_STRING_PENALTY: f64 = 60.0;
const STRING_CROSS_COST: f64 = 2.0;
const SAME_STRING_BONUS: f64 = -0.5;
const STRETCH_PENALTY: f64 = 3.0;
const STRETCH_THRESHOLD: u8 = 4;

/// Cost of placing a single note at a candidate position (emission cost).
fn emission_cost(candidate: &Candidate) -> f64 {
    let mut cost = 0.0;

    // A/D/G open string penalty (E open is fine)
    if candidate.fret == 0 && candidate.string > 0 {
        cost += OPEN_STRING_PENALTY;
    }

    // Comfort zone
    cost += match candidate.fret {
        0 => 0.0, // open strings handled above
        1 => 0.0,
        2..=9 => -1.5,
        10..=14 => 1.0,
        _ => 3.0, // 15+
    };

    cost
}

/// Cost of transitioning from one candidate to the next (transition cost).
fn transition_cost(prev: &Candidate, curr: &Candidate) -> f64 {
    let mut cost = 0.0;

    // String crossing
    let string_diff = (curr.string as i8 - prev.string as i8).unsigned_abs();
    if string_diff == 0 {
        cost += SAME_STRING_BONUS;
    } else {
        cost += STRING_CROSS_COST * string_diff as f64;
    }

    // Hand position movement
    let fret_diff = if prev.fret == 0 || curr.fret == 0 {
        0 // open strings don't require hand position
    } else {
        (curr.fret as i8 - prev.fret as i8).unsigned_abs()
    };

    cost += match fret_diff {
        0..=1 => 0.0,
        2..=3 => fret_diff as f64 * 1.5,
        4..=5 => fret_diff as f64 * 3.0,
        _ => fret_diff as f64 * 5.0,
    };

    // Cross stretching: different strings, fret span > threshold
    if string_diff > 0 && prev.fret > 0 && curr.fret > 0 {
        let span = (curr.fret as i8 - prev.fret as i8).unsigned_abs();
        if span > STRETCH_THRESHOLD {
            cost += (span - STRETCH_THRESHOLD) as f64 * STRETCH_PENALTY;
        }
    }

    cost
}

/// Run Viterbi DP to find the optimal fingering path for a sequence of MIDI notes.
/// Returns a TabSheet with optimized note positions.
pub fn optimize(notes: &[MidiNote], tuning: Tuning, tempo: f64, time_sig: (u8, u8)) -> TabSheet {
    if notes.is_empty() {
        return TabSheet {
            notes: vec![],
            tempo,
            time_signature: time_sig,
            tuning,
            key_transpose: 0,
        };
    }

    let all_candidates: Vec<Vec<Candidate>> = notes
        .iter()
        .map(|n| pitch_to_candidates(n.pitch, tuning))
        .collect();

    // dp[i] = (min_cost, backtrack_index) for each candidate of note i
    let n = notes.len();
    let mut dp: Vec<Vec<(f64, usize)>> = Vec::with_capacity(n);

    // Initialize first note
    let first_costs: Vec<(f64, usize)> = all_candidates[0]
        .iter()
        .map(|c| (emission_cost(c), 0))
        .collect();
    dp.push(first_costs);

    // Fill DP table
    for i in 1..n {
        let prev_candidates = &all_candidates[i - 1];
        let curr_candidates = &all_candidates[i];

        let mut curr_dp = Vec::with_capacity(curr_candidates.len());
        for curr_c in curr_candidates {
            let e_cost = emission_cost(curr_c);
            let mut best_cost = f64::INFINITY;
            let mut best_prev = 0;

            for (j, prev_c) in prev_candidates.iter().enumerate() {
                let total = dp[i - 1][j].0 + transition_cost(prev_c, curr_c) + e_cost;
                if total < best_cost {
                    best_cost = total;
                    best_prev = j;
                }
            }

            curr_dp.push((best_cost, best_prev));
        }
        dp.push(curr_dp);
    }

    // Backtrack
    let last = &dp[n - 1];
    let mut best_idx = 0;
    let mut best_cost = f64::INFINITY;
    for (i, &(cost, _)) in last.iter().enumerate() {
        if cost < best_cost {
            best_cost = cost;
            best_idx = i;
        }
    }

    let mut path = vec![0usize; n];
    path[n - 1] = best_idx;
    for i in (1..n).rev() {
        path[i - 1] = dp[i][path[i]].1;
    }

    // Build TabNotes
    let tab_notes: Vec<TabNote> = notes
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let c = &all_candidates[i][path[i]];
            TabNote {
                string: c.string,
                fret: c.fret,
                midi_pitch: note.pitch,
                onset: note.onset,
                duration: note.offset - note.onset,
                origin: NoteOrigin::Optimized,
            }
        })
        .collect();

    TabSheet {
        notes: tab_notes,
        tempo,
        time_signature: time_sig,
        tuning,
        key_transpose: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note(pitch: u8, onset: f64, offset: f64) -> MidiNote {
        MidiNote { pitch, onset, offset, velocity: 80 }
    }

    #[test]
    fn open_e_string_preferred_over_nothing() {
        // E1 (28) — only candidate is E string open, should work fine
        let notes = vec![make_note(28, 0.0, 0.5)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes.len(), 1);
        assert_eq!(sheet.notes[0].string, 0);
        assert_eq!(sheet.notes[0].fret, 0);
    }

    #[test]
    fn avoids_adg_open_strings() {
        // A1 (33) = E:5 or A:0. Should prefer E:5 due to open string penalty on A.
        let notes = vec![make_note(33, 0.0, 0.5)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].string, 0);
        assert_eq!(sheet.notes[0].fret, 5);
    }

    #[test]
    fn prefers_nearby_frets_for_sequence() {
        // Sequence: E:5 -> F:6 -> G:8 (all on E string, consecutive frets)
        // MIDI: A1=33, Bb1=34, C2=36
        let notes = vec![
            make_note(33, 0.0, 0.5), // A1
            make_note(34, 0.5, 1.0), // Bb1
            make_note(36, 1.0, 1.5), // C2
        ];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        // All should stay on E string for minimal position movement
        // 33-28=5, 34-28=6, 36-28=8 — all on E string in comfort zone
        assert!(sheet.notes.iter().all(|n| n.string == 0));
    }

    #[test]
    fn comfort_zone_preference() {
        // G2 (MIDI 43) = E:15, A:10, D:5, G:0
        // Should prefer D:5 (comfort zone 2-9) over A:10 or E:15
        // G:0 has open string penalty
        let notes = vec![make_note(43, 0.0, 0.5)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].string, 2); // D string
        assert_eq!(sheet.notes[0].fret, 5);
    }

    #[test]
    fn empty_notes_returns_empty_sheet() {
        let sheet = optimize(&[], Tuning::Standard4, 120.0, (4, 4));
        assert!(sheet.notes.is_empty());
    }

    #[test]
    fn handles_notes_with_single_candidate() {
        // E1 (28) and G#4 (MIDI 67-1=66? no, let's use MIDI 67 = G:24)
        // MIDI 67 only has G:24
        let notes = vec![make_note(28, 0.0, 0.5), make_note(67, 0.5, 1.0)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].fret, 0); // E open
        assert_eq!(sheet.notes[1].string, 3); // G string
        assert_eq!(sheet.notes[1].fret, 24);
    }
}
```

- [ ] **Step 2: Add viterbi module to lib.rs**

Update `crates/tab-engine/src/lib.rs`:

```rust
pub mod midi;
pub mod tab;
pub mod viterbi;

pub use midi::MidiNote;
pub use tab::{Candidate, NoteOrigin, TabNote, TabSheet, Tuning, pitch_to_candidates};
pub use viterbi::optimize;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tab-engine
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/
git commit -m "feat(tab-engine): add Viterbi DP fingering optimization"
```

---

## Task 4: Tab Engine — Transpose

**Files:**
- Create: `crates/tab-engine/src/transpose.rs`
- Modify: `crates/tab-engine/src/lib.rs`

- [ ] **Step 1: Write transpose module with tests**

Create `crates/tab-engine/src/transpose.rs`:

```rust
use crate::midi::MidiNote;
use crate::tab::{NoteOrigin, Tuning};
use crate::viterbi;

const BASS_MIN: u8 = 28; // E1
const BASS_MAX: u8 = 67; // G string 24th fret

/// Transpose MIDI notes by the given number of semitones and re-run Viterbi optimization.
/// Notes that fall outside the bass range are octave-corrected.
pub fn transpose(
    original_notes: &[MidiNote],
    semitones: i8,
    tuning: Tuning,
    tempo: f64,
    time_sig: (u8, u8),
) -> crate::tab::TabSheet {
    let transposed: Vec<(MidiNote, Option<i8>)> = original_notes
        .iter()
        .map(|note| {
            let raw = note.pitch as i16 + semitones as i16;
            let (final_pitch, octave_shift) = clamp_to_bass_range(raw);
            (
                MidiNote {
                    pitch: final_pitch,
                    onset: note.onset,
                    offset: note.offset,
                    velocity: note.velocity,
                },
                octave_shift,
            )
        })
        .collect();

    let midi_notes: Vec<MidiNote> = transposed.iter().map(|(n, _)| n.clone()).collect();
    let mut sheet = viterbi::optimize(&midi_notes, tuning, tempo, time_sig);
    sheet.key_transpose = semitones;

    // Tag octave-shifted notes
    for (i, (_, shift)) in transposed.iter().enumerate() {
        if let Some(s) = shift {
            sheet.notes[i].origin = NoteOrigin::OctaveShifted(*s);
        }
    }

    sheet
}

/// Clamp a raw MIDI pitch to the bass range, returning (clamped_pitch, octave_shift).
/// octave_shift is None if no correction was needed.
fn clamp_to_bass_range(raw: i16) -> (u8, Option<i8>) {
    if raw < BASS_MIN as i16 {
        let corrected = raw + 12;
        if corrected >= BASS_MIN as i16 && corrected <= BASS_MAX as i16 {
            (corrected as u8, Some(1)) // shifted up
        } else {
            // Still out of range after one octave shift — clamp to minimum
            (BASS_MIN, Some(1))
        }
    } else if raw > BASS_MAX as i16 {
        let corrected = raw - 12;
        if corrected >= BASS_MIN as i16 && corrected <= BASS_MAX as i16 {
            (corrected as u8, Some(-1)) // shifted down
        } else {
            (BASS_MAX, Some(-1))
        }
    } else {
        (raw as u8, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note(pitch: u8, onset: f64) -> MidiNote {
        MidiNote { pitch, onset, offset: onset + 0.5, velocity: 80 }
    }

    #[test]
    fn transpose_within_range_no_octave_shift() {
        let notes = vec![make_note(33, 0.0)]; // A1
        let sheet = transpose(&notes, 2, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].midi_pitch, 35); // B1
        assert_eq!(sheet.key_transpose, 2);
        assert!(matches!(sheet.notes[0].origin, NoteOrigin::Optimized));
    }

    #[test]
    fn transpose_down_below_range_gets_octave_shifted_up() {
        let notes = vec![make_note(28, 0.0)]; // E1, lowest note
        let sheet = transpose(&notes, -1, Tuning::Standard4, 120.0, (4, 4));
        // 28 - 1 = 27, below range → 27 + 12 = 39 (D#2)
        assert_eq!(sheet.notes[0].midi_pitch, 39);
        assert!(matches!(sheet.notes[0].origin, NoteOrigin::OctaveShifted(1)));
    }

    #[test]
    fn transpose_up_above_range_gets_octave_shifted_down() {
        let notes = vec![make_note(67, 0.0)]; // Highest note
        let sheet = transpose(&notes, 1, Tuning::Standard4, 120.0, (4, 4));
        // 67 + 1 = 68, above range → 68 - 12 = 56 (Ab3)
        assert_eq!(sheet.notes[0].midi_pitch, 56);
        assert!(matches!(sheet.notes[0].origin, NoteOrigin::OctaveShifted(-1)));
    }

    #[test]
    fn transpose_zero_preserves_notes() {
        let notes = vec![make_note(40, 0.0), make_note(45, 0.5)];
        let sheet = transpose(&notes, 0, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].midi_pitch, 40);
        assert_eq!(sheet.notes[1].midi_pitch, 45);
        assert_eq!(sheet.key_transpose, 0);
    }

    #[test]
    fn clamp_handles_extreme_shift() {
        let (pitch, shift) = clamp_to_bass_range(10); // Way below range
        assert_eq!(pitch, 28); // Clamped to minimum
        assert_eq!(shift, Some(1));
    }
}
```

- [ ] **Step 2: Add transpose module to lib.rs**

Update `crates/tab-engine/src/lib.rs`:

```rust
pub mod midi;
pub mod tab;
pub mod transpose;
pub mod viterbi;

pub use midi::MidiNote;
pub use tab::{Candidate, NoteOrigin, TabNote, TabSheet, Tuning, pitch_to_candidates};
pub use transpose::transpose;
pub use viterbi::optimize;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tab-engine
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/
git commit -m "feat(tab-engine): add transpose with octave correction"
```

---

## Task 5: Tab Engine — ASCII Export

**Files:**
- Create: `crates/tab-engine/src/export/mod.rs`
- Create: `crates/tab-engine/src/export/ascii.rs`
- Modify: `crates/tab-engine/src/lib.rs`

- [ ] **Step 1: Write ASCII export with tests**

Create `crates/tab-engine/src/export/mod.rs`:

```rust
pub mod ascii;
pub mod musicxml;
```

Create `crates/tab-engine/src/export/ascii.rs`:

```rust
use crate::tab::{TabNote, TabSheet, Tuning};

const CHARS_PER_BEAT: usize = 4;
const BEATS_PER_MEASURE: usize = 4;
const CHARS_PER_MEASURE: usize = CHARS_PER_BEAT * BEATS_PER_MEASURE; // 16
const MEASURES_PER_LINE: usize = 4;

/// Export a TabSheet to ASCII text tab format.
pub fn export(sheet: &TabSheet) -> String {
    if sheet.notes.is_empty() {
        return String::from("(empty tab)");
    }

    let string_labels = match sheet.tuning {
        Tuning::Standard4 => ["E", "A", "D", "G"],
    };
    let num_strings = string_labels.len();

    // Determine total duration in beats
    let last_note = sheet.notes.iter().map(|n| n.onset + n.duration).fold(0.0f64, f64::max);
    let beat_duration = 60.0 / sheet.tempo;
    let total_beats = (last_note / beat_duration).ceil() as usize;
    let total_measures = ((total_beats + BEATS_PER_MEASURE - 1) / BEATS_PER_MEASURE).max(1);
    let total_chars = total_measures * CHARS_PER_MEASURE;

    // Build a grid: strings (top=G, bottom=E) x char positions
    let mut grid: Vec<Vec<String>> = vec![vec!["-".to_string(); total_chars]; num_strings];

    // Place notes
    for note in &sheet.notes {
        let beat_pos = note.onset / beat_duration;
        let char_pos = (beat_pos * CHARS_PER_BEAT as f64).round() as usize;
        if char_pos < total_chars {
            let fret_str = note.fret.to_string();
            let s = note.string as usize;
            // Place fret number (may be 1-2 chars)
            for (i, ch) in fret_str.chars().enumerate() {
                if char_pos + i < total_chars {
                    grid[s][char_pos + i] = ch.to_string();
                }
            }
        }
    }

    // Render lines (MEASURES_PER_LINE measures per output line)
    let mut output = String::new();
    let mut measure_offset = 0;

    while measure_offset < total_measures {
        let line_measures = (total_measures - measure_offset).min(MEASURES_PER_LINE);
        let start_char = measure_offset * CHARS_PER_MEASURE;
        let end_char = (start_char + line_measures * CHARS_PER_MEASURE).min(total_chars);

        // Print strings from high (G) to low (E)
        for s in (0..num_strings).rev() {
            output.push_str(string_labels[s]);
            output.push('|');
            for c in start_char..end_char {
                output.push_str(&grid[s][c]);
                // Add measure bar
                if (c + 1) % CHARS_PER_MEASURE == 0 && c + 1 < end_char {
                    output.push('|');
                }
            }
            output.push('|');
            output.push('\n');
        }
        output.push('\n');

        measure_offset += line_measures;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tab::{NoteOrigin, TabNote};

    fn make_tab_note(string: u8, fret: u8, onset: f64) -> TabNote {
        TabNote {
            string, fret,
            midi_pitch: 0,
            onset,
            duration: 0.5,
            origin: NoteOrigin::Normal,
        }
    }

    #[test]
    fn empty_sheet_returns_placeholder() {
        let sheet = TabSheet {
            notes: vec![],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        assert_eq!(export(&sheet), "(empty tab)");
    }

    #[test]
    fn single_note_renders() {
        let sheet = TabSheet {
            notes: vec![make_tab_note(0, 5, 0.0)], // E string, 5th fret
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let result = export(&sheet);
        assert!(result.contains("G|"));
        assert!(result.contains("E|"));
        // The E line should contain "5"
        let e_line = result.lines().find(|l| l.starts_with("E|")).unwrap();
        assert!(e_line.contains('5'));
    }

    #[test]
    fn two_digit_fret_renders() {
        let sheet = TabSheet {
            notes: vec![make_tab_note(0, 12, 0.0)],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let result = export(&sheet);
        let e_line = result.lines().find(|l| l.starts_with("E|")).unwrap();
        assert!(e_line.contains("12"));
    }
}
```

- [ ] **Step 2: Add export module to lib.rs**

Update `crates/tab-engine/src/lib.rs`:

```rust
pub mod export;
pub mod midi;
pub mod tab;
pub mod transpose;
pub mod viterbi;

pub use midi::MidiNote;
pub use tab::{Candidate, NoteOrigin, TabNote, TabSheet, Tuning, pitch_to_candidates};
pub use transpose::transpose;
pub use viterbi::optimize;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tab-engine
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/
git commit -m "feat(tab-engine): add ASCII text tab export"
```

---

## Task 6: Tab Engine — MusicXML Export

**Files:**
- Create: `crates/tab-engine/src/export/musicxml.rs`

- [ ] **Step 1: Write MusicXML export with tests**

Create `crates/tab-engine/src/export/musicxml.rs`:

```rust
use crate::tab::{TabSheet, Tuning};

/// Export a TabSheet to MusicXML format (tab-only subset).
pub fn export(sheet: &TabSheet) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">"#);
    xml.push('\n');
    xml.push_str(r#"<score-partwise version="4.0">"#);
    xml.push('\n');

    // Part list
    xml.push_str("  <part-list>\n");
    xml.push_str(r#"    <score-part id="P1">"#);
    xml.push('\n');
    xml.push_str("      <part-name>Bass Guitar</part-name>\n");
    xml.push_str("    </score-part>\n");
    xml.push_str("  </part-list>\n");

    // Part
    xml.push_str(r#"  <part id="P1">"#);
    xml.push('\n');

    if sheet.notes.is_empty() {
        // Single empty measure
        xml.push_str("    <measure number=\"1\">\n");
        write_attributes(&mut xml, sheet);
        xml.push_str("      <note><rest/><duration>4</duration><type>whole</type></note>\n");
        xml.push_str("    </measure>\n");
    } else {
        let beat_duration = 60.0 / sheet.tempo;
        let beats_per_measure = sheet.time_signature.0 as f64;

        // Group notes into measures
        let mut measure_num = 1;
        let mut current_beat = 0.0;
        let mut note_idx = 0;
        let mut first_measure = true;

        while note_idx < sheet.notes.len() {
            let measure_start = current_beat;
            let measure_end = current_beat + beats_per_measure;

            xml.push_str(&format!("    <measure number=\"{}\">\n", measure_num));

            if first_measure {
                write_attributes(&mut xml, sheet);
                first_measure = false;
            }

            while note_idx < sheet.notes.len() {
                let note = &sheet.notes[note_idx];
                let note_beat = note.onset / beat_duration;
                if note_beat >= measure_end {
                    break;
                }

                let duration_beats = note.duration / beat_duration;
                let duration_divisions = (duration_beats * 4.0).round() as u32; // divisions = 4 per beat
                let note_type = duration_to_type(duration_beats);

                xml.push_str("      <note>\n");
                write_pitch_for_tab(&mut xml, note.string, note.fret, &sheet.tuning);
                xml.push_str(&format!("        <duration>{}</duration>\n", duration_divisions.max(1)));
                xml.push_str(&format!("        <type>{}</type>\n", note_type));
                xml.push_str("        <notations>\n");
                xml.push_str("          <technical>\n");
                xml.push_str(&format!("            <string>{}</string>\n", note.string + 1));
                xml.push_str(&format!("            <fret>{}</fret>\n", note.fret));
                xml.push_str("          </technical>\n");
                xml.push_str("        </notations>\n");
                xml.push_str("      </note>\n");

                note_idx += 1;
            }

            xml.push_str("    </measure>\n");
            current_beat = measure_end;
            measure_num += 1;
        }
    }

    xml.push_str("  </part>\n");
    xml.push_str("</score-partwise>\n");
    xml
}

fn write_attributes(xml: &mut String, sheet: &TabSheet) {
    let num_strings = match sheet.tuning {
        Tuning::Standard4 => 4,
    };
    xml.push_str("      <attributes>\n");
    xml.push_str("        <divisions>4</divisions>\n");
    xml.push_str(&format!("        <time><beats>{}</beats><beat-type>{}</beat-type></time>\n",
        sheet.time_signature.0, sheet.time_signature.1));
    xml.push_str(&format!(
        "        <clef><sign>TAB</sign><line>{}</line></clef>\n",
        num_strings
    ));
    xml.push_str(&format!(
        "        <staff-details><staff-lines>{}</staff-lines></staff-details>\n",
        num_strings
    ));
    xml.push_str("      </attributes>\n");
}

fn write_pitch_for_tab(xml: &mut String, string: u8, fret: u8, tuning: &Tuning) {
    let open_pitches = tuning.open_pitches();
    let midi = open_pitches[string as usize] + fret;
    let octave = (midi / 12) as i8 - 1;
    let step = match midi % 12 {
        0 => "C", 1 => "C", 2 => "D", 3 => "D", 4 => "E",
        5 => "F", 6 => "F", 7 => "G", 8 => "G", 9 => "A",
        10 => "A", 11 => "B", _ => unreachable!(),
    };
    let alter = match midi % 12 {
        1 | 3 | 6 | 8 | 10 => Some(1),
        _ => None,
    };

    xml.push_str("        <pitch>\n");
    xml.push_str(&format!("          <step>{}</step>\n", step));
    if let Some(a) = alter {
        xml.push_str(&format!("          <alter>{}</alter>\n", a));
    }
    xml.push_str(&format!("          <octave>{}</octave>\n", octave));
    xml.push_str("        </pitch>\n");
}

fn duration_to_type(beats: f64) -> &'static str {
    if beats >= 3.5 { "whole" }
    else if beats >= 1.5 { "half" }
    else if beats >= 0.75 { "quarter" }
    else if beats >= 0.375 { "eighth" }
    else { "16th" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tab::{NoteOrigin, TabNote, TabSheet};

    fn make_tab_note(string: u8, fret: u8, onset: f64, duration: f64) -> TabNote {
        TabNote {
            string, fret,
            midi_pitch: 0,
            onset, duration,
            origin: NoteOrigin::Normal,
        }
    }

    #[test]
    fn empty_sheet_produces_valid_xml() {
        let sheet = TabSheet {
            notes: vec![],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let xml = export(&sheet);
        assert!(xml.contains("score-partwise"));
        assert!(xml.contains("<rest/>"));
        assert!(xml.contains("Bass Guitar"));
    }

    #[test]
    fn single_note_produces_tab_notation() {
        let sheet = TabSheet {
            notes: vec![make_tab_note(0, 5, 0.0, 0.5)],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let xml = export(&sheet);
        assert!(xml.contains("<string>1</string>"));
        assert!(xml.contains("<fret>5</fret>"));
        assert!(xml.contains("<sign>TAB</sign>"));
    }

    #[test]
    fn xml_starts_with_declaration() {
        let sheet = TabSheet {
            notes: vec![],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let xml = export(&sheet);
        assert!(xml.starts_with("<?xml"));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p tab-engine
```

Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/
git commit -m "feat(tab-engine): add MusicXML export"
```

---

## Task 7: Python Sidecar — Protocol & Entry

**Files:**
- Create: `python/ai-pipeline/pyproject.toml`
- Create: `python/ai-pipeline/requirements.txt`
- Create: `python/ai-pipeline/protocol.py`
- Create: `python/ai-pipeline/main.py`
- Create: `python/ai-pipeline/tests/test_protocol.py`

- [ ] **Step 1: Create `python/ai-pipeline/pyproject.toml`**

```toml
[project]
name = "fanthom-ai-pipeline"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "yt-dlp>=2024.0.0",
    "demucs>=4.0.0",
    "basic-pitch>=0.3.0",
]

[project.optional-dependencies]
dev = ["pytest>=8.0"]
```

- [ ] **Step 2: Create `python/ai-pipeline/requirements.txt`**

```
yt-dlp>=2024.0.0
demucs>=4.0.0
basic-pitch>=0.3.0
pytest>=8.0
```

- [ ] **Step 3: Write protocol helpers with tests**

Create `python/ai-pipeline/protocol.py`:

```python
import json
import sys


def send_progress(step: str, percent: int, message: str) -> None:
    """Send a progress message to stdout."""
    msg = {"type": "progress", "step": step, "percent": percent, "message": message}
    print(json.dumps(msg), flush=True)


def send_result(step: str, data: dict) -> None:
    """Send a result message to stdout."""
    msg = {"type": "result", "step": step, "data": data}
    print(json.dumps(msg), flush=True)


def send_error(step: str, message: str) -> None:
    """Send an error message to stdout."""
    msg = {"type": "error", "step": step, "message": message}
    print(json.dumps(msg), flush=True)


def read_command() -> dict | None:
    """Read a JSON command from stdin. Returns None on EOF."""
    line = sys.stdin.readline()
    if not line:
        return None
    return json.loads(line.strip())
```

Create `python/ai-pipeline/tests/__init__.py` (empty file).

Create `python/ai-pipeline/tests/test_protocol.py`:

```python
import json
from io import StringIO
from unittest.mock import patch

from protocol import read_command, send_error, send_progress, send_result


def test_send_progress(capsys):
    send_progress("extract", 45, "Downloading...")
    captured = capsys.readouterr()
    msg = json.loads(captured.out.strip())
    assert msg["type"] == "progress"
    assert msg["step"] == "extract"
    assert msg["percent"] == 45
    assert msg["message"] == "Downloading..."


def test_send_result(capsys):
    send_result("extract", {"audio_path": "/tmp/audio.wav"})
    captured = capsys.readouterr()
    msg = json.loads(captured.out.strip())
    assert msg["type"] == "result"
    assert msg["data"]["audio_path"] == "/tmp/audio.wav"


def test_send_error(capsys):
    send_error("extract", "Video unavailable")
    captured = capsys.readouterr()
    msg = json.loads(captured.out.strip())
    assert msg["type"] == "error"
    assert msg["message"] == "Video unavailable"


def test_read_command():
    input_data = '{"command": "extract", "params": {"url": "https://youtube.com/watch?v=abc"}}\n'
    with patch("sys.stdin", StringIO(input_data)):
        cmd = read_command()
    assert cmd["command"] == "extract"
    assert cmd["params"]["url"] == "https://youtube.com/watch?v=abc"


def test_read_command_eof():
    with patch("sys.stdin", StringIO("")):
        cmd = read_command()
    assert cmd is None
```

- [ ] **Step 4: Write main.py dispatcher**

Create `python/ai-pipeline/main.py`:

```python
#!/usr/bin/env python3
"""Fanthom AI Pipeline — stdin/stdout JSON Lines sidecar."""

import sys
import traceback

from protocol import read_command, send_error


def main() -> None:
    while True:
        cmd = read_command()
        if cmd is None:
            break

        command = cmd.get("command")
        params = cmd.get("params", {})

        try:
            if command == "extract":
                from extract import run_extract
                run_extract(params)
            elif command == "separate":
                from separate import run_separate
                run_separate(params)
            elif command == "transcribe":
                from transcribe import run_transcribe
                run_transcribe(params)
            else:
                send_error("unknown", f"Unknown command: {command}")
        except Exception as e:
            send_error(command or "unknown", f"{type(e).__name__}: {e}")
            traceback.print_exc(file=sys.stderr)


if __name__ == "__main__":
    main()
```

- [ ] **Step 5: Run protocol tests**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom
python -m pytest python/ai-pipeline/tests/test_protocol.py -v
```

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add python/
git commit -m "feat(ai-pipeline): add JSON Lines protocol and main dispatcher"
```

---

## Task 8: Python Sidecar — Audio Extraction

**Files:**
- Create: `python/ai-pipeline/extract.py`
- Create: `python/ai-pipeline/tests/test_extract.py`

- [ ] **Step 1: Write extract module**

Create `python/ai-pipeline/extract.py`:

```python
"""YouTube audio extraction via yt-dlp."""

import os
import subprocess

from protocol import send_error, send_progress, send_result


def run_extract(params: dict) -> None:
    url = params["url"]
    output_dir = params["output_dir"]
    os.makedirs(output_dir, exist_ok=True)

    output_path = os.path.join(output_dir, "original.wav")

    send_progress("extract", 0, "Starting audio extraction...")

    try:
        cmd = [
            "yt-dlp",
            "--extract-audio",
            "--audio-format", "wav",
            "--audio-quality", "0",
            "--output", output_path.replace(".wav", ".%(ext)s"),
            "--no-playlist",
            url,
        ]

        process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        send_progress("extract", 30, "Downloading audio...")

        _, stderr = process.communicate()

        if process.returncode != 0:
            send_error("extract", f"yt-dlp failed: {stderr.strip()}")
            return

        # yt-dlp may output with a different extension before converting
        if not os.path.exists(output_path):
            # Check for common alternative names
            for ext in [".wav", ".webm", ".m4a", ".mp3"]:
                alt = output_path.replace(".wav", ext)
                if os.path.exists(alt) and alt != output_path:
                    os.rename(alt, output_path)
                    break

        if not os.path.exists(output_path):
            send_error("extract", "Output file not found after extraction")
            return

        send_progress("extract", 100, "Audio extraction complete")
        send_result("extract", {"audio_path": output_path})

    except FileNotFoundError:
        send_error("extract", "yt-dlp not found. Please install: pip install yt-dlp")
```

- [ ] **Step 2: Write extract unit test (mocked)**

Create `python/ai-pipeline/tests/test_extract.py`:

```python
import json
import os
from unittest.mock import MagicMock, patch

from extract import run_extract


def test_run_extract_success(tmp_path, capsys):
    output_dir = str(tmp_path / "song123")
    output_path = os.path.join(output_dir, "original.wav")

    def mock_popen(*args, **kwargs):
        # Simulate yt-dlp creating the output file
        os.makedirs(output_dir, exist_ok=True)
        with open(output_path, "wb") as f:
            f.write(b"RIFF" + b"\x00" * 100)  # fake WAV header
        mock = MagicMock()
        mock.communicate.return_value = ("", "")
        mock.returncode = 0
        return mock

    with patch("extract.subprocess.Popen", side_effect=mock_popen):
        run_extract({"url": "https://youtube.com/watch?v=test", "output_dir": output_dir})

    captured = capsys.readouterr()
    messages = [json.loads(line) for line in captured.out.strip().split("\n") if line]
    result_msgs = [m for m in messages if m["type"] == "result"]
    assert len(result_msgs) == 1
    assert result_msgs[0]["data"]["audio_path"] == output_path


def test_run_extract_ytdlp_failure(tmp_path, capsys):
    output_dir = str(tmp_path / "song456")

    mock = MagicMock()
    mock.communicate.return_value = ("", "ERROR: Video unavailable")
    mock.returncode = 1

    with patch("extract.subprocess.Popen", return_value=mock):
        run_extract({"url": "https://youtube.com/watch?v=bad", "output_dir": output_dir})

    captured = capsys.readouterr()
    messages = [json.loads(line) for line in captured.out.strip().split("\n") if line]
    error_msgs = [m for m in messages if m["type"] == "error"]
    assert len(error_msgs) == 1
    assert "Video unavailable" in error_msgs[0]["message"]
```

- [ ] **Step 3: Run tests**

```bash
python -m pytest python/ai-pipeline/tests/test_extract.py -v
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add python/
git commit -m "feat(ai-pipeline): add yt-dlp audio extraction"
```

---

## Task 9: Python Sidecar — Source Separation

**Files:**
- Create: `python/ai-pipeline/separate.py`
- Create: `python/ai-pipeline/tests/test_separate.py`

- [ ] **Step 1: Write separate module**

Create `python/ai-pipeline/separate.py`:

```python
"""Bass track separation via Demucs."""

import os
import subprocess

from protocol import send_error, send_progress, send_result


def run_separate(params: dict) -> None:
    audio_path = params["audio_path"]
    output_dir = params["output_dir"]

    if not os.path.exists(audio_path):
        send_error("separate", f"Audio file not found: {audio_path}")
        return

    send_progress("separate", 0, "Starting source separation...")

    try:
        cmd = [
            "python", "-m", "demucs",
            "--two-stems", "bass",
            "-n", "htdemucs",
            "-o", output_dir,
            audio_path,
        ]

        process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        send_progress("separate", 30, "Separating bass track (this may take a while)...")

        _, stderr = process.communicate()

        if process.returncode != 0:
            send_error("separate", f"Demucs failed: {stderr.strip()}")
            return

        # Demucs outputs to: output_dir/htdemucs/<filename>/bass.wav
        filename = os.path.splitext(os.path.basename(audio_path))[0]
        bass_path_demucs = os.path.join(output_dir, "htdemucs", filename, "bass.wav")

        # Move to expected location
        bass_path = os.path.join(output_dir, "bass.wav")
        if os.path.exists(bass_path_demucs):
            os.rename(bass_path_demucs, bass_path)
        elif os.path.exists(bass_path):
            pass  # Already in the right place
        else:
            send_error("separate", "Bass track not found in Demucs output")
            return

        send_progress("separate", 100, "Source separation complete")
        send_result("separate", {"bass_path": bass_path})

    except FileNotFoundError:
        send_error("separate", "Demucs not found. Please install: pip install demucs")
```

- [ ] **Step 2: Write separate unit test (mocked)**

Create `python/ai-pipeline/tests/test_separate.py`:

```python
import json
import os
from unittest.mock import MagicMock, patch

from separate import run_separate


def test_run_separate_success(tmp_path, capsys):
    # Setup: create a fake input audio file
    audio_path = str(tmp_path / "original.wav")
    with open(audio_path, "wb") as f:
        f.write(b"RIFF" + b"\x00" * 100)

    output_dir = str(tmp_path / "output")
    bass_output = os.path.join(output_dir, "htdemucs", "original", "bass.wav")

    def mock_popen(*args, **kwargs):
        os.makedirs(os.path.dirname(bass_output), exist_ok=True)
        with open(bass_output, "wb") as f:
            f.write(b"RIFF" + b"\x00" * 50)
        mock = MagicMock()
        mock.communicate.return_value = ("", "")
        mock.returncode = 0
        return mock

    with patch("separate.subprocess.Popen", side_effect=mock_popen):
        run_separate({"audio_path": audio_path, "output_dir": output_dir})

    captured = capsys.readouterr()
    messages = [json.loads(line) for line in captured.out.strip().split("\n") if line]
    result_msgs = [m for m in messages if m["type"] == "result"]
    assert len(result_msgs) == 1
    assert result_msgs[0]["data"]["bass_path"].endswith("bass.wav")


def test_run_separate_missing_audio(tmp_path, capsys):
    run_separate({"audio_path": str(tmp_path / "missing.wav"), "output_dir": str(tmp_path)})
    captured = capsys.readouterr()
    messages = [json.loads(line) for line in captured.out.strip().split("\n") if line]
    error_msgs = [m for m in messages if m["type"] == "error"]
    assert len(error_msgs) == 1
    assert "not found" in error_msgs[0]["message"]
```

- [ ] **Step 3: Run tests**

```bash
python -m pytest python/ai-pipeline/tests/test_separate.py -v
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add python/
git commit -m "feat(ai-pipeline): add Demucs bass track separation"
```

---

## Task 10: Python Sidecar — Pitch Detection

**Files:**
- Create: `python/ai-pipeline/transcribe.py`
- Create: `python/ai-pipeline/tests/test_transcribe.py`

- [ ] **Step 1: Write transcribe module**

Create `python/ai-pipeline/transcribe.py`:

```python
"""Pitch detection via basic-pitch (Spotify)."""

import json
import os

from protocol import send_error, send_progress, send_result


def run_transcribe(params: dict) -> None:
    bass_path = params["bass_path"]
    output_dir = params["output_dir"]

    if not os.path.exists(bass_path):
        send_error("transcribe", f"Bass audio not found: {bass_path}")
        return

    send_progress("transcribe", 0, "Starting pitch detection...")

    try:
        from basic_pitch.inference import predict

        send_progress("transcribe", 20, "Running pitch detection model...")

        model_output, midi_data, note_events = predict(bass_path)

        # Convert note_events to our JSON format
        # basic-pitch note_events: list of (onset, offset, pitch, velocity, [confidence])
        notes = []
        for event in note_events:
            notes.append({
                "pitch": int(event[2]),
                "onset": float(event[0]),
                "offset": float(event[1]),
                "velocity": min(127, max(0, int(event[3] * 127))),
            })

        # Sort by onset time
        notes.sort(key=lambda n: n["onset"])

        # Save to JSON
        midi_json_path = os.path.join(output_dir, "midi.json")
        with open(midi_json_path, "w") as f:
            json.dump(notes, f, indent=2)

        send_progress("transcribe", 100, "Pitch detection complete")
        send_result("transcribe", {
            "midi_json_path": midi_json_path,
            "note_count": len(notes),
        })

    except ImportError:
        send_error("transcribe", "basic-pitch not found. Please install: pip install basic-pitch")
    except Exception as e:
        send_error("transcribe", f"Pitch detection failed: {e}")
```

- [ ] **Step 2: Write transcribe unit test (mocked)**

Create `python/ai-pipeline/tests/test_transcribe.py`:

```python
import json
import os
from unittest.mock import patch

from transcribe import run_transcribe


def test_run_transcribe_success(tmp_path, capsys):
    bass_path = str(tmp_path / "bass.wav")
    with open(bass_path, "wb") as f:
        f.write(b"RIFF" + b"\x00" * 100)

    output_dir = str(tmp_path)

    # Mock basic-pitch predict to return fake note events
    # Format: (onset, offset, pitch, velocity)
    fake_note_events = [
        (0.0, 0.5, 33, 0.8),
        (0.5, 1.0, 35, 0.7),
        (1.0, 1.5, 40, 0.9),
    ]

    with patch("transcribe.predict", return_value=(None, None, fake_note_events)):
        run_transcribe({"bass_path": bass_path, "output_dir": output_dir})

    captured = capsys.readouterr()
    messages = [json.loads(line) for line in captured.out.strip().split("\n") if line]
    result_msgs = [m for m in messages if m["type"] == "result"]
    assert len(result_msgs) == 1
    assert result_msgs[0]["data"]["note_count"] == 3

    # Check the saved JSON
    midi_json_path = result_msgs[0]["data"]["midi_json_path"]
    with open(midi_json_path) as f:
        notes = json.load(f)
    assert len(notes) == 3
    assert notes[0]["pitch"] == 33
    assert notes[1]["onset"] == 0.5


def test_run_transcribe_missing_audio(tmp_path, capsys):
    run_transcribe({"bass_path": str(tmp_path / "missing.wav"), "output_dir": str(tmp_path)})
    captured = capsys.readouterr()
    messages = [json.loads(line) for line in captured.out.strip().split("\n") if line]
    error_msgs = [m for m in messages if m["type"] == "error"]
    assert len(error_msgs) == 1
    assert "not found" in error_msgs[0]["message"]
```

- [ ] **Step 3: Run tests**

```bash
python -m pytest python/ai-pipeline/tests/test_transcribe.py -v
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add python/
git commit -m "feat(ai-pipeline): add basic-pitch pitch detection"
```

---

## Task 11: Tauri App — Scaffold & SQLite

**Files:**
- Create: `apps/desktop/` (via `pnpm create tauri-app`)
- Create: `apps/desktop/src-tauri/src/db.rs`
- Create: `apps/desktop/src-tauri/src/state.rs`
- Modify: `apps/desktop/src-tauri/src/main.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`

- [ ] **Step 1: Scaffold Tauri app**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom
pnpm create tauri-app apps/desktop --template react-ts --manager pnpm --yes
```

If the Tauri CLI scaffold doesn't support the output directory directly, create manually:

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom/apps
pnpm create tauri-app desktop --template react-ts --manager pnpm --yes
```

- [ ] **Step 2: Install frontend dependencies**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom/apps/desktop
pnpm add -D tailwindcss @tailwindcss/vite
```

- [ ] **Step 3: Configure Tailwind in `vite.config.ts`**

Update `apps/desktop/vite.config.ts`:

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
```

- [ ] **Step 4: Add Rust dependencies to `apps/desktop/src-tauri/Cargo.toml`**

Add under `[dependencies]`:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
rmp-serde = "1"
tab-engine = { path = "../../crates/tab-engine" }
```

- [ ] **Step 5: Write SQLite database module**

Create `apps/desktop/src-tauri/src/db.rs`:

```rust
use rusqlite::{Connection, Result, params};
use std::path::Path;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS songs (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    source_url  TEXT,
    duration    REAL,
    tempo       REAL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tabs (
    id          TEXT PRIMARY KEY,
    song_id     TEXT NOT NULL REFERENCES songs(id),
    tuning      TEXT NOT NULL DEFAULT 'standard4',
    transpose   INTEGER NOT NULL DEFAULT 0,
    tab_data    BLOB NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);
"#;

pub fn open(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

pub fn insert_song(conn: &Connection, id: &str, title: &str, source_url: Option<&str>, duration: Option<f64>, tempo: Option<f64>) -> Result<()> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO songs (id, title, source_url, duration, tempo, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, title, source_url, duration, tempo, &now, &now],
    )?;
    Ok(())
}

pub fn insert_tab(conn: &Connection, id: &str, song_id: &str, tuning: &str, transpose: i32, tab_data: &[u8]) -> Result<()> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO tabs (id, song_id, tuning, transpose, tab_data, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, song_id, tuning, transpose, tab_data, &now],
    )?;
    Ok(())
}

pub fn get_tab_data(conn: &Connection, tab_id: &str) -> Result<Vec<u8>> {
    conn.query_row(
        "SELECT tab_data FROM tabs WHERE id = ?1",
        params![tab_id],
        |row| row.get(0),
    )
}

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp without chrono dependency
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    format!("{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn create_schema_and_insert_song() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();

        insert_song(&conn, "song-1", "Test Song", Some("https://youtube.com/watch?v=abc"), Some(180.0), Some(120.0)).unwrap();

        let title: String = conn.query_row(
            "SELECT title FROM songs WHERE id = 'song-1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(title, "Test Song");
    }

    #[test]
    fn insert_and_retrieve_tab() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();

        insert_song(&conn, "s1", "Song", None, None, None).unwrap();
        let tab_data = vec![1u8, 2, 3, 4, 5];
        insert_tab(&conn, "t1", "s1", "standard4", 0, &tab_data).unwrap();

        let retrieved = get_tab_data(&conn, "t1").unwrap();
        assert_eq!(retrieved, tab_data);
    }
}
```

- [ ] **Step 6: Write state module**

Create `apps/desktop/src-tauri/src/state.rs`:

```rust
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub data_dir: PathBuf,
    pub songs_dir: PathBuf,
}

impl AppState {
    pub fn new(db: Connection, data_dir: PathBuf) -> Self {
        let songs_dir = data_dir.join("songs");
        std::fs::create_dir_all(&songs_dir).ok();
        Self {
            db: Mutex::new(db),
            data_dir,
            songs_dir,
        }
    }
}
```

- [ ] **Step 7: Update main.rs**

Replace `apps/desktop/src-tauri/src/main.rs`:

```rust
mod db;
mod state;

use state::AppState;
use std::path::PathBuf;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("failed to get app data dir");
            let data_dir = app_data.join("data");
            std::fs::create_dir_all(&data_dir)?;

            let db_path = data_dir.join("fanthom.db");
            let conn = db::open(&db_path).expect("failed to open database");

            app.manage(AppState::new(conn, data_dir));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 8: Run Rust tests**

```bash
cargo test -p fanthom-desktop
```

(The package name may differ based on Tauri scaffold — use whatever is in `apps/desktop/src-tauri/Cargo.toml` `[package].name`.)

Expected: db tests pass.

- [ ] **Step 9: Commit**

```bash
git add apps/ Cargo.toml
git commit -m "feat(desktop): scaffold Tauri app with SQLite database"
```

---

## Task 12: Tauri App — Sidecar & Pipeline Commands

**Files:**
- Create: `apps/desktop/src-tauri/src/sidecar.rs`
- Create: `apps/desktop/src-tauri/src/commands/mod.rs`
- Create: `apps/desktop/src-tauri/src/commands/pipeline.rs`
- Create: `apps/desktop/src-tauri/src/commands/tab.rs`
- Modify: `apps/desktop/src-tauri/src/main.rs`

- [ ] **Step 1: Write sidecar management**

Create `apps/desktop/src-tauri/src/sidecar.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

#[derive(Debug, Serialize)]
pub struct SidecarRequest {
    pub command: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct SidecarMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub step: String,
    #[serde(default)]
    pub percent: Option<u32>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

pub struct Sidecar {
    child: Child,
}

impl Sidecar {
    /// Spawn the Python AI pipeline sidecar.
    /// In dev mode, runs `python main.py` directly.
    /// In production, runs the bundled binary.
    pub fn spawn() -> Result<Self, String> {
        // Dev mode: run Python directly
        let child = Command::new("python")
            .arg("python/ai-pipeline/main.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn sidecar: {e}"))?;

        Ok(Self { child })
    }

    /// Send a command and collect all messages until a result or error is received.
    pub fn send_command(
        &mut self,
        request: &SidecarRequest,
        mut on_progress: impl FnMut(&SidecarMessage),
    ) -> Result<serde_json::Value, String> {
        let stdin = self.child.stdin.as_mut().ok_or("No stdin")?;
        let request_json = serde_json::to_string(request).map_err(|e| e.to_string())?;
        writeln!(stdin, "{}", request_json).map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;

        let stdout = self.child.stdout.as_mut().ok_or("No stdout")?;
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() {
                continue;
            }

            let msg: SidecarMessage = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse sidecar message: {e}: {line}"))?;

            match msg.msg_type.as_str() {
                "progress" => on_progress(&msg),
                "result" => return Ok(msg.data.unwrap_or(serde_json::Value::Null)),
                "error" => return Err(msg.message.unwrap_or_else(|| "Unknown error".to_string())),
                _ => {}
            }
        }

        Err("Sidecar closed unexpectedly".to_string())
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
```

- [ ] **Step 2: Write pipeline command**

Create `apps/desktop/src-tauri/src/commands/mod.rs`:

```rust
pub mod pipeline;
pub mod tab;
```

Create `apps/desktop/src-tauri/src/commands/pipeline.rs`:

```rust
use crate::db;
use crate::sidecar::{Sidecar, SidecarRequest};
use crate::state::AppState;
use serde::Serialize;
use tab_engine::{MidiNote, Tuning, optimize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
pub struct PipelineProgress {
    pub step: String,
    pub percent: u32,
    pub message: String,
}

#[tauri::command]
pub async fn process_url(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<tab_engine::TabSheet, String> {
    let song_id = uuid::Uuid::new_v4().to_string();
    let song_dir = state.songs_dir.join(&song_id);
    std::fs::create_dir_all(&song_dir).map_err(|e| e.to_string())?;
    let song_dir_str = song_dir.to_string_lossy().to_string();

    let mut sidecar = Sidecar::spawn()?;

    let emit_progress = |app: &AppHandle, step: &str, percent: u32, message: &str| {
        let _ = app.emit("pipeline:progress", PipelineProgress {
            step: step.to_string(),
            percent,
            message: message.to_string(),
        });
    };

    // Step 1: Extract audio
    emit_progress(&app, "extract", 0, "Starting audio extraction...");
    let extract_result = sidecar.send_command(
        &SidecarRequest {
            command: "extract".to_string(),
            params: serde_json::json!({ "url": url, "output_dir": song_dir_str }),
        },
        |msg| {
            let _ = app.emit("pipeline:progress", PipelineProgress {
                step: msg.step.clone(),
                percent: msg.percent.unwrap_or(0),
                message: msg.message.clone().unwrap_or_default(),
            });
        },
    )?;

    let audio_path = extract_result["audio_path"]
        .as_str()
        .ok_or("Missing audio_path in extract result")?
        .to_string();

    // Step 2: Separate bass track
    emit_progress(&app, "separate", 0, "Starting source separation...");
    let separate_result = sidecar.send_command(
        &SidecarRequest {
            command: "separate".to_string(),
            params: serde_json::json!({ "audio_path": audio_path, "output_dir": song_dir_str }),
        },
        |msg| {
            let _ = app.emit("pipeline:progress", PipelineProgress {
                step: msg.step.clone(),
                percent: msg.percent.unwrap_or(0),
                message: msg.message.clone().unwrap_or_default(),
            });
        },
    )?;

    let bass_path = separate_result["bass_path"]
        .as_str()
        .ok_or("Missing bass_path in separate result")?
        .to_string();

    // Step 3: Pitch detection
    emit_progress(&app, "transcribe", 0, "Starting pitch detection...");
    let transcribe_result = sidecar.send_command(
        &SidecarRequest {
            command: "transcribe".to_string(),
            params: serde_json::json!({ "bass_path": bass_path, "output_dir": song_dir_str }),
        },
        |msg| {
            let _ = app.emit("pipeline:progress", PipelineProgress {
                step: msg.step.clone(),
                percent: msg.percent.unwrap_or(0),
                message: msg.message.clone().unwrap_or_default(),
            });
        },
    )?;

    let midi_json_path = transcribe_result["midi_json_path"]
        .as_str()
        .ok_or("Missing midi_json_path")?;

    // Step 4: Tab Engine (Rust, in-process)
    emit_progress(&app, "convert", 0, "Generating tab notation...");

    let midi_json = std::fs::read_to_string(midi_json_path).map_err(|e| e.to_string())?;
    let midi_notes: Vec<MidiNote> = serde_json::from_str(&midi_json).map_err(|e| e.to_string())?;

    let sheet = optimize(&midi_notes, Tuning::Standard4, 120.0, (4, 4));

    // Save to DB
    let tab_data = rmp_serde::to_vec(&sheet).map_err(|e| e.to_string())?;
    let tab_id = uuid::Uuid::new_v4().to_string();
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::insert_song(&conn, &song_id, "Untitled", Some(&url), None, Some(sheet.tempo)).map_err(|e| e.to_string())?;
        db::insert_tab(&conn, &tab_id, &song_id, "standard4", 0, &tab_data).map_err(|e| e.to_string())?;
    }

    emit_progress(&app, "convert", 100, "Tab generation complete!");

    Ok(sheet)
}
```

- [ ] **Step 3: Write tab commands (transpose, optimize toggle, export)**

Create `apps/desktop/src-tauri/src/commands/tab.rs`:

```rust
use crate::state::AppState;
use tab_engine::{MidiNote, TabSheet, Tuning};

#[tauri::command]
pub fn transpose(
    state: tauri::State<'_, AppState>,
    midi_notes_json: String,
    semitones: i8,
) -> Result<TabSheet, String> {
    let notes: Vec<MidiNote> = serde_json::from_str(&midi_notes_json).map_err(|e| e.to_string())?;
    let sheet = tab_engine::transpose(&notes, semitones, Tuning::Standard4, 120.0, (4, 4));
    Ok(sheet)
}

#[tauri::command]
pub fn toggle_optimization(
    midi_notes_json: String,
    enabled: bool,
) -> Result<TabSheet, String> {
    let notes: Vec<MidiNote> = serde_json::from_str(&midi_notes_json).map_err(|e| e.to_string())?;

    if enabled {
        Ok(tab_engine::optimize(&notes, Tuning::Standard4, 120.0, (4, 4)))
    } else {
        // Greedy: pick first candidate (lowest string, lowest fret)
        let sheet_notes: Vec<tab_engine::TabNote> = notes.iter().map(|n| {
            let candidates = tab_engine::pitch_to_candidates(n.pitch, Tuning::Standard4);
            let c = candidates.first().expect("no candidates for pitch");
            tab_engine::TabNote {
                string: c.string,
                fret: c.fret,
                midi_pitch: n.pitch,
                onset: n.onset,
                duration: n.offset - n.onset,
                origin: tab_engine::NoteOrigin::Normal,
            }
        }).collect();
        Ok(TabSheet {
            notes: sheet_notes,
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        })
    }
}

#[tauri::command]
pub fn export_tab(sheet_json: String, format: String) -> Result<String, String> {
    let sheet: TabSheet = serde_json::from_str(&sheet_json).map_err(|e| e.to_string())?;
    match format.as_str() {
        "ascii" => Ok(tab_engine::export::ascii::export(&sheet)),
        "musicxml" => Ok(tab_engine::export::musicxml::export(&sheet)),
        _ => Err(format!("Unknown format: {format}")),
    }
}
```

- [ ] **Step 4: Update main.rs to register commands**

Replace `apps/desktop/src-tauri/src/main.rs`:

```rust
mod commands;
mod db;
mod sidecar;
mod state;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("failed to get app data dir");
            let data_dir = app_data.join("data");
            std::fs::create_dir_all(&data_dir)?;

            let db_path = data_dir.join("fanthom.db");
            let conn = db::open(&db_path).expect("failed to open database");

            app.manage(AppState::new(conn, data_dir));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pipeline::process_url,
            commands::tab::transpose,
            commands::tab::toggle_optimization,
            commands::tab::export_tab,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Verify it compiles**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom
cargo build -p fanthom-desktop
```

Expected: Compiles successfully (may have warnings, no errors).

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src-tauri/
git commit -m "feat(desktop): add sidecar management, pipeline orchestration, tab commands"
```

---

## Task 13: Frontend — App Shell & State

**Files:**
- Create: `apps/desktop/src/lib/types.ts`
- Create: `apps/desktop/src/lib/tauri.ts`
- Create: `apps/desktop/src/state.ts`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/main.tsx`
- Modify: `apps/desktop/src/index.css` (Tailwind)

- [ ] **Step 1: Add Tailwind import to `index.css`**

Replace `apps/desktop/src/index.css`:

```css
@import "tailwindcss";
```

- [ ] **Step 2: Create shared types**

Create `apps/desktop/src/lib/types.ts`:

```typescript
export interface MidiNote {
  pitch: number;
  onset: number;
  offset: number;
  velocity: number;
}

export type NoteOrigin =
  | "Normal"
  | "Optimized"
  | { OctaveShifted: number };

export interface TabNote {
  string: number;
  fret: number;
  midi_pitch: number;
  onset: number;
  duration: number;
  origin: NoteOrigin;
}

export interface TabSheet {
  notes: TabNote[];
  tempo: number;
  time_signature: [number, number];
  tuning: string;
  key_transpose: number;
}

export interface PipelineProgress {
  step: string;
  percent: number;
  message: string;
}

export type PipelineState =
  | { status: "idle" }
  | { status: "processing"; step: string; percent: number; message: string }
  | { status: "done" }
  | { status: "error"; message: string };
```

- [ ] **Step 3: Create Tauri invoke/listen wrappers**

Create `apps/desktop/src/lib/tauri.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PipelineProgress, TabSheet } from "./types";

export async function processUrl(url: string): Promise<TabSheet> {
  return invoke<TabSheet>("process_url", { url });
}

export async function transposeTab(
  midiNotesJson: string,
  semitones: number,
): Promise<TabSheet> {
  return invoke<TabSheet>("transpose", { midiNotesJson, semitones });
}

export async function toggleOptimization(
  midiNotesJson: string,
  enabled: boolean,
): Promise<TabSheet> {
  return invoke<TabSheet>("toggle_optimization", { midiNotesJson, enabled });
}

export async function exportTab(
  sheetJson: string,
  format: "ascii" | "musicxml",
): Promise<string> {
  return invoke<string>("export_tab", { sheetJson, format });
}

export function onPipelineProgress(
  callback: (progress: PipelineProgress) => void,
): Promise<UnlistenFn> {
  return listen<PipelineProgress>("pipeline:progress", (event) => {
    callback(event.payload);
  });
}
```

- [ ] **Step 4: Create state management**

Create `apps/desktop/src/state.ts`:

```typescript
import {
  createContext,
  useContext,
  type Dispatch,
} from "react";
import type { PipelineState, TabSheet } from "./lib/types";

export interface AppState {
  pipeline: PipelineState;
  tabSheet: TabSheet | null;
  midiNotesJson: string | null;
  transpose: number;
  optimized: boolean;
}

export const initialState: AppState = {
  pipeline: { status: "idle" },
  tabSheet: null,
  midiNotesJson: null,
  transpose: 0,
  optimized: true,
};

export type AppAction =
  | { type: "PIPELINE_START" }
  | { type: "PIPELINE_PROGRESS"; step: string; percent: number; message: string }
  | { type: "PIPELINE_DONE"; tabSheet: TabSheet; midiNotesJson: string }
  | { type: "PIPELINE_ERROR"; message: string }
  | { type: "SET_TAB"; tabSheet: TabSheet }
  | { type: "SET_TRANSPOSE"; value: number }
  | { type: "SET_OPTIMIZED"; value: boolean }
  | { type: "RESET" };

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "PIPELINE_START":
      return { ...state, pipeline: { status: "processing", step: "extract", percent: 0, message: "Starting..." } };
    case "PIPELINE_PROGRESS":
      return { ...state, pipeline: { status: "processing", step: action.step, percent: action.percent, message: action.message } };
    case "PIPELINE_DONE":
      return { ...state, pipeline: { status: "done" }, tabSheet: action.tabSheet, midiNotesJson: action.midiNotesJson };
    case "PIPELINE_ERROR":
      return { ...state, pipeline: { status: "error", message: action.message } };
    case "SET_TAB":
      return { ...state, tabSheet: action.tabSheet };
    case "SET_TRANSPOSE":
      return { ...state, transpose: action.value };
    case "SET_OPTIMIZED":
      return { ...state, optimized: action.value };
    case "RESET":
      return initialState;
    default:
      return state;
  }
}

export const AppStateContext = createContext<AppState>(initialState);
export const AppDispatchContext = createContext<Dispatch<AppAction>>(() => {});

export function useAppState() {
  return useContext(AppStateContext);
}

export function useAppDispatch() {
  return useContext(AppDispatchContext);
}
```

- [ ] **Step 5: Update App.tsx**

Replace `apps/desktop/src/App.tsx`:

```tsx
import { useReducer } from "react";
import { AppStateContext, AppDispatchContext, appReducer, initialState } from "./state";
import { UrlInput } from "./components/UrlInput";
import { PipelineProgress } from "./components/PipelineProgress";
import { TabCanvas } from "./components/TabCanvas";
import { TransposeControl } from "./components/TransposeControl";
import { OptimizeToggle } from "./components/OptimizeToggle";
import { ExportMenu } from "./components/ExportMenu";

export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialState);

  return (
    <AppStateContext.Provider value={state}>
      <AppDispatchContext.Provider value={dispatch}>
        <div className="flex flex-col h-screen bg-zinc-950 text-zinc-100">
          {/* Header */}
          <header className="border-b border-zinc-800 p-4">
            <UrlInput />
          </header>

          {/* Main */}
          <main className="flex-1 overflow-hidden relative">
            {state.pipeline.status === "processing" && <PipelineProgress />}
            {state.tabSheet && <TabCanvas />}
            {state.pipeline.status === "idle" && !state.tabSheet && (
              <div className="flex items-center justify-center h-full text-zinc-500">
                Paste a YouTube URL to get started
              </div>
            )}
            {state.pipeline.status === "error" && (
              <div className="flex items-center justify-center h-full text-red-400">
                {state.pipeline.message}
              </div>
            )}
          </main>

          {/* Controls */}
          {state.tabSheet && (
            <footer className="border-t border-zinc-800 p-4 flex items-center gap-6">
              <TransposeControl />
              <OptimizeToggle />
              <ExportMenu />
            </footer>
          )}
        </div>
      </AppDispatchContext.Provider>
    </AppStateContext.Provider>
  );
}
```

- [ ] **Step 6: Update main.tsx**

Replace `apps/desktop/src/main.tsx`:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

- [ ] **Step 7: Verify frontend compiles**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom/apps/desktop
pnpm typecheck
```

Expected: No type errors. (This step will fail until we create the component files in the next tasks — placeholder stubs are needed first.)

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/
git commit -m "feat(frontend): add app shell, state management, Tauri wrappers, types"
```

---

## Task 14: Frontend — URL Input & Pipeline Progress

**Files:**
- Create: `apps/desktop/src/components/UrlInput.tsx`
- Create: `apps/desktop/src/components/PipelineProgress.tsx`

- [ ] **Step 1: Create UrlInput component**

Create `apps/desktop/src/components/UrlInput.tsx`:

```tsx
import { useState } from "react";
import { useAppDispatch, useAppState } from "../state";
import { processUrl, onPipelineProgress } from "../lib/tauri";

const YOUTUBE_REGEX = /^https?:\/\/(www\.)?(youtube\.com\/watch\?v=|youtu\.be\/)/;

export function UrlInput() {
  const [url, setUrl] = useState("");
  const dispatch = useAppDispatch();
  const state = useAppState();
  const isProcessing = state.pipeline.status === "processing";

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!url.trim() || isProcessing) return;

    if (!YOUTUBE_REGEX.test(url)) {
      dispatch({ type: "PIPELINE_ERROR", message: "Please enter a valid YouTube URL" });
      return;
    }

    dispatch({ type: "PIPELINE_START" });

    const unlisten = await onPipelineProgress((progress) => {
      dispatch({
        type: "PIPELINE_PROGRESS",
        step: progress.step,
        percent: progress.percent,
        message: progress.message,
      });
    });

    try {
      const tabSheet = await processUrl(url);
      dispatch({ type: "PIPELINE_DONE", tabSheet, midiNotesJson: "" });
    } catch (err) {
      dispatch({ type: "PIPELINE_ERROR", message: String(err) });
    } finally {
      unlisten();
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex gap-3">
      <input
        type="text"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="Paste YouTube URL here..."
        disabled={isProcessing}
        className="flex-1 bg-zinc-900 border border-zinc-700 rounded-lg px-4 py-2 text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-zinc-500"
      />
      <button
        type="submit"
        disabled={isProcessing || !url.trim()}
        className="bg-amber-600 hover:bg-amber-500 disabled:bg-zinc-700 disabled:text-zinc-500 text-white font-medium px-6 py-2 rounded-lg transition-colors"
      >
        {isProcessing ? "Processing..." : "Fathom"}
      </button>
    </form>
  );
}
```

- [ ] **Step 2: Create PipelineProgress component**

Create `apps/desktop/src/components/PipelineProgress.tsx`:

```tsx
import { useAppState } from "../state";

const STEPS = [
  { key: "extract", label: "Extract Audio" },
  { key: "separate", label: "Separate Bass" },
  { key: "transcribe", label: "Detect Pitch" },
  { key: "convert", label: "Generate Tab" },
];

export function PipelineProgress() {
  const state = useAppState();
  if (state.pipeline.status !== "processing") return null;

  const currentStep = state.pipeline.step;
  const currentIdx = STEPS.findIndex((s) => s.key === currentStep);

  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center gap-6 bg-zinc-950/80 backdrop-blur-sm z-10">
      <div className="flex gap-2 items-center">
        {STEPS.map((step, i) => {
          const isDone = i < currentIdx;
          const isCurrent = i === currentIdx;
          return (
            <div key={step.key} className="flex items-center gap-2">
              <div
                className={`w-3 h-3 rounded-full ${
                  isDone
                    ? "bg-green-400"
                    : isCurrent
                      ? "bg-amber-400 animate-pulse"
                      : "bg-zinc-700"
                }`}
              />
              <span
                className={`text-sm ${
                  isDone
                    ? "text-green-400"
                    : isCurrent
                      ? "text-amber-400"
                      : "text-zinc-600"
                }`}
              >
                {step.label}
              </span>
              {i < STEPS.length - 1 && (
                <div className={`w-8 h-px ${isDone ? "bg-green-400" : "bg-zinc-700"}`} />
              )}
            </div>
          );
        })}
      </div>
      <p className="text-zinc-400 text-sm">{state.pipeline.message}</p>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/
git commit -m "feat(frontend): add UrlInput and PipelineProgress components"
```

---

## Task 15: Frontend — Tab Canvas Rendering

**Files:**
- Create: `apps/desktop/src/components/TabCanvas.tsx`

- [ ] **Step 1: Create TabCanvas component**

Create `apps/desktop/src/components/TabCanvas.tsx`:

```tsx
import { useRef, useEffect, useCallback } from "react";
import { useAppState } from "../state";
import type { TabNote, NoteOrigin } from "../lib/types";

const STRING_LABELS = ["E", "A", "D", "G"];
const COLORS = {
  Normal: "#E8A723",       // Gold
  Optimized: "#4ADE80",    // Green
  OctaveShifted: "#60A5FA", // Blue
  Technique: "#F472B6",    // Pink
};

const LINE_HEIGHT = 32;
const LEFT_MARGIN = 40;
const TOP_MARGIN = 20;
const PIXELS_PER_SECOND = 100;
const NOTE_FONT = "bold 14px monospace";
const LABEL_FONT = "12px monospace";

function getNoteColor(origin: NoteOrigin): string {
  if (origin === "Normal") return COLORS.Normal;
  if (origin === "Optimized") return COLORS.Optimized;
  if (typeof origin === "object" && "OctaveShifted" in origin) return COLORS.OctaveShifted;
  return COLORS.Normal;
}

export function TabCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const state = useAppState();
  const tabSheet = state.tabSheet;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container || !tabSheet) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Size canvas to container
    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;

    // Calculate needed width based on content
    const lastNote = tabSheet.notes.reduce(
      (max, n) => Math.max(max, n.onset + n.duration),
      0,
    );
    const contentWidth = Math.max(
      rect.width,
      LEFT_MARGIN + lastNote * PIXELS_PER_SECOND + 100,
    );

    canvas.width = contentWidth * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${contentWidth}px`;
    canvas.style.height = `${rect.height}px`;
    ctx.scale(dpr, dpr);

    // Clear
    ctx.fillStyle = "#09090b"; // zinc-950
    ctx.fillRect(0, 0, contentWidth, rect.height);

    const numStrings = STRING_LABELS.length;

    // Draw string lines (bottom = E, top = G)
    ctx.strokeStyle = "#3f3f46"; // zinc-700
    ctx.lineWidth = 1;
    for (let i = 0; i < numStrings; i++) {
      const y = TOP_MARGIN + (numStrings - 1 - i) * LINE_HEIGHT;
      ctx.beginPath();
      ctx.moveTo(LEFT_MARGIN, y);
      ctx.lineTo(contentWidth, y);
      ctx.stroke();
    }

    // Draw string labels
    ctx.fillStyle = "#a1a1aa"; // zinc-400
    ctx.font = LABEL_FONT;
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    for (let i = 0; i < numStrings; i++) {
      const y = TOP_MARGIN + (numStrings - 1 - i) * LINE_HEIGHT;
      ctx.fillText(STRING_LABELS[i], LEFT_MARGIN - 10, y);
    }

    // Draw notes
    ctx.font = NOTE_FONT;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const note of tabSheet.notes) {
      const x = LEFT_MARGIN + note.onset * PIXELS_PER_SECOND;
      const y = TOP_MARGIN + (numStrings - 1 - note.string) * LINE_HEIGHT;

      // Background circle
      const fretStr = note.fret.toString();
      const textWidth = ctx.measureText(fretStr).width;
      const radius = Math.max(textWidth / 2 + 4, 10);

      ctx.fillStyle = "#09090b";
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fill();

      // Fret number
      ctx.fillStyle = getNoteColor(note.origin);
      ctx.fillText(fretStr, x, y);
    }
  }, [tabSheet]);

  useEffect(() => {
    draw();
    window.addEventListener("resize", draw);
    return () => window.removeEventListener("resize", draw);
  }, [draw]);

  return (
    <div ref={containerRef} className="w-full h-full overflow-x-auto overflow-y-hidden">
      <canvas ref={canvasRef} />
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/components/TabCanvas.tsx
git commit -m "feat(frontend): add Canvas 2D tab rendering with color-coded notes"
```

---

## Task 16: Frontend — Controls & Export

**Files:**
- Create: `apps/desktop/src/components/TransposeControl.tsx`
- Create: `apps/desktop/src/components/OptimizeToggle.tsx`
- Create: `apps/desktop/src/components/ExportMenu.tsx`

- [ ] **Step 1: Create TransposeControl**

Create `apps/desktop/src/components/TransposeControl.tsx`:

```tsx
import { useAppState, useAppDispatch } from "../state";
import { transposeTab } from "../lib/tauri";

export function TransposeControl() {
  const state = useAppState();
  const dispatch = useAppDispatch();

  async function handleChange(e: React.ChangeEvent<HTMLInputElement>) {
    const semitones = parseInt(e.target.value, 10);
    dispatch({ type: "SET_TRANSPOSE", value: semitones });

    if (!state.midiNotesJson) return;

    try {
      const newTab = await transposeTab(state.midiNotesJson, semitones);
      dispatch({ type: "SET_TAB", tabSheet: newTab });
    } catch (err) {
      console.error("Transpose failed:", err);
    }
  }

  return (
    <div className="flex items-center gap-3">
      <label className="text-zinc-400 text-sm">Transpose</label>
      <span className="text-zinc-500 text-xs w-6 text-right">
        {state.transpose > 0 ? `+${state.transpose}` : state.transpose}
      </span>
      <input
        type="range"
        min={-12}
        max={12}
        value={state.transpose}
        onChange={handleChange}
        className="w-40 accent-amber-500"
      />
    </div>
  );
}
```

- [ ] **Step 2: Create OptimizeToggle**

Create `apps/desktop/src/components/OptimizeToggle.tsx`:

```tsx
import { useAppState, useAppDispatch } from "../state";
import { toggleOptimization } from "../lib/tauri";

export function OptimizeToggle() {
  const state = useAppState();
  const dispatch = useAppDispatch();

  async function handleToggle() {
    const newValue = !state.optimized;
    dispatch({ type: "SET_OPTIMIZED", value: newValue });

    if (!state.midiNotesJson) return;

    try {
      const newTab = await toggleOptimization(state.midiNotesJson, newValue);
      dispatch({ type: "SET_TAB", tabSheet: newTab });
    } catch (err) {
      console.error("Toggle optimization failed:", err);
    }
  }

  return (
    <button
      onClick={handleToggle}
      className={`px-4 py-1.5 rounded-lg text-sm font-medium transition-colors ${
        state.optimized
          ? "bg-green-900/50 text-green-400 border border-green-700"
          : "bg-zinc-800 text-zinc-400 border border-zinc-700"
      }`}
    >
      Optimize {state.optimized ? "ON" : "OFF"}
    </button>
  );
}
```

- [ ] **Step 3: Create ExportMenu**

Create `apps/desktop/src/components/ExportMenu.tsx`:

```tsx
import { useState } from "react";
import { useAppState } from "../state";
import { exportTab } from "../lib/tauri";

export function ExportMenu() {
  const state = useAppState();
  const [open, setOpen] = useState(false);

  async function handleExport(format: "ascii" | "musicxml") {
    setOpen(false);
    if (!state.tabSheet) return;

    try {
      const sheetJson = JSON.stringify(state.tabSheet);
      const result = await exportTab(sheetJson, format);

      if (format === "ascii") {
        await navigator.clipboard.writeText(result);
      } else {
        // MusicXML: trigger download
        const blob = new Blob([result], { type: "application/xml" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "tab.musicxml";
        a.click();
        URL.revokeObjectURL(url);
      }
    } catch (err) {
      console.error("Export failed:", err);
    }
  }

  return (
    <div className="relative ml-auto">
      <button
        onClick={() => setOpen(!open)}
        className="bg-zinc-800 hover:bg-zinc-700 text-zinc-300 px-4 py-1.5 rounded-lg text-sm font-medium border border-zinc-700 transition-colors"
      >
        Export
      </button>
      {open && (
        <div className="absolute bottom-full mb-2 right-0 bg-zinc-800 border border-zinc-700 rounded-lg shadow-xl overflow-hidden">
          <button
            onClick={() => handleExport("ascii")}
            className="block w-full text-left px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-700"
          >
            Copy ASCII Tab
          </button>
          <button
            onClick={() => handleExport("musicxml")}
            className="block w-full text-left px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-700"
          >
            Download MusicXML
          </button>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Verify frontend compiles**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom/apps/desktop
pnpm typecheck
```

Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/components/
git commit -m "feat(frontend): add TransposeControl, OptimizeToggle, ExportMenu"
```

---

## Task 17: Integration Smoke Test

**Files:**
- No new files — this task verifies the full build works.

- [ ] **Step 1: Run all Rust tests**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom
cargo test --workspace
```

Expected: All tab-engine tests pass. Desktop tests pass (db module).

- [ ] **Step 2: Run all Python tests**

```bash
python -m pytest python/ai-pipeline/tests/ -v
```

Expected: All protocol, extract, separate, transcribe tests pass.

- [ ] **Step 3: Run frontend typecheck**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom/apps/desktop
pnpm typecheck
```

Expected: No type errors.

- [ ] **Step 4: Build the Tauri app in dev mode**

```bash
cd /Users/limdongyoung0/Develop/d0lim/fanthom
pnpm dev
```

Expected: Tauri window opens, URL input is visible, no runtime errors in console.

- [ ] **Step 5: Commit any fixes from integration**

```bash
git add -A
git commit -m "fix: resolve integration issues from smoke test"
```

(Skip this step if no fixes were needed.)

---

## Task 18: CI/CD Setup

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create CI workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rust:
    name: Rust Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run tests
        run: cargo test --workspace
      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings

  python:
    name: Python Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.11"
      - name: Install dependencies
        run: pip install pytest
      - name: Run tests
        run: python -m pytest python/ai-pipeline/tests/ -v
        env:
          PYTHONPATH: python/ai-pipeline

  frontend:
    name: Frontend Checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: "22"
          cache: pnpm
      - name: Install dependencies
        run: pnpm install
      - name: Typecheck
        run: pnpm typecheck
```

- [ ] **Step 2: Commit**

```bash
git add .github/
git commit -m "ci: add GitHub Actions workflows for Rust, Python, and frontend"
```

---

## Spec Coverage Check

| Spec Section | Task(s) |
|-------------|---------|
| 1. Overview (goals, users) | Covered by overall architecture |
| 2.1 Architecture (Tauri Command-centric) | Task 11, 12 |
| 2.2 Monorepo structure | Task 1 |
| 3.1 Pipeline data flow | Task 12 (pipeline command) |
| 3.2 Python sidecar protocol | Task 7 |
| 3.3 Progress to UI | Task 12, 14 |
| 4.1 Data model | Task 2 |
| 4.2 Viterbi DP | Task 3 |
| 4.3 Transpose | Task 4 |
| 4.4 Export (ASCII) | Task 5 |
| 4.4 Export (MusicXML) | Task 6 |
| 5.1 SQLite schema | Task 11 |
| 5.2 Filesystem structure | Task 11, 12 |
| 6.1-6.3 Frontend layout/state/components | Task 13, 14, 15, 16 |
| 6.4 Tauri integration | Task 13 |
| 6.5 Color convention | Task 15 |
| 7.1 Dev environment | Task 1 |
| 7.3 CI/CD | Task 18 |
| 8 MVP scope | All P0 features covered in Tasks 2-16 |
| 9 Tech stack | Verified across all tasks |
