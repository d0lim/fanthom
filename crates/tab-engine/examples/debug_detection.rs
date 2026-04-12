//! Debug tool: dump pitch detection details for a WAV file.
//!
//! Usage: cargo run --release -p tab-engine --example debug_detection -- <wav_path>

use tab_engine::transcribe_wav;

fn midi_to_note_name(midi: u8) -> String {
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (midi as i32 / 12) - 1;
    let note = names[(midi % 12) as usize];
    format!("{note}{octave}")
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: debug_detection <wav_path>");
    eprintln!("Analyzing: {path}");

    let start = std::time::Instant::now();
    let notes = transcribe_wav(&path).expect("transcribe failed");
    let elapsed = start.elapsed();

    eprintln!("Detected {} notes in {:.2}s", notes.len(), elapsed.as_secs_f32());
    eprintln!();

    println!("# Detected Notes");
    println!("#  idx | onset   offset  dur   | pitch (MIDI name) | velocity | technique");
    println!("# -----+----------------------+-------------------+----------+----------");
    for (i, n) in notes.iter().enumerate() {
        let dur = n.offset - n.onset;
        let name = midi_to_note_name(n.pitch);
        let tech = n.technique.map_or("—".to_string(), |t| format!("{t:?}"));
        println!(
            "{:5} | {:.3}s  {:.3}s  {:.3}s | {:3} ({:<4}) | {:3} | {}",
            i, n.onset, n.offset, dur, n.pitch, name, n.velocity, tech
        );
    }

    // Stats
    eprintln!();
    eprintln!("# Stats");
    if !notes.is_empty() {
        let durs: Vec<f64> = notes.iter().map(|n| n.offset - n.onset).collect();
        let min_dur = durs.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_dur = durs.iter().cloned().fold(0.0_f64, f64::max);
        let avg_dur: f64 = durs.iter().sum::<f64>() / durs.len() as f64;

        let pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
        let min_p = *pitches.iter().min().unwrap();
        let max_p = *pitches.iter().max().unwrap();

        eprintln!("  Duration: min={:.3}s, max={:.3}s, avg={:.3}s", min_dur, max_dur, avg_dur);
        eprintln!("  Pitch range: {}..={} ({}..={})",
            midi_to_note_name(min_p), midi_to_note_name(max_p), min_p, max_p);

        // Pitch histogram
        let mut counts = std::collections::HashMap::new();
        for &p in &pitches {
            *counts.entry(p).or_insert(0u32) += 1;
        }
        let mut counts_vec: Vec<_> = counts.iter().collect();
        counts_vec.sort_by_key(|&(p, _)| *p);
        eprintln!("  Pitch histogram:");
        for (p, c) in counts_vec {
            eprintln!("    {:3} ({:<4}): {}", p, midi_to_note_name(*p), "=".repeat(*c as usize));
        }

        // Inter-onset intervals
        let ioi: Vec<f64> = notes.windows(2).map(|w| w[1].onset - w[0].onset).collect();
        if !ioi.is_empty() {
            let avg_ioi: f64 = ioi.iter().sum::<f64>() / ioi.len() as f64;
            let min_ioi = ioi.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_ioi = ioi.iter().cloned().fold(0.0_f64, f64::max);
            eprintln!("  Inter-onset interval: min={:.3}s, max={:.3}s, avg={:.3}s", min_ioi, max_ioi, avg_ioi);
        }
    }
}
