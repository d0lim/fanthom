# Fanthom — Design Specification

**Depth of Sound, Surfaced.**

Version: 1.0
Date: 2026-04-12
License: MIT

---

## 1. Overview

Fanthom is an open-source desktop application (Tauri) that extracts bass tracks from YouTube videos and generates optimized bass guitar tablature using AI. The entire pipeline — audio extraction, source separation, pitch detection, and tab generation with fingering optimization — runs locally on the user's machine.

### Goals

- Automate the full workflow: YouTube URL → isolated bass track → optimized tab notation
- Run entirely offline after initial setup (no server dependency)
- Provide accurate fingering positions via Viterbi DP optimization
- Export to open formats (MusicXML, ASCII text tab)

### Target Users

**Primary:** Bass guitar players (hobbyist to intermediate) who learn songs from YouTube.
**Secondary:** Band musicians (guitarists, keyboardists) in the rhythm section. Instrument expansion planned for future phases.

---

## 2. Architecture

### 2.1 High-Level Architecture

Tauri Command-centric architecture. Rust is the orchestration core; React is the presentation layer; Python runs as a sidecar process for AI workloads.

```
[React UI] <--invoke()--> [Tauri Commands (Rust)] <--stdin/stdout JSON--> [Python Sidecar]
                                |
                          Tab Engine (Rust lib)
                          SQLite (rusqlite)
                          File System
```

### 2.2 Monorepo Structure

```
fanthom/
├── apps/
│   └── desktop/                 # Tauri app
│       ├── src/                 # React + Vite (Frontend)
│       ├── src-tauri/           # Rust (Tauri commands, sidecar mgmt)
│       │   ├── src/
│       │   │   ├── main.rs
│       │   │   ├── commands/    # invoke() handlers
│       │   │   ├── pipeline/    # Pipeline orchestration
│       │   │   ├── sidecar/     # Python sidecar management
│       │   │   └── db/          # SQLite access
│       │   ├── Cargo.toml
│       │   └── tauri.conf.json
│       ├── package.json
│       └── vite.config.ts
├── crates/
│   └── tab-engine/              # Pure Rust library
│       ├── src/
│       │   ├── lib.rs
│       │   ├── midi.rs          # MIDI parsing/conversion
│       │   ├── tab.rs           # MIDI -> (string, fret) mapping
│       │   ├── viterbi.rs       # Fingering optimization DP
│       │   ├── transpose.rs     # Transposition + octave correction
│       │   └── export/
│       │       ├── musicxml.rs
│       │       └── ascii.rs
│       └── Cargo.toml
├── python/
│   └── ai-pipeline/             # Python sidecar
│       ├── main.py              # CLI entrypoint (stdin/stdout JSON)
│       ├── extract.py           # yt-dlp audio extraction
│       ├── separate.py          # Demucs source separation
│       ├── transcribe.py        # basic-pitch pitch detection
│       ├── requirements.txt
│       └── pyproject.toml
├── .mise.toml                   # Rust + Node + Python runtimes
├── Cargo.toml                   # Workspace root
├── package.json                 # pnpm workspace root
├── pnpm-workspace.yaml
└── LICENSE                      # MIT
```

Key decisions:
- `crates/tab-engine` is a pure Rust library with no Tauri dependency. Independently testable, reusable as CLI tool or WASM build.
- Python sidecar communicates via JSON Lines over stdin/stdout. Bundled as a binary via PyInstaller at build time.
- `apps/desktop/src-tauri/` is the orchestrator — coordinates Python calls, tab engine, DB, and file management.

---

## 3. Pipeline Data Flow

### 3.1 Processing Pipeline

```
User inputs YouTube URL
        |
        v
[React] --invoke("process_url")--> [Rust Command]
        |
        v
   Step 1: yt-dlp extract (Python)
        | audio.wav
        v
   Step 2: Demucs v4 separate (Python)
        | bass.wav
        v
   Step 3: basic-pitch transcribe (Python)
        | MIDI notes (JSON)
        v
   Step 4: Tab Engine (Rust, in-process)
        | Viterbi DP optimization
        v
   Optimized Tab JSON
        |
        +--------+--------+
        v                 v
    SQLite save      Return to React
```

