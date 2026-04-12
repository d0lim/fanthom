import { useRef, useEffect, useCallback } from "react";
import { useAppState } from "../state";
import type { NoteOrigin } from "../lib/types";

const STRING_LABELS = ["E", "A", "D", "G"];
const COLORS = {
  Normal: "#E8A723",
  Optimized: "#4ADE80",
  OctaveShifted: "#60A5FA",
  Technique: "#F472B6",
  Slide: "#F472B6",
};

const LINE_HEIGHT = 28;
const LEFT_MARGIN = 36;
const RIGHT_MARGIN = 16;
const TOP_MARGIN = 16;
const ROW_GAP = 32;
const NOTE_FONT = "bold 13px monospace";
const LABEL_FONT = "11px monospace";
const MEASURE_FONT = "10px monospace";
const PLAYHEAD_COLOR = "#ef4444"; // red-500

function getNoteColor(origin: NoteOrigin): string {
  if (origin === "Normal") return COLORS.Normal;
  if (origin === "Optimized") return COLORS.Optimized;
  if (typeof origin === "object" && "OctaveShifted" in origin) return COLORS.OctaveShifted;
  return COLORS.Normal;
}

export function TabCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const layoutRef = useRef<{
    totalRows: number;
    rowHeight: number;
    rowStringHeight: number;
    measuresPerRow: number;
    secPerMeasure: number;
    pixelsPerSecond: number;
    pixelsPerMeasure: number;
    canvasWidth: number;
    canvasHeight: number;
    startOffset: number;
  } | null>(null);
  const state = useAppState();
  const tabSheet = state.tabSheet;
  const bpm = state.bpm;
  const startOffset = state.startOffset;
  const playbackTime = state.playbackTime;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container || !tabSheet) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    const numStrings = STRING_LABELS.length;

    const beatsPerMeasure = tabSheet.time_signature[0];
    const secPerBeat = 60.0 / bpm;
    const secPerMeasure = secPerBeat * beatsPerMeasure;

    const availableWidth = rect.width - LEFT_MARGIN - RIGHT_MARGIN;
    const minPixelsPerMeasure = 120;
    const measuresPerRow = Math.max(1, Math.floor(availableWidth / minPixelsPerMeasure));
    const pixelsPerMeasure = availableWidth / measuresPerRow;
    const pixelsPerSecond = pixelsPerMeasure / secPerMeasure;

    const lastTime = tabSheet.notes.reduce(
      (max, n) => Math.max(max, n.onset + n.duration),
      0,
    );
    const totalDuration = Math.max(lastTime - startOffset, secPerMeasure);
    const totalMeasures = Math.ceil(totalDuration / secPerMeasure);
    const totalRows = Math.ceil(totalMeasures / measuresPerRow);

    const rowStringHeight = (numStrings - 1) * LINE_HEIGHT;
    const rowHeight = rowStringHeight + ROW_GAP;
    const canvasHeight = TOP_MARGIN + totalRows * rowHeight + ROW_GAP;

    // Save layout for overlay
    layoutRef.current = {
      totalRows, rowHeight, rowStringHeight, measuresPerRow,
      secPerMeasure, pixelsPerSecond, pixelsPerMeasure,
      canvasWidth: rect.width, canvasHeight, startOffset,
    };

    // Set canvas size
    canvas.width = rect.width * dpr;
    canvas.height = canvasHeight * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${canvasHeight}px`;
    ctx.scale(dpr, dpr);

    // Also size overlay
    const overlay = overlayRef.current;
    if (overlay) {
      overlay.width = rect.width * dpr;
      overlay.height = canvasHeight * dpr;
      overlay.style.width = `${rect.width}px`;
      overlay.style.height = `${canvasHeight}px`;
    }

    ctx.fillStyle = "#09090b";
    ctx.fillRect(0, 0, rect.width, canvasHeight);

    for (let row = 0; row < totalRows; row++) {
      const rowY = TOP_MARGIN + row * rowHeight;
      const rowStartMeasure = row * measuresPerRow;
      const rowStartTime = startOffset + rowStartMeasure * secPerMeasure;
      const rowEndTime = rowStartTime + measuresPerRow * secPerMeasure;

      ctx.strokeStyle = "#3f3f46";
      ctx.lineWidth = 1;
      for (let s = 0; s < numStrings; s++) {
        const y = rowY + (numStrings - 1 - s) * LINE_HEIGHT;
        ctx.beginPath();
        ctx.moveTo(LEFT_MARGIN, y);
        ctx.lineTo(rect.width - RIGHT_MARGIN, y);
        ctx.stroke();
      }

      ctx.fillStyle = "#71717a";
      ctx.font = LABEL_FONT;
      ctx.textAlign = "right";
      ctx.textBaseline = "middle";
      for (let s = 0; s < numStrings; s++) {
        const y = rowY + (numStrings - 1 - s) * LINE_HEIGHT;
        ctx.fillText(STRING_LABELS[s], LEFT_MARGIN - 8, y);
      }

      for (let m = 0; m <= measuresPerRow; m++) {
        const measureIdx = rowStartMeasure + m;
        if (measureIdx > totalMeasures) break;
        const x = LEFT_MARGIN + m * pixelsPerMeasure;

        ctx.strokeStyle = m === 0 ? "#71717a" : "#3f3f46";
        ctx.lineWidth = m === 0 ? 2 : 1;
        ctx.beginPath();
        ctx.moveTo(x, rowY);
        ctx.lineTo(x, rowY + rowStringHeight);
        ctx.stroke();

        if (m < measuresPerRow && measureIdx < totalMeasures) {
          ctx.fillStyle = "#52525b";
          ctx.font = MEASURE_FONT;
          ctx.textAlign = "left";
          ctx.textBaseline = "bottom";
          ctx.fillText(`${measureIdx + 1}`, x + 3, rowY - 2);
        }

        if (m < measuresPerRow) {
          ctx.strokeStyle = "#27272a";
          ctx.lineWidth = 0.5;
          for (let beat = 1; beat < beatsPerMeasure; beat++) {
            const bx = x + (beat / beatsPerMeasure) * pixelsPerMeasure;
            ctx.beginPath();
            ctx.setLineDash([2, 4]);
            ctx.moveTo(bx, rowY);
            ctx.lineTo(bx, rowY + rowStringHeight);
            ctx.stroke();
            ctx.setLineDash([]);
          }
        }
      }

      ctx.font = NOTE_FONT;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      for (const note of tabSheet.notes) {
        if (note.onset < rowStartTime || note.onset >= rowEndTime) continue;

        const relTime = note.onset - rowStartTime;
        const x = LEFT_MARGIN + relTime * pixelsPerSecond;
        const y = rowY + (numStrings - 1 - note.string) * LINE_HEIGHT;

        const fretStr = note.fret.toString();
        const textWidth = ctx.measureText(fretStr).width;
        const radius = Math.max(textWidth / 2 + 4, 10);

        // Draw background circle to clear string line
        ctx.fillStyle = "#09090b";
        ctx.beginPath();
        ctx.arc(x, y, radius, 0, Math.PI * 2);
        ctx.fill();

        // Draw slide connector line if this is a slide note
        if (note.technique === "Slide") {
          // Find previous note on the same string in this row
          const prevNote = tabSheet.notes
            .filter(
              (n) =>
                n.string === note.string &&
                n.onset < note.onset &&
                n.onset >= rowStartTime &&
                n.onset < rowEndTime
            )
            .sort((a, b) => b.onset - a.onset)[0];

          if (prevNote) {
            const prevRelTime = prevNote.onset - rowStartTime;
            const prevX = LEFT_MARGIN + prevRelTime * pixelsPerSecond;
            const prevY = rowY + (numStrings - 1 - prevNote.string) * LINE_HEIGHT;

            // Draw diagonal slide line
            ctx.strokeStyle = COLORS.Slide;
            ctx.lineWidth = 1.5;
            ctx.beginPath();
            ctx.moveTo(prevX + radius, prevY);
            ctx.lineTo(x - radius, y);
            ctx.stroke();
          }

          // Draw fret number in slide color
          ctx.fillStyle = COLORS.Slide;
        } else {
          ctx.fillStyle = getNoteColor(note.origin);
        }

        ctx.fillText(fretStr, x, y);
      }
    }
  }, [tabSheet, bpm, startOffset]);

  // Draw playhead on overlay canvas
  const drawPlayhead = useCallback((time: number) => {
    const overlay = overlayRef.current;
    const layout = layoutRef.current;
    if (!overlay || !layout) return;

    const dpr = window.devicePixelRatio || 1;
    const ctx = overlay.getContext("2d");
    if (!ctx) return;

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, overlay.width, overlay.height);
    ctx.scale(dpr, dpr);

    if (time < 0) return;

    const elapsed = time - layout.startOffset;
    if (elapsed < 0) return;

    const measureIdx = elapsed / layout.secPerMeasure;
    const row = Math.floor(measureIdx / layout.measuresPerRow);
    if (row >= layout.totalRows) return;

    const rowStartMeasure = row * layout.measuresPerRow;
    const rowStartTime = layout.startOffset + rowStartMeasure * layout.secPerMeasure;
    const relTime = time - rowStartTime;
    const x = LEFT_MARGIN + relTime * layout.pixelsPerSecond;
    const rowY = TOP_MARGIN + row * layout.rowHeight;

    // Red playhead line
    ctx.strokeStyle = PLAYHEAD_COLOR;
    ctx.lineWidth = 2;
    ctx.globalAlpha = 0.8;
    ctx.beginPath();
    ctx.moveTo(x, rowY - 4);
    ctx.lineTo(x, rowY + layout.rowStringHeight + 4);
    ctx.stroke();

    // Small triangle at top
    ctx.fillStyle = PLAYHEAD_COLOR;
    ctx.beginPath();
    ctx.moveTo(x, rowY - 4);
    ctx.lineTo(x - 4, rowY - 10);
    ctx.lineTo(x + 4, rowY - 10);
    ctx.closePath();
    ctx.fill();
    ctx.globalAlpha = 1.0;

    // Auto-scroll to keep playhead visible
    const container = containerRef.current;
    if (container) {
      const headY = rowY + layout.rowHeight / 2;
      const scrollTop = container.scrollTop;
      const viewHeight = container.clientHeight;
      if (headY < scrollTop || headY > scrollTop + viewHeight - 60) {
        container.scrollTo({ top: Math.max(0, rowY - 40), behavior: "smooth" });
      }
    }
  }, []);

  useEffect(() => {
    draw();
    window.addEventListener("resize", draw);
    return () => window.removeEventListener("resize", draw);
  }, [draw]);

  // Animate playhead
  useEffect(() => {
    if (playbackTime != null && playbackTime >= 0) {
      drawPlayhead(playbackTime);
    } else {
      drawPlayhead(-1);
    }
  }, [playbackTime, drawPlayhead]);

  return (
    <div ref={containerRef} className="w-full h-full overflow-y-auto overflow-x-hidden relative">
      <canvas ref={canvasRef} className="block" />
      <canvas ref={overlayRef} className="absolute top-0 left-0 pointer-events-none" />
    </div>
  );
}
