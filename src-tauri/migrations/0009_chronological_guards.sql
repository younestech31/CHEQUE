-- Version 9: enforce chronological consistency for source and execution events.
CREATE TRIGGER trg_v9_deposit_not_before_cheque_receipt
BEFORE INSERT ON cheque_deposits
WHEN NEW.deposit_date < (SELECT received_date FROM cheques WHERE id=NEW.cheque_id)
BEGIN
 SELECT RAISE(ABORT,'cheque deposit date cannot precede cheque receipt date');
END;

CREATE TRIGGER trg_v9_collection_not_before_bank_entry
BEFORE INSERT ON collections
WHEN NEW.collection_date < (SELECT entry_date FROM bank_statement_entries WHERE id=NEW.bank_entry_id)
BEGIN
 SELECT RAISE(ABORT,'collection date cannot precede bank statement entry date');
END;

CREATE TRIGGER trg_v9_payment_execution_not_before_preparation
BEFORE UPDATE OF status ON outgoing_payments
WHEN NEW.status='executed' AND NEW.execution_date < OLD.preparation_date
BEGIN
 SELECT RAISE(ABORT,'payment execution date cannot precede preparation date');
END;