Steps 1-3 are sequential — each depends on the output of the previous step. Step 4 runs in-process in Rust with no sidecar overhead.

### 3.2 Python Sidecar Protocol

Rust and Python communicate via JSON Lines over stdin/stdout. Each pipeline step is an independent command.

**Request (Rust -> Python):**
```json
{"command": "extract", "params": {"url": "https://youtube.com/...", "output_dir": "/tmp/fanthom/abc123"}}
```

**Progress (Python -> Rust):**
```json
{"type": "progress", "step": "extract", "percent": 45, "message": "Downloading audio..."}
```

**Result (Python -> Rust):**
```json
{"type": "result", "step": "extract", "data": {"audio_path": "/tmp/fanthom/abc123/audio.wav"}}
```

**Error (Python -> Rust):**
```json
{"type": "error", "step": "extract", "message": "Video unavailable"}
```

### 3.3 Progress Reporting to UI

```
Python stdout -> Rust (parse) -> Tauri event emit -> React listener -> UI update
```

Rust commands use `app_handle.emit()` to push real-time progress events. React listens via `listen()` to update the 4-step progress bar.

---

## 4. Tab Engine (Rust)

### 4.1 Data Model

```rust
struct MidiNote {
    pitch: u8,          // MIDI number (28=E1 .. 67=G4)
    onset: f64,         // Start time in seconds
    offset: f64,        // End time in seconds
    velocity: u8,       // Intensity (0-127)
}

struct TabNote {
    string: u8,         // String number (0=E, 1=A, 2=D, 3=G)
    fret: u8,           // Fret number (0-24)
    midi_pitch: u8,     // Original MIDI pitch
    onset: f64,
    duration: f64,
    origin: NoteOrigin,
}

enum NoteOrigin {
    Normal,             // Default mapping (gold)
    Optimized,          // Repositioned by Viterbi (green)
    OctaveShifted(i8),  // +1 = 8va up, -1 = 8vb down (blue)
}

struct TabSheet {
    notes: Vec<TabNote>,
    tempo: f64,
    time_signature: (u8, u8),
    tuning: Tuning,
    key_transpose: i8,
}

enum Tuning {
    Standard4,  // E-A-D-G
    Standard5,  // B-E-A-D-G  (Phase 2)
    Standard6,  // B-E-A-D-G-C (Phase 2)
}
```

### 4.2 Viterbi DP Fingering Optimization

For each note in the sequence:
1. Generate candidates: pitch -> possible (string, fret) combinations (up to 4 for 4-string bass)
2. Compute transition cost from each previous candidate to each current candidate
3. Backtrack to find minimum-cost path

**Cost function constants:**

| Factor | Value | Constant |
|--------|-------|----------|
| A/D/G open string penalty | +60 | `OPEN_STRING_PENALTY` |
| Hand position movement | 0-5x multiplier | `position_move_cost()` |
| String crossing | +2 per string, -0.5 same string | `STRING_CROSS_COST` |
| Cross stretching | +3 per fret (beyond 4-fret span) | `STRETCH_PENALTY` |
| Comfort zone | -1.5 (frets 2-9), +1 (10-14), +3 (15+) | `comfort_zone_cost()` |

**Time complexity:** O(N * C^2) where N = total notes, C = candidates per note. With C <= 4 for 4-string bass, effectively O(N) linear time. A 5-minute song (~500-1000 notes) completes in milliseconds.

### 4.3 Transpose

MIDI absolute value based transposition. After shifting, notes outside the bass range (E1=MIDI 28 to 24th fret=MIDI 67) are corrected by octave (+12/-12). Octave-shifted notes are tagged as `OctaveShifted` and rendered in blue.

