use crate::state::AppState;
use tab_engine::{MidiNote, TabSheet, Tuning};

#[tauri::command]
pub fn transpose(
    _state: tauri::State<'_, AppState>,
    midi_notes_json: String,
    semitones: i8,
    bpm: f64,
) -> Result<TabSheet, String> {
    let notes: Vec<MidiNote> = serde_json::from_str(&midi_notes_json).map_err(|e| e.to_string())?;
    let tempo = if bpm > 0.0 { bpm } else { 120.0 };
    let sheet = tab_engine::transpose(&notes, semitones, Tuning::Standard4, tempo, (4, 4));
    Ok(sheet)
}

#[tauri::command]
pub fn toggle_optimization(
    midi_notes_json: String,
    enabled: bool,
    bpm: f64,
) -> Result<TabSheet, String> {
    let notes: Vec<MidiNote> = serde_json::from_str(&midi_notes_json).map_err(|e| e.to_string())?;
    let tempo = if bpm > 0.0 { bpm } else { 120.0 };

    if enabled {
        Ok(tab_engine::optimize(&notes, Tuning::Standard4, tempo, (4, 4)))
    } else {
        let sheet_notes: Vec<tab_engine::TabNote> = notes.iter().map(|n| {
            let candidates = tab_engine::pitch_to_candidates(n.pitch, Tuning::Standard4);
            let c = candidates.first().expect("no candidates for pitch");
            tab_engine::TabNote {
                string: c.string,
                fret: c.fret,
                midi_pitch: n.pitch,
                onset: n.onset,
                duration: n.offset - n.onset,
                origin: tab_engine::NoteOrigin::Normal,
            }
        }).collect();
        Ok(TabSheet {
            notes: sheet_notes,
            tempo,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        })
    }
}

#[tauri::command]
pub fn regenerate_tab(
    midi_notes_json: String,
    bpm: f64,
    semitones: i8,
    optimized: bool,
) -> Result<TabSheet, String> {
    let notes: Vec<MidiNote> = serde_json::from_str(&midi_notes_json).map_err(|e| e.to_string())?;
    let tempo = if bpm > 0.0 { bpm } else { 120.0 };

    if semitones != 0 {
        Ok(tab_engine::transpose(&notes, semitones, Tuning::Standard4, tempo, (4, 4)))
    } else if optimized {
        Ok(tab_engine::optimize(&notes, Tuning::Standard4, tempo, (4, 4)))
    } else {
        let sheet_notes: Vec<tab_engine::TabNote> = notes.iter().map(|n| {
            let candidates = tab_engine::pitch_to_candidates(n.pitch, Tuning::Standard4);
            let c = candidates.first().expect("no candidates for pitch");
            tab_engine::TabNote {
                string: c.string,
                fret: c.fret,
                midi_pitch: n.pitch,
                onset: n.onset,
                duration: n.offset - n.onset,
                origin: tab_engine::NoteOrigin::Normal,
            }
        }).collect();
        Ok(TabSheet {
            notes: sheet_notes,
            tempo,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: semitones,
        })
    }
}

#[tauri::command]
pub fn export_tab(sheet_json: String, format: String) -> Result<String, String> {
    let sheet: TabSheet = serde_json::from_str(&sheet_json).map_err(|e| e.to_string())?;
    match format.as_str() {
        "ascii" => Ok(tab_engine::export::ascii::export(&sheet)),
        "musicxml" => Ok(tab_engine::export::musicxml::export(&sheet)),
        _ => Err(format!("Unknown format: {format}")),
    }
}
