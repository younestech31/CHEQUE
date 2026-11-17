-- Version 2: cheque custody, bank statement reconciliation and immutable source matching.
-- Applied once by the Rust migration runner inside a single transaction.
ALTER TABLE cheques ADD COLUMN issue_date TEXT;
ALTER TABLE cheques ADD COLUMN notes TEXT;
ALTER TABLE bank_statement_entries ADD COLUMN description TEXT;
ALTER TABLE collections ADD COLUMN confirmation_reference TEXT;
ALTER TABLE collections ADD COLUMN notes TEXT;
ALTER TABLE audit_log ADD COLUMN after_json TEXT;

CREATE TABLE cheque_deposits(
 id INTEGER PRIMARY KEY,
 cheque_id INTEGER NOT NULL REFERENCES cheques(id),
 deposit_date TEXT NOT NULL,
 deposit_reference TEXT,
 bank_name TEXT,
 amount_minor INTEGER NOT NULL CHECK(amount_minor>0),
 notes TEXT
);
CREATE TABLE collection_cheques(
 id INTEGER PRIMARY KEY,
 collection_id INTEGER NOT NULL REFERENCES collections(id),
 cheque_id INTEGER NOT NULL REFERENCES cheques(id),
 matched_minor INTEGER NOT NULL CHECK(matched_minor>0),
 UNIQUE(collection_id,cheque_id)
);

CREATE INDEX IF NOT EXISTS idx_cheques_received_date ON cheques(received_date DESC,id DESC);
CREATE INDEX IF NOT EXISTS idx_cheques_status ON cheques(status,received_date DESC);
CREATE INDEX IF NOT EXISTS idx_deposits_cheque ON cheque_deposits(cheque_id,deposit_date);
CREATE INDEX IF NOT EXISTS idx_statement_entries_date ON bank_statement_entries(entry_date DESC,id DESC);
CREATE INDEX IF NOT EXISTS idx_statement_entries_reference ON bank_statement_entries(statement_reference);
CREATE INDEX IF NOT EXISTS idx_collections_bank_entry ON collections(bank_entry_id,collection_date);
CREATE INDEX IF NOT EXISTS idx_collection_cheques_cheque ON collection_cheques(cheque_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_statement_entry_dedup
 ON bank_statement_entries(statement_reference,entry_date,COALESCE(bank_transaction_reference,''),credit_minor,debit_minor,COALESCE(description,''));
CREATE UNIQUE INDEX IF NOT EXISTS idx_cheque_duplicate_guard
 ON cheques(cheque_number,COALESCE(bank_name,''),amount_minor,received_date);

CREATE TRIGGER trg_deposit_within_cheque_value
BEFORE INSERT ON cheque_deposits BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM cheques WHERE id=NEW.cheque_id) THEN RAISE(ABORT,'unknown cheque') END;
 SELECT CASE WHEN (SELECT COALESCE(SUM(amount_minor),0) FROM cheque_deposits WHERE cheque_id=NEW.cheque_id)+NEW.amount_minor>(SELECT amount_minor FROM cheques WHERE id=NEW.cheque_id) THEN RAISE(ABORT,'deposited amount exceeds cheque face value') END;
 SELECT CASE WHEN (SELECT status FROM cheques WHERE id=NEW.cheque_id) IN('rejected','returned','cancelled','collected') THEN RAISE(ABORT,'cheque cannot be deposited in its current status') END;
END;

CREATE TRIGGER trg_collection_requires_credit_entry
BEFORE INSERT ON collections BEGIN
 SELECT CASE WHEN COALESCE((SELECT credit_minor FROM bank_statement_entries WHERE id=NEW.bank_entry_id),0)<=0 THEN RAISE(ABORT,'collection must reference a credit bank statement entry') END;
 SELECT CASE WHEN COALESCE((SELECT SUM(amount_minor) FROM collections WHERE bank_entry_id=NEW.bank_entry_id),0)+NEW.amount_minor>(SELECT credit_minor FROM bank_statement_entries WHERE id=NEW.bank_entry_id) THEN RAISE(ABORT,'collections exceed bank statement credit') END;
END;

CREATE TRIGGER trg_collection_cheque_must_be_deposited
BEFORE INSERT ON collection_cheques BEGIN
 SELECT CASE WHEN COALESCE((SELECT SUM(amount_minor) FROM cheque_deposits WHERE cheque_id=NEW.cheque_id),0)<COALESCE((SELECT SUM(matched_minor) FROM collection_cheques WHERE cheque_id=NEW.cheque_id),0)+NEW.matched_minor THEN RAISE(ABORT,'matched amount exceeds deposited cheque amount') END;
 SELECT CASE WHEN COALESCE((SELECT SUM(matched_minor) FROM collection_cheques WHERE collection_id=NEW.collection_id),0)+NEW.matched_minor>(SELECT amount_minor FROM collections WHERE id=NEW.collection_id) THEN RAISE(ABORT,'matched amount exceeds collection value') END;
 SELECT CASE WHEN (SELECT status FROM cheques WHERE id=NEW.cheque_id) IN('rejected','returned','cancelled') THEN RAISE(ABORT,'rejected, returned or cancelled cheque cannot be matched') END;
END;

CREATE TRIGGER trg_no_delete_cheques BEFORE DELETE ON cheques BEGIN SELECT RAISE(ABORT,'cheque history cannot be deleted'); END;
CREATE TRIGGER trg_no_delete_deposits BEFORE DELETE ON cheque_deposits BEGIN SELECT RAISE(ABORT,'deposit history cannot be deleted'); END;
CREATE TRIGGER trg_no_update_deposits BEFORE UPDATE ON cheque_deposits BEGIN SELECT RAISE(ABORT,'deposit history is immutable'); END;
CREATE TRIGGER trg_no_update_collection_cheques BEFORE UPDATE ON collection_cheques BEGIN SELECT RAISE(ABORT,'cheque matching history is immutable'); END;
CREATE TRIGGER trg_no_delete_collection_cheques BEFORE DELETE ON collection_cheques BEGIN SELECT RAISE(ABORT,'cheque matching history cannot be deleted'); END;
