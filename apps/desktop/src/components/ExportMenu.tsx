import { useState } from "react";
import { useAppState } from "../state";
import { exportTab } from "../lib/tauri";

export function ExportMenu() {
  const state = useAppState();
  const [open, setOpen] = useState(false);

  async function handleExport(format: "ascii" | "musicxml") {
    setOpen(false);
    if (!state.tabSheet) return;

    try {
      const sheetJson = JSON.stringify(state.tabSheet);
      const result = await exportTab(sheetJson, format);

      if (format === "ascii") {
        await navigator.clipboard.writeText(result);
      } else {
        const blob = new Blob([result], { type: "application/xml" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "tab.musicxml";
        a.click();
        URL.revokeObjectURL(url);
      }
    } catch (err) {
      console.error("Export failed:", err);
    }
  }

  return (
    <div className="relative ml-auto">
      <button
        onClick={() => setOpen(!open)}
        className="bg-zinc-800 hover:bg-zinc-700 text-zinc-300 px-4 py-1.5 rounded-lg text-sm font-medium border border-zinc-700 transition-colors"
      >
        Export
      </button>
      {open && (
        <div className="absolute bottom-full mb-2 right-0 bg-zinc-800 border border-zinc-700 rounded-lg shadow-xl overflow-hidden">
          <button
            onClick={() => handleExport("ascii")}
            className="block w-full text-left px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-700"
          >
            Copy ASCII Tab
          </button>
          <button
            onClick={() => handleExport("musicxml")}
            className="block w-full text-left px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-700"
          >
            Download MusicXML
          </button>
        </div>
      )}
    </div>
  );
}
