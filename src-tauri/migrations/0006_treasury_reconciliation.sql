-- Version 6: opening balance and bank debit reconciliation.
ALTER TABLE treasury_ledger ADD COLUMN statement_entry_id INTEGER REFERENCES bank_statement_entries(id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_v6_ledger_statement_entry
 ON treasury_ledger(statement_entry_id) WHERE statement_entry_id IS NOT NULL;

CREATE TABLE treasury_opening_balance(
 id INTEGER PRIMARY KEY CHECK(id=1),
 amount_minor INTEGER NOT NULL CHECK(amount_minor>0),
 balance_date TEXT NOT NULL,
 statement_reference TEXT NOT NULL,
 created_at TEXT NOT NULL DEFAULT(datetime('now'))
);

CREATE TRIGGER trg_v6_opening_balance_ledger
AFTER INSERT ON treasury_opening_balance
BEGIN
 INSERT INTO treasury_ledger(movement_type,movement_date,amount_minor,direction)
 VALUES('opening_balance',NEW.balance_date,NEW.amount_minor,'in');
END;

CREATE TABLE bank_debit_reconciliations(
 id INTEGER PRIMARY KEY,
 bank_entry_id INTEGER NOT NULL REFERENCES bank_statement_entries(id),
 amount_minor INTEGER NOT NULL CHECK(amount_minor>0),
 reconciliation_type TEXT NOT NULL CHECK(reconciliation_type IN('payment','external')),
 payment_id INTEGER REFERENCES outgoing_payments(id),
 created_at TEXT NOT NULL DEFAULT(datetime('now')),
 CHECK((reconciliation_type='payment' AND payment_id IS NOT NULL) OR
       (reconciliation_type='external' AND payment_id IS NULL))
);

CREATE INDEX idx_v6_debit_reconciliation_entry ON bank_debit_reconciliations(bank_entry_id);
CREATE INDEX idx_v6_debit_reconciliation_payment ON bank_debit_reconciliations(payment_id) WHERE payment_id IS NOT NULL;
CREATE UNIQUE INDEX idx_v6_external_debit_once
 ON bank_debit_reconciliations(bank_entry_id) WHERE reconciliation_type='external';

CREATE TRIGGER trg_v6_reconcile_only_debit
BEFORE INSERT ON bank_debit_reconciliations
BEGIN
 SELECT CASE WHEN COALESCE((SELECT debit_minor FROM bank_statement_entries WHERE id=NEW.bank_entry_id),0)<=0
  THEN RAISE(ABORT,'reconciliation requires a debit bank statement entry') END;
 SELECT CASE WHEN
  (SELECT COALESCE(SUM(amount_minor),0) FROM bank_debit_reconciliations WHERE bank_entry_id=NEW.bank_entry_id)+NEW.amount_minor
  >(SELECT debit_minor FROM bank_statement_entries WHERE id=NEW.bank_entry_id)
  THEN RAISE(ABORT,'reconciliations exceed bank debit amount') END;
 SELECT CASE WHEN NEW.reconciliation_type='external' AND
  (NEW.amount_minor<>(SELECT debit_minor FROM bank_statement_entries WHERE id=NEW.bank_entry_id)
   OR EXISTS(SELECT 1 FROM bank_debit_reconciliations WHERE bank_entry_id=NEW.bank_entry_id))
  THEN RAISE(ABORT,'external debit must reconcile the full unreconciled statement line') END;
 SELECT CASE WHEN NEW.reconciliation_type='payment' AND
  (SELECT status FROM outgoing_payments WHERE id=NEW.payment_id)<>'executed'
  THEN RAISE(ABORT,'only executed payments can be matched to bank debits') END;
 SELECT CASE WHEN NEW.reconciliation_type='payment' AND
  (SELECT COALESCE(SUM(amount_minor),0) FROM bank_debit_reconciliations
   WHERE payment_id=NEW.payment_id AND reconciliation_type='payment')+NEW.amount_minor
  >(SELECT COALESCE(SUM(amount_minor),0) FROM payment_details WHERE payment_id=NEW.payment_id)
  THEN RAISE(ABORT,'bank debit match exceeds executed payment amount') END;
END;

CREATE TRIGGER trg_v6_external_debit_ledger
AFTER INSERT ON bank_debit_reconciliations
WHEN NEW.reconciliation_type='external'
BEGIN
 INSERT INTO treasury_ledger(movement_type,statement_entry_id,movement_date,amount_minor,direction)
 SELECT 'external_debit',NEW.bank_entry_id,e.entry_date,NEW.amount_minor,'out'
 FROM bank_statement_entries e WHERE e.id=NEW.bank_entry_id;
END;

CREATE TRIGGER trg_v6_reconciliation_immutable_update
BEFORE UPDATE ON bank_debit_reconciliations
BEGIN SELECT RAISE(ABORT,'bank debit reconciliation history is immutable'); END;
CREATE TRIGGER trg_v6_reconciliation_immutable_delete
BEFORE DELETE ON bank_debit_reconciliations
BEGIN SELECT RAISE(ABORT,'bank debit reconciliation history cannot be deleted'); END;
CREATE TRIGGER trg_v6_opening_balance_immutable_update
BEFORE UPDATE ON treasury_opening_balance
BEGIN SELECT RAISE(ABORT,'opening balance is immutable'); END;
CREATE TRIGGER trg_v6_opening_balance_immutable_delete
BEFORE DELETE ON treasury_opening_balance
BEGIN SELECT RAISE(ABORT,'opening balance history cannot be deleted'); END;
