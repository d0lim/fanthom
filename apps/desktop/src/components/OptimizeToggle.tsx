import { useAppState, useAppDispatch } from "../state";
import { toggleOptimization } from "../lib/tauri";

export function OptimizeToggle() {
  const state = useAppState();
  const dispatch = useAppDispatch();

  async function handleToggle() {
    const newValue = !state.optimized;
    dispatch({ type: "SET_OPTIMIZED", value: newValue });

    if (!state.midiNotesJson) return;

    try {
      const newTab = await toggleOptimization(state.midiNotesJson, newValue);
      dispatch({ type: "SET_TAB", tabSheet: newTab });
    } catch (err) {
      console.error("Toggle optimization failed:", err);
    }
  }

  return (
    <button
      onClick={handleToggle}
      className={`px-4 py-1.5 rounded-lg text-sm font-medium transition-colors ${
        state.optimized
          ? "bg-green-900/50 text-green-400 border border-green-700"
          : "bg-zinc-800 text-zinc-400 border border-zinc-700"
      }`}
    >
      Optimize {state.optimized ? "ON" : "OFF"}
    </button>
  );
}
