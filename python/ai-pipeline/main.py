#!/usr/bin/env python3
"""Fanthom AI Pipeline — stdin/stdout JSON Lines sidecar."""

import logging
import os
import sys
import traceback

from protocol import read_command, send_error


def setup_logging() -> None:
    log_dir = os.path.join(os.path.expanduser("~"), "fanthom", "logs")
    os.makedirs(log_dir, exist_ok=True)
    log_path = os.path.join(log_dir, "sidecar.log")

    logging.basicConfig(
        level=logging.DEBUG,
        format="%(asctime)s [%(levelname)s] %(message)s",
        handlers=[
            logging.FileHandler(log_path, encoding="utf-8"),
            logging.StreamHandler(sys.stderr),
        ],
    )
    logging.info("Sidecar started (PID %d)", os.getpid())
    logging.info("Python: %s", sys.executable)
    logging.info("Working dir: %s", os.getcwd())


def main() -> None:
    setup_logging()

    while True:
        cmd = read_command()
        if cmd is None:
            logging.info("stdin closed, shutting down")
            break

        command = cmd.get("command")
        params = cmd.get("params", {})
        logging.info("Received command: %s, params: %s", command, params)

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
            logging.exception("Command '%s' failed with exception", command)
            send_error(command or "unknown", f"{type(e).__name__}: {e}")
            traceback.print_exc(file=sys.stderr)


if __name__ == "__main__":
    main()