```
transpose(sheet, semitones):
  for note in sheet.notes:
    new_pitch = note.midi_pitch + semitones
    if new_pitch < 28:
      new_pitch += 12
      note.origin = OctaveShifted(+1)
    elif new_pitch > 67:
      new_pitch -= 12
      note.origin = OctaveShifted(-1)
  re-run viterbi on transposed notes
```

### 4.4 Export

- **MusicXML**: Direct XML generation from `TabSheet`. No external library needed — the tab-only subset of MusicXML is straightforward.
- **ASCII Tab**: Standard 4-line text format, split by measures.

```
G|-------5---7---|
D|---5-7---------|
A|-7-------------|
E|---------------|
```

---

## 5. Data Storage

### 5.1 SQLite Schema

```sql
CREATE TABLE songs (
    id          TEXT PRIMARY KEY,  -- UUID
    title       TEXT NOT NULL,
    source_url  TEXT,              -- YouTube URL (nullable for file uploads)
    duration    REAL,
    tempo       REAL,
    created_at  TEXT NOT NULL,     -- ISO 8601
    updated_at  TEXT NOT NULL
);

CREATE TABLE tabs (
    id          TEXT PRIMARY KEY,
    song_id     TEXT NOT NULL REFERENCES songs(id),
    tuning      TEXT NOT NULL DEFAULT 'standard4',
    transpose   INTEGER NOT NULL DEFAULT 0,
    tab_data    BLOB NOT NULL,     -- MessagePack serialized TabSheet
    created_at  TEXT NOT NULL
);

CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);
```

Tab data is serialized with MessagePack (`rmp-serde` in Rust) — smaller and faster than JSON.

### 5.2 Filesystem Structure

```
~/fanthom/
├── data/
│   ├── fanthom.db              # SQLite database
│   └── songs/
│       └── {song_id}/
│           ├── original.wav    # Extracted original audio
│           ├── bass.wav        # Separated bass track
│           └── midi.json       # basic-pitch output (intermediate)
└── config/
    └── settings.toml           # App-level settings (window size, theme)
```

- Audio files stay on the filesystem, not in the database
- Song directories enable easy manual deletion and backup
- `settings.toml` for app-level UI config; SQLite `settings` table for song-related preferences

---

## 6. Frontend (React)

### 6.1 Layout

Single-page application with 3 areas:

```
+---------------------------------------------+
|  Header: URL input bar + file upload button  |
+---------------------------------------------+
|                                              |
|  Main: Tab notation (Canvas 2D)              |
|  - Scrollable notation area                  |
|  - Note colors: gold/green/blue/pink         |
|                                              |
+---------------------------------------------+
|  Controls:                                   |
|  [Transpose  -12 ====o==== +12]              |
|  [Optimize ON/OFF]  [Export v]               |
+---------------------------------------------+
```

### 6.2 State Management

React Context + useReducer. No external state library needed — the state is simple and component tree is shallow.

```typescript
interface AppState {
  pipeline: PipelineState; // idle | processing(step, percent) | done | error(msg)
  currentSong: Song | null;
  tabSheet: TabSheet | null;
  transpose: number;       // -12 to +12
  optimized: boolean;
}
```

### 6.3 Core Components

| Component | Responsibility |
|-----------|---------------|
| `UrlInput` | YouTube URL input + validation + file upload |
| `PipelineProgress` | 4-step progress display (extract -> separate -> transcribe -> convert) |
| `TabCanvas` | Canvas 2D tab rendering with scroll and zoom |
| `TransposeControl` | +/-12 semitone slider |
| `OptimizeToggle` | Viterbi fingering optimization ON/OFF |
| `ExportMenu` | MusicXML / ASCII text export |

### 6.4 Tauri Integration

