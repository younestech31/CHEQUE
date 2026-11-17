use rusqlite::{Connection, Result};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct Database { pub connection: Connection }

impl Database {
    pub fn open(app: &AppHandle) -> Result<Self> {
        let mut path: PathBuf = app.path().app_data_dir().expect("application data directory unavailable");
        std::fs::create_dir_all(&path).expect("cannot create application data directory");
        path.push("cheque.sqlite3");
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.execute_batch(include_str!("../../migrations/0001_initial.sql"))?;
        connection.execute_batch(include_str!("../../migrations/0002_cheques_and_statement_workflow.sql"))?;
        Ok(Self { connection })
    }
}
