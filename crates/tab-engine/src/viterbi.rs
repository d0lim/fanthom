use crate::midi::MidiNote;
use crate::tab::{Candidate, NoteOrigin, TabNote, TabSheet, Tuning, pitch_to_candidates};

const OPEN_STRING_PENALTY: f64 = 60.0;
const STRING_CROSS_COST: f64 = 2.0;
const SAME_STRING_BONUS: f64 = -0.5;
const STRETCH_PENALTY: f64 = 3.0;
const STRETCH_THRESHOLD: u8 = 4;
// Small tie-break preference for higher-pitched strings (A, D, G over E).
const HIGHER_STRING_BONUS: f64 = 0.25;
// E string at high frets is uncomfortable (thickest string, long reach from nut).
// Strong enough to overcome the same-string-continuation bias when a more
// comfortable A/D position exists.
const E_STRING_HIGH_FRET_PENALTY: f64 = 2.5;

fn emission_cost(candidate: &Candidate) -> f64 {
    let mut cost = 0.0;
    if candidate.fret == 0 && candidate.string > 0 {
        cost += OPEN_STRING_PENALTY;
    }
    // Graduated fret comfort: low frets (2-4) are the natural hand position,
    // difficulty rises past fret 7 due to reach and string tension.
    cost += match candidate.fret {
        0 => 0.0,
        1 => -2.0,
        2 => -3.0,
        3 => -2.8,
        4 => -2.6,
        5 => -2.4,
        6 => -2.2,
        7 => -2.0,
        8 => -1.2,
        9 => -0.6,
        10..=14 => 1.0,
        _ => 3.0,
    };
    if candidate.string == 0 && candidate.fret >= 7 {
        cost += E_STRING_HIGH_FRET_PENALTY;
    }
    cost -= candidate.string as f64 * HIGHER_STRING_BONUS;
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
    // Reduced fret-jump costs: short hand movements are natural on bass, not
    // disproportionately expensive. Previous multipliers (1.5 / 3.0 / 5.0) were
    // too punishing for normal hand repositioning across strings.
    cost += match fret_diff {
        0..=1 => 0.0,
        2..=3 => fret_diff as f64 * 1.0,
        4..=5 => fret_diff as f64 * 2.0,
        _ => fret_diff as f64 * 3.5,
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
        // Adjacent notes should stay within a reasonable hand span — no huge
        // fret jumps. Exact string choice depends on fret-cost balance.
        for w in sheet.notes.windows(2) {
            let diff = (w[1].fret as i8 - w[0].fret as i8).unsigned_abs();
            assert!(
                diff <= 4,
                "adjacent notes should be within 4 frets, got fret {} -> {}",
                w[0].fret,
                w[1].fret
            );
        }
    }

    #[test]
    fn transposed_passage_prefers_a_string_over_e_high_frets() {
        // Original passage on A string: B1, B1, B1, C#2, E2 (frets 2,2,2,4,7).
        // After -2 semitones: A1, A1, A1, B1, D2.
        // A1 is forced to E-5 (A-0 has open-string penalty), but B1 and D2
        // should stay on A string (A-2, A-5) rather than migrate to the
        // uncomfortable E-7 / E-10 positions.
        let notes = vec![
            make_note(33, 0.0, 0.5), // A1
            make_note(33, 0.5, 1.0),
            make_note(33, 1.0, 1.5),
            make_note(35, 1.5, 2.0), // B1
            make_note(38, 2.0, 2.5), // D2
        ];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!((sheet.notes[0].string, sheet.notes[0].fret), (0, 5));
        assert_eq!((sheet.notes[1].string, sheet.notes[1].fret), (0, 5));
        assert_eq!((sheet.notes[2].string, sheet.notes[2].fret), (0, 5));
        assert_eq!((sheet.notes[3].string, sheet.notes[3].fret), (1, 2), "B1 should go to A-2, not E-7");
        assert_eq!((sheet.notes[4].string, sheet.notes[4].fret), (1, 5), "D2 should go to A-5, not E-10");
    }

    #[test]
    fn comfort_zone_preference() {
        let notes = vec![make_note(43, 0.0, 0.5)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].string, 2);
        assert_eq!(sheet.notes[0].fret, 5);
    }

    #[test]
    fn tied_comfort_zone_prefers_higher_string() {
        // C2 (MIDI 36) has two comfort-zone candidates:
        //   E string fret 8 (emission -1.5) vs A string fret 3 (emission -1.5)
        // With the tie-break bonus, A string should win because it's more
        // comfortable to play mid-fret notes on a higher string.
        let notes = vec![make_note(36, 0.0, 0.5)];
        let sheet = optimize(&notes, Tuning::Standard4, 120.0, (4, 4));
        assert_eq!(sheet.notes[0].string, 1, "C2 should prefer A string over E string");
        assert_eq!(sheet.notes[0].fret, 3);
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
