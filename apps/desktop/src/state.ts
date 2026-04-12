import {
  createContext,
  useContext,
  type Dispatch,
} from "react";
import type { PipelineState, TabSheet } from "./lib/types";

export interface AppState {
  pipeline: PipelineState;
  tabSheet: TabSheet | null;
  midiNotesJson: string | null;
  bassPath: string | null;
  transpose: number;
  optimized: boolean;
  bpm: number;
  startOffset: number;
}

export const initialState: AppState = {
  pipeline: { status: "idle" },
  tabSheet: null,
  midiNotesJson: null,
  bassPath: null,
  transpose: 0,
  optimized: true,
  bpm: 120,
  startOffset: 0,
};

export type AppAction =
  | { type: "PIPELINE_START" }
  | { type: "PIPELINE_PROGRESS"; step: string; percent: number; message: string }
  | { type: "PIPELINE_DONE"; tabSheet: TabSheet; midiNotesJson: string; bassPath: string }
  | { type: "PIPELINE_ERROR"; message: string }
  | { type: "SET_TAB"; tabSheet: TabSheet }
  | { type: "SET_TRANSPOSE"; value: number }
  | { type: "SET_OPTIMIZED"; value: boolean }
  | { type: "SET_BPM"; value: number }
  | { type: "SET_START_OFFSET"; value: number }
  | { type: "RESET" };

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "PIPELINE_START":
      return { ...state, pipeline: { status: "processing", step: "extract", percent: 0, message: "Starting..." } };
    case "PIPELINE_PROGRESS":
      return { ...state, pipeline: { status: "processing", step: action.step, percent: action.percent, message: action.message } };
    case "PIPELINE_DONE":
      return {
        ...state,
        pipeline: { status: "done" },
        tabSheet: action.tabSheet,
        midiNotesJson: action.midiNotesJson,
        bassPath: action.bassPath,
        transpose: 0,
      };
    case "PIPELINE_ERROR":
      return { ...state, pipeline: { status: "error", message: action.message } };
    case "SET_TAB":
      return { ...state, tabSheet: action.tabSheet };
    case "SET_TRANSPOSE":
      return { ...state, transpose: action.value };
    case "SET_OPTIMIZED":
      return { ...state, optimized: action.value };
    case "SET_BPM":
      return { ...state, bpm: action.value };
    case "SET_START_OFFSET":
      return { ...state, startOffset: action.value };
    case "RESET":
      return initialState;
    default:
      return state;
  }
}

export const AppStateContext = createContext<AppState>(initialState);
export const AppDispatchContext = createContext<Dispatch<AppAction>>(() => {});

export function useAppState() {
  return useContext(AppStateContext);
}

export function useAppDispatch() {
  return useContext(AppDispatchContext);
}
