import { useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useAppState } from "../state";

export function AudioPlayer() {
  const state = useAppState();
  const audioRef = useRef<HTMLAudioElement>(null);
  const [playing, setPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);

  if (!state.bassPath) return null;

  const audioSrc = convertFileSrc(state.bassPath);

  function togglePlay() {
    const audio = audioRef.current;
    if (!audio) return;
    if (playing) {
      audio.pause();
    } else {
      audio.play();
    }
    setPlaying(!playing);
  }

  function handleTimeUpdate() {
    const audio = audioRef.current;
    if (audio) setCurrentTime(audio.currentTime);
  }

  function handleLoadedMetadata() {
    const audio = audioRef.current;
    if (audio) setDuration(audio.duration);
  }

  function handleSeek(e: React.ChangeEvent<HTMLInputElement>) {
    const audio = audioRef.current;
    if (audio) {
      audio.currentTime = parseFloat(e.target.value);
      setCurrentTime(audio.currentTime);
    }
  }

  function handleEnded() {
    setPlaying(false);
  }

  function formatTime(sec: number): string {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  return (
    <div className="flex items-center gap-3">
      <audio
        ref={audioRef}
        src={audioSrc}
        onTimeUpdate={handleTimeUpdate}
        onLoadedMetadata={handleLoadedMetadata}
        onEnded={handleEnded}
      />
      <button
        onClick={togglePlay}
        className="w-8 h-8 flex items-center justify-center bg-zinc-800 hover:bg-zinc-700 rounded-md text-zinc-300 text-sm font-mono transition-colors"
      >
        {playing ? "||" : ">"}
      </button>
      <input
        type="range"
        min={0}
        max={duration || 0}
        step={0.1}
        value={currentTime}
        onChange={handleSeek}
        className="w-32 accent-amber-500"
      />
      <span className="text-zinc-500 text-xs font-mono w-16">
        {formatTime(currentTime)}/{formatTime(duration)}
      </span>
    </div>
  );
}
