-- Version 8: immutable bank-statement closing balance verification snapshots.
CREATE TABLE treasury_closing_verifications(
 id INTEGER PRIMARY KEY,
 statement_reference TEXT NOT NULL,
 closing_date TEXT NOT NULL,
 statement_balance_minor INTEGER NOT NULL CHECK(statement_balance_minor>=0),
 computed_balance_minor INTEGER NOT NULL,
 difference_minor INTEGER NOT NULL,
 pending_credit_minor INTEGER NOT NULL CHECK(pending_credit_minor>=0),
 pending_debit_minor INTEGER NOT NULL CHECK(pending_debit_minor>=0),
 pending_credit_count INTEGER NOT NULL CHECK(pending_credit_count>=0),
 pending_debit_count INTEGER NOT NULL CHECK(pending_debit_count>=0),
 status TEXT NOT NULL CHECK(status IN('matched','pending','mismatch')),
 created_at TEXT NOT NULL DEFAULT(datetime('now')),
 UNIQUE(statement_reference,closing_date)
);
CREATE INDEX idx_v8_closing_date ON treasury_closing_verifications(closing_date DESC,id DESC);
CREATE TRIGGER trg_v8_closing_immutable_update
BEFORE UPDATE ON treasury_closing_verifications
BEGIN SELECT RAISE(ABORT,'treasury closing verification history is immutable'); END;
CREATE TRIGGER trg_v8_closing_immutable_delete
BEFORE DELETE ON treasury_closing_verifications
BEGIN SELECT RAISE(ABORT,'treasury closing verification history cannot be deleted'); END;
