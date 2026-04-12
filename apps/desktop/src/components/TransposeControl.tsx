import { useAppState, useAppDispatch } from "../state";
import { transposeTab } from "../lib/tauri";

export function TransposeControl() {
  const state = useAppState();
  const dispatch = useAppDispatch();

  async function handleChange(e: React.ChangeEvent<HTMLInputElement>) {
    const semitones = parseInt(e.target.value, 10);
    dispatch({ type: "SET_TRANSPOSE", value: semitones });

    if (!state.midiNotesJson) return;

    try {
      const newTab = await transposeTab(state.midiNotesJson, semitones, state.bpm);
      dispatch({ type: "SET_TAB", tabSheet: newTab });
    } catch (err) {
      console.error("Transpose failed:", err);
    }
  }

  return (
    <div className="flex items-center gap-3">
      <label className="text-zinc-400 text-sm">Transpose</label>
      <span className="text-zinc-500 text-xs w-6 text-right">
        {state.transpose > 0 ? `+${state.transpose}` : state.transpose}
      </span>
      <input
        type="range"
        min={-12}
        max={12}
        value={state.transpose}
        onChange={handleChange}
        className="w-40 accent-amber-500"
      />
    </div>
  );
}
