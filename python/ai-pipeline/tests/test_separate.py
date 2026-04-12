import json
import os
from unittest.mock import MagicMock, patch

from separate import run_separate


def test_run_separate_success(tmp_path, capsys):
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
