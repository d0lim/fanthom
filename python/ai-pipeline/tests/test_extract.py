import json
import os
from unittest.mock import MagicMock, patch

from extract import run_extract


def test_run_extract_success(tmp_path, capsys):
    output_dir = str(tmp_path / "song123")
    output_path = os.path.join(output_dir, "original.wav")

    def mock_popen(*args, **kwargs):
        os.makedirs(output_dir, exist_ok=True)
        with open(output_path, "wb") as f:
            f.write(b"RIFF" + b"\x00" * 100)
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
