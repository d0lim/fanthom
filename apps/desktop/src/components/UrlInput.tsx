import { useState } from "react";
import { useAppDispatch, useAppState } from "../state";
import { processUrl, onPipelineProgress } from "../lib/tauri";

const YOUTUBE_REGEX = /^https?:\/\/(www\.)?(youtube\.com\/watch\?v=|youtu\.be\/)/;

export function UrlInput() {
  const [url, setUrl] = useState("");
  const dispatch = useAppDispatch();
  const state = useAppState();
  const isProcessing = state.pipeline.status === "processing";

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!url.trim() || isProcessing) return;

    if (!YOUTUBE_REGEX.test(url)) {
      dispatch({ type: "PIPELINE_ERROR", message: "Please enter a valid YouTube URL" });
      return;
    }

    dispatch({ type: "PIPELINE_START" });

    const unlisten = await onPipelineProgress((progress) => {
      dispatch({
        type: "PIPELINE_PROGRESS",
        step: progress.step,
        percent: progress.percent,
        message: progress.message,
      });
    });

    try {
      const tabSheet = await processUrl(url);
      dispatch({ type: "PIPELINE_DONE", tabSheet, midiNotesJson: "" });
    } catch (err) {
      dispatch({ type: "PIPELINE_ERROR", message: String(err) });
    } finally {
      unlisten();
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex gap-3">
      <input
        type="text"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="Paste YouTube URL here..."
        disabled={isProcessing}
        className="flex-1 bg-zinc-900 border border-zinc-700 rounded-lg px-4 py-2 text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-zinc-500"
      />
      <button
        type="submit"
        disabled={isProcessing || !url.trim()}
        className="bg-amber-600 hover:bg-amber-500 disabled:bg-zinc-700 disabled:text-zinc-500 text-white font-medium px-6 py-2 rounded-lg transition-colors"
      >
        {isProcessing ? "Processing..." : "Fathom"}
      </button>
    </form>
  );
}
