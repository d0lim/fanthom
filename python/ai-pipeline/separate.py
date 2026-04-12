"""Bass track separation via Demucs."""

import os
import subprocess

from protocol import send_error, send_progress, send_result


def run_separate(params: dict) -> None:
    audio_path = params["audio_path"]
    output_dir = params["output_dir"]

    if not os.path.exists(audio_path):
        send_error("separate", f"Audio file not found: {audio_path}")
        return

    send_progress("separate", 0, "Starting source separation...")

    try:
        cmd = [
            "python", "-m", "demucs",
            "--two-stems", "bass",
            "-n", "htdemucs",
            "-o", output_dir,
            audio_path,
        ]

        process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        send_progress("separate", 30, "Separating bass track (this may take a while)...")

        _, stderr = process.communicate()

        if process.returncode != 0:
            send_error("separate", f"Demucs failed: {stderr.strip()}")
            return

        filename = os.path.splitext(os.path.basename(audio_path))[0]
        bass_path_demucs = os.path.join(output_dir, "htdemucs", filename, "bass.wav")

        bass_path = os.path.join(output_dir, "bass.wav")
        if os.path.exists(bass_path_demucs):
            os.rename(bass_path_demucs, bass_path)
        elif os.path.exists(bass_path):
            pass
        else:
            send_error("separate", "Bass track not found in Demucs output")
            return

        send_progress("separate", 100, "Source separation complete")
        send_result("separate", {"bass_path": bass_path})

    except FileNotFoundError:
        send_error("separate", "Demucs not found. Please install: pip install demucs")
