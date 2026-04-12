use crate::tab::{TabSheet, Tuning};

pub fn export(sheet: &TabSheet) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">"#);
    xml.push('\n');
    xml.push_str(r#"<score-partwise version="4.0">"#);
    xml.push('\n');

    xml.push_str("  <part-list>\n");
    xml.push_str(r#"    <score-part id="P1">"#);
    xml.push('\n');
    xml.push_str("      <part-name>Bass Guitar</part-name>\n");
    xml.push_str("    </score-part>\n");
    xml.push_str("  </part-list>\n");

    xml.push_str(r#"  <part id="P1">"#);
    xml.push('\n');

    if sheet.notes.is_empty() {
        xml.push_str("    <measure number=\"1\">\n");
        write_attributes(&mut xml, sheet);
        xml.push_str("      <note><rest/><duration>4</duration><type>whole</type></note>\n");
        xml.push_str("    </measure>\n");
    } else {
        let beat_duration = 60.0 / sheet.tempo;
        let beats_per_measure = sheet.time_signature.0 as f64;

        let mut measure_num = 1;
        let mut current_beat = 0.0;
        let mut note_idx = 0;
        let mut first_measure = true;

        while note_idx < sheet.notes.len() {
            let measure_end = current_beat + beats_per_measure;

            xml.push_str(&format!("    <measure number=\"{}\">\n", measure_num));

            if first_measure {
                write_attributes(&mut xml, sheet);
                first_measure = false;
            }

            while note_idx < sheet.notes.len() {
                let note = &sheet.notes[note_idx];
                let note_beat = note.onset / beat_duration;
                if note_beat >= measure_end {
                    break;
                }

                let duration_beats = note.duration / beat_duration;
                let duration_divisions = (duration_beats * 4.0).round() as u32;
                let note_type = duration_to_type(duration_beats);

                xml.push_str("      <note>\n");
                write_pitch_for_tab(&mut xml, note.string, note.fret, &sheet.tuning);
                xml.push_str(&format!("        <duration>{}</duration>\n", duration_divisions.max(1)));
                xml.push_str(&format!("        <type>{}</type>\n", note_type));
                xml.push_str("        <notations>\n");
                xml.push_str("          <technical>\n");
                xml.push_str(&format!("            <string>{}</string>\n", note.string + 1));
                xml.push_str(&format!("            <fret>{}</fret>\n", note.fret));
                xml.push_str("          </technical>\n");
                xml.push_str("        </notations>\n");
                xml.push_str("      </note>\n");

                note_idx += 1;
            }

            xml.push_str("    </measure>\n");
            current_beat = measure_end;
            measure_num += 1;
        }
    }

    xml.push_str("  </part>\n");
    xml.push_str("</score-partwise>\n");
    xml
}

fn write_attributes(xml: &mut String, sheet: &TabSheet) {
    let num_strings = match sheet.tuning {
        Tuning::Standard4 => 4,
    };
    xml.push_str("      <attributes>\n");
    xml.push_str("        <divisions>4</divisions>\n");
    xml.push_str(&format!("        <time><beats>{}</beats><beat-type>{}</beat-type></time>\n",
        sheet.time_signature.0, sheet.time_signature.1));
    xml.push_str(&format!(
        "        <clef><sign>TAB</sign><line>{}</line></clef>\n",
        num_strings
    ));
    xml.push_str(&format!(
        "        <staff-details><staff-lines>{}</staff-lines></staff-details>\n",
        num_strings
    ));
    xml.push_str("      </attributes>\n");
}

fn write_pitch_for_tab(xml: &mut String, string: u8, fret: u8, tuning: &Tuning) {
    let open_pitches = tuning.open_pitches();
    let midi = open_pitches[string as usize] + fret;
    let octave = (midi / 12) as i8 - 1;
    let step = match midi % 12 {
        0 => "C", 1 => "C", 2 => "D", 3 => "D", 4 => "E",
        5 => "F", 6 => "F", 7 => "G", 8 => "G", 9 => "A",
        10 => "A", 11 => "B", _ => unreachable!(),
    };
    let alter = match midi % 12 {
        1 | 3 | 6 | 8 | 10 => Some(1),
        _ => None,
    };

    xml.push_str("        <pitch>\n");
    xml.push_str(&format!("          <step>{}</step>\n", step));
    if let Some(a) = alter {
        xml.push_str(&format!("          <alter>{}</alter>\n", a));
    }
    xml.push_str(&format!("          <octave>{}</octave>\n", octave));
    xml.push_str("        </pitch>\n");
}

fn duration_to_type(beats: f64) -> &'static str {
    if beats >= 3.5 { "whole" }
    else if beats >= 1.5 { "half" }
    else if beats >= 0.75 { "quarter" }
    else if beats >= 0.375 { "eighth" }
    else { "16th" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tab::{NoteOrigin, TabNote, TabSheet};

    fn make_tab_note(string: u8, fret: u8, onset: f64, duration: f64) -> TabNote {
        TabNote {
            string, fret,
            midi_pitch: 0,
            onset, duration,
            origin: NoteOrigin::Normal,
        }
    }

    #[test]
    fn empty_sheet_produces_valid_xml() {
        let sheet = TabSheet {
            notes: vec![],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let xml = export(&sheet);
        assert!(xml.contains("score-partwise"));
        assert!(xml.contains("<rest/>"));
        assert!(xml.contains("Bass Guitar"));
    }

    #[test]
    fn single_note_produces_tab_notation() {
        let sheet = TabSheet {
            notes: vec![make_tab_note(0, 5, 0.0, 0.5)],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let xml = export(&sheet);
        assert!(xml.contains("<string>1</string>"));
        assert!(xml.contains("<fret>5</fret>"));
        assert!(xml.contains("<sign>TAB</sign>"));
    }

    #[test]
    fn xml_starts_with_declaration() {
        let sheet = TabSheet {
            notes: vec![],
            tempo: 120.0,
            time_signature: (4, 4),
            tuning: Tuning::Standard4,
            key_transpose: 0,
        };
        let xml = export(&sheet);
        assert!(xml.starts_with("<?xml"));
    }
}
