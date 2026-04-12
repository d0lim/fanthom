export interface MidiNote {
  pitch: number;
  onset: number;
  offset: number;
  velocity: number;
}

export type NoteOrigin =
  | "Normal"
  | "Optimized"
  | { OctaveShifted: number };

export interface TabNote {
  string: number;
  fret: number;
  midi_pitch: number;
  onset: number;
  duration: number;
  origin: NoteOrigin;
}

export interface TabSheet {
  notes: TabNote[];
  tempo: number;
  time_signature: [number, number];
  tuning: string;
  key_transpose: number;
}

export interface PipelineProgress {
  step: string;
  percent: number;
  message: string;
}

export type PipelineState =
  | { status: "idle" }
  | { status: "processing"; step: string; percent: number; message: string }
  | { status: "done" }
  | { status: "error"; message: string };
