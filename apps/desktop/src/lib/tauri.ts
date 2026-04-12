import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PipelineProgress, PipelineResult, TabSheet } from "./types";

export async function processUrl(url: string): Promise<PipelineResult> {
  return invoke<PipelineResult>("process_url", { url });
}

export async function transposeTab(
  midiNotesJson: string,
  semitones: number,
  bpm: number,
): Promise<TabSheet> {
  return invoke<TabSheet>("transpose", { midiNotesJson, semitones, bpm });
}

export async function toggleOptimization(
  midiNotesJson: string,
  enabled: boolean,
  bpm: number,
): Promise<TabSheet> {
  return invoke<TabSheet>("toggle_optimization", { midiNotesJson, enabled, bpm });
}

export async function regenerateTab(
  midiNotesJson: string,
  bpm: number,
  semitones: number,
  optimized: boolean,
): Promise<TabSheet> {
  return invoke<TabSheet>("regenerate_tab", { midiNotesJson, bpm, semitones, optimized });
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
