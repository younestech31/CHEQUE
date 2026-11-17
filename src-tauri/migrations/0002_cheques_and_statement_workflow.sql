-- Additive migration: preserves all data from schema v1.
PRAGMA foreign_keys = ON;
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
CREATE TRIGGER IF NOT EXISTS trg_deposit_within_cheque_value
BEFORE INSERT ON cheque_deposits BEGIN
 SELECT CASE WHEN (SELECT COALESCE(SUM(amount_minor),0) FROM cheque_deposits WHERE cheque_id=NEW.cheque_id)+NEW.amount_minor>(SELECT amount_minor FROM cheques WHERE id=NEW.cheque_id) THEN RAISE(ABORT,'deposited amount exceeds cheque face value') END;
 SELECT CASE WHEN (SELECT status FROM cheques WHERE id=NEW.cheque_id) IN('rejected','returned','cancelled','collected') THEN RAISE(ABORT,'cheque cannot be deposited in its current status') END;
END;
CREATE TRIGGER IF NOT EXISTS trg_collection_requires_credit_entry
BEFORE INSERT ON collections BEGIN
 SELECT CASE WHEN COALESCE((SELECT credit_minor FROM bank_statement_entries WHERE id=NEW.bank_entry_id),0)<=0 THEN RAISE(ABORT,'collection must reference a credit bank statement entry') END;
END;
CREATE TRIGGER IF NOT EXISTS trg_collection_cheque_must_be_deposited
BEFORE INSERT ON collection_cheques BEGIN
 SELECT CASE WHEN (SELECT COALESCE(SUM(amount_minor),0) FROM cheque_deposits WHERE cheque_id=NEW.cheque_id)<(SELECT COALESCE(SUM(matched_minor),0) FROM collection_cheques WHERE cheque_id=NEW.cheque_id)+NEW.matched_minor THEN RAISE(ABORT,'matched amount exceeds deposited cheque amount') END;
 SELECT CASE WHEN (SELECT status FROM cheques WHERE id=NEW.cheque_id) IN('rejected','returned','cancelled') THEN RAISE(ABORT,'rejected, returned or cancelled cheque cannot be matched') END;
END;
CREATE TRIGGER IF NOT EXISTS trg_no_delete_cheques BEFORE DELETE ON cheques BEGIN SELECT RAISE(ABORT,'cheque history cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_delete_deposits BEFORE DELETE ON cheque_deposits BEGIN SELECT RAISE(ABORT,'deposit history cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_update_deposits BEFORE UPDATE ON cheque_deposits BEGIN SELECT RAISE(ABORT,'deposit history is immutable'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_update_collection_cheques BEFORE UPDATE ON collection_cheques BEGIN SELECT RAISE(ABORT,'cheque matching history is immutable'); END;
CREATE TRIGGER IF NOT EXISTS trg_no_delete_collection_cheques BEFORE DELETE ON collection_cheques BEGIN SELECT RAISE(ABORT,'cheque matching history cannot be deleted'); END;
INSERT OR IGNORE INTO schema_migrations(version) VALUES(2);
