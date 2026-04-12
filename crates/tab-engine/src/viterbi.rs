use crate::midi::MidiNote;
use crate::tab::{Candidate, NoteOrigin, TabNote, TabSheet, Tuning, pitch_to_candidates};

const OPEN_STRING_PENALTY: f64 = 60.0;
const STRING_CROSS_COST: f64 = 2.0;
const SAME_STRING_BONUS: f64 = -0.5;
const STRETCH_PENALTY: f64 = 3.0;
const STRETCH_THRESHOLD: u8 = 4;

fn emission_cost(candidate: &Candidate) -> f64 {
    let mut cost = 0.0;
    if candidate.fret == 0 && candidate.string > 0 {
        cost += OPEN_STRING_PENALTY;
    }
    cost += match candidate.fret {
        0 => 0.0,
        1 => 0.0,
        2..=9 => -1.5,
        10..=14 => 1.0,
        _ => 3.0,
    };
    cost
}

fn transition_cost(prev: &Candidate, curr: &Candidate) -> f64 {
    let mut cost = 0.0;
    let string_diff = (curr.string as i8 - prev.string as i8).unsigned_abs();
    if string_diff == 0 {
        cost += SAME_STRING_BONUS;
    } else {
        cost += STRING_CROSS_COST * string_diff as f64;
    }
    let fret_diff = if prev.fret == 0 || curr.fret == 0 {
        0
    } else {
        (curr.fret as i8 - prev.fret as i8).unsigned_abs()
    };
    cost += match fret_diff {
        0..=1 => 0.0,
        2..=3 => fret_diff as f64 * 1.5,
        4..=5 => fret_diff as f64 * 3.0,
        _ => fret_diff as f64 * 5.0,
    };
    if string_diff > 0 && prev.fret > 0 && curr.fret > 0 {
        let span = (curr.fret as i8 - prev.fret as i8).unsigned_abs();
        if span > STRETCH_THRESHOLD {
            cost += (span - STRETCH_THRESHOLD) as f64 * STRETCH_PENALTY;
        }
    }
    cost
}

pub fn optimize(notes: &[MidiNote], tuning: Tuning, tempo: f64, time_sig: (u8, u8)) -> TabSheet {
    if notes.is_empty() {
        return TabSheet {
            notes: vec![],
            tempo,
            time_signature: time_sig,
            tuning,
            key_transpose: 0,
        };
    }

    let all_candidates: Vec<Vec<Candidate>> = notes
        .iter()
        .map(|n| pitch_to_candidates(n.pitch, tuning))
        .collect();

    let n = notes.len();
    let mut dp: Vec<Vec<(f64, usize)>> = Vec::with_capacity(n);

    let first_costs: Vec<(f64, usize)> = all_candidates[0]
        .iter()
        .map(|c| (emission_cost(c), 0))
        .collect();
    dp.push(first_costs);

    for i in 1..n {
        let prev_candidates = &all_candidates[i - 1];
        let curr_candidates = &all_candidates[i];
        let mut curr_dp = Vec::with_capacity(curr_candidates.len());
        for curr_c in curr_candidates {
            let e_cost = emission_cost(curr_c);
            let mut best_cost = f64::INFINITY;
            let mut best_prev = 0;
            for (j, prev_c) in prev_candidates.iter().enumerate() {
                // Slides must stay on the same string
                if notes[i].technique == Some(crate::midi::Technique::Slide)
                    && prev_c.string != curr_c.string
                {
                    continue;
                }
                let total = dp[i - 1][j].0 + transition_cost(prev_c, curr_c) + e_cost;
                if total < best_cost {
                    best_cost = total;
                    best_prev = j;
                }
            }
            // Fallback: if slide constraint made all paths invalid, relax it
            if best_cost == f64::INFINITY {
                for (j, prev_c) in prev_candidates.iter().enumerate() {
                    let total = dp[i - 1][j].0 + transition_cost(prev_c, curr_c) + e_cost;
                    if total < best_cost {
                        best_cost = total;
                        best_prev = j;
                    }
                }
            }
            curr_dp.push((best_cost, best_prev));
        }
        dp.push(curr_dp);
    }

    let last = &dp[n - 1];
    let mut best_idx = 0;
    let mut best_cost = f64::INFINITY;
    for (i, &(cost, _)) in last.iter().enumerate() {
        if cost < best_cost {
            best_cost = cost;
            best_idx = i;
        }
    }

    let mut path = vec![0usize; n];
    path[n - 1] = best_idx;
    for i in (1..n).rev() {
        path[i - 1] = dp[i][path[i]].1;
    }

    let tab_notes: Vec<TabNote> = notes
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let c = &all_candidates[i][path[i]];
            TabNote {
                string: c.string,
                fret: c.fret,
                midi_pitch: note.pitch,
                onset: note.onset,
                duration: note.offset - note.onset,
                origin: NoteOrigin::Optimized,
                technique: note.technique,
            }
        })
        .collect();

    TabSheet {
        notes: tab_notes,
        tempo,
        time_signature: time_sig,
        tuning,
        key_transpose: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note(pitch: u8, onset: f64, offset: f64) -> MidiNote {
        MidiNote { pitch, onset, offset, velocity: 80, technique: None }
    }

    #[test]
    fn open_e_string_preferred_over_nothing() {
        let notes = vec![make_note(28, 0.0, 0.5)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes.len(), 1);
        assert_eq!(sheet.notes[0].string, 0);
        assert_eq!(sheet.notes[0].fret, 0);
    }

    #[test]
    fn avoids_adg_open_strings() {
        let notes = vec![make_note(33, 0.0, 0.5)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].string, 0);
        assert_eq!(sheet.notes[0].fret, 5);
    }

    #[test]
    fn prefers_nearby_frets_for_sequence() {
        let notes = vec![
            make_note(33, 0.0, 0.5),
            make_note(34, 0.5, 1.0),
            make_note(36, 1.0, 1.5),
        ];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert!(sheet.notes.iter().all(|n| n.string == 0));
    }

    #[test]
    fn comfort_zone_preference() {
        let notes = vec![make_note(43, 0.0, 0.5)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].string, 2);
        assert_eq!(sheet.notes[0].fret, 5);
    }

    #[test]
    fn slide_notes_use_same_string() {
        use crate::midi::Technique;
        let notes = vec![
            MidiNote { pitch: 33, onset: 0.0, offset: 0.5, velocity: 80, technique: None },
            MidiNote { pitch: 35, onset: 0.5, offset: 1.0, velocity: 80, technique: Some(Technique::Slide) },
        ];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(
            sheet.notes[0].string, sheet.notes[1].string,
            "slide notes must be on the same string"
        );
    }

    #[test]
    fn empty_notes_returns_empty_sheet() {
        let sheet = optimize(&[], Tuning::Standard4, 120.0, (4, 4));
        assert!(sheet.notes.is_empty());
    }

    #[test]
    fn handles_notes_with_single_candidate() {
        let notes = vec![make_note(28, 0.0, 0.5), make_note(67, 0.5, 1.0)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].fret, 0);
        assert_eq!(sheet.notes[1].string, 3);
        assert_eq!(sheet.notes[1].fret, 24);
    }
}