```typescript
// Start URL processing
const result = await invoke<TabSheet>("process_url", { url });

// Listen for pipeline progress
await listen<ProgressEvent>("pipeline:progress", (event) => {
  dispatch({ type: "PROGRESS", payload: event.payload });
});

// Transpose (recalculated in Rust, returned instantly)
const newTab = await invoke<TabSheet>("transpose", { songId, semitones: 3 });

// Toggle fingering optimization
const newTab = await invoke<TabSheet>("toggle_optimization", { songId, enabled: true });
```

Transpose and optimization toggles are computed in Rust in milliseconds, enabling instant UI response.

### 6.5 Tab Rendering Color Convention

| Color | Hex | Meaning |
|-------|-----|---------|
| Gold | #E8A723 | Normal notes — default tab notation |
| Green | #4ADE80 | Optimized notes — repositioned by Viterbi DP |
| Blue | #60A5FA | Octave-shifted notes — transposed outside bass range |
| Pink | #F472B6 | Technique markers — slap (S), pop (P), etc. |

---

## 7. Build & Distribution

### 7.1 Development Environment

```toml
# .mise.toml
[tools]
node = "22"
python = "3.11"
rust = "1.83"

[env]
PYTHONPATH = "./python/ai-pipeline"
```

```bash
mise install
pnpm install
pip install -r python/ai-pipeline/requirements.txt
cargo build
pnpm --filter desktop tauri dev
```

### 7.2 Python Sidecar Bundling

The Python AI pipeline is packaged into a standalone binary via PyInstaller and registered as a Tauri sidecar.

```jsonc
// tauri.conf.json (excerpt)
{
  "bundle": {
    "externalBin": ["binaries/ai-pipeline"]
  }
}
```

Build steps:
1. PyInstaller packages `ai-pipeline` into a single binary
2. Binary is registered in `tauri.conf.json` `externalBin`
3. Platform-specific binaries are included automatically (macOS/Windows/Linux)

### 7.3 CI/CD (GitHub Actions)

| Workflow | Trigger | Description |
|----------|---------|-------------|
| `ci.yml` | PR, push to main | Rust tests + Clippy, frontend lint + typecheck, Python tests |
| `release.yml` | Git tag `v*` | Tauri build -> macOS (.dmg), Windows (.msi), Linux (.AppImage). Upload to GitHub Releases |

Uses `tauri-apps/tauri-action` for automated cross-platform builds.

### 7.4 Target Platforms (MVP)

1. **macOS** (Apple Silicon + Intel) — primary
2. **Windows** — secondary
3. **Linux** (AppImage) — tertiary

---

## 8. MVP Scope

### Included (P0)

- YouTube URL audio extraction (yt-dlp sidecar)
- Demucs bass track separation (Python sidecar)
- basic-pitch pitch detection (Python sidecar)
- MIDI to tab conversion + Viterbi DP fingering optimization (Rust)
- Canvas 2D tab rendering with color-coded notes
- Transpose +/-12 semitones with octave correction
- Export: MusicXML + ASCII text tab

### Excluded from MVP (Future Phases)

- Technique detection and marking (slap/pop/hammer-on) — Phase 2
- Loop playback with BPM control — Phase 2
- PDF export — Phase 2
- 5-string / 6-string bass support — Phase 2
- User accounts / cloud sync — not planned (desktop-first)
- Community tab sharing — Phase 3
- Guitar/drum track support — Phase 3

---

## 9. Tech Stack Summary

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Desktop shell | Tauri v2 | Native window, sidecar management, system APIs |
| Frontend | React + Vite + Tailwind | UI, Canvas tab rendering |
| Core engine | Rust (tab-engine crate) | MIDI conversion, Viterbi DP, transpose, export |
| AI pipeline | Python (Demucs + basic-pitch) | Audio separation, pitch detection |
| Audio extraction | yt-dlp (Python) | YouTube audio download |
| Database | SQLite (rusqlite) | Song metadata, tab storage |
| Serialization | MessagePack (rmp-serde) | Tab data binary format |
| Runtime management | mise | Node, Python, Rust version pinning |
| Build | PyInstaller + tauri-action | Sidecar bundling, cross-platform release |
