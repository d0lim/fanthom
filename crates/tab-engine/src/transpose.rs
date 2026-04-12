use crate::midi::MidiNote;
use crate::tab::{NoteOrigin, Tuning};
use crate::viterbi;

const BASS_MIN: u8 = 28;
const BASS_MAX: u8 = 67;

pub fn transpose(
    original_notes: &[MidiNote],
    semitones: i8,
    tuning: Tuning,
    tempo: f64,
    time_sig: (u8, u8),
) -> crate::tab::TabSheet {
    let transposed: Vec<(MidiNote, Option<i8>)> = original_notes
        .iter()
        .map(|note| {
            let raw = note.pitch as i16 + semitones as i16;
            let (final_pitch, octave_shift) = clamp_to_bass_range(raw);
            (
                MidiNote {
                    pitch: final_pitch,
                    onset: note.onset,
                    offset: note.offset,
                    velocity: note.velocity,
                    technique: note.technique,
                },
                octave_shift,
            )
        })
        .collect();

    let midi_notes: Vec<MidiNote> = transposed.iter().map(|(n, _)| n.clone()).collect();
    let mut sheet = viterbi::optimize(&midi_notes, tuning, tempo, time_sig);
    sheet.key_transpose = semitones;

    for (i, (_, shift)) in transposed.iter().enumerate() {
        if let Some(s) = shift {
            sheet.notes[i].origin = NoteOrigin::OctaveShifted(*s);
        }
    }

    sheet
}

fn clamp_to_bass_range(raw: i16) -> (u8, Option<i8>) {
    if raw < BASS_MIN as i16 {
        let corrected = raw + 12;
        if corrected >= BASS_MIN as i16 && corrected <= BASS_MAX as i16 {
            (corrected as u8, Some(1))
        } else {
            (BASS_MIN, Some(1))
        }
    } else if raw > BASS_MAX as i16 {
        let corrected = raw - 12;
        if corrected >= BASS_MIN as i16 && corrected <= BASS_MAX as i16 {
            (corrected as u8, Some(-1))
        } else {
            (BASS_MAX, Some(-1))
        }
    } else {
        (raw as u8, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note(pitch: u8, onset: f64) -> MidiNote {
        MidiNote { pitch, onset, offset: onset + 0.5, velocity: 80, technique: None }
    }

    #[test]
    fn transpose_within_range_no_octave_shift() {
        let notes = vec![make_note(33, 0.0)];
        let sheet = transpose(&notes, 2, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].midi_pitch, 35);
        assert_eq!(sheet.key_transpose, 2);
        assert!(matches!(sheet.notes[0].origin, NoteOrigin::Optimized));
    }

    #[test]
    fn transpose_down_below_range_gets_octave_shifted_up() {
        let notes = vec![make_note(28, 0.0)];
        let sheet = transpose(&notes, -1, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].midi_pitch, 39);
        assert!(matches!(sheet.notes[0].origin, NoteOrigin::OctaveShifted(1)));
    }

    #[test]
    fn transpose_up_above_range_gets_octave_shifted_down() {
        let notes = vec![make_note(67, 0.0)];
        let sheet = transpose(&notes, 1, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].midi_pitch, 56);
        assert!(matches!(sheet.notes[0].origin, NoteOrigin::OctaveShifted(-1)));
    }

    #[test]
    fn transpose_zero_preserves_notes() {
        let notes = vec![make_note(40, 0.0), make_note(45, 0.5)];
        let sheet = transpose(&notes, 0, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].midi_pitch, 40);
        assert_eq!(sheet.notes[1].midi_pitch, 45);
        assert_eq!(sheet.key_transpose, 0);
    }

    #[test]
    fn clamp_handles_extreme_shift() {
        let (pitch, shift) = clamp_to_bass_range(10);
        assert_eq!(pitch, 28);
        assert_eq!(shift, Some(1));
    }
}
