import json
import sys


def send_progress(step: str, percent: int, message: str) -> None:
    msg = {"type": "progress", "step": step, "percent": percent, "message": message}
    print(json.dumps(msg), flush=True)


def send_result(step: str, data: dict) -> None:
    msg = {"type": "result", "step": step, "data": data}
    print(json.dumps(msg), flush=True)


def send_error(step: str, message: str) -> None:
    msg = {"type": "error", "step": step, "message": message}
    print(json.dumps(msg), flush=True)


def read_command() -> dict | None:
    line = sys.stdin.readline()
    if not line:
        return None
    return json.loads(line.strip())
