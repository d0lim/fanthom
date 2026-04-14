use rusqlite::{Connection, Result, params};
use std::path::Path;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS songs (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    source_url  TEXT,
    duration    REAL,
    tempo       REAL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tabs (
    id          TEXT PRIMARY KEY,
    song_id     TEXT NOT NULL REFERENCES songs(id),
    tuning      TEXT NOT NULL DEFAULT 'standard4',
    transpose   INTEGER NOT NULL DEFAULT 0,
    tab_data    BLOB NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);
"#;

pub fn open(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

pub fn insert_song(conn: &Connection, id: &str, title: &str, source_url: Option<&str>, duration: Option<f64>, tempo: Option<f64>) -> Result<()> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO songs (id, title, source_url, duration, tempo, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, title, source_url, duration, tempo, &now, &now],
    )?;
    Ok(())
}

pub fn insert_tab(conn: &Connection, id: &str, song_id: &str, tuning: &str, transpose: i32, tab_data: &[u8]) -> Result<()> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO tabs (id, song_id, tuning, transpose, tab_data, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, song_id, tuning, transpose, tab_data, &now],
    )?;
    Ok(())
}

fn chrono_now() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    format!("{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_schema_and_insert_song() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        insert_song(&conn, "song-1", "Test Song", Some("https://youtube.com/watch?v=abc"), Some(180.0), Some(120.0)).unwrap();
        let title: String = conn.query_row(
            "SELECT title FROM songs WHERE id = 'song-1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(title, "Test Song");
    }

    #[test]
    fn insert_and_retrieve_tab() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        insert_song(&conn, "s1", "Song", None, None, None).unwrap();
        let tab_data = vec![1u8, 2, 3, 4, 5];
        insert_tab(&conn, "t1", "s1", "standard4", 0, &tab_data).unwrap();
        let retrieved: Vec<u8> = conn.query_row(
            "SELECT tab_data FROM tabs WHERE id = 't1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(retrieved, tab_data);
    }
}
