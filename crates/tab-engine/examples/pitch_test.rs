use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: pitch_test <wav_path>");
    println!("Reading: {path}");
    let start = Instant::now();
    match tab_engine::transcribe_wav(&path) {
        Ok(notes) => {
            println!("Done in {:?}, found {} notes", start.elapsed(), notes.len());
            for n in notes.iter().take(10) {
                println!(
                    "  pitch={:3} onset={:6.2}s offset={:6.2}s vel={:3}",
                    n.pitch, n.onset, n.offset, n.velocity
                );
            }
        }
        Err(e) => println!("ERROR: {e}"),
    }
}
