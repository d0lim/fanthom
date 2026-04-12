use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Tuning {
    Standard4, // E=28, A=33, D=38, G=43
}

impl Tuning {
    pub fn open_pitches(&self) -> &[u8] {
        match self {
            Tuning::Standard4 => &[28, 33, 38, 43],
        }
    }

    pub fn num_strings(&self) -> u8 {
        self.open_pitches().len() as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NoteOrigin {
    Normal,
    Optimized,
    OctaveShifted(i8),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabNote {
    pub string: u8,
    pub fret: u8,
    pub midi_pitch: u8,
    pub onset: f64,
    pub duration: f64,
    pub origin: NoteOrigin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSheet {
    pub notes: Vec<TabNote>,
    pub tempo: f64,
    pub time_signature: (u8, u8),
    pub tuning: Tuning,
    pub key_transpose: i8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub string: u8,
    pub fret: u8,
}

const MAX_FRET: u8 = 24;

pub fn pitch_to_candidates(pitch: u8, tuning: Tuning) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for (i, &open) in tuning.open_pitches().iter().enumerate() {
        if pitch >= open && pitch - open <= MAX_FRET {
            candidates.push(Candidate {
                string: i as u8,
                fret: pitch - open,
            });
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e_open_string_has_one_candidate() {
        let candidates = pitch_to_candidates(28, Tuning::Standard4);
        assert_eq!(candidates, vec![Candidate { string: 0, fret: 0 }]);
    }

    #[test]
    fn a_open_has_two_candidates() {
        let candidates = pitch_to_candidates(33, Tuning::Standard4);
        assert_eq!(candidates, vec![
            Candidate { string: 0, fret: 5 },
            Candidate { string: 1, fret: 0 },
        ]);
    }

    #[test]
    fn middle_note_has_multiple_candidates() {
        let candidates = pitch_to_candidates(38, Tuning::Standard4);
        assert_eq!(candidates, vec![
            Candidate { string: 0, fret: 10 },
            Candidate { string: 1, fret: 5 },
            Candidate { string: 2, fret: 0 },
        ]);
    }

    #[test]
    fn high_note_on_g_string_only() {
        let candidates = pitch_to_candidates(67, Tuning::Standard4);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], Candidate { string: 3, fret: 24 });
    }

    #[test]
    fn out_of_range_returns_empty() {
        let candidates = pitch_to_candidates(27, Tuning::Standard4);
        assert!(candidates.is_empty());
        let candidates = pitch_to_candidates(68, Tuning::Standard4);
        assert!(candidates.is_empty());
    }
}
