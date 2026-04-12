import { useAppState, useAppDispatch } from "../state";
import { regenerateTab } from "../lib/tauri";

export function BpmControl() {
  const state = useAppState();
  const dispatch = useAppDispatch();

  async function handleBpmChange(e: React.ChangeEvent<HTMLInputElement>) {
    const value = parseInt(e.target.value, 10);
    if (isNaN(value) || value < 20 || value > 300) return;
    dispatch({ type: "SET_BPM", value });

    if (!state.midiNotesJson) return;
    try {
      const newTab = await regenerateTab(
        state.midiNotesJson,
        value,
        state.transpose,
        state.optimized,
      );
      dispatch({ type: "SET_TAB", tabSheet: newTab });
    } catch (err) {
      console.error("BPM change failed:", err);
    }
  }

  async function handleOffsetChange(e: React.ChangeEvent<HTMLInputElement>) {
    const value = parseFloat(e.target.value);
    if (isNaN(value) || value < 0) return;
    dispatch({ type: "SET_START_OFFSET", value });
    // Offset only affects rendering, no backend call needed
  }

  return (
    <div className="flex items-center gap-4">
      <div className="flex items-center gap-2">
        <label className="text-zinc-400 text-sm">BPM</label>
        <input
          type="number"
          min={20}
          max={300}
          value={state.bpm}
          onChange={handleBpmChange}
          className="w-16 bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-zinc-100 text-sm text-center focus:outline-none focus:border-zinc-500"
        />
      </div>
      <div className="flex items-center gap-2">
        <label className="text-zinc-400 text-sm">Start</label>
        <input
          type="number"
          min={0}
          step={0.1}
          value={state.startOffset}
          onChange={handleOffsetChange}
          className="w-20 bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-zinc-100 text-sm text-center focus:outline-none focus:border-zinc-500"
        />
        <span className="text-zinc-500 text-xs">s</span>
      </div>
    </div>
  );
}
