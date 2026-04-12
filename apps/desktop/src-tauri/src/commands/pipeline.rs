use crate::db;
use crate::sidecar::{Sidecar, SidecarRequest};
use crate::state::AppState;
use serde::Serialize;
use tab_engine::{Tuning, optimize, transcribe_wav};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct PipelineProgress {
    pub step: String,
    pub percent: u32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PipelineResult {
    pub tab_sheet: tab_engine::TabSheet,
    pub midi_notes_json: String,
    pub bass_path: String,
}

#[tauri::command]
pub async fn process_url(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<PipelineResult, String> {
    let song_id = uuid::Uuid::new_v4().to_string();
    let song_dir = state.songs_dir.join(&song_id);
    std::fs::create_dir_all(&song_dir).map_err(|e| e.to_string())?;
    let song_dir_str = song_dir.to_string_lossy().to_string();

    let mut sidecar = Sidecar::spawn(&app)?;

    let emit_progress = |app: &AppHandle, step: &str, percent: u32, message: &str| {
        let _ = app.emit("pipeline:progress", PipelineProgress {
            step: step.to_string(),
            percent,
            message: message.to_string(),
        });
    };

    // Step 1: Extract audio
    emit_progress(&app, "extract", 0, "Starting audio extraction...");
    let extract_result = sidecar.send_command(
        &SidecarRequest {
            command: "extract".to_string(),
            params: serde_json::json!({ "url": url, "output_dir": song_dir_str }),
        },
        |msg| {
            let _ = app.emit("pipeline:progress", PipelineProgress {
                step: msg.step.clone(),
                percent: msg.percent.unwrap_or(0),
                message: msg.message.clone().unwrap_or_default(),
            });
        },
    )?;

    let audio_path = extract_result["audio_path"]
        .as_str()
        .ok_or("Missing audio_path in extract result")?
        .to_string();

    // Step 2: Separate bass track
    emit_progress(&app, "separate", 0, "Starting source separation...");
    let separate_result = sidecar.send_command(
        &SidecarRequest {
            command: "separate".to_string(),
            params: serde_json::json!({ "audio_path": audio_path, "output_dir": song_dir_str }),
        },
        |msg| {
            let _ = app.emit("pipeline:progress", PipelineProgress {
                step: msg.step.clone(),
                percent: msg.percent.unwrap_or(0),
                message: msg.message.clone().unwrap_or_default(),
            });
        },
    )?;

    let bass_path = separate_result["bass_path"]
        .as_str()
        .ok_or("Missing bass_path in separate result")?
        .to_string();

    // Step 3: Pitch detection (Rust — YIN algorithm)
    log::info!("Starting Rust pitch detection on: {}", bass_path);
    emit_progress(&app, "transcribe", 0, "Starting pitch detection...");
    let midi_notes = transcribe_wav(&bass_path)
        .map_err(|e| format!("Pitch detection failed: {e}"))?;
    log::info!("Pitch detection complete: {} notes found", midi_notes.len());
    emit_progress(&app, "transcribe", 100, "Pitch detection complete");

    // Serialize midi notes for frontend (used by transpose/optimize)
    let midi_notes_json = serde_json::to_string(&midi_notes).map_err(|e| e.to_string())?;

    // Step 4: Tab Engine (Rust, in-process)
    emit_progress(&app, "convert", 0, "Generating tab notation...");

    let sheet = optimize(&midi_notes, Tuning::Standard4, 120.0, (4, 4));

    // Save to DB
    let tab_data = rmp_serde::to_vec(&sheet).map_err(|e| e.to_string())?;
    let tab_id = uuid::Uuid::new_v4().to_string();
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::insert_song(&conn, &song_id, "Untitled", Some(&url), None, Some(sheet.tempo)).map_err(|e| e.to_string())?;
        db::insert_tab(&conn, &tab_id, &song_id, "standard4", 0, &tab_data).map_err(|e| e.to_string())?;
    }

    emit_progress(&app, "convert", 100, "Tab generation complete!");

    Ok(PipelineResult {
        tab_sheet: sheet,
        midi_notes_json,
        bass_path,
    })
}
