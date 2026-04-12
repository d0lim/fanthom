use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub data_dir: PathBuf,
    pub songs_dir: PathBuf,
}

impl AppState {
    pub fn new(db: Connection, data_dir: PathBuf) -> Self {
        let songs_dir = data_dir.join("songs");
        std::fs::create_dir_all(&songs_dir).ok();
        Self {
            db: Mutex::new(db),
            data_dir,
            songs_dir,
        }
    }
}
