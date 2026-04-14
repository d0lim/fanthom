use crate::tab::{TabSheet, Tuning};

const CHARS_PER_BEAT: usize = 4;
const BEATS_PER_MEASURE: usize = 4;
const CHARS_PER_MEASURE: usize = CHARS_PER_BEAT * BEATS_PER_MEASURE;
const MEASURES_PER_LINE: usize = 4;

pub fn export(sheet: &TabSheet) -> String {
    if sheet.notes.is_empty() {
        return String::from("(empty tab)");
    }

    let string_labels = match sheet.tuning {
        Tuning::Standard4 => ["E", "A", "D", "G"],
    };
    let num_strings = string_labels.len();

    let last_note = sheet.notes.iter().map(|n| n.onset + n.duration).fold(0.0f64, f64::max);
    let beat_duration = 60.0 / sheet.tempo;
    let total_beats = (last_note / beat_duration).ceil() as usize;
    let total_measures = total_beats.div_ceil(BEATS_PER_MEASURE).max(1);
    let total_chars = total_measures * CHARS_PER_MEASURE;

    let mut grid: Vec<Vec<String>> = vec![vec!["-".to_string(); total_chars]; num_strings];

    for note in &sheet.notes {
        let beat_pos = note.onset / beat_duration;
        let char_pos = (beat_pos * CHARS_PER_BEAT as f64).round() as usize;
        if char_pos < total_chars {
            // Add slide marker before fret number
            if note.technique == Some(crate::midi::Technique::Slide) && char_pos > 0 {
                grid[note.string as usize][char_pos - 1] = "/".to_string();
            }

            let fret_str = note.fret.to_string();
            let s = note.string as usize;
            for (i, ch) in fret_str.chars().enumerate() {
                if char_pos + i < total_chars {
                    grid[s][char_pos + i] = ch.to_string();
                }
            }
        }
    }

    let mut output = String::new();
    let mut measure_offset = 0;

    while measure_offset < total_measures {
        let line_measures = (total_measures - measure_offset).min(MEASURES_PER_LINE);
        let start_char = measure_offset * CHARS_PER_MEASURE;
        let end_char = (start_char + line_measures * CHARS_PER_MEASURE).min(total_chars);

        for s in (0..num_strings).rev() {
            output.push_str(string_labels[s]);
            output.push('|');
            for (c, cell) in grid[s].iter().enumerate().take(end_char).skip(start_char) {
                output.push_str(cell);
                if (c + 1) % CHARS_PER_MEASURE == 0 && c + 1 < end_char {
                    output.push('|');
                }
            }
            output.push('|');
            output.push('\n');
        }
        output.push('\n');

        measure_offset += line_measures;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tab::{NoteOrigin, TabNote};

    fn make_tab_note(string: u8, fret: u8, onset: f64) -> TabNote {
        TabNote {
            string, fret,
            midi_pitch: 0,
            onset,
            duration: 0.5,
            origin: NoteOrigin::Normal,
            technique: None,
        }
    }

    #[test]
    fn empty_sheet_returns_placeholder() {
        let sheet = TabSheet {
            notes: vec![],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        assert_eq!(export(&sheet), "(empty tab)");
    }

    #[test]
    fn single_note_renders() {
        let sheet = TabSheet {
            notes: vec![make_tab_note(0, 5, 0.0)],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let result = export(&sheet);
        assert!(result.contains("G|"));
        assert!(result.contains("E|"));
        let e_line = result.lines().find(|l| l.starts_with("E|")).unwrap();
        assert!(e_line.contains('5'));
    }

    #[test]
    fn two_digit_fret_renders() {
        let sheet = TabSheet {
            notes: vec![make_tab_note(0, 12, 0.0)],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let result = export(&sheet);
        let e_line = result.lines().find(|l| l.starts_with("E|")).unwrap();
        assert!(e_line.contains("12"));
    }

    #[test]
    fn slide_renders_with_slash() {
        use crate::midi::Technique;
        let sheet = TabSheet {
            notes: vec![
                TabNote {
                    string: 0, fret: 5, midi_pitch: 33,
                    onset: 0.0, duration: 0.5,
                    origin: NoteOrigin::Normal,
                    technique: None,
                },
                TabNote {
                    string: 0, fret: 7, midi_pitch: 35,
                    onset: 0.5, duration: 0.5,
                    origin: NoteOrigin::Normal,
                    technique: Some(Technique::Slide),
                },
            ],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let result = export(&sheet);
        let e_line = result.lines().find(|l| l.starts_with("E|")).unwrap();
        assert!(e_line.contains('/') || e_line.contains('\\'), "slide should have / or \\ marker: {}", e_line);
    }
}
