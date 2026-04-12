import { useRef, useEffect, useCallback } from "react";
import { useAppState } from "../state";
import type { NoteOrigin } from "../lib/types";

const STRING_LABELS = ["E", "A", "D", "G"];
const COLORS = {
  Normal: "#E8A723",
  Optimized: "#4ADE80",
  OctaveShifted: "#60A5FA",
  Technique: "#F472B6",
};

const LINE_HEIGHT = 32;
const LEFT_MARGIN = 40;
const TOP_MARGIN = 20;
const PIXELS_PER_SECOND = 100;
const NOTE_FONT = "bold 14px monospace";
const LABEL_FONT = "12px monospace";

function getNoteColor(origin: NoteOrigin): string {
  if (origin === "Normal") return COLORS.Normal;
  if (origin === "Optimized") return COLORS.Optimized;
  if (typeof origin === "object" && "OctaveShifted" in origin) return COLORS.OctaveShifted;
  return COLORS.Normal;
}

export function TabCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const state = useAppState();
  const tabSheet = state.tabSheet;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container || !tabSheet) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;

    const lastNote = tabSheet.notes.reduce(
      (max, n) => Math.max(max, n.onset + n.duration),
      0,
    );
    const contentWidth = Math.max(
      rect.width,
      LEFT_MARGIN + lastNote * PIXELS_PER_SECOND + 100,
    );

    canvas.width = contentWidth * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${contentWidth}px`;
    canvas.style.height = `${rect.height}px`;
    ctx.scale(dpr, dpr);

    ctx.fillStyle = "#09090b";
    ctx.fillRect(0, 0, contentWidth, rect.height);

    const numStrings = STRING_LABELS.length;

    ctx.strokeStyle = "#3f3f46";
    ctx.lineWidth = 1;
    for (let i = 0; i < numStrings; i++) {
      const y = TOP_MARGIN + (numStrings - 1 - i) * LINE_HEIGHT;
      ctx.beginPath();
      ctx.moveTo(LEFT_MARGIN, y);
      ctx.lineTo(contentWidth, y);
      ctx.stroke();
    }

    ctx.fillStyle = "#a1a1aa";
    ctx.font = LABEL_FONT;
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    for (let i = 0; i < numStrings; i++) {
      const y = TOP_MARGIN + (numStrings - 1 - i) * LINE_HEIGHT;
      ctx.fillText(STRING_LABELS[i], LEFT_MARGIN - 10, y);
    }

    ctx.font = NOTE_FONT;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const note of tabSheet.notes) {
      const x = LEFT_MARGIN + note.onset * PIXELS_PER_SECOND;
      const y = TOP_MARGIN + (numStrings - 1 - note.string) * LINE_HEIGHT;

      const fretStr = note.fret.toString();
      const textWidth = ctx.measureText(fretStr).width;
      const radius = Math.max(textWidth / 2 + 4, 10);

      ctx.fillStyle = "#09090b";
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fill();

      ctx.fillStyle = getNoteColor(note.origin);
      ctx.fillText(fretStr, x, y);
    }
  }, [tabSheet]);

  useEffect(() => {
    draw();
    window.addEventListener("resize", draw);
    return () => window.removeEventListener("resize", draw);
  }, [draw]);

  return (
    <div ref={containerRef} className="w-full h-full overflow-x-auto overflow-y-hidden">
      <canvas ref={canvasRef} />
    </div>
  );
}
