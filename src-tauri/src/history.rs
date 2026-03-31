use anyhow::Result;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

static MIGRATIONS: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS transcriptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            model TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            duration_ms INTEGER
        );",
    ),
    M::up("ALTER TABLE transcriptions ADD COLUMN audio_path TEXT;"),
    M::up("ALTER TABLE transcriptions ADD COLUMN input_tokens INTEGER;"),
    M::up("ALTER TABLE transcriptions ADD COLUMN output_tokens INTEGER;"),
    M::up(
        "CREATE TABLE IF NOT EXISTS statistics (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            total_duration_ms INTEGER NOT NULL DEFAULT 0,
            count INTEGER NOT NULL DEFAULT 0
        );",
    ),
    M::up("INSERT OR IGNORE INTO statistics (id) VALUES (1);"),
];

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub text: String,
    pub model: String,
    pub timestamp: i64,
    pub duration_ms: Option<i64>,
    pub audio_path: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Statistics {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_duration_ms: i64,
    pub count: i64,
}

pub struct HistoryManager {
    conn: Mutex<Connection>,
    data_dir: PathBuf,
}

impl HistoryManager {
    pub fn new() -> Result<Self> {
        let data_dir = crate::data_dir();
        std::fs::create_dir_all(&data_dir)?;

        // Also create audio dir
        let audio_dir = data_dir.join("audio");
        std::fs::create_dir_all(&audio_dir)?;

        let db_path = data_dir.join("history.db");

        let mut conn = Connection::open(&db_path)?;
        let migrations = Migrations::new(MIGRATIONS.to_vec());
        migrations.to_latest(&mut conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            data_dir,
        })
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.data_dir.join("audio")
    }

    pub fn add_entry(
        &self,
        text: &str,
        model: &str,
        duration_ms: Option<i64>,
        audio_path: Option<&str>,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<HistoryEntry> {
        let conn = self.conn.lock().unwrap();
        let timestamp = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO transcriptions (text, model, timestamp, duration_ms, audio_path, input_tokens, output_tokens) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![text, model, timestamp, duration_ms, audio_path, input_tokens, output_tokens],
        )?;
        let id = conn.last_insert_rowid();
        Ok(HistoryEntry {
            id,
            text: text.to_string(),
            model: model.to_string(),
            timestamp,
            duration_ms,
            audio_path: audio_path.map(|s| s.to_string()),
            input_tokens,
            output_tokens,
        })
    }

    pub fn get_entry_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, text, model, timestamp, duration_ms, audio_path, input_tokens, output_tokens FROM transcriptions WHERE id = ?1",
        )?;
        let entry = stmt
            .query_row([id], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    model: row.get(2)?,
                    timestamp: row.get(3)?,
                    duration_ms: row.get(4)?,
                    audio_path: row.get(5)?,
                    input_tokens: row.get(6)?,
                    output_tokens: row.get(7)?,
                })
            })
            .ok();
        Ok(entry)
    }

    pub fn update_entry(
        &self,
        id: i64,
        text: &str,
        model: &str,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let timestamp = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE transcriptions SET text = ?1, model = ?2, timestamp = ?3, input_tokens = ?4, output_tokens = ?5 WHERE id = ?6",
            rusqlite::params![text, model, timestamp, input_tokens, output_tokens, id],
        )?;
        Ok(())
    }

    pub fn get_entries(&self) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, text, model, timestamp, duration_ms, audio_path, input_tokens, output_tokens FROM transcriptions ORDER BY timestamp DESC",
        )?;
        let entries = stmt
            .query_map([], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    model: row.get(2)?,
                    timestamp: row.get(3)?,
                    duration_ms: row.get(4)?,
                    audio_path: row.get(5)?,
                    input_tokens: row.get(6)?,
                    output_tokens: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub fn delete_entry(&self, id: i64) -> Result<()> {
        // Also delete audio file if exists
        let conn = self.conn.lock().unwrap();
        let audio_path: Option<String> = conn
            .query_row(
                "SELECT audio_path FROM transcriptions WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        if let Some(path) = audio_path {
            let _ = std::fs::remove_file(&path);
        }
        conn.execute("DELETE FROM transcriptions WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<()> {
        // Delete all audio files
        let audio_dir = self.audio_dir();
        if audio_dir.exists() {
            let _ = std::fs::remove_dir_all(&audio_dir);
            let _ = std::fs::create_dir_all(&audio_dir);
        }
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM transcriptions", [])?;
        Ok(())
    }

    pub fn update_statistics(
        &self,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
        duration_ms: Option<i64>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE statistics SET 
                input_tokens = input_tokens + ?1,
                output_tokens = output_tokens + ?2,
                total_duration_ms = total_duration_ms + ?3,
                count = count + 1
            WHERE id = 1",
            rusqlite::params![
                input_tokens.unwrap_or(0),
                output_tokens.unwrap_or(0),
                duration_ms.unwrap_or(0)
            ],
        )?;
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<Statistics> {
        let conn = self.conn.lock().unwrap();
        let stats = conn.query_row(
            "SELECT input_tokens, output_tokens, total_duration_ms, count FROM statistics WHERE id = 1",
            [],
            |row| {
                Ok(Statistics {
                    input_tokens: row.get(0)?,
                    output_tokens: row.get(1)?,
                    total_duration_ms: row.get(2)?,
                    count: row.get(3)?,
                })
            },
        )?;
        Ok(stats)
    }

    pub fn clear_statistics(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE statistics SET input_tokens = 0, output_tokens = 0, total_duration_ms = 0, count = 0 WHERE id = 1",
            [],
        )?;
        Ok(())
    }
}
