# Fanthom

**Depth of Sound, Surfaced.**

Fanthom is an open-source desktop app that extracts bass tracks from YouTube videos and generates optimized bass guitar tablature using AI. The entire pipeline runs locally on your machine — no server, no subscription, no data leaving your computer.

## How It Works

```
YouTube URL
    |
    v
1. Audio Extraction (yt-dlp)
    |
    v
2. Bass Track Separation (Demucs v4)
    |
    v
3. Pitch Detection (basic-pitch)
    |
    v
4. Tab Generation + Fingering Optimization (Viterbi DP)
    |
    v
Optimized Bass Tab
```

Paste a YouTube URL, and Fanthom will:

1. **Extract** the audio from the video
2. **Separate** the bass track using Meta's Demucs neural network
3. **Detect** pitches using Spotify's basic-pitch model
4. **Generate** a tab with Viterbi DP fingering optimization that minimizes hand movement

## Features

- **Viterbi DP Fingering Optimization** — Finds the optimal fingering path across the entire song, minimizing hand position jumps, avoiding awkward stretches, and preferring the comfort zone (frets 2-9)
- **Transpose** — Shift up or down by up to 12 semitones with automatic octave correction for notes that fall outside the bass range
- **Color-Coded Notation** — Gold for normal notes, green for optimized positions, blue for octave-shifted notes
- **Export** — MusicXML (open in MuseScore, Guitar Pro, etc.) or ASCII text tab (copy to clipboard)
- **Fully Local** — All processing happens on your machine. No cloud, no account, no tracking.

## Getting Started

### Prerequisites

- [mise](https://mise.jdx.dev/) — Polyglot runtime manager

### Setup

```bash
git clone https://github.com/d0lim/fanthom.git
cd fanthom
mise install
mise run setup
```

This installs Node.js, Python, Rust, and all project dependencies.

### Run

```bash
mise run dev
```

Opens the Fanthom desktop app in development mode.

### Test

```bash
mise run test          # All tests (Rust + Python + TypeScript)
mise run test:rust     # Rust only
mise run test:python   # Python only
mise run test:frontend # TypeScript typecheck only
```

## Architecture

Fanthom is a Tauri v2 desktop app with a monorepo structure:

```
fanthom/
├── crates/tab-engine/     # Rust — Tab generation engine (pure library)
│   ├── midi.rs            #   MIDI note parsing
│   ├── tab.rs             #   Note-to-fretboard mapping
│   ├── viterbi.rs         #   Viterbi DP fingering optimization
│   ├── transpose.rs       #   Transposition with octave correction
│   └── export/            #   MusicXML + ASCII text export
├── python/ai-pipeline/    # Python — AI sidecar process
│   ├── extract.py         #   yt-dlp audio extraction
│   ├── separate.py        #   Demucs source separation
│   └── transcribe.py      #   basic-pitch pitch detection
├── apps/desktop/          # Tauri v2 desktop app
│   ├── src/               #   React + Tailwind frontend
│   └── src-tauri/         #   Rust backend (commands, SQLite, sidecar mgmt)
└── .github/workflows/     # CI
```

**Data flow:** React UI invokes Rust commands via Tauri. Rust orchestrates the Python sidecar (JSON Lines over stdin/stdout) for AI workloads, then runs the tab engine in-process for instant tab generation and transpose.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri v2 |
| Frontend | React, Vite, Tailwind CSS |
| Tab Engine | Rust |
| AI Pipeline | Python (Demucs, basic-pitch, yt-dlp) |
| Database | SQLite |
| Runtime Management | mise |

## Viterbi DP Optimization

The fingering optimizer uses a Viterbi algorithm (same principle as speech recognition) to find the minimum-cost path through all possible fret positions:

| Cost Factor | Effect |
|------------|--------|
| A/D/G open string penalty | Avoids open strings with poor tone control |
| Hand position movement | Penalizes large fret jumps |
| String crossing | Small penalty per string change |
| Cross stretching | Penalizes wide finger spans across strings |
| Comfort zone | Prefers frets 2-9, penalizes high frets |

Time complexity is O(N) for 4-string bass (at most 4 candidates per note), processing a full song in milliseconds.

## Roadmap

- [ ] Technique detection (slap, pop, hammer-on, pull-off)
- [ ] Loop playback with BPM control
- [ ] PDF export
- [ ] 5-string / 6-string bass support
- [ ] Guitar and drum track support

## License

[MIT](LICENSE)
