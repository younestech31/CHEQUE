use rusqlite::{params, Connection, OptionalExtension, Result, TransactionBehavior};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct Database { pub connection: Connection }

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name=?2",
        params![table, column],
        |row| row.get(0),
    )?;
    if exists == 0 {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"))?;
    }
    Ok(())
}

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
            ensure_column(&tx, "cheques", "issue_date", "TEXT")?;
            ensure_column(&tx, "cheques", "notes", "TEXT")?;
            ensure_column(&tx, "bank_statement_entries", "description", "TEXT")?;
            ensure_column(&tx, "collections", "confirmation_reference", "TEXT")?;
            ensure_column(&tx, "collections", "notes", "TEXT")?;
            ensure_column(&tx, "audit_log", "after_json", "TEXT")?;
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
            ensure_column(&tx, "allocations", "operation_type", "TEXT NOT NULL DEFAULT 'غير محدد'")?;
            tx.execute_batch(include_str!("../../migrations/0003_classification.sql"))?;
            tx.execute("INSERT INTO schema_migrations(version) VALUES(3)", [])?;
            tx.commit()?;
        }

        let v4: Option<i64> = connection.query_row(
            "SELECT version FROM schema_migrations WHERE version=4",
            [],
            |row| row.get(0),
        ).optional()?;

        if v4.is_none() {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            ensure_column(&tx, "outgoing_payments", "external_reference", "TEXT")?;
            ensure_column(&tx, "outgoing_payments", "notes", "TEXT")?;
            tx.execute_batch(include_str!("../../migrations/0004_payments_and_freezes.sql"))?;
            tx.execute("INSERT INTO schema_migrations(version) VALUES(4)", [])?;
            tx.commit()?;
        }

        let v5: Option<i64> = connection.query_row(
            "SELECT version FROM schema_migrations WHERE version=5",
            [],
            |row| row.get(0),
        ).optional()?;

        if v5.is_none() {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(include_str!("../../migrations/0005_lock_payment_detail_moves.sql"))?;
            tx.execute("INSERT INTO schema_migrations(version) VALUES(5)", [])?;
            tx.commit()?;
        }

        let v6: Option<i64> = connection.query_row(
            "SELECT version FROM schema_migrations WHERE version=6",
            [],
            |row| row.get(0),
        ).optional()?;

        if v6.is_none() {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(include_str!("../../migrations/0006_treasury_reconciliation.sql"))?;
            tx.execute("INSERT INTO schema_migrations(version) VALUES(6)", [])?;
            tx.commit()?;
        }

        let v7: Option<i64> = connection.query_row(
            "SELECT version FROM schema_migrations WHERE version=7",
            [],
            |row| row.get(0),
        ).optional()?;

        if v7.is_none() {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(include_str!("../../migrations/0007_bank_match_date_guard.sql"))?;
            tx.execute("INSERT INTO schema_migrations(version) VALUES(7)", [])?;
            tx.commit()?;
        }

        Ok(Self { connection })
    }
}
