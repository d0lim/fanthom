import json
import os
from unittest.mock import patch

from transcribe import run_transcribe


def test_run_transcribe_success(tmp_path, capsys):
    bass_path = str(tmp_path / "bass.wav")
    with open(bass_path, "wb") as f:
        f.write(b"RIFF" + b"\x00" * 100)

    output_dir = str(tmp_path)

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
