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
