"""Pitch detection via basic-pitch (Spotify)."""

import json
import logging
import os

from protocol import send_error, send_progress, send_result

# Lazy-loaded by _ensure_loaded(). Tests can patch `transcribe.predict` directly.
predict = None


def _ensure_loaded() -> None:
    """Import basic-pitch on first use. Numba JIT makes this slow on the first call."""
    global predict
    if predict is not None:
        return
    from basic_pitch.inference import predict as _p
    predict = _p


def run_transcribe(params: dict) -> None:
    bass_path = params["bass_path"]
    output_dir = params["output_dir"]

    if not os.path.exists(bass_path):
        send_error("transcribe", f"Bass audio not found: {bass_path}")
        return

    send_progress("transcribe", 0, "Starting pitch detection...")

    try:
        send_progress("transcribe", 5, "Loading pitch detection model (first run may take a while)...")
        logging.info("Loading basic-pitch model...")
        _ensure_loaded()
        logging.info("Model loaded")

        send_progress("transcribe", 20, "Running pitch detection...")
        logging.info("Running predict on %s", bass_path)

        model_output, midi_data, note_events = predict(bass_path)

        logging.info("Prediction complete, %d note events", len(note_events))
        send_progress("transcribe", 80, "Processing note events...")

        notes = []
        for event in note_events:
            notes.append({
                "pitch": int(event[2]),
                "onset": float(event[0]),
                "offset": float(event[1]),
                "velocity": min(127, max(0, int(event[3] * 127))),
            })

        notes.sort(key=lambda n: n["onset"])

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
        logging.exception("Pitch detection failed")
        send_error("transcribe", f"Pitch detection failed: {e}")
