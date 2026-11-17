-- Version 7: prevent matching a bank debit to a future-dated payment.
DROP TRIGGER IF EXISTS trg_v6_reconcile_only_debit;
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
  (SELECT execution_date FROM outgoing_payments WHERE id=NEW.payment_id) >
  (SELECT entry_date FROM bank_statement_entries WHERE id=NEW.bank_entry_id)
  THEN RAISE(ABORT,'payment execution date is after the bank debit date') END;
 SELECT CASE WHEN NEW.reconciliation_type='payment' AND
  (SELECT COALESCE(SUM(amount_minor),0) FROM bank_debit_reconciliations
   WHERE payment_id=NEW.payment_id AND reconciliation_type='payment')+NEW.amount_minor
  >(SELECT COALESCE(SUM(amount_minor),0) FROM payment_details WHERE payment_id=NEW.payment_id)
  THEN RAISE(ABORT,'bank debit match exceeds executed payment amount') END;
END;
