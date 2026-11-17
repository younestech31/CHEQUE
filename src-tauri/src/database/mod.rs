use rusqlite::{Connection, Result, OptionalExtension, TransactionBehavior};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct Database { pub connection: Connection }

impl Database {
    pub fn open(app: &AppHandle) -> Result<Self> {
        let mut path: PathBuf = app.path().app_data_dir().expect("application data directory unavailable");
        std::fs::create_dir_all(&path).expect("cannot create application data directory");
        path.push("cheque.sqlite3");
        let mut connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;

        connection.execute_batch(include_str!("../../migrations/0001_initial.sql"))?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT(datetime('now')));")?;

        let v2: Option<i64> = connection.query_row(
            "SELECT version FROM schema_migrations WHERE version=2",
            [],
            |row| row.get(0),
        ).optional()?;

        if v2.is_none() {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(include_str!("../../migrations/0002_cheques_and_statement_workflow.sql"))?;
            tx.execute("INSERT INTO schema_migrations(version) VALUES(2)", [])?;
            tx.commit()?;
        }

        let v3: Option<i64> = connection.query_row(
            "SELECT version FROM schema_migrations WHERE version=3",
            [],
            |row| row.get(0),
        ).optional()?;

        if v3.is_none() {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(include_str!("../../migrations/0003_classification.sql"))?;
            tx.execute("INSERT INTO schema_migrations(version) VALUES(3)", [])?;
            tx.commit()?;
        }

        Ok(Self { connection })
    }
}
