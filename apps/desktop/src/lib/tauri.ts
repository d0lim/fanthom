import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PipelineProgress, TabSheet } from "./types";

export async function processUrl(url: string): Promise<TabSheet> {
  return invoke<TabSheet>("process_url", { url });
}

export async function transposeTab(
  midiNotesJson: string,
  semitones: number,
): Promise<TabSheet> {
  return invoke<TabSheet>("transpose", { midiNotesJson, semitones });
}

export async function toggleOptimization(
  midiNotesJson: string,
  enabled: boolean,
): Promise<TabSheet> {
  return invoke<TabSheet>("toggle_optimization", { midiNotesJson, enabled });
}

export async function exportTab(
  sheetJson: string,
  format: "ascii" | "musicxml",
): Promise<string> {
  return invoke<string>("export_tab", { sheetJson, format });
}

export function onPipelineProgress(
  callback: (progress: PipelineProgress) => void,
): Promise<UnlistenFn> {
  return listen<PipelineProgress>("pipeline:progress", (event) => {
    callback(event.payload);
  });
}
