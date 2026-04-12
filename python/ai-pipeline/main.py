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
