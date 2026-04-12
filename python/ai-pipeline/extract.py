"""YouTube audio extraction via yt-dlp."""

import os
import subprocess

from protocol import send_error, send_progress, send_result


def run_extract(params: dict) -> None:
    url = params["url"]
    output_dir = params["output_dir"]
    os.makedirs(output_dir, exist_ok=True)

    output_path = os.path.join(output_dir, "original.wav")

    send_progress("extract", 0, "Starting audio extraction...")

    try:
        cmd = [
            "yt-dlp",
            "--extract-audio",
            "--audio-format", "wav",
            "--audio-quality", "0",
            "--output", output_path.replace(".wav", ".%(ext)s"),
            "--no-playlist",
            url,
        ]

        process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        send_progress("extract", 30, "Downloading audio...")

        _, stderr = process.communicate()

        if process.returncode != 0:
            send_error("extract", f"yt-dlp failed: {stderr.strip()}")
            return

        if not os.path.exists(output_path):
            for ext in [".wav", ".webm", ".m4a", ".mp3"]:
                alt = output_path.replace(".wav", ext)
                if os.path.exists(alt) and alt != output_path:
                    os.rename(alt, output_path)
                    break

        if not os.path.exists(output_path):
            send_error("extract", "Output file not found after extraction")
            return

        send_progress("extract", 100, "Audio extraction complete")
        send_result("extract", {"audio_path": output_path})

    except FileNotFoundError:
        send_error("extract", "yt-dlp not found. Please install: pip install yt-dlp")
