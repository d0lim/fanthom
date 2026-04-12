use serde::{Deserialize, Serialize};

/// How a note was articulated.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Technique {
    Slide,
}

/// A single note extracted from pitch detection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MidiNote {
    pub pitch: u8,
    pub onset: f64,
    pub offset: f64,
    pub velocity: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technique: Option<Technique>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_midi_note_from_json() {
        let json = r#"{"pitch": 40, "onset": 1.5, "offset": 2.0, "velocity": 100}"#;
        let note: MidiNote = serde_json::from_str(json).unwrap();
        assert_eq!(note.pitch, 40);
        assert_eq!(note.onset, 1.5);
        assert_eq!(note.offset, 2.0);
        assert_eq!(note.velocity, 100);
    }

    #[test]
    fn deserialize_midi_note_sequence() {
        let json = r#"[
            {"pitch": 28, "onset": 0.0, "offset": 0.5, "velocity": 80},
            {"pitch": 33, "onset": 0.5, "offset": 1.0, "velocity": 90}
        ]"#;
        let notes: Vec<MidiNote> = serde_json::from_str(json).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].pitch, 28);
        assert_eq!(notes[1].pitch, 33);
    }

    #[test]
    fn deserialize_midi_note_with_technique() {
        let json = r#"{"pitch": 40, "onset": 1.5, "offset": 2.0, "velocity": 100, "technique": "Slide"}"#;
        let note: MidiNote = serde_json::from_str(json).unwrap();
        assert_eq!(note.technique, Some(Technique::Slide));
    }

    #[test]
    fn deserialize_midi_note_without_technique_defaults_none() {
        let json = r#"{"pitch": 40, "onset": 1.5, "offset": 2.0, "velocity": 100}"#;
        let note: MidiNote = serde_json::from_str(json).unwrap();
        assert_eq!(note.technique, None);
    }

    #[test]
    fn serialize_midi_note_without_technique_omits_field() {
        let note = MidiNote { pitch: 40, onset: 1.0, offset: 2.0, velocity: 80, technique: None };
        let json = serde_json::to_string(&note).unwrap();
        assert!(!json.contains("technique"));
    }
}
